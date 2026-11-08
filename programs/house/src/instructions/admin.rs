//! The house's settings. Nothing here moves money: the split and any loosening of
//! the funding caps change only after a public timelock, the project token is
//! set once and after a timelock, prices (the admin's alone) only affect future
//! sales, and the keeper's funding is capped.

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::Mint;
use arena::escrow::{transfer_fee_bps, validate_wager_mint};

use crate::{constants::*, error::HouseError, events::*, state::*};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct HouseParams {
    pub keeper: Pubkey,
    pub ops_wallet: Pubkey,
    pub split: Split,
    pub fund_max_bps: u16,
    pub fund_every_secs: u32,
}

// ---- initialize ----------------------------------------------------------------

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + HouseConfig::INIT_SPACE, seeds = [HOUSE_SEED], bump)]
    pub house: Account<'info, HouseConfig>,
    /// CHECK: the treasury PDA: a plain system account, created here with its rent.
    #[account(mut, seeds = [TREASURY_SEED], bump)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the prize vault PDA, likewise.
    #[account(mut, seeds = [PRIZE_SEED], bump)]
    pub prize: UncheckedAccount<'info>,
    /// CHECK: the buyback PDA, likewise.
    #[account(mut, seeds = [BUYBACK_SEED], bump)]
    pub buyback: UncheckedAccount<'info>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ HouseError::Unauthorized)]
    pub program: Program<'info, crate::program::House>,
    /// Only the upgrade authority may bootstrap the house.
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()) @ HouseError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, params: HouseParams) -> Result<()> {
    require!(params.split.ok(), HouseError::InvalidSplit);
    require!(
        params.fund_max_bps <= HARD_MAX_FUND_BPS
            && params.keeper != Pubkey::default()
            && params.ops_wallet != Pubkey::default(),
        HouseError::InvalidParams
    );
    // The three vaults hold SOL as plain system accounts: give each its rent
    // floor now, so a fee smaller than the floor can always land in them.
    let floor = Rent::get()?.minimum_balance(0);
    for vault in [
        &ctx.accounts.treasury,
        &ctx.accounts.prize,
        &ctx.accounts.buyback,
    ] {
        let need = floor.saturating_sub(vault.lamports());
        if need > 0 {
            system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.key(),
                    system_program::Transfer {
                        from: ctx.accounts.admin.to_account_info(),
                        to: vault.to_account_info(),
                    },
                ),
                need,
            )?;
        }
    }
    let h = &mut ctx.accounts.house;
    h.admin = ctx.accounts.admin.key();
    h.pending_admin = Pubkey::default();
    h.keeper = params.keeper;
    h.ops_wallet = params.ops_wallet;
    h.split = params.split;
    h.pending_split = Split::default();
    h.split_after = 0;
    h.token_mint = Pubkey::default();
    h.fund_max_bps = params.fund_max_bps;
    h.fund_every_secs = params.fund_every_secs;
    h.pending_fund_max_bps = 0;
    h.pending_fund_every_secs = 0;
    h.fund_caps_after = 0;
    h.last_fund_at = i64::MIN; // never: the first funding is never too soon
    h.paused = false;
    h.treasury_bump = ctx.bumps.treasury;
    h.prize_bump = ctx.bumps.prize;
    h.buyback_bump = ctx.bumps.buyback;
    h.bump = ctx.bumps.house;
    emit!(HouseChanged { admin: h.admin });
    Ok(())
}

// ---- admin ---------------------------------------------------------------------

#[derive(Accounts)]
pub struct AdminHouse<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [HOUSE_SEED], bump = house.bump, has_one = admin @ HouseError::Unauthorized)]
    pub house: Account<'info, HouseConfig>,
}

pub fn handle_set_roles(
    ctx: Context<AdminHouse>,
    keeper: Pubkey,
    ops_wallet: Pubkey,
) -> Result<()> {
    require!(
        keeper != Pubkey::default() && ops_wallet != Pubkey::default(),
        HouseError::InvalidParams
    );
    let h = &mut ctx.accounts.house;
    h.keeper = keeper;
    h.ops_wallet = ops_wallet;
    emit!(HouseChanged { admin: h.admin });
    Ok(())
}

/// Tighter caps (a smaller share, a longer period) apply at once; looser ones
/// wait the split's timelock (`apply_fund_caps`), so a stolen admin key can't
/// empty the prize vault into cups.
pub fn handle_set_fund_caps(
    ctx: Context<AdminHouse>,
    fund_max_bps: u16,
    fund_every_secs: u32,
) -> Result<()> {
    require!(fund_max_bps <= HARD_MAX_FUND_BPS, HouseError::InvalidParams);
    let now = Clock::get()?.unix_timestamp;
    let h = &mut ctx.accounts.house;
    if fund_max_bps <= h.fund_max_bps && fund_every_secs >= h.fund_every_secs {
        h.fund_max_bps = fund_max_bps;
        h.fund_every_secs = fund_every_secs;
        h.fund_caps_after = 0;
    } else {
        h.pending_fund_max_bps = fund_max_bps;
        h.pending_fund_every_secs = fund_every_secs;
        h.fund_caps_after = now + SPLIT_TIMELOCK_SECS;
    }
    emit!(HouseChanged { admin: h.admin });
    Ok(())
}

/// Anyone, once the timelock on looser caps has passed.
pub fn handle_apply_fund_caps(ctx: Context<ApplySplit>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let h = &mut ctx.accounts.house;
    require!(h.fund_caps_after != 0, HouseError::NothingPending);
    require!(now >= h.fund_caps_after, HouseError::TooEarly);
    h.fund_max_bps = h.pending_fund_max_bps;
    h.fund_every_secs = h.pending_fund_every_secs;
    h.fund_caps_after = 0;
    emit!(HouseChanged { admin: h.admin });
    Ok(())
}

/// A new split, effective once a week has passed (anyone sees it coming).
pub fn handle_propose_split(ctx: Context<AdminHouse>, split: Split) -> Result<()> {
    require!(split.ok(), HouseError::InvalidSplit);
    let now = Clock::get()?.unix_timestamp;
    let h = &mut ctx.accounts.house;
    h.pending_split = split;
    h.split_after = now + SPLIT_TIMELOCK_SECS;
    emit!(SplitProposed {
        split,
        after: h.split_after,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ApplySplit<'info> {
    #[account(mut, seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Account<'info, HouseConfig>,
}

/// Anyone, once the timelock has passed.
pub fn handle_apply_split(ctx: Context<ApplySplit>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let h = &mut ctx.accounts.house;
    require!(h.split_after != 0, HouseError::NothingPending);
    require!(now >= h.split_after, HouseError::TooEarly);
    h.split = h.pending_split;
    h.pending_split = Split::default();
    h.split_after = 0;
    emit!(SplitApplied { split: h.split });
    Ok(())
}

/// A transfer-fee token's fee in force and its scheduled one must both be at
/// most `HARD_MAX_TOKEN_FEE_BPS` (its fee authority, not us, can raise it).
pub fn check_token_fee(mint: &AccountInfo) -> Result<()> {
    if let Some((now, newer)) = transfer_fee_bps(mint)? {
        require!(
            now <= HARD_MAX_TOKEN_FEE_BPS && newer <= HARD_MAX_TOKEN_FEE_BPS,
            HouseError::TransferFeeTooHigh
        );
    }
    Ok(())
}

/// What the project token must be: a mint the escrow can account for, its fee
/// within the house's cap. (StonkFun's have their mint and freeze authorities
/// revoked; what protects the vault from a token named by a stolen key is the
/// timelock: two days for anyone to see it coming.)
fn check_project_token(mint: &InterfaceAccount<Mint>) -> Result<()> {
    validate_wager_mint(&mint.to_account_info(), true)?;
    check_token_fee(&mint.to_account_info())
}

#[derive(Accounts)]
pub struct ProposeTokenMint<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump, has_one = admin @ HouseError::Unauthorized)]
    pub house: Box<Account<'info, HouseConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingToken::INIT_SPACE, seeds = [TOKEN_SEED], bump)]
    pub pending: Box<Account<'info, PendingToken>>,
    /// The project token's mint itself.
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

/// The project token, proposed: anyone applies it once `TOKEN_TIMELOCK_SECS`
/// have passed (a new proposal starts the wait again). Only while the house has
/// none: it is set once.
pub fn handle_propose_token_mint(ctx: Context<ProposeTokenMint>) -> Result<()> {
    require!(
        ctx.accounts.house.token_mint == Pubkey::default(),
        HouseError::AlreadySet
    );
    check_project_token(&ctx.accounts.mint)?;
    let after = Clock::get()?
        .unix_timestamp
        .checked_add(TOKEN_TIMELOCK_SECS)
        .ok_or(HouseError::Overflow)?;
    let mint = ctx.accounts.mint.key();
    let p = &mut ctx.accounts.pending;
    p.mint = mint;
    p.after = after;
    p.bump = ctx.bumps.pending;
    emit!(TokenMintProposed { mint, after });
    Ok(())
}

#[derive(Accounts)]
pub struct ApplyTokenMint<'info> {
    #[account(mut, seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Box<Account<'info, HouseConfig>>,
    #[account(seeds = [TOKEN_SEED], bump = pending.bump)]
    pub pending: Box<Account<'info, PendingToken>>,
    #[account(address = pending.mint @ HouseError::NotTheToken)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
}

/// Anyone, once the timelock has passed: from then on the project token's share
/// of fees is burned, other tokens' buyback share builds up in the buyback vault
/// and its auctions sell for this token. Checked again as it takes effect.
pub fn handle_apply_token_mint(ctx: Context<ApplyTokenMint>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let p = &ctx.accounts.pending;
    require!(
        ctx.accounts.house.token_mint == Pubkey::default(),
        HouseError::AlreadySet
    );
    require!(p.mint != Pubkey::default(), HouseError::NothingPending);
    require!(now >= p.after, HouseError::TooEarly);
    check_project_token(&ctx.accounts.mint)?;
    let mint = p.mint;
    ctx.accounts.house.token_mint = mint;
    emit!(TokenMintApplied { mint });
    Ok(())
}

/// Admin may pause or unpause sales; it never stops anything that pays out.
pub fn handle_set_paused(ctx: Context<AdminHouse>, paused: bool) -> Result<()> {
    ctx.accounts.house.paused = paused;
    Ok(())
}

pub fn handle_propose_admin(ctx: Context<AdminHouse>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.house.pending_admin = new_admin;
    Ok(())
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Account<'info, HouseConfig>,
}

pub fn handle_accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    let h = &mut ctx.accounts.house;
    require!(
        h.pending_admin != Pubkey::default() && h.pending_admin == ctx.accounts.new_admin.key(),
        HouseError::NotPendingAdmin
    );
    h.admin = h.pending_admin;
    h.pending_admin = Pubkey::default();
    emit!(HouseChanged { admin: h.admin });
    Ok(())
}

// ---- tokens: a price table and running totals per mint ------------------------------

#[derive(Accounts)]
#[instruction(mint_key: Pubkey)]
pub struct AddMint<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump, has_one = admin @ HouseError::Unauthorized)]
    pub house: Account<'info, HouseConfig>,
    #[account(init, payer = admin, space = 8 + PriceTable::INIT_SPACE, seeds = [PRICE_SEED, mint_key.as_ref()], bump)]
    pub price_table: Account<'info, PriceTable>,
    #[account(init, payer = admin, space = 8 + Totals::INIT_SPACE, seeds = [TOTALS_SEED, mint_key.as_ref()], bump)]
    pub totals: Account<'info, Totals>,
    /// Absent when `mint_key` is NATIVE_SOL.
    pub mint: Option<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

/// A token the house sells in and keeps totals for (prices start at 0: not for sale).
pub fn handle_add_mint(ctx: Context<AddMint>, mint_key: Pubkey) -> Result<()> {
    let (token_program, decimals) = if mint_key == NATIVE_SOL {
        require!(
            ctx.accounts.mint.is_none(),
            HouseError::TokenAccountsMismatch
        );
        (anchor_lang::system_program::ID, 9u8)
    } else {
        let mint = ctx
            .accounts
            .mint
            .as_ref()
            .ok_or(HouseError::TokenAccountsMismatch)?;
        require_keys_eq!(mint.key(), mint_key, HouseError::TokenAccountsMismatch);
        // the arena's vetting: no permanent delegate (it could take the vaults'
        // balances), transfer hook, pausable or default-frozen mint
        validate_wager_mint(&mint.to_account_info(), true)?;
        (*mint.to_account_info().owner, mint.decimals)
    };
    let p = &mut ctx.accounts.price_table;
    p.mint = mint_key;
    p.token_program = token_program;
    p.decimals = decimals;
    p.prices = [[0; MAX_LEVEL as usize]; DEPARTMENTS];
    p.enabled = false;
    p.updated_at = Clock::get()?.unix_timestamp;
    p.bump = ctx.bumps.price_table;
    let t = &mut ctx.accounts.totals;
    t.mint = mint_key;
    t.bump = ctx.bumps.totals;
    Ok(())
}

#[derive(Accounts)]
pub struct SetPrices<'info> {
    /// The admin alone: levels are never refunded, so a price only the admin can
    /// set is a price a leaked server key can't drop to nothing.
    pub authority: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Account<'info, HouseConfig>,
    #[account(mut, seeds = [PRICE_SEED, price_table.mint.as_ref()], bump = price_table.bump)]
    pub price_table: Account<'info, PriceTable>,
}

/// New prices for future sales. A buyer's `max_price` protects them from a change
/// between quote and purchase; levels can't be resold, so a stale price is a
/// mispricing, never an exploit.
pub fn handle_set_prices(
    ctx: Context<SetPrices>,
    prices: [[u64; MAX_LEVEL as usize]; DEPARTMENTS],
    enabled: bool,
) -> Result<()> {
    let who = ctx.accounts.authority.key();
    require!(who == ctx.accounts.house.admin, HouseError::Unauthorized);
    let p = &mut ctx.accounts.price_table;
    p.prices = prices;
    p.enabled = enabled;
    p.updated_at = Clock::get()?.unix_timestamp;
    emit!(PricesSet {
        mint: p.mint,
        enabled
    });
    Ok(())
}

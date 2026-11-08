//! The house's money: `sweep` splits what the treasury took in (every fee and
//! sale) by the split; `fund_cup` lets the keeper fund a cup from the prize
//! vault, within the caps, by creating it in arena with the vault as organizer
//! and sponsor (the vault gets back whatever isn't won).

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::{
    self, BurnChecked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{constants::*, error::HouseError, events::*, state::*};

pub(crate) fn bps_of(amount: u64, bps: u16) -> Result<u64> {
    u64::try_from((amount as u128) * (bps as u128) / (BPS_DENOMINATOR as u128))
        .map_err(|_| error!(HouseError::Overflow))
}

/// The lamports a system-owned vault holds above its rent floor.
pub(crate) fn spendable(vault: &AccountInfo) -> Result<u64> {
    Ok(vault
        .lamports()
        .saturating_sub(Rent::get()?.minimum_balance(vault.data_len())))
}

// ---- sweep -----------------------------------------------------------------------

#[derive(Accounts)]
#[instruction(mint_key: Pubkey)]
pub struct Sweep<'info> {
    #[account(seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Box<Account<'info, HouseConfig>>,
    /// CHECK: the treasury PDA (a system account); signs for its own transfers.
    #[account(mut, seeds = [TREASURY_SEED], bump = house.treasury_bump)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the prize vault PDA.
    #[account(mut, seeds = [PRIZE_SEED], bump = house.prize_bump)]
    pub prize: UncheckedAccount<'info>,
    /// CHECK: the buyback PDA.
    #[account(mut, seeds = [BUYBACK_SEED], bump = house.buyback_bump)]
    pub buyback: UncheckedAccount<'info>,
    /// CHECK: the studio's wallet.
    #[account(mut, address = house.ops_wallet)]
    pub ops_wallet: UncheckedAccount<'info>,
    #[account(mut, seeds = [TOTALS_SEED, mint_key.as_ref()], bump = totals.bump)]
    pub totals: Box<Account<'info, Totals>>,
    /// Token sweeps: the mint (writable: the project token's share is burned) and
    /// the four parties' token accounts of it (the buyback's only when it gets a
    /// share that isn't burned).
    #[account(mut)]
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub treasury_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub ops_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub prize_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub buyback_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

/// Anyone: split what the treasury holds in one token (or SOL) by the split.
/// No project token yet: the buyback share goes to prizes. The project token
/// itself: its buyback share is burned at once (a burn is not a transfer, so it
/// pays no transfer fee). Other tokens' buyback share builds up for the auction.
pub fn handle_sweep<'info>(ctx: Context<'info, Sweep<'info>>, mint_key: Pubkey) -> Result<()> {
    let a = &ctx.accounts;
    let h = &a.house;
    let has_token = h.token_mint != Pubkey::default();
    let burn = has_token && mint_key == h.token_mint;
    let bump = [h.treasury_bump];
    let seeds: &[&[u8]] = &[TREASURY_SEED, &bump];

    let amount = if mint_key == NATIVE_SOL {
        require!(
            a.mint.is_none() && a.treasury_tokens.is_none(),
            HouseError::TokenAccountsMismatch
        );
        spendable(&a.treasury.to_account_info())?
    } else {
        let mint = a.mint.as_ref().ok_or(HouseError::TokenAccountsMismatch)?;
        let t = a
            .treasury_tokens
            .as_ref()
            .ok_or(HouseError::TokenAccountsMismatch)?;
        require!(
            mint.key() == mint_key && t.mint == mint_key && t.owner == a.treasury.key(),
            HouseError::TokenAccountsMismatch
        );
        t.amount
    };
    if amount == 0 {
        return Ok(());
    }
    let to_ops = bps_of(amount, h.split.ops_bps)?;
    let buyback = bps_of(amount, h.split.buyback_bps)?;
    let mut to_prize = amount - to_ops - buyback; // (the prize share takes the rounding dust)
    let (mut to_buyback, mut burned) = (0u64, 0u64);
    if !has_token {
        to_prize += buyback;
    } else if burn {
        burned = buyback;
    } else {
        to_buyback = buyback;
    }

    if mint_key == NATIVE_SOL {
        for (to, amount) in [
            (&a.ops_wallet, to_ops),
            (&a.prize, to_prize),
            (&a.buyback, to_buyback),
        ] {
            if amount > 0 {
                system_program::transfer(
                    CpiContext::new_with_signer(
                        a.system_program.key(),
                        system_program::Transfer {
                            from: a.treasury.to_account_info(),
                            to: to.to_account_info(),
                        },
                        &[seeds],
                    ),
                    amount,
                )?;
            }
        }
    } else {
        let mint = a.mint.as_ref().unwrap();
        let from = a.treasury_tokens.as_ref().unwrap();
        let program = a
            .token_program
            .as_ref()
            .ok_or(HouseError::TokenAccountsMismatch)?;
        require_keys_eq!(
            *mint.to_account_info().owner,
            program.key(),
            HouseError::TokenAccountsMismatch
        );
        let send = |to: &Option<Box<InterfaceAccount<'info, TokenAccount>>>,
                    owner: Pubkey,
                    amount: u64|
         -> Result<()> {
            if amount == 0 {
                return Ok(());
            }
            let to = to.as_ref().ok_or(HouseError::TokenAccountsMismatch)?;
            require!(
                to.mint == mint_key && to.owner == owner,
                HouseError::TokenAccountsMismatch
            );
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    program.key(),
                    TransferChecked {
                        from: from.to_account_info(),
                        mint: mint.to_account_info(),
                        to: to.to_account_info(),
                        authority: a.treasury.to_account_info(),
                    },
                    &[seeds],
                ),
                amount,
                mint.decimals,
            )
        };
        send(&a.ops_tokens, h.ops_wallet, to_ops)?;
        send(&a.prize_tokens, a.prize.key(), to_prize)?;
        send(&a.buyback_tokens, a.buyback.key(), to_buyback)?;
        if burned > 0 {
            token_interface::burn_checked(
                CpiContext::new_with_signer(
                    program.key(),
                    BurnChecked {
                        mint: mint.to_account_info(),
                        from: from.to_account_info(),
                        authority: a.treasury.to_account_info(),
                    },
                    &[seeds],
                ),
                burned,
                mint.decimals,
            )?;
        }
    }

    let t = &mut ctx.accounts.totals;
    t.swept_ops = t.swept_ops.saturating_add(to_ops);
    t.swept_prize = t.swept_prize.saturating_add(to_prize);
    t.swept_buyback = t.swept_buyback.saturating_add(to_buyback);
    t.burned = t.burned.saturating_add(burned);
    emit!(Swept {
        mint: mint_key,
        to_ops,
        to_prize,
        to_buyback,
        burned,
    });
    Ok(())
}

// ---- fund a cup from the prize vault --------------------------------------------------

#[derive(Accounts)]
pub struct FundCup<'info> {
    /// Pays the cup's rent.
    #[account(mut)]
    pub keeper: Signer<'info>,
    #[account(mut, seeds = [HOUSE_SEED], bump = house.bump,
        constraint = house.keeper == keeper.key() @ HouseError::Unauthorized)]
    pub house: Box<Account<'info, HouseConfig>>,
    /// CHECK: the prize vault PDA: the cup's organizer and sponsor (signs the CPI).
    #[account(mut, seeds = [PRIZE_SEED], bump = house.prize_bump)]
    pub prize: UncheckedAccount<'info>,
    #[account(mut, seeds = [TOTALS_SEED, totals.mint.as_ref()], bump = totals.bump)]
    pub totals: Box<Account<'info, Totals>>,
    /// CHECK: arena's config (arena checks it).
    #[account(mut)]
    pub arena_config: UncheckedAccount<'info>,
    /// CHECK: the cup's ruleset (arena checks it).
    pub ruleset: UncheckedAccount<'info>,
    /// CHECK: arena's allowed-mint account for the cup's token (arena checks it).
    pub allowed_mint: UncheckedAccount<'info>,
    /// CHECK: the new cup, created by arena.
    #[account(mut)]
    pub cup: UncheckedAccount<'info>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    /// CHECK: the cup's escrow token account (created earlier in the transaction; arena checks it).
    #[account(mut)]
    pub escrow_tokens: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub prize_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub arena_program: Program<'info, arena::program::Arena>,
    pub system_program: Program<'info, System>,
}

/// The keeper funds a cup (the weekly time trial) from the prize vault: at most
/// `fund_max_bps` of the vault's balance in that token, at most once per
/// `fund_every_secs`. The cup is arena's, with the vault as organizer and
/// sponsor, so what isn't won comes back to the vault.
pub fn handle_fund_cup(ctx: Context<FundCup>, id: u64, params: arena::CupParams) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let a = &ctx.accounts;
    let h = &a.house;
    require!(!h.paused, HouseError::Paused);
    // a free board cup only (the weekly time trial): no entries to rank whoever
    // enters, so a leaked keeper can't shape a cup only an accomplice can win
    require!(
        params.kind == arena::CupKind::Board && params.entry_fee == 0 && params.max_entrants == 0,
        HouseError::NotABoardCup
    );
    require!(
        now.saturating_sub(h.last_fund_at) >= h.fund_every_secs as i64,
        HouseError::FundTooSoon
    );
    let mint_key = a.mint.as_ref().map(|m| m.key()).unwrap_or(NATIVE_SOL);
    require_keys_eq!(a.totals.mint, mint_key, HouseError::TokenAccountsMismatch);
    let balance = match &a.prize_tokens {
        None => spendable(&a.prize.to_account_info())?,
        Some(t) => {
            require!(
                t.owner == a.prize.key() && t.mint == mint_key,
                HouseError::TokenAccountsMismatch
            );
            t.amount
        }
    };
    require!(
        params.guarantee <= bps_of(balance, h.fund_max_bps)?,
        HouseError::FundTooLarge
    );

    let bump = [h.prize_bump];
    let seeds: &[&[u8]] = &[PRIZE_SEED, &bump];
    let guarantee = params.guarantee;
    arena::cpi::create_cup(
        CpiContext::new_with_signer(
            a.arena_program.key(),
            arena::cpi::accounts::CreateCup {
                organizer: a.prize.to_account_info(),
                payer: a.keeper.to_account_info(),
                config: a.arena_config.to_account_info(),
                ruleset: a.ruleset.to_account_info(),
                allowed_mint: a.allowed_mint.to_account_info(),
                cup: a.cup.to_account_info(),
                host: None,
                holder_tokens: None,
                mint: a.mint.as_ref().map(|m| m.to_account_info()),
                escrow_tokens: a.escrow_tokens.as_ref().map(|e| e.to_account_info()),
                organizer_tokens: a.prize_tokens.as_ref().map(|t| t.to_account_info()),
                token_program: a.token_program.as_ref().map(|t| t.to_account_info()),
                system_program: a.system_program.to_account_info(),
            },
            &[seeds],
        ),
        id,
        params,
    )?;

    let cup = a.cup.key();
    ctx.accounts.house.last_fund_at = now;
    let t = &mut ctx.accounts.totals;
    t.funded = t.funded.saturating_add(guarantee);
    emit!(CupFunded {
        cup,
        mint: mint_key,
        guarantee,
    });
    Ok(())
}

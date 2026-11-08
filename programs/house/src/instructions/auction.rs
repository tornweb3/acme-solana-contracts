//! The buyback auction: what the buyback vault holds in a token (or SOL) - the
//! buyback share `sweep` sets aside once the project token exists - sold a lot
//! at a time for the project token, which the buyer burns from their own wallet
//! in the same instruction. A Dutch auction: the lot's price in the project
//! token falls evenly from a start price to a floor, so the first buyer to find
//! it worth it fills it; no oracle and no swap code. The admin sets each coin's
//! lowest rate; the keeper starts each auction within bounds (a quarter of the
//! vault at most, a day apart at least, its floor no lower than the admin's rate
//! nor, for a week after a sale, half that sale's rate); anyone buys.

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::{
    self, BurnChecked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::*,
    error::HouseError,
    events::*,
    instructions::money::{bps_of, spendable},
    state::*,
};

/// Is `price` for `lot` at least `bps` of the rate `ref_price` per `ref_lot`?
fn rate_at_least(price: u64, lot: u64, ref_price: u64, ref_lot: u64, bps: u16) -> Result<bool> {
    let wide = |a: u64, b: u64, c: u64| (a as u128).checked_mul(b as u128)?.checked_mul(c as u128);
    match (
        wide(price, ref_lot, BPS_DENOMINATOR),
        wide(ref_price, lot, bps as u64),
    ) {
        (Some(have), Some(need)) => Ok(have >= need),
        _ => err!(HouseError::Overflow),
    }
}

// ---- the admin's lowest rate -------------------------------------------------------

#[derive(Accounts)]
#[instruction(lot_mint: Pubkey)]
pub struct SetAuctionFloor<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump, has_one = admin @ HouseError::Unauthorized)]
    pub house: Box<Account<'info, HouseConfig>>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + Auction::INIT_SPACE,
        seeds = [AUCTION_SEED, lot_mint.as_ref()],
        bump
    )]
    pub auction: Box<Account<'info, Auction>>,
    pub system_program: Program<'info, System>,
}

/// The admin: the lowest rate an auction of `lot_mint` (`NATIVE_SOL` for SOL)
/// may sell at - `min_price` of the project token for `min_lot` of it. No
/// auction of a coin starts before its rate is set; a running one keeps its
/// floor.
pub fn handle_set_auction_floor(
    ctx: Context<SetAuctionFloor>,
    lot_mint: Pubkey,
    min_lot: u64,
    min_price: u64,
) -> Result<()> {
    // (the project token itself is never sold: its buyback share is burned; before it
    // is set, `token_mint` is the default key, which is also SOL's "mint")
    let token = ctx.accounts.house.token_mint;
    require!(
        min_lot > 0 && min_price > 0 && (token == Pubkey::default() || lot_mint != token),
        HouseError::InvalidParams
    );
    let bump = ctx.bumps.auction;
    let au = &mut ctx.accounts.auction;
    au.lot_mint = lot_mint;
    au.min_lot = min_lot;
    au.min_price = min_price;
    au.bump = bump;
    emit!(AuctionFloorSet {
        lot_mint,
        min_lot,
        min_price
    });
    Ok(())
}

// ---- start --------------------------------------------------------------------

#[derive(Accounts)]
#[instruction(lot_mint: Pubkey)]
pub struct StartAuction<'info> {
    pub keeper: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump,
        constraint = house.keeper == keeper.key() @ HouseError::Unauthorized)]
    pub house: Box<Account<'info, HouseConfig>>,
    /// Made with the admin's lowest rate (`set_auction_floor`): none, no auction.
    #[account(mut, seeds = [AUCTION_SEED, lot_mint.as_ref()], bump = auction.bump)]
    pub auction: Box<Account<'info, Auction>>,
    /// CHECK: the buyback vault PDA (SOL lots: its lamports).
    #[account(seeds = [BUYBACK_SEED], bump = house.buyback_bump)]
    pub buyback: UncheckedAccount<'info>,
    /// Token lots: the buyback vault's account of `lot_mint`.
    pub buyback_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
}

/// The keeper puts `lot` of `lot_mint` (`NATIVE_SOL` for SOL) up for the project
/// token: priced `start_price` for the whole lot now, falling evenly to
/// `floor_price` over `decline_secs`, held there until `run_secs` from now.
pub fn handle_start_auction(
    ctx: Context<StartAuction>,
    lot_mint: Pubkey,
    lot: u64,
    start_price: u64,
    floor_price: u64,
    decline_secs: u32,
    run_secs: u32,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let a = &ctx.accounts;
    let h = &a.house;
    require!(!h.paused, HouseError::Paused);
    require!(h.token_mint != Pubkey::default(), HouseError::NoToken);
    // (the project token itself is never sold: its buyback share is burned)
    require!(lot_mint != h.token_mint, HouseError::InvalidParams);
    require!(
        lot > 0
            && floor_price > 0
            && start_price >= floor_price
            && (MIN_DECLINE_SECS..=MAX_DECLINE_SECS).contains(&decline_secs)
            && run_secs >= decline_secs
            && run_secs <= MAX_AUCTION_SECS,
        HouseError::InvalidParams
    );
    let au = &a.auction;
    require!(au.min_lot > 0, HouseError::InvalidParams);
    require!(!au.running(now), HouseError::AuctionRunning);
    require!(
        now.saturating_sub(au.start_at) >= AUCTION_EVERY_SECS,
        HouseError::AuctionTooSoon
    );
    // what the vault holds in that token now: the lot is at most a quarter of it
    let balance = if lot_mint == NATIVE_SOL {
        require!(
            a.buyback_tokens.is_none(),
            HouseError::TokenAccountsMismatch
        );
        spendable(&a.buyback.to_account_info())?
    } else {
        let t = a
            .buyback_tokens
            .as_ref()
            .ok_or(HouseError::TokenAccountsMismatch)?;
        require!(
            t.mint == lot_mint && t.owner == a.buyback.key(),
            HouseError::TokenAccountsMismatch
        );
        t.amount
    };
    require!(
        lot <= bps_of(balance, AUCTION_MAX_LOT_BPS)?,
        HouseError::LotTooLarge
    );
    // the floor's rate (the price per unit of lot): never below the admin's, and
    // for a week after a sale never below half of that sale's
    require!(
        rate_at_least(
            floor_price,
            lot,
            au.min_price,
            au.min_lot,
            BPS_DENOMINATOR as u16
        )?,
        HouseError::FloorTooLow
    );
    if au.last_lot > 0 && now.saturating_sub(au.last_sold_at) < ANCHOR_SECS {
        require!(
            rate_at_least(
                floor_price,
                lot,
                au.last_price,
                au.last_lot,
                FLOOR_ANCHOR_BPS
            )?,
            HouseError::FloorTooLow
        );
    }

    let end_at = now
        .checked_add(run_secs as i64)
        .ok_or(HouseError::Overflow)?;
    let au = &mut ctx.accounts.auction;
    au.lot = lot;
    au.start_price = start_price;
    au.floor_price = floor_price;
    au.start_at = now;
    au.decline_secs = decline_secs;
    au.end_at = end_at;
    emit!(AuctionStarted {
        lot_mint,
        lot,
        start_price,
        floor_price,
        start_at: now,
        decline_secs,
        end_at,
    });
    Ok(())
}

// ---- buy ------------------------------------------------------------------------

#[derive(Accounts)]
pub struct BuyLot<'info> {
    /// Burns the price from their own account; receives the lot (SOL: here).
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Box<Account<'info, HouseConfig>>,
    #[account(mut, seeds = [AUCTION_SEED, auction.lot_mint.as_ref()], bump = auction.bump)]
    pub auction: Box<Account<'info, Auction>>,
    /// The project token's running totals (what was burned).
    #[account(mut, seeds = [TOTALS_SEED, house.token_mint.as_ref()], bump = token_totals.bump)]
    pub token_totals: Box<Account<'info, Totals>>,
    /// The project token's mint: the burn takes its supply down.
    #[account(mut, address = house.token_mint @ HouseError::NotTheToken)]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,
    /// The buyer's account of the project token, burned from.
    #[account(mut)]
    pub buyer_tokens: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    /// CHECK: the buyback vault PDA: pays the lot (SOL), or signs for its token account.
    #[account(mut, seeds = [BUYBACK_SEED], bump = house.buyback_bump)]
    pub buyback: UncheckedAccount<'info>,
    /// Token lots: the lot's mint, the vault's account of it, the buyer's
    /// (owned by the buyer) and their token program.
    pub lot_mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub buyback_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub buyer_lot_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub lot_token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

/// Anyone: the running auction's lot, at its price now - at most `max_price` of
/// the project token, for a lot of at least `min_lot` (a quote's guards) -
/// burned from the buyer's own account; the lot paid to them from the vault.
pub fn handle_buy_lot(ctx: Context<BuyLot>, max_price: u64, min_lot: u64) -> Result<()> {
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    let a = &ctx.accounts;
    require!(!a.house.paused, HouseError::Paused);
    let au = &a.auction;
    require!(au.running(now), HouseError::NoAuction);
    let price = au.price_at(now);
    require!(price <= max_price, HouseError::PriceTooHigh);
    require!(au.lot >= min_lot, HouseError::LotTooSmall);
    let (lot, lot_mint) = (au.lot, au.lot_mint);

    // the price: burned from the buyer's own account of the project token (a
    // burn is not a transfer, so it pays no transfer fee)
    let t = &a.buyer_tokens;
    require!(
        t.mint == a.house.token_mint
            && t.owner == a.buyer.key()
            && *a.token_mint.to_account_info().owner == a.token_program.key(),
        HouseError::TokenAccountsMismatch
    );
    token_interface::burn_checked(
        CpiContext::new(
            a.token_program.key(),
            BurnChecked {
                mint: a.token_mint.to_account_info(),
                from: t.to_account_info(),
                authority: a.buyer.to_account_info(),
            },
        ),
        price,
        a.token_mint.decimals,
    )?;

    // the lot: from the vault, signed by it
    let bump = [a.house.buyback_bump];
    let seeds: &[&[u8]] = &[BUYBACK_SEED, &bump];
    if lot_mint == NATIVE_SOL {
        require!(
            a.lot_mint.is_none() && a.buyback_tokens.is_none() && a.buyer_lot_tokens.is_none(),
            HouseError::TokenAccountsMismatch
        );
        require!(
            lot <= spendable(&a.buyback.to_account_info())?,
            HouseError::VaultShort
        );
        system_program::transfer(
            CpiContext::new_with_signer(
                a.system_program.key(),
                system_program::Transfer {
                    from: a.buyback.to_account_info(),
                    to: a.buyer.to_account_info(),
                },
                &[seeds],
            ),
            lot,
        )?;
    } else {
        let (Some(mint), Some(from), Some(to), Some(program)) = (
            &a.lot_mint,
            &a.buyback_tokens,
            &a.buyer_lot_tokens,
            &a.lot_token_program,
        ) else {
            return err!(HouseError::TokenAccountsMismatch);
        };
        require!(
            mint.key() == lot_mint
                && *mint.to_account_info().owner == program.key()
                && from.mint == lot_mint
                && from.owner == a.buyback.key()
                && to.mint == lot_mint
                && to.owner == a.buyer.key(),
            HouseError::TokenAccountsMismatch
        );
        require!(lot <= from.amount, HouseError::VaultShort);
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                program.key(),
                TransferChecked {
                    from: from.to_account_info(),
                    mint: mint.to_account_info(),
                    to: to.to_account_info(),
                    authority: a.buyback.to_account_info(),
                },
                &[seeds],
            ),
            lot,
            mint.decimals,
        )?;
    }

    let buyer = a.buyer.key();
    let au = &mut ctx.accounts.auction;
    au.lot = 0;
    au.last_lot = lot;
    au.last_price = price;
    au.last_sold_at = now;
    au.sold = au.sold.saturating_add(1);
    au.paid_out = au.paid_out.saturating_add(lot);
    au.burned = au.burned.saturating_add(price);
    let totals = &mut ctx.accounts.token_totals;
    totals.burned = totals.burned.saturating_add(price);
    emit!(LotBought {
        lot_mint,
        lot,
        price,
        buyer,
        slot: clock.slot,
    });
    Ok(())
}

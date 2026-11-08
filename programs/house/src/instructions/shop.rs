//! The upgrade shop: six departments, five levels each, bought one level at a
//! time in SOL or a token with a price table, paid into the treasury (so the
//! split applies to sales too). Levels belong to the wallet's account; they are
//! never tokens, and there is no grant, sale, refund or transfer.

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{
    constants::*, error::HouseError, events::*, instructions::admin::check_token_fee, state::*,
};

#[derive(Accounts)]
pub struct BuyUpgrade<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// Pays the upgrades account's rent on a first purchase (the race server's
    /// sponsor, or the owner).
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [HOUSE_SEED], bump = house.bump)]
    pub house: Box<Account<'info, HouseConfig>>,
    #[account(seeds = [PRICE_SEED, price_table.mint.as_ref()], bump = price_table.bump)]
    pub price_table: Box<Account<'info, PriceTable>>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + Upgrades::INIT_SPACE,
        seeds = [UPGRADES_SEED, owner.key().as_ref()],
        bump
    )]
    pub upgrades: Box<Account<'info, Upgrades>>,
    /// CHECK: the treasury PDA: SOL payments land here.
    #[account(mut, seeds = [TREASURY_SEED], bump = house.treasury_bump)]
    pub treasury: UncheckedAccount<'info>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub owner_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub treasury_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

/// Buys `level` of `dept` (0..6 in `src/game/upgrades.js` order) at the table's
/// price, at most `max_price`. Only the next level: a replayed or duplicated
/// transaction fails instead of charging twice.
pub fn handle_buy_upgrade(
    ctx: Context<BuyUpgrade>,
    dept: u8,
    level: u8,
    max_price: u64,
) -> Result<()> {
    let a = &ctx.accounts;
    require!(!a.house.paused, HouseError::Paused);
    let table = &a.price_table;
    require!(table.enabled, HouseError::NotForSale);
    require!((dept as usize) < DEPARTMENTS, HouseError::InvalidParams);
    let up = &a.upgrades;
    require!(
        up.version == 0 || up.version == UPGRADES_VERSION,
        HouseError::InvalidParams
    );
    let current = up.levels[dept as usize];
    require!(
        level == current + 1 && level <= MAX_LEVEL,
        HouseError::WrongLevel
    );
    let price = table.prices[dept as usize][level as usize - 1];
    require!(price > 0, HouseError::NotForSale);
    require!(price <= max_price, HouseError::PriceTooHigh);

    if table.mint == NATIVE_SOL {
        require!(
            a.mint.is_none() && a.owner_tokens.is_none(),
            HouseError::TokenAccountsMismatch
        );
        system_program::transfer(
            CpiContext::new(
                a.system_program.key(),
                system_program::Transfer {
                    from: a.owner.to_account_info(),
                    to: a.treasury.to_account_info(),
                },
            ),
            price,
        )?;
    } else {
        let (Some(mint), Some(from), Some(to), Some(program)) = (
            &a.mint,
            &a.owner_tokens,
            &a.treasury_tokens,
            &a.token_program,
        ) else {
            return err!(HouseError::TokenAccountsMismatch);
        };
        require!(
            mint.key() == table.mint
                && program.key() == table.token_program
                && from.mint == table.mint
                && from.owner == a.owner.key()
                && to.mint == table.mint
                && to.owner == a.treasury.key(),
            HouseError::TokenAccountsMismatch
        );
        // (a transfer-fee token: the treasury nets the price less the fee, and
        // sells in it only while that fee is within the house's cap)
        check_token_fee(&mint.to_account_info())?;
        token_interface::transfer_checked(
            CpiContext::new(
                program.key(),
                TransferChecked {
                    from: from.to_account_info(),
                    mint: mint.to_account_info(),
                    to: to.to_account_info(),
                    authority: a.owner.to_account_info(),
                },
            ),
            price,
            mint.decimals,
        )?;
    }

    let owner = a.owner.key();
    let mint = table.mint;
    let bump = ctx.bumps.upgrades;
    let u = &mut ctx.accounts.upgrades;
    if u.version == 0 {
        u.version = UPGRADES_VERSION;
        u.owner = owner;
        u.bump = bump;
    }
    u.levels[dept as usize] = level;
    u.updated_at = Clock::get()?.unix_timestamp;
    emit!(UpgradeBought {
        owner,
        dept,
        level,
        mint,
        price,
        levels: u.levels,
        slot: Clock::get()?.slot,
    });
    Ok(())
}

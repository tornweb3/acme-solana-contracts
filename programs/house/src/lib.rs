//! ACME house: the house's own money and the shop. Every fee arena computes and
//! every upgrade sale lands in the treasury PDA; `sweep` splits it (studio /
//! prize vault / the project token's buyback, a split that changes only after a
//! public timelock); the keeper funds cups from the prize vault within caps; the
//! shop sells upgrade levels; the buyback vault is sold in Dutch auctions for
//! the project token, which their buyers burn. Player money never passes
//! through here: it stays in arena's escrows.
//!
//! Design: `docs/solana-plan.md` (in the game repo).

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw");

#[program]
pub mod house {
    use super::*;

    // ---- admin ----
    pub fn initialize(ctx: Context<Initialize>, params: HouseParams) -> Result<()> {
        instructions::admin::handle_initialize(ctx, params)
    }
    pub fn set_roles(ctx: Context<AdminHouse>, keeper: Pubkey, ops_wallet: Pubkey) -> Result<()> {
        instructions::admin::handle_set_roles(ctx, keeper, ops_wallet)
    }
    pub fn set_fund_caps(
        ctx: Context<AdminHouse>,
        fund_max_bps: u16,
        fund_every_secs: u32,
    ) -> Result<()> {
        instructions::admin::handle_set_fund_caps(ctx, fund_max_bps, fund_every_secs)
    }
    pub fn apply_fund_caps(ctx: Context<ApplySplit>) -> Result<()> {
        instructions::admin::handle_apply_fund_caps(ctx)
    }
    pub fn propose_split(ctx: Context<AdminHouse>, split: Split) -> Result<()> {
        instructions::admin::handle_propose_split(ctx, split)
    }
    pub fn apply_split(ctx: Context<ApplySplit>) -> Result<()> {
        instructions::admin::handle_apply_split(ctx)
    }
    pub fn propose_token_mint(ctx: Context<ProposeTokenMint>) -> Result<()> {
        instructions::admin::handle_propose_token_mint(ctx)
    }
    pub fn apply_token_mint(ctx: Context<ApplyTokenMint>) -> Result<()> {
        instructions::admin::handle_apply_token_mint(ctx)
    }
    pub fn set_paused(ctx: Context<AdminHouse>, paused: bool) -> Result<()> {
        instructions::admin::handle_set_paused(ctx, paused)
    }
    pub fn propose_admin(ctx: Context<AdminHouse>, new_admin: Pubkey) -> Result<()> {
        instructions::admin::handle_propose_admin(ctx, new_admin)
    }
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::admin::handle_accept_admin(ctx)
    }
    pub fn add_mint(ctx: Context<AddMint>, mint_key: Pubkey) -> Result<()> {
        instructions::admin::handle_add_mint(ctx, mint_key)
    }
    pub fn set_prices(
        ctx: Context<SetPrices>,
        prices: [[u64; MAX_LEVEL as usize]; DEPARTMENTS],
        enabled: bool,
    ) -> Result<()> {
        instructions::admin::handle_set_prices(ctx, prices, enabled)
    }

    // ---- money ----
    pub fn sweep<'info>(ctx: Context<'info, Sweep<'info>>, mint_key: Pubkey) -> Result<()> {
        instructions::money::handle_sweep(ctx, mint_key)
    }
    pub fn fund_cup(ctx: Context<FundCup>, id: u64, params: arena::CupParams) -> Result<()> {
        instructions::money::handle_fund_cup(ctx, id, params)
    }

    // ---- the buyback auction ----
    pub fn set_auction_floor(
        ctx: Context<SetAuctionFloor>,
        lot_mint: Pubkey,
        min_lot: u64,
        min_price: u64,
    ) -> Result<()> {
        instructions::auction::handle_set_auction_floor(ctx, lot_mint, min_lot, min_price)
    }
    pub fn start_auction(
        ctx: Context<StartAuction>,
        lot_mint: Pubkey,
        lot: u64,
        start_price: u64,
        floor_price: u64,
        decline_secs: u32,
        run_secs: u32,
    ) -> Result<()> {
        instructions::auction::handle_start_auction(
            ctx,
            lot_mint,
            lot,
            start_price,
            floor_price,
            decline_secs,
            run_secs,
        )
    }
    pub fn buy_lot(ctx: Context<BuyLot>, max_price: u64, min_lot: u64) -> Result<()> {
        instructions::auction::handle_buy_lot(ctx, max_price, min_lot)
    }

    // ---- shop ----
    pub fn buy_upgrade(
        ctx: Context<BuyUpgrade>,
        dept: u8,
        level: u8,
        max_price: u64,
    ) -> Result<()> {
        instructions::shop::handle_buy_upgrade(ctx, dept, level, max_price)
    }
}

use anchor_lang::prelude::*;

use crate::state::Split;

#[event]
pub struct HouseChanged {
    pub admin: Pubkey,
}

#[event]
pub struct SplitProposed {
    pub split: Split,
    pub after: i64,
}

#[event]
pub struct SplitApplied {
    pub split: Split,
}

#[event]
pub struct PricesSet {
    pub mint: Pubkey,
    pub enabled: bool,
}

#[event]
pub struct Swept {
    pub mint: Pubkey,
    pub to_ops: u64,
    pub to_prize: u64,
    pub to_buyback: u64,
    /// The project token's buyback share, burned instead of kept.
    pub burned: u64,
}

#[event]
pub struct CupFunded {
    pub cup: Pubkey,
    pub mint: Pubkey,
    pub guarantee: u64,
}

#[event]
pub struct TokenMintProposed {
    pub mint: Pubkey,
    pub after: i64,
}

#[event]
pub struct TokenMintApplied {
    pub mint: Pubkey,
}

#[event]
pub struct AuctionFloorSet {
    pub lot_mint: Pubkey,
    pub min_lot: u64,
    pub min_price: u64,
}

#[event]
pub struct AuctionStarted {
    pub lot_mint: Pubkey,
    pub lot: u64,
    pub start_price: u64,
    pub floor_price: u64,
    pub start_at: i64,
    pub decline_secs: u32,
    pub end_at: i64,
}

#[event]
pub struct LotBought {
    pub lot_mint: Pubkey,
    pub lot: u64,
    /// The project token the buyer burned.
    pub price: u64,
    pub buyer: Pubkey,
    pub slot: u64,
}

#[event]
pub struct UpgradeBought {
    pub owner: Pubkey,
    pub dept: u8,
    pub level: u8,
    pub mint: Pubkey,
    pub price: u64,
    pub levels: [u8; 8],
    pub slot: u64,
}

use anchor_lang::prelude::*;

pub const HOUSE_SEED: &[u8] = b"house";
/// Every fee and sale arrives here (arena's `Config.treasury`).
pub const TREASURY_SEED: &[u8] = b"treasury";
/// Funds cups (arena's `Config.prize_vault`; also receives escrow surpluses).
pub const PRIZE_SEED: &[u8] = b"prize";
/// The project token's buyback share builds up here.
pub const BUYBACK_SEED: &[u8] = b"buyback";
pub const PRICE_SEED: &[u8] = b"price";
pub const UPGRADES_SEED: &[u8] = b"upgrades";
pub const TOTALS_SEED: &[u8] = b"totals";
/// The project token proposed, waiting out its timelock (`PendingToken`).
pub const TOKEN_SEED: &[u8] = b"token";
/// The buyback auction of one token (or SOL) for the project token (`Auction`).
pub const AUCTION_SEED: &[u8] = b"auction";

/// Prices and totals in native SOL use this "mint" (all zeros).
pub const NATIVE_SOL: Pubkey = Pubkey::new_from_array([0u8; 32]);

pub const BPS_DENOMINATOR: u64 = 10_000;
/// A new split takes effect a week after it is proposed, for anyone to see.
pub const SPLIT_TIMELOCK_SECS: i64 = 7 * 86_400;
/// A funded cup may never take more than half the prize vault at once.
pub const HARD_MAX_FUND_BPS: u16 = 5_000;

/// The project token takes effect two days after it is proposed (as arena's
/// role changes): from then on the buyback vault fills and its auctions sell
/// for that token, so a stolen admin key can't name a token of its own unseen.
pub const TOKEN_TIMELOCK_SECS: i64 = 2 * 86_400;
/// The most a transfer-fee token (the project token, or a token the shop sells
/// in) may take per transfer for the house to accept it or sell in it.
pub const HARD_MAX_TOKEN_FEE_BPS: u16 = 500;

/// The buyback auction: one lot at a time per token, at most a quarter of the
/// vault's balance in it, at most one auction a day, its price falling evenly
/// from the start price to the floor over 1 h to 7 days and held there until
/// it ends (14 days at most). A floor is never below the admin's lowest rate
/// for that coin and, for a week after a sale, half that sale's rate: the
/// keeper (a server key) can't sell the vault cheap.
pub const AUCTION_MAX_LOT_BPS: u16 = 2_500;
pub const AUCTION_EVERY_SECS: i64 = 86_400;
pub const MIN_DECLINE_SECS: u32 = 3_600;
pub const MAX_DECLINE_SECS: u32 = 7 * 86_400;
pub const MAX_AUCTION_SECS: u32 = 14 * 86_400;
pub const FLOOR_ANCHOR_BPS: u16 = 5_000;
pub const ANCHOR_SECS: i64 = 7 * 86_400;

pub const DEPARTMENTS: usize = 6;
pub const MAX_LEVEL: u8 = 5;
pub const UPGRADES_VERSION: u8 = 1;

use anchor_lang::prelude::*;

// ---- PDA seeds --------------------------------------------------------------
pub const CONFIG_SEED: &[u8] = b"config";
pub const RULESET_SEED: &[u8] = b"ruleset";
pub const MINT_SEED: &[u8] = b"mint";
pub const ROOM_SEED: &[u8] = b"room";
pub const CUP_SEED: &[u8] = b"cup";
pub const ENTRY_SEED: &[u8] = b"entry";
pub const HOST_SEED: &[u8] = b"host";
/// The house program's upgrades account of a player: `["upgrades", owner]`
/// under `HOUSE_ID`. Arena reads it raw (a GARAGE room's levels snapshot).
pub const UPGRADES_SEED: &[u8] = b"upgrades";

/// The house program (treasury split, prize vault, upgrade shop). A compile-time
/// constant on purpose: not in `Config`, so no admin can point arena at a fake
/// shop whose "upgrades" accounts it would trust.
pub const HOUSE_ID: Pubkey = pubkey!("5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw");
/// The house's vaults (PDAs under `HOUSE_ID`): arena's treasury and prize vault
/// must be exactly these.
pub const HOUSE_TREASURY_SEED: &[u8] = b"treasury";
pub const HOUSE_PRIZE_SEED: &[u8] = b"prize";

/// A new guardian, gate or resolver set takes effect only this long after it is
/// proposed: two days for anyone (and the guardian) to see it coming.
pub const ROLES_TIMELOCK_SECS: i64 = 2 * 86_400;

/// A room or cup whose `mint` is this address is in native SOL (lamports held by
/// the PDA itself). It is the System Program id: all zeros.
pub const NATIVE_SOL: Pubkey = Pubkey::new_from_array([0u8; 32]);

/// SPL Memo (v3). A Token-2022 account whose owner turned on required memos
/// takes no transfer in unless a memo comes just before it; a payout to one
/// is preceded by a memo through this program (passed as the last remaining
/// account), so no player can block a room's or cup's payouts that way.
pub const MEMO_PROGRAM_ID: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

// ---- Fees: hard ceilings the admin can never exceed --------------------------
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const HARD_MAX_ROOM_FEE_BPS: u16 = 1_000; // 10% of the pot
pub const HARD_MAX_CUP_FEE_BPS: u16 = 1_500; // 15% of entries
/// The most a transfer-fee mint (the project token) may take per transfer for
/// the escrow to accept new deposits in it.
pub const HARD_MAX_TRANSFER_FEE_BPS: u16 = 500;

// ---- Sizes ------------------------------------------------------------------
pub const MIN_ROOM_PLAYERS: u8 = 2;
pub const MAX_ROOM_PLAYERS: u8 = 8;
pub const MAX_RESOLVERS: usize = 4;
pub const MAX_PAYOUT_PLACES: usize = 32; // claimed places are a u32 bitmap
pub const MAX_URI_LEN: usize = 200;
pub const DEPARTMENTS: usize = 6;
pub const MAX_LEVEL: u8 = 5;
/// A GARAGE car's rating, in tenths: 700 (stock) + Σ level × points.
pub const STOCK_RATING: u16 = 700;

// ---- A room's race: what the race server can run (and settle) within its windows
pub const MAX_LAPS: u8 = 20;
pub const MAX_RACE_AI: u8 = 19;

// ---- Windows (seconds). Every deadline is bounded, so no escrow can be held
// longer than these, and every state has a permissionless exit.
pub const MIN_WINDOW_SECS: u32 = 10;
pub const MAX_JOIN_WINDOW_SECS: u32 = 86_400; // an open room: 24 h at most
pub const MAX_SETTLE_WINDOW_SECS: u32 = 86_400; // a locked room: 24 h at most
pub const MAX_CHALLENGE_WINDOW_SECS: u32 = 7 * 86_400;
pub const MAX_RESULTS_WINDOW_SECS: u32 = 30 * 86_400;
pub const MAX_CLAIM_WINDOW_SECS: u32 = 30 * 86_400;
pub const MAX_CUP_SECS: i64 = 90 * 86_400;
/// A finalized cup with a place nobody can be paid (its winner unreachable)
/// closes anyway this long after its claim window.
pub const FORFEIT_SECS: i64 = 7 * 86_400;

/// A finishing time of `DNF` means no valid result (didn't finish, retired,
/// disqualified, didn't start). Any other value is a race time in ms, lower wins.
pub const DNF: u32 = u32::MAX;

/// Merkle domain tags: leaves and inner nodes can never be confused.
pub const LEAF_PREFIX: &[u8] = &[0x00];
pub const NODE_PREFIX: &[u8] = &[0x01];

/// The house program's `Upgrades` account, read raw: Anchor discriminator (8),
/// version u8, owner (32), levels [u8; 8] (6 used), updated_at i64, bump u8.
pub const UPGRADES_VERSION: u8 = 1;
pub const UPGRADES_LEVELS_AT: usize = 8 + 1 + 32;
pub const UPGRADES_OWNER_AT: usize = 8 + 1;
/// sha256("account:Upgrades")[..8], the house program's Anchor discriminator.
pub const UPGRADES_DISCRIMINATOR: [u8; 8] = [124, 166, 118, 161, 178, 46, 191, 24];

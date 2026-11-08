use anchor_lang::prelude::*;

use crate::constants::*;

/// How every fee and sale is split by `sweep`, in bps of 10000.
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default,
)]
pub struct Split {
    /// The studio's running costs: servers, the audit, development.
    pub ops_bps: u16,
    /// The prize vault: the weekly stock prizes and other funded cups.
    pub prize_bps: u16,
    /// The project token's buyback (burned); until the token exists, prizes.
    pub buyback_bps: u16,
}

impl Split {
    pub fn ok(&self) -> bool {
        self.ops_bps as u32 + self.prize_bps as u32 + self.buyback_bps as u32
            == BPS_DENOMINATOR as u32
    }
}

/// The house's settings. PDA `["house"]`.
#[account]
#[derive(InitSpace)]
pub struct HouseConfig {
    pub admin: Pubkey,
    pub pending_admin: Pubkey,
    /// May fund weekly cups (within the caps).
    pub keeper: Pubkey,
    /// Where the studio's share goes (its token accounts for SPL).
    pub ops_wallet: Pubkey,
    pub split: Split,
    /// A new split takes effect only after `split_after` (a public timelock).
    pub pending_split: Split,
    pub split_after: i64,
    /// The project token, set ONCE (`Pubkey::default()` = none yet): its share of
    /// fees is burned, and the buyback share of other tokens builds up for it.
    pub token_mint: Pubkey,
    /// A funded cup's guarantee may take at most this share of the prize vault's
    /// balance in that token, and at most once per `fund_every_secs`.
    pub fund_max_bps: u16,
    pub fund_every_secs: u32,
    /// Looser caps, in force from `fund_caps_after` (the split's timelock; 0 =
    /// nothing pending). Tighter ones apply at once.
    pub pending_fund_max_bps: u16,
    pub pending_fund_every_secs: u32,
    pub fund_caps_after: i64,
    /// When a cup was last funded (`i64::MIN`: never).
    pub last_fund_at: i64,
    /// Stops sales and cup funding. Never stops sweeping or anything that pays out.
    pub paused: bool,
    pub treasury_bump: u8,
    pub prize_bump: u8,
    pub buyback_bump: u8,
    pub bump: u8,
}

/// Upgrade prices in one token: department × level 1-5, in base units
/// (0 = not for sale). PDA `["price", mint]`; SOL is `mint = NATIVE_SOL`.
#[account]
#[derive(InitSpace)]
pub struct PriceTable {
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub decimals: u8,
    pub prices: [[u64; MAX_LEVEL as usize]; DEPARTMENTS],
    pub enabled: bool,
    pub updated_at: i64,
    pub bump: u8,
}

/// A player's upgrade levels: one account per wallet, for every car look.
/// PDA `["upgrades", owner]`. A fixed layout the arena program reads raw (the
/// levels snapshot of a GARAGE room): discriminator (8) | version (1) |
/// owner (32) | levels [u8; 8] (6 used, `src/game/upgrades.js` order: pu, aero,
/// tyre, brake, chassis, dur) | updated_at | bump. Never a token: levels can't
/// be sold or moved.
#[account]
#[derive(InitSpace)]
pub struct Upgrades {
    pub version: u8,
    pub owner: Pubkey,
    pub levels: [u8; 8],
    pub updated_at: i64,
    pub bump: u8,
}

/// The project token, proposed and waiting out `TOKEN_TIMELOCK_SECS` before
/// anyone applies it to the house. PDA `["token"]`.
#[account]
#[derive(InitSpace)]
pub struct PendingToken {
    pub mint: Pubkey,
    pub after: i64,
    pub bump: u8,
}

/// The buyback auction of one token (or SOL): the buyback vault's balance sold,
/// a lot at a time, for the project token, which the buyer burns from their own
/// wallet. The price, in the project token, for the whole lot, falls evenly
/// from `start_price` at `start_at` to `floor_price` over `decline_secs` and
/// holds there until `end_at`. No oracle: the falling price finds the market.
/// PDA `["auction", lot_mint]` (`NATIVE_SOL` for SOL).
#[account]
#[derive(InitSpace)]
pub struct Auction {
    pub lot_mint: Pubkey,
    /// For sale now (0: nothing; an auction past `end_at` is over too).
    pub lot: u64,
    pub start_price: u64,
    pub floor_price: u64,
    pub start_at: i64,
    pub decline_secs: u32,
    pub end_at: i64,
    /// The admin's lowest rate: no floor prices the lot below `min_price` of
    /// the project token per `min_lot` of it (`set_auction_floor`).
    pub min_lot: u64,
    pub min_price: u64,
    /// The last sale's lot, price and time: for `ANCHOR_SECS` after it, a new
    /// floor is held to at least `FLOOR_ANCHOR_BPS` of its rate.
    pub last_lot: u64,
    pub last_price: u64,
    pub last_sold_at: i64,
    /// Running totals: lots sold, what they paid out, the project token burned.
    pub sold: u32,
    pub paid_out: u64,
    pub burned: u64,
    pub bump: u8,
}

impl Auction {
    /// The whole lot's price at `now` (rounded up: the house's side).
    pub fn price_at(&self, now: i64) -> u64 {
        let elapsed = now.saturating_sub(self.start_at).max(0) as u128;
        let decline = self.decline_secs as u128;
        if decline == 0 || elapsed >= decline {
            return self.floor_price;
        }
        let span = (self.start_price - self.floor_price) as u128;
        self.start_price - (span * elapsed / decline) as u64
    }

    /// An auction with something for sale until `end_at`.
    pub fn running(&self, now: i64) -> bool {
        self.lot > 0 && now < self.end_at
    }
}

/// Running totals per token, for anyone to read (the project page shows them).
/// PDA `["totals", mint]`.
#[account]
#[derive(InitSpace)]
pub struct Totals {
    pub mint: Pubkey,
    pub swept_ops: u64,
    pub swept_prize: u64,
    pub swept_buyback: u64,
    pub burned: u64,
    pub funded: u64,
    pub bump: u8,
}

use anchor_lang::prelude::*;

use crate::constants::*;

/// Protocol settings. One per deployment, PDA `["config"]`.
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    /// Two-step admin transfer: `propose_admin` sets this, the new key accepts.
    /// `Pubkey::default()` = nothing pending.
    pub pending_admin: Pubkey,
    /// Emergency key: may pause, reject posted cup results, cancel cups and revoke
    /// every resolver. Never moves money anywhere except back to its owners.
    pub guardian: Pubkey,
    /// Co-signs every new room, join and cup entry (ping, region, age, invite
    /// checks off-chain). `Pubkey::default()` = ungated. It can only admit money;
    /// it can never block an exit.
    pub gate: Pubkey,
    /// Keys that lock, settle and void rooms and post cup results (the race
    /// server). Checked against the CURRENT set when used, so rotating a key never
    /// strands a room in flight.
    #[max_len(MAX_RESOLVERS)]
    pub resolvers: Vec<Pubkey>,
    /// The house treasury PDA: every fee goes here (its token accounts for SPL).
    pub treasury: Pubkey,
    /// The house prize-vault PDA: escrow surpluses go here, and it may organize
    /// funded cups (the house program signs for it).
    pub prize_vault: Pubkey,
    /// Holder perks: the project token, set once (`Pubkey::default()` = none yet).
    pub holder_mint: Pubkey,
    /// A room created by a wallet holding at least this many base units of
    /// `holder_mint` gets `holder_room_fee_bps` (if lower than its mint's fee).
    pub holder_min: u64,
    pub holder_room_fee_bps: u16,
    /// A wallet holding at least this many may organize community cups.
    pub host_min: u64,
    /// A locked room must be settled or voided within this; after it anyone
    /// refunds every stake, no fee.
    pub settle_window_secs: u32,
    pub challenge_window_secs: u32,
    pub results_window_secs: u32,
    pub claim_window_secs: u32,
    /// Stops new rooms, joins, cups and entries. Never stops settling, refunds,
    /// claims or closing.
    pub paused: bool,
    /// Cups made so far (a counter: a cup's id is its creator's choice).
    pub cup_count: u64,
    /// A new guardian, gate and resolver set, in force from `roles_after`
    /// (`ROLES_TIMELOCK_SECS` after it was proposed; 0 = nothing pending).
    pub pending_guardian: Pubkey,
    pub pending_gate: Pubkey,
    #[max_len(MAX_RESOLVERS)]
    pub pending_resolvers: Vec<Pubkey>,
    pub roles_after: i64,
    pub bump: u8,
}

impl Config {
    pub fn is_resolver(&self, key: &Pubkey) -> bool {
        self.resolvers.iter().any(|r| r == key)
    }
}

/// One version of the game's rules (physics, track, race rules, upgrade
/// effects), hashed by the game build. Rooms and cups name the ruleset they are
/// raced under, so a new sim version can never re-rule a race in flight.
/// PDA `["ruleset", id_le]`.
#[account]
#[derive(InitSpace)]
pub struct Ruleset {
    pub id: u32,
    pub hash: [u8; 32],
    /// A GARAGE car's rating points per department level (`src/game/upgrades.js`
    /// order: pu, aero, tyre, brake, chassis, dur), in tenths of a rating point.
    pub rating_points: [u8; DEPARTMENTS],
    pub enabled: bool,
    pub bump: u8,
}

impl Ruleset {
    /// 700 + Σ level × points: a car's rating in tenths (700 stock, 1000 full).
    pub fn rating(&self, levels: &[u8; DEPARTMENTS]) -> u16 {
        let mut r = STOCK_RATING as u32;
        for (l, p) in levels.iter().zip(self.rating_points.iter()) {
            r += *l as u32 * *p as u32;
        }
        r.min(u16::MAX as u32) as u16
    }
}

/// A token allowed for rooms and cups, with its limits and fees.
/// PDA `["mint", mint]`. SOL itself is `mint = NATIVE_SOL`.
#[account]
#[derive(InitSpace)]
pub struct AllowedMint {
    pub mint: Pubkey,
    /// Owner program of the mint (Token or Token-2022); System Program for SOL.
    pub token_program: Pubkey,
    pub decimals: u8,
    pub min_stake: u64,
    pub max_stake: u64,
    /// House fee on a room's pot in this token, and on a cup's entries.
    pub room_fee_bps: u16,
    pub cup_fee_bps: u16,
    /// The mint carries a transfer fee (the project token): deposits are grossed
    /// up and new deposits stop if its fee goes above `max_transfer_fee_bps`.
    pub transfer_fee: bool,
    pub max_transfer_fee_bps: u16,
    /// The holder fee applies only to rooms staking at most this much (renting a
    /// balance to dodge fees must cost more than it saves).
    pub holder_max_stake: u64,
    pub enabled: bool,
    pub bump: u8,
}

/// What race a room runs: the race server builds exactly this race.
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default,
)]
pub struct RaceSpec {
    pub track: u8,
    /// 0 sprint, 1 grand prix.
    pub format: u8,
    pub laps: u8,
    /// AI cars on the grid.
    pub ai: u8,
    /// 0 day, 1 sunset, 2 night.
    pub time: u8,
    /// 0 clear, 1 cloudy, 2 light, 3 rain, 4 storm, 5 changing.
    pub sky: u8,
    /// 0 starts when full, 1 the creator starts it (two players or more).
    pub start_mode: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum RoomState {
    /// Taking players. Stakes can be withdrawn (leave / cancel).
    Open,
    /// The race is on; the resolver settles or voids it.
    Locked,
    Settled,
    Refunded,
}

pub const CLASS_EQUAL: u8 = 0;
pub const CLASS_GARAGE: u8 = 1;
pub const ACCESS_OPEN: u8 = 0;
pub const ACCESS_INVITE: u8 = 1;

/// Why a room was refunded instead of settled.
pub const VOID_NONE: u8 = 0;
pub const VOID_CANCELLED: u8 = 1; // the creator cancelled an open room
pub const VOID_EXPIRED: u8 = 2; // a deadline passed (anyone refunded)
pub const VOID_RACE: u8 = 3; // the race couldn't produce a result (resolver)
pub const VOID_TIED: u8 = 4; // settled with nobody ahead: everyone refunded

/// A live race's pot: everyone stakes the same, the race server runs the race,
/// the resolver settles it and the program pays the fastest at once.
/// PDA `["room", code]`, where `code` is the race server's 8-character room id.
/// The escrow is the PDA itself (SOL) or its associated token account.
///
/// The first fields form a fixed prefix after the 8-byte discriminator, so the
/// lobby can filter with memcmp: state @8, access @9, class @10, mint @11,
/// stake @43, max_players @51, max_rating @52.
#[account]
#[derive(InitSpace)]
pub struct Room {
    pub state: RoomState,
    pub access: u8,
    pub class: u8,
    pub mint: Pubkey,
    pub stake: u64,
    pub max_players: u8,
    /// GARAGE rooms: the highest car rating allowed, in tenths; 0 = no cap.
    pub max_rating: u16,
    pub code: [u8; 8],
    pub creator: Pubkey,
    pub ruleset: Pubkey,
    pub race: RaceSpec,
    pub token_program: Pubkey,
    /// Terms frozen at creation: config changes never touch a room in flight.
    pub fee_bps: u16,
    pub transfer_fee: bool,
    /// New stakes stop if the token's transfer fee goes above this.
    pub max_transfer_fee_bps: u16,
    pub treasury: Pubkey,
    pub prize_vault: Pubkey,
    /// Paid the room's rent (the server's sponsor, or the creator); gets it back.
    pub rent_payer: Pubkey,
    /// Seat order = the race's grid order.
    #[max_len(MAX_ROOM_PLAYERS)]
    pub players: Vec<Pubkey>,
    /// GARAGE rooms: each player's upgrade levels when they joined.
    #[max_len(MAX_ROOM_PLAYERS)]
    pub levels: Vec<[u8; DEPARTMENTS]>,
    /// Drawn after the room locks (`draw_seed`): hash(room ‖ the hash of the slot
    /// it locked in ‖ that slot). All zeros until then.
    pub seed: [u8; 32],
    /// Final times in seat order, set at settlement. `DNF` = no result.
    #[max_len(MAX_ROOM_PLAYERS)]
    pub times_ms: Vec<u32>,
    /// Bit i set = players[i] was paid a winner's share.
    pub winners: u8,
    pub share: u64,
    pub to_treasury: u64,
    /// sha256 of the race's full record (canonical JSON), published by the race
    /// server: anyone can run the race again from it and check these times.
    pub record_hash: [u8; 32],
    pub void_reason: u8,
    pub settled_by: Pubkey,
    pub created_at: i64,
    pub join_deadline: i64,
    pub settle_window_secs: u32,
    pub locked_at: i64,
    /// The slot the room locked in: its hash, unknown while the locking
    /// transaction ran, is the seed's source.
    pub locked_slot: u64,
    /// After this (locked), anyone can refund every stake, no fee.
    pub settle_deadline: i64,
    pub bump: u8,
}

impl Room {
    pub fn is_sol(&self) -> bool {
        self.mint == NATIVE_SOL
    }
    pub fn pot(&self) -> Result<u64> {
        self.stake
            .checked_mul(self.players.len() as u64)
            .ok_or_else(|| error!(crate::error::ArenaError::Overflow))
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum CupKind {
    /// Players enter (paying the entry fee, if any); prizes go to entrants.
    Entries,
    /// No entries on-chain: a board the race server keeps (the free weekly time
    /// trial). Prizes come from the guarantee; a leaf names the winner's wallet.
    Board,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum CupState {
    /// Running until `ends_at`; then waiting for results.
    Open,
    /// Results posted, in the challenge window.
    Posted,
    /// Prizes claimable.
    Finalized,
    /// Every entry and the guarantee go back.
    Cancelled,
}

/// A prize event: a leaderboard over a window, an optional entry fee, a payout
/// table, an optional guarantee and an optional featured stock.
/// PDA `["cup", id_le]`; escrow is the PDA (SOL) or its ATA.
#[account]
#[derive(InitSpace)]
pub struct Cup {
    pub id: u64,
    pub kind: CupKind,
    pub class: u8,
    pub organizer: Pubkey,
    /// Paid the guarantee; gets back what isn't won.
    pub sponsor: Pubkey,
    /// Paid the cup's rent (the organizer, or the house keeper); gets it back.
    pub rent_payer: Pubkey,
    /// A community cup's host (`Pubkey::default()` = none) and its share of the
    /// house fee, paid at close.
    pub host: Pubkey,
    pub host_share_bps: u16,
    pub ruleset: Pubkey,
    /// sha256 of the event's race spec, published with it.
    pub spec_hash: [u8; 32],
    /// The event's featured stock: winners may swap their prize into it in their
    /// own claim transaction. Informational: the program never holds it.
    pub featured_mint: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub transfer_fee: bool,
    pub entry_fee: u64,
    pub fee_bps: u16,
    pub treasury: Pubkey,
    /// Anything left above what the cup owed goes here at close.
    pub prize_vault: Pubkey,
    /// New entries stop if the token's transfer fee goes above this.
    pub max_transfer_fee_bps: u16,
    /// Share of the pool per place, in bps, summing to 10000.
    #[max_len(MAX_PAYOUT_PLACES)]
    pub payout_bps: Vec<u16>,
    /// Minimum pool, deposited by `sponsor` at creation.
    pub guarantee: u64,
    pub starts_at: i64,
    pub ends_at: i64,
    /// Results must be posted before this, or every entry can be refunded.
    pub results_deadline: i64,
    pub challenge_window_secs: u32,
    pub claim_window_secs: u32,
    /// 0 = unlimited.
    pub max_entrants: u32,
    pub entrants: u32,
    pub refunded: u32,
    pub state: CupState,
    /// Merkle root over leaves hash(0x00 ‖ cup ‖ rank_le16 ‖ wallet) for every
    /// paid place. Ranks only: the program computes every amount.
    pub results_root: [u8; 32],
    pub scorers: u32,
    /// Hash of the full published standings (every ranked run and its record).
    pub standings_hash: [u8; 32],
    #[max_len(MAX_URI_LEN)]
    pub results_uri: String,
    pub posted_at: i64,
    pub challenge_ends: i64,
    /// Before this only the winner may claim (and swap into the featured stock);
    /// after it anyone may push the prize to the winner.
    pub claim_until: i64,
    /// Set by `finalize`.
    pub pool: u64,
    pub places: u8,
    pub used_bps: u32,
    /// Bit r-1 set = rank r paid.
    pub claimed: u32,
    pub paid: u64,
    pub guarantee_returned: bool,
    pub bump: u8,
}

impl Cup {
    pub fn is_sol(&self) -> bool {
        self.mint == NATIVE_SOL
    }
    /// Everything that went into escrow (guarantee + every entry).
    pub fn total_in(&self) -> Result<u64> {
        (self.entry_fee as u128)
            .checked_mul(self.entrants as u128)
            .and_then(|e| e.checked_add(self.guarantee as u128))
            .and_then(|t| u64::try_from(t).ok())
            .ok_or_else(|| error!(crate::error::ArenaError::Overflow))
    }
    pub fn all_claimed(&self) -> bool {
        let mask: u64 = (1u64 << self.places) - 1;
        (self.claimed as u64) & mask == mask
    }
}

/// One player's cup entry. PDA `["entry", cup, player]`.
#[account]
#[derive(InitSpace)]
pub struct Entry {
    pub cup: Pubkey,
    pub player: Pubkey,
    /// Paid the entry's rent (the race server's sponsor, or the player); gets it back.
    pub rent_payer: Pubkey,
    pub refunded: bool,
    pub bump: u8,
}

/// A wallet the admin lets organize community cups (the path with no token).
/// PDA `["host", wallet]`.
#[account]
#[derive(InitSpace)]
pub struct Host {
    pub wallet: Pubkey,
    /// Hosting allowed until this unix time.
    pub until: i64,
    pub bump: u8,
}

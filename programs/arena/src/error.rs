use anchor_lang::prelude::*;

#[error_code]
pub enum ArenaError {
    #[msg("Signer is not allowed to do this")]
    Unauthorized,
    #[msg("New rooms, joins, cups and entries are paused")]
    Paused,
    #[msg("A parameter is out of range")]
    InvalidParams,
    #[msg("Fee is above the hard cap")]
    FeeTooHigh,
    #[msg("Window is outside the allowed bounds")]
    InvalidWindow,
    #[msg("Ruleset is disabled")]
    RulesetDisabled,
    #[msg("Token is not enabled for rooms and cups")]
    MintDisabled,
    #[msg("Stake is outside this token's limits")]
    StakeOutOfRange,
    #[msg("Token mint has an extension the escrow does not support")]
    UnsupportedMint,
    #[msg("The token's transfer fee is above what this escrow accepts")]
    TransferFeeTooHigh,
    #[msg("Token accounts were missing, extra, or did not match the room's token")]
    TokenAccountsMismatch,
    #[msg("Escrow received a different amount than the stake")]
    DepositMismatch,
    #[msg("Escrow cannot cover this payment")]
    InsufficientEscrow,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("The gate's signature is required")]
    GateRequired,
    #[msg("Room is not open")]
    RoomNotOpen,
    #[msg("Room is not locked")]
    RoomNotLocked,
    #[msg("Room is already full")]
    RoomFull,
    #[msg("Player has already joined")]
    AlreadyJoined,
    #[msg("Signer is not a player in this room")]
    NotAPlayer,
    #[msg("The creator cannot leave; cancel the room instead")]
    CreatorCannotLeave,
    #[msg("Join deadline has passed")]
    JoinClosed,
    #[msg("Deadline has not passed yet")]
    TooEarly,
    #[msg("Deadline has passed")]
    TooLate,
    #[msg("A room needs at least two players to start")]
    NotEnoughPlayers,
    #[msg("Results must give one time per player")]
    InvalidResults,
    #[msg("Payout accounts do not match the players")]
    PayoutAccountsMismatch,
    #[msg("Nothing to refund in this state")]
    NotRefundable,
    #[msg("Account cannot be closed yet")]
    NotClosable,
    #[msg("The upgrades account is not this player's")]
    BadUpgrades,
    #[msg("This car's rating is above the room's cap")]
    RatingTooHigh,
    #[msg("The holder account does not qualify")]
    NotAHolder,
    #[msg("Cup is not open")]
    CupNotOpen,
    #[msg("Cup is full")]
    CupFull,
    #[msg("This cup takes no entries")]
    NoEntries,
    #[msg("Results have not been posted")]
    ResultsNotPosted,
    #[msg("Cup is not finalized")]
    NotFinalized,
    #[msg("Rank is outside the paid places")]
    InvalidRank,
    #[msg("Prize already claimed")]
    AlreadyClaimed,
    #[msg("Merkle proof does not match the posted results")]
    InvalidProof,
    #[msg("Entry was refunded")]
    EntryRefunded,
    #[msg("Payout table must have 1-32 places summing to 10000 bps")]
    InvalidPayoutTable,
    #[msg("Results URI is too long")]
    UriTooLong,
    #[msg("No admin change is pending for this key")]
    NotPendingAdmin,
    #[msg("This can only be set once")]
    AlreadySet,
    #[msg("The room's seed is drawn already")]
    SeedDrawn,
    #[msg("The room's seed isn't drawn yet")]
    SeedNotDrawn,
    #[msg("The slot the room locked in is too old to draw its seed from: it can only be voided or refunded")]
    SeedExpired,
    #[msg("The room's stake isn't the one expected")]
    StakeChanged,
    #[msg("Only a room its creator starts can be locked; before its join deadline")]
    NotLockable,
    #[msg("Nothing is pending")]
    NothingPending,
    #[msg("A payout account requires a memo: pass the memo program as the last remaining account")]
    MemoRequired,
}

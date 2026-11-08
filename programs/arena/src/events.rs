use anchor_lang::prelude::*;

#[event]
pub struct ConfigChanged {
    pub admin: Pubkey,
}

#[event]
pub struct RolesProposed {
    pub guardian: Pubkey,
    pub gate: Pubkey,
    pub after: i64,
}

#[event]
pub struct ResolversRevoked {
    pub by: Pubkey,
}

#[event]
pub struct RoomCreated {
    pub room: Pubkey,
    pub code: [u8; 8],
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub stake: u64,
    pub max_players: u8,
    pub fee_bps: u16,
    pub class: u8,
    pub join_deadline: i64,
}

#[event]
pub struct RoomJoined {
    pub room: Pubkey,
    pub player: Pubkey,
    pub players: u8,
}

#[event]
pub struct RoomLeft {
    pub room: Pubkey,
    pub player: Pubkey,
}

#[event]
pub struct RoomLocked {
    pub room: Pubkey,
    /// The seed comes from this slot's hash, once it is over (`draw_seed`).
    pub locked_slot: u64,
    pub players: u8,
    pub settle_deadline: i64,
}

#[event]
pub struct RoomSeeded {
    pub room: Pubkey,
    pub seed: [u8; 32],
}

#[event]
pub struct RoomSettled {
    pub room: Pubkey,
    pub times_ms: Vec<u32>,
    pub record_hash: [u8; 32],
    pub winners: u8,
    /// What each winner was sent from escrow (before any transfer fee).
    pub share: u64,
    pub to_treasury: u64,
    pub settled_by: Pubkey,
}

#[event]
pub struct RoomRefunded {
    pub room: Pubkey,
    pub reason: u8,
    pub record_hash: [u8; 32],
    pub by: Pubkey,
}

#[event]
pub struct RoomClosed {
    pub room: Pubkey,
    /// Anything above what the room owed (a donation, a reward pushed to the
    /// escrow), sent to the prize vault.
    pub surplus: u64,
}

#[event]
pub struct CupCreated {
    pub cup: Pubkey,
    pub id: u64,
    pub organizer: Pubkey,
    pub mint: Pubkey,
    pub entry_fee: u64,
    pub guarantee: u64,
    pub featured_mint: Pubkey,
    pub ends_at: i64,
}

#[event]
pub struct CupEntered {
    pub cup: Pubkey,
    pub player: Pubkey,
    pub entrants: u32,
}

#[event]
pub struct ResultsPosted {
    pub cup: Pubkey,
    pub results_root: [u8; 32],
    pub scorers: u32,
    pub standings_hash: [u8; 32],
    pub results_uri: String,
    pub challenge_ends: i64,
}

#[event]
pub struct ResultsRejected {
    pub cup: Pubkey,
    pub by: Pubkey,
}

#[event]
pub struct CupFinalized {
    pub cup: Pubkey,
    pub pool: u64,
    pub places: u8,
    pub claim_until: i64,
}

#[event]
pub struct PrizeClaimed {
    pub cup: Pubkey,
    pub rank: u16,
    pub wallet: Pubkey,
    pub amount: u64,
    /// The winner signed (and may have swapped it into the featured stock), or
    /// it was pushed after the claim window.
    pub pushed: bool,
}

#[event]
pub struct CupCancelled {
    pub cup: Pubkey,
    pub by: Pubkey,
}

#[event]
pub struct EntryRefunded {
    pub cup: Pubkey,
    pub player: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CupClosed {
    pub cup: Pubkey,
    pub to_sponsor: u64,
    pub to_host: u64,
    pub to_treasury: u64,
}

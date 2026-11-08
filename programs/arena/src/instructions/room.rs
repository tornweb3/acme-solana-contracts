//! Rooms: the pots of live races. Everyone stakes the same amount in SOL or an
//! allowed token; the race server runs the race; its resolver key settles it and
//! the program pays the fastest in the same transaction.
//!
//!   create_room ─► join_room … ─► full ("starts when full": locks itself)
//!     or lock_room (resolver: the creator started it, two players or more)
//!     ─► draw_seed (anyone, once the slot it locked in is over: the race's seed)
//!     ─► the race ─► settle_room (resolver: times + the race record's hash)
//!                 or void_room   (resolver: no result; every stake back)
//!   Exits that need nobody's permission: refund_room after join_deadline (still
//!   open) or settle_deadline (locked). Open rooms: leave_room / cancel_room.
//!
//! What the resolver can't do: pay anyone but the room's own players, take more
//! than the frozen fee, or act after the deadline. Payout accounts arrive as
//! remaining_accounts: for a settlement, one per WINNER in seat order (a loser's
//! closed token account can never block it); for every refund, one per player.
//! Each is the player's wallet (SOL) or a token account of the room's mint owned
//! by the player. Any payout may end with the memo program, as the last
//! remaining account: a token account that requires memos is paid with one.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use solana_sha256_hasher::hashv;

use crate::{
    constants::*,
    error::ArenaError,
    escrow::{
        check_fee_cap, deposit, lamports_above_rent, split_memo, token_side, transfer_from_pda,
        validate_wager_mint, Escrow, TokenSide,
    },
    events::*,
    payout::room_outcome,
    state::*,
};

/// `slot`'s hash from the SlotHashes sysvar (read raw: a u64 count, then
/// (slot u64, hash [u8; 32]) newest first; the last ~512 slots), if still there.
fn slot_hash(slot_hashes: &AccountInfo, slot: u64) -> Result<Option<[u8; 32]>> {
    let data = slot_hashes.try_borrow_data()?;
    if data.len() < 8 {
        return Ok(None);
    }
    let count = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
    for i in 0..count {
        let at = 8 + i * 40;
        if data.len() < at + 40 {
            break;
        }
        if u64::from_le_bytes(data[at..at + 8].try_into().unwrap()) == slot {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[at + 8..at + 40]);
            return Ok(Some(hash));
        }
    }
    Ok(None)
}

fn with_escrow<'info, R>(
    room: &Account<'info, Room>,
    token: Option<TokenSide<'info>>,
    memo: Option<AccountInfo<'info>>,
    f: impl FnOnce(&Escrow<'_, 'info>) -> Result<R>,
) -> Result<R> {
    let bump = [room.bump];
    let seeds: [&[u8]; 3] = [ROOM_SEED, &room.code, &bump];
    f(&Escrow {
        pda: room.to_account_info(),
        seeds: &seeds,
        token,
        memo,
    })
}

fn refund_everyone<'info>(
    room: &Room,
    escrow: &Escrow<'_, 'info>,
    payouts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        payouts.len() == room.players.len(),
        ArenaError::PayoutAccountsMismatch
    );
    for (player, to) in room.players.iter().zip(payouts) {
        escrow.pay(to, player, room.stake)?;
    }
    Ok(())
}

/// When a gate is set, it must co-sign new money (create, join).
fn check_gate(config: &Config, gate: &Option<Signer>) -> Result<()> {
    if config.gate != Pubkey::default() {
        let g = gate.as_ref().ok_or(ArenaError::GateRequired)?;
        require_keys_eq!(g.key(), config.gate, ArenaError::GateRequired);
    }
    Ok(())
}

/// A player's upgrade levels from the house program's `["upgrades", player]`
/// account (GARAGE rooms). The account must be exactly that PDA; empty = stock.
pub fn read_levels(player: &Pubkey, upgrades: &AccountInfo) -> Result<[u8; DEPARTMENTS]> {
    let (want, _) = Pubkey::find_program_address(&[UPGRADES_SEED, player.as_ref()], &HOUSE_ID);
    require_keys_eq!(upgrades.key(), want, ArenaError::BadUpgrades);
    if upgrades.data_is_empty() {
        return Ok([0; DEPARTMENTS]);
    }
    require_keys_eq!(*upgrades.owner, HOUSE_ID, ArenaError::BadUpgrades);
    let data = upgrades.try_borrow_data()?;
    require!(
        data.len() >= UPGRADES_LEVELS_AT + DEPARTMENTS
            && data[..8] == UPGRADES_DISCRIMINATOR
            && data[8] == UPGRADES_VERSION
            && data[UPGRADES_OWNER_AT..UPGRADES_OWNER_AT + 32] == player.as_ref()[..],
        ArenaError::BadUpgrades
    );
    let mut levels = [0u8; DEPARTMENTS];
    levels.copy_from_slice(&data[UPGRADES_LEVELS_AT..UPGRADES_LEVELS_AT + DEPARTMENTS]);
    require!(
        levels.iter().all(|&l| l <= MAX_LEVEL),
        ArenaError::BadUpgrades
    );
    Ok(levels)
}

/// A GARAGE room's levels snapshot and cap check; an EQUAL room races the full
/// car (all levels recorded as 0, the upgrades account not needed).
fn seat_levels(
    class: u8,
    max_rating: u16,
    ruleset: &Ruleset,
    player: &Pubkey,
    upgrades: &Option<UncheckedAccount>,
) -> Result<[u8; DEPARTMENTS]> {
    if class != CLASS_GARAGE {
        return Ok([0; DEPARTMENTS]);
    }
    let acct = upgrades.as_ref().ok_or(ArenaError::BadUpgrades)?;
    let levels = read_levels(player, &acct.to_account_info())?;
    if max_rating > 0 {
        require!(
            ruleset.rating(&levels) <= max_rating,
            ArenaError::RatingTooHigh
        );
    }
    Ok(levels)
}

// ---- create ---------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RoomParams {
    pub stake: u64,
    pub max_players: u8,
    pub join_window_secs: u32,
    pub class: u8,
    /// GARAGE only: the highest rating allowed, in tenths (0 = no cap).
    pub max_rating: u16,
    pub access: u8,
    pub race: RaceSpec,
}

#[derive(Accounts)]
#[instruction(code: [u8; 8])]
pub struct CreateRoom<'info> {
    /// Stakes, and takes the first seat.
    #[account(mut)]
    pub creator: Signer<'info>,
    /// Pays the room's rent (the race server's sponsor, or the creator); it gets
    /// it back when the room closes.
    #[account(mut)]
    pub payer: Signer<'info>,
    pub gate: Option<Signer<'info>>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    pub ruleset: Box<Account<'info, Ruleset>>,
    #[account(seeds = [MINT_SEED, allowed_mint.mint.as_ref()], bump = allowed_mint.bump)]
    pub allowed_mint: Box<Account<'info, AllowedMint>>,
    #[account(
        init,
        payer = payer,
        space = 8 + Room::INIT_SPACE,
        seeds = [ROOM_SEED, &code],
        bump
    )]
    pub room: Box<Account<'info, Room>>,
    /// CHECK: GARAGE rooms: the creator's `["upgrades", creator]` account under the
    /// house program (may not exist yet: stock). Checked in the handler.
    pub upgrades: Option<UncheckedAccount<'info>>,
    /// The creator's project-token account, to claim the holder fee for the room.
    pub holder_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    /// Token rooms only (all four absent for SOL). The escrow is the room's
    /// associated token account; create it (idempotently) earlier in the same
    /// transaction.
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub creator_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_room(
    ctx: Context<CreateRoom>,
    code: [u8; 8],
    params: RoomParams,
) -> Result<()> {
    let a = &ctx.accounts;
    let now = Clock::get()?.unix_timestamp;
    let p = &params;
    require!(!a.config.paused, ArenaError::Paused);
    check_gate(&a.config, &a.gate)?;
    require!(a.ruleset.enabled, ArenaError::RulesetDisabled);
    require!(a.allowed_mint.enabled, ArenaError::MintDisabled);
    require!(
        p.stake >= a.allowed_mint.min_stake && p.stake <= a.allowed_mint.max_stake,
        ArenaError::StakeOutOfRange
    );
    require!(
        (MIN_ROOM_PLAYERS..=MAX_ROOM_PLAYERS).contains(&p.max_players),
        ArenaError::InvalidParams
    );
    require!(
        (MIN_WINDOW_SECS..=MAX_JOIN_WINDOW_SECS).contains(&p.join_window_secs),
        ArenaError::InvalidWindow
    );
    require!(
        (p.class == CLASS_EQUAL && p.max_rating == 0) || p.class == CLASS_GARAGE,
        ArenaError::InvalidParams
    );
    // a race the race server can run, and settle within the settle window
    require!(
        (p.access == ACCESS_OPEN || p.access == ACCESS_INVITE)
            && (1..=MAX_LAPS).contains(&p.race.laps)
            && p.race.format <= 1
            && p.race.time <= 2
            && p.race.sky <= 5
            && p.race.ai <= MAX_RACE_AI
            && p.race.start_mode <= 1,
        ArenaError::InvalidParams
    );

    let token = token_side(
        &a.allowed_mint.mint,
        &a.allowed_mint.token_program,
        a.allowed_mint.transfer_fee,
        &a.room.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    if let Some(t) = &token {
        // vetted again now, not only when it was allowed: a mint closed and made
        // again at the same address could carry new extensions
        require!(
            validate_wager_mint(&t.mint, a.allowed_mint.transfer_fee)?
                == a.allowed_mint.transfer_fee,
            ArenaError::UnsupportedMint
        );
        if t.transfer_fee {
            check_fee_cap(&t.mint, a.allowed_mint.max_transfer_fee_bps)?;
        }
    }

    let creator = a.creator.key();
    let levels = seat_levels(p.class, p.max_rating, &a.ruleset, &creator, &a.upgrades)?;

    // The house fee: the token's, or the holder fee when the creator qualifies.
    let mut fee_bps = a.allowed_mint.room_fee_bps;
    if let Some(h) = a.holder_tokens.as_deref() {
        let c = &a.config;
        // the creator's own associated account: an account whose owner can be
        // moved (no ImmutableOwner) could be lent for one transaction
        let own = get_associated_token_address_with_program_id(
            &creator,
            &c.holder_mint,
            h.to_account_info().owner,
        );
        require!(
            c.holder_mint != Pubkey::default()
                && h.key() == own
                && h.mint == c.holder_mint
                && h.owner == creator
                && h.amount >= c.holder_min
                && p.stake <= a.allowed_mint.holder_max_stake,
            ArenaError::NotAHolder
        );
        fee_bps = fee_bps.min(c.holder_room_fee_bps);
    }

    deposit(
        &a.creator.to_account_info(),
        &a.room.to_account_info(),
        &token,
        a.creator_tokens.as_deref(),
        &a.system_program.to_account_info(),
        p.stake,
    )?;

    let (mint, token_program, transfer_fee, max_fee) = (
        a.allowed_mint.mint,
        a.allowed_mint.token_program,
        a.allowed_mint.transfer_fee,
        a.allowed_mint.max_transfer_fee_bps,
    );
    let (treasury, prize_vault, settle_window_secs) = (
        a.config.treasury,
        a.config.prize_vault,
        a.config.settle_window_secs,
    );
    let ruleset = a.ruleset.key();
    let payer = a.payer.key();
    let key = a.room.key();

    let r = &mut ctx.accounts.room;
    r.state = RoomState::Open;
    r.access = p.access;
    r.class = p.class;
    r.mint = mint;
    r.stake = p.stake;
    r.max_players = p.max_players;
    r.max_rating = p.max_rating;
    r.code = code;
    r.creator = creator;
    r.ruleset = ruleset;
    r.race = p.race;
    r.token_program = token_program;
    r.fee_bps = fee_bps;
    r.transfer_fee = transfer_fee;
    r.max_transfer_fee_bps = max_fee;
    r.treasury = treasury;
    r.prize_vault = prize_vault;
    r.rent_payer = payer;
    r.players = vec![creator];
    r.levels = vec![levels];
    r.created_at = now;
    r.join_deadline = now + p.join_window_secs as i64;
    r.settle_window_secs = settle_window_secs;
    r.bump = ctx.bumps.room;

    emit!(RoomCreated {
        room: key,
        code,
        creator,
        mint,
        stake: p.stake,
        max_players: p.max_players,
        fee_bps,
        class: p.class,
        join_deadline: r.join_deadline,
    });
    Ok(())
}

// ---- join -----------------------------------------------------------------------

#[derive(Accounts)]
pub struct JoinRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    pub gate: Option<Signer<'info>>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
    #[account(address = room.ruleset)]
    pub ruleset: Box<Account<'info, Ruleset>>,
    /// CHECK: GARAGE rooms: the player's upgrades account; checked in the handler.
    pub upgrades: Option<UncheckedAccount<'info>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub player_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

/// `expected_stake`: the stake the player saw - a room closed and opened again
/// under the same code with another stake can't take theirs.
pub fn handle_join_room(ctx: Context<JoinRoom>, expected_stake: u64) -> Result<()> {
    let a = &ctx.accounts;
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    let player = a.player.key();
    require!(!a.config.paused, ArenaError::Paused);
    check_gate(&a.config, &a.gate)?;
    require!(a.room.state == RoomState::Open, ArenaError::RoomNotOpen);
    require!(a.room.stake == expected_stake, ArenaError::StakeChanged);
    require!(now < a.room.join_deadline, ArenaError::JoinClosed);
    require!(
        a.room.players.len() < a.room.max_players as usize,
        ArenaError::RoomFull
    );
    require!(!a.room.players.contains(&player), ArenaError::AlreadyJoined);
    let levels = seat_levels(
        a.room.class,
        a.room.max_rating,
        &a.ruleset,
        &player,
        &a.upgrades,
    )?;

    let token = token_side(
        &a.room.mint,
        &a.room.token_program,
        a.room.transfer_fee,
        &a.room.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    if let Some(t) = &token {
        if t.transfer_fee {
            check_fee_cap(&t.mint, a.room.max_transfer_fee_bps)?;
        }
    }
    deposit(
        &a.player.to_account_info(),
        &a.room.to_account_info(),
        &token,
        a.player_tokens.as_deref(),
        &a.system_program.to_account_info(),
        a.room.stake,
    )?;

    let key = a.room.key();
    let r = &mut ctx.accounts.room;
    r.players.push(player);
    r.levels.push(levels);
    emit!(RoomJoined {
        room: key,
        player,
        players: r.players.len() as u8,
    });
    // a room that starts when full locks as it fills (its seed comes next)
    if r.players.len() == r.max_players as usize && r.race.start_mode == 0 {
        lock(r, key, now, clock.slot);
    }
    Ok(())
}

/// No more joins or leaves. The seed is not drawn here: whoever sends the
/// locking transaction (the last player to join, who holds it signed) could
/// otherwise see it and land it only when it suits them.
fn lock(r: &mut Room, key: Pubkey, now: i64, slot: u64) {
    r.state = RoomState::Locked;
    r.seed = [0u8; 32];
    r.locked_at = now;
    r.locked_slot = slot;
    r.settle_deadline = now + r.settle_window_secs as i64;
    emit!(RoomLocked {
        room: key,
        locked_slot: slot,
        players: r.players.len() as u8,
        settle_deadline: r.settle_deadline,
    });
}

// ---- the seed -------------------------------------------------------------------

#[derive(Accounts)]
pub struct DrawSeed<'info> {
    #[account(mut, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
    /// CHECK: the SlotHashes sysvar, read raw.
    #[account(address = solana_sdk_ids::sysvar::slot_hashes::ID)]
    pub slot_hashes: UncheckedAccount<'info>,
}

/// Anyone, once the slot the room locked in is over: the race's seed (its grid
/// order, its lights) from that slot's hash, which didn't exist while the
/// locking transaction ran - so nobody, the server included, could see it or
/// choose it. Within ~512 slots (a few minutes); after that the room can only be
/// voided or refunded.
pub fn handle_draw_seed(ctx: Context<DrawSeed>) -> Result<()> {
    let key = ctx.accounts.room.key();
    let r = &ctx.accounts.room;
    require!(r.state == RoomState::Locked, ArenaError::RoomNotLocked);
    require!(r.seed == [0u8; 32], ArenaError::SeedDrawn);
    let hash = slot_hash(&ctx.accounts.slot_hashes.to_account_info(), r.locked_slot)?.ok_or(
        if Clock::get()?.slot > r.locked_slot {
            ArenaError::SeedExpired
        } else {
            ArenaError::TooEarly
        },
    )?;
    let seed = hashv(&[key.as_ref(), &hash, &r.locked_slot.to_le_bytes()]).to_bytes();
    ctx.accounts.room.seed = seed;
    emit!(RoomSeeded { room: key, seed });
    Ok(())
}

// ---- leave / cancel (open rooms only) ---------------------------------------------

#[derive(Accounts)]
pub struct LeaveRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    /// Token rooms: where the stake goes back to (owned by the player).
    #[account(mut)]
    pub player_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
}

pub fn handle_leave_room<'info>(ctx: Context<'info, LeaveRoom<'info>>) -> Result<()> {
    let a = &ctx.accounts;
    let player = a.player.key();
    require!(a.room.state == RoomState::Open, ArenaError::RoomNotOpen);
    require!(player != a.room.creator, ArenaError::CreatorCannotLeave);
    let idx = a
        .room
        .players
        .iter()
        .position(|p| *p == player)
        .ok_or(ArenaError::NotAPlayer)?;

    let token = token_side(
        &a.room.mint,
        &a.room.token_program,
        a.room.transfer_fee,
        &a.room.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    let to = match &a.player_tokens {
        Some(t) => t.to_account_info(),
        None => a.player.to_account_info(),
    };
    let (_, memo) = split_memo(ctx.remaining_accounts);
    with_escrow(&a.room, token, memo, |escrow| {
        escrow.pay(&to, &player, a.room.stake)
    })?;

    let key = a.room.key();
    let r = &mut ctx.accounts.room;
    r.players.remove(idx);
    r.levels.remove(idx);
    emit!(RoomLeft { room: key, player });
    Ok(())
}

/// Accounts shared by every instruction that pays out of a room's escrow.
#[derive(Accounts)]
pub struct RoomPayout<'info> {
    #[account(mut, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
}

impl<'info> RoomPayout<'info> {
    fn token(&self) -> Result<Option<TokenSide<'info>>> {
        token_side(
            &self.room.mint,
            &self.room.token_program,
            self.room.transfer_fee,
            &self.room.key(),
            self.mint.as_deref(),
            self.escrow_tokens.as_deref(),
            &self.token_program,
        )
    }

    /// Every stake back, no fee; the room ends Refunded with why.
    fn refund_all(
        &mut self,
        payouts: &[AccountInfo<'info>],
        reason: u8,
        record_hash: [u8; 32],
        by: Pubkey,
    ) -> Result<()> {
        let token = self.token()?;
        let (payouts, memo) = split_memo(payouts);
        with_escrow(&self.room, token, memo, |e| {
            refund_everyone(&self.room, e, payouts)
        })?;
        let key = self.room.key();
        self.room.state = RoomState::Refunded;
        self.room.void_reason = reason;
        self.room.record_hash = record_hash;
        emit!(RoomRefunded {
            room: key,
            reason,
            record_hash,
            by,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CancelRoom<'info> {
    pub creator: Signer<'info>,
    #[account(constraint = payout.room.creator == creator.key() @ ArenaError::Unauthorized)]
    pub payout: RoomPayout<'info>,
}

pub fn handle_cancel_room<'info>(ctx: Context<'info, CancelRoom<'info>>) -> Result<()> {
    require!(
        ctx.accounts.payout.room.state == RoomState::Open,
        ArenaError::RoomNotOpen
    );
    let by = ctx.accounts.creator.key();
    ctx.accounts
        .payout
        .refund_all(ctx.remaining_accounts, VOID_CANCELLED, [0u8; 32], by)
}

// ---- the resolver: lock, settle, void ----------------------------------------------

#[derive(Accounts)]
pub struct LockRoom<'info> {
    pub resolver: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump,
        constraint = config.is_resolver(&resolver.key()) @ ArenaError::Unauthorized)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
}

/// A room its creator starts (two players or more), before its join deadline:
/// no more joins or leaves (its seed comes next). A room that starts when full
/// locks itself as it fills.
pub fn handle_lock_room(ctx: Context<LockRoom>) -> Result<()> {
    let clock = Clock::get()?;
    let key = ctx.accounts.room.key();
    let r = &mut ctx.accounts.room;
    require!(r.state == RoomState::Open, ArenaError::RoomNotOpen);
    require!(
        r.race.start_mode == 1 && clock.unix_timestamp < r.join_deadline,
        ArenaError::NotLockable
    );
    require!(
        r.players.len() >= MIN_ROOM_PLAYERS as usize,
        ArenaError::NotEnoughPlayers
    );
    lock(r, key, clock.unix_timestamp, clock.slot);
    Ok(())
}

#[derive(Accounts)]
pub struct SettleRoom<'info> {
    pub resolver: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump,
        constraint = config.is_resolver(&resolver.key()) @ ArenaError::Unauthorized)]
    pub config: Box<Account<'info, Config>>,
    pub payout: RoomPayout<'info>,
    /// CHECK: the room's treasury (SOL) or its token account (SPL); checked when paid.
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
}

/// The race's result: one time per seat (`DNF` for no result) and the hash of
/// its full record. The fastest takes the pot less the frozen fee, paid now;
/// tied best times split it; everyone tied (or nobody finished) refunds all.
/// remaining_accounts: the WINNERS' payout accounts in seat order (every
/// player's, when it comes out as a refund).
pub fn handle_settle_room<'info>(
    ctx: Context<'info, SettleRoom<'info>>,
    times_ms: Vec<u32>,
    record_hash: [u8; 32],
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let resolver = ctx.accounts.resolver.key();
    let p = &mut ctx.accounts.payout;
    require!(p.room.state == RoomState::Locked, ArenaError::RoomNotLocked);
    require!(p.room.seed != [0u8; 32], ArenaError::SeedNotDrawn);
    require!(now < p.room.settle_deadline, ArenaError::TooLate);
    require!(
        times_ms.len() == p.room.players.len(),
        ArenaError::InvalidResults
    );
    let outcome = room_outcome(p.room.stake, p.room.fee_bps, &times_ms)?;
    p.room.times_ms = times_ms.clone();
    if outcome.refund_all {
        return p.refund_all(ctx.remaining_accounts, VOID_TIED, record_hash, resolver);
    }

    let (payouts, memo) = split_memo(ctx.remaining_accounts);
    require!(
        payouts.len() == outcome.winners.count_ones() as usize,
        ArenaError::PayoutAccountsMismatch
    );
    let token = p.token()?;
    let treasury = ctx.accounts.treasury.to_account_info();
    with_escrow(&p.room, token, memo, |escrow| {
        let winners = p
            .room
            .players
            .iter()
            .enumerate()
            .filter(|(i, _)| outcome.winners & (1 << i) != 0);
        for ((_, player), to) in winners.zip(payouts) {
            escrow.pay(to, player, outcome.share)?;
        }
        escrow.pay(&treasury, &p.room.treasury, outcome.to_treasury)
    })?;

    let key = p.room.key();
    let r = &mut p.room;
    r.state = RoomState::Settled;
    r.record_hash = record_hash;
    r.winners = outcome.winners;
    r.share = outcome.share;
    r.to_treasury = outcome.to_treasury;
    r.settled_by = resolver;
    emit!(RoomSettled {
        room: key,
        times_ms,
        record_hash,
        winners: outcome.winners,
        share: outcome.share,
        to_treasury: outcome.to_treasury,
        settled_by: resolver,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct VoidRoom<'info> {
    pub resolver: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump,
        constraint = config.is_resolver(&resolver.key()) @ ArenaError::Unauthorized)]
    pub config: Box<Account<'info, Config>>,
    pub payout: RoomPayout<'info>,
}

/// The race couldn't produce a result (the server stopped, fewer than two on
/// the grid, a result that didn't check out): every stake back, no fee.
/// remaining_accounts: every player's payout account, in seat order.
pub fn handle_void_room<'info>(
    ctx: Context<'info, VoidRoom<'info>>,
    reason: u8,
    record_hash: [u8; 32],
) -> Result<()> {
    require!(
        matches!(
            ctx.accounts.payout.room.state,
            RoomState::Open | RoomState::Locked
        ),
        ArenaError::NotRefundable
    );
    let by = ctx.accounts.resolver.key();
    ctx.accounts.payout.refund_all(
        ctx.remaining_accounts,
        reason.max(VOID_RACE),
        record_hash,
        by,
    )
}

/// Anyone, once a deadline has passed with no settlement: every stake back, no fee.
#[derive(Accounts)]
pub struct RefundRoom<'info> {
    pub payout: RoomPayout<'info>,
}

pub fn handle_refund_room<'info>(ctx: Context<'info, RefundRoom<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let r = &ctx.accounts.payout.room;
    let expired = match r.state {
        RoomState::Open => now >= r.join_deadline,
        RoomState::Locked => now >= r.settle_deadline,
        _ => return err!(ArenaError::NotRefundable),
    };
    require!(expired, ArenaError::TooEarly);
    ctx.accounts.payout.refund_all(
        ctx.remaining_accounts,
        VOID_EXPIRED,
        [0u8; 32],
        Pubkey::default(),
    )
}

// ---- close ----------------------------------------------------------------------

#[derive(Accounts)]
pub struct CloseRoom<'info> {
    #[account(mut, close = rent_payer, seeds = [ROOM_SEED, &room.code], bump = room.bump)]
    pub room: Box<Account<'info, Room>>,
    /// CHECK: receives the rent; must be whoever paid it.
    #[account(mut, address = room.rent_payer)]
    pub rent_payer: UncheckedAccount<'info>,
    /// CHECK: the prize vault: SOL left above what the room owed goes here.
    #[account(mut, address = room.prize_vault)]
    pub prize_vault: UncheckedAccount<'info>,
    /// CHECK: token rooms: the prize vault's token account of the room's mint, for
    /// tokens left in the escrow (needed only when there are some); checked when paid.
    #[account(mut)]
    pub prize_vault_tokens: Option<UncheckedAccount<'info>>,
    /// Writable: closing a transfer-fee token's escrow harvests its withheld fees into the mint.
    #[account(mut)]
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
}

/// Anyone, after settlement or refund. Anything still in the escrow above what
/// the room owed (a donation, a reward pushed to it) goes to the prize vault;
/// transfer fees withheld in the escrow are harvested to the mint; the rent goes
/// back to whoever paid it.
pub fn handle_close_room<'info>(ctx: Context<'info, CloseRoom<'info>>) -> Result<()> {
    let a = &ctx.accounts;
    require!(
        matches!(a.room.state, RoomState::Settled | RoomState::Refunded),
        ArenaError::NotClosable
    );
    let token = token_side(
        &a.room.mint,
        &a.room.token_program,
        a.room.transfer_fee,
        &a.room.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    let vault = a.room.prize_vault;
    let is_sol = a.room.is_sol();
    let vault_tokens = a.prize_vault_tokens.as_ref().map(|t| t.to_account_info());
    let (_, memo) = split_memo(ctx.remaining_accounts);
    with_escrow(&a.room, token, memo, |e| {
        if !is_sol {
            e.close_token_escrow(
                vault_tokens.as_ref(),
                &vault,
                &a.rent_payer.to_account_info(),
            )?;
        }
        Ok(())
    })?;
    // SOL above the room's own rent (any room: a SOL room's leftovers, or a reward
    // pushed to a token room's escrow PDA) goes to the prize vault itself
    let room_info = a.room.to_account_info();
    let surplus = lamports_above_rent(&room_info)?;
    transfer_from_pda(&room_info, &a.prize_vault.to_account_info(), surplus)?;
    emit!(RoomClosed {
        room: a.room.key(),
        surplus,
    });
    Ok(())
}

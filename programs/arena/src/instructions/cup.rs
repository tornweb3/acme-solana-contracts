//! Cups: prize events over a window. A Board cup (the free weekly time trial)
//! takes no entries on-chain: its prizes are the guarantee and its board is the
//! race server's. An Entries cup takes entries (with or without a fee).
//!
//!   create_cup ─► enter_cup … ─► (ends_at) post_results (resolver)
//!     ─► challenge window (guardian/admin may reject_results) ─► finalize (anyone)
//!     ─► claim_prize per paid place: the winner, until claim_until (and may swap
//!        the prize into the featured stock in the same transaction); then anyone
//!        pushes it to the winner ─► close_cup
//!   No results by results_deadline, or cancelled: refund_entry per entry +
//!   return_guarantee, then close. A place that can never be paid (its winner
//!   unreachable) doesn't hold the rest: `FORFEIT_SECS` after the claim window
//!   the cup closes anyway, the unpaid prize back to whoever funded it.
//!
//! The resolver posts only a Merkle root of (rank, wallet); the program computes
//! every amount from the payout table, so a resolver can misorder players (which
//! the published standings and the challenge window exist to catch) but can never
//! pay out more than the pool, or pay anyone the root doesn't name.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    constants::*,
    error::ArenaError,
    escrow::{
        check_fee_cap, deposit, lamports_above_rent, split_memo, token_side, transfer_from_pda,
        validate_wager_mint, Escrow, TokenSide,
    },
    events::*,
    merkle,
    payout::{cup_leftover, cup_pool, cup_prize, payout_table_ok},
    state::*,
};

/// The most a community cup's host may take of the house fee.
pub const MAX_HOST_SHARE_BPS: u16 = 5_000;

/// `memo`: the memo program, when passed as the last remaining account (a
/// payout into a token account that requires memos is preceded by one).
fn with_escrow<'info, R>(
    c: &Account<'info, Cup>,
    token: Option<TokenSide<'info>>,
    memo: Option<AccountInfo<'info>>,
    f: impl FnOnce(&Escrow<'_, 'info>) -> Result<R>,
) -> Result<R> {
    let id = c.id.to_le_bytes();
    let bump = [c.bump];
    let seeds: [&[u8]; 3] = [CUP_SEED, &id, &bump];
    f(&Escrow {
        pda: c.to_account_info(),
        seeds: &seeds,
        token,
        memo,
    })
}

/// Token-side accounts for instructions that pay out of a cup's escrow.
#[derive(Accounts)]
pub struct CupVault<'info> {
    #[account(mut, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
}

impl<'info> CupVault<'info> {
    fn token(&self) -> Result<Option<TokenSide<'info>>> {
        token_side(
            &self.cup.mint,
            &self.cup.token_program,
            self.cup.transfer_fee,
            &self.cup.key(),
            self.mint.as_deref(),
            self.escrow_tokens.as_deref(),
            &self.token_program,
        )
    }
}

// ---- create ---------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CupParams {
    pub kind: CupKind,
    pub class: u8,
    pub entry_fee: u64,
    pub payout_bps: Vec<u16>,
    pub guarantee: u64,
    pub starts_at: i64,
    pub ends_at: i64,
    pub max_entrants: u32,
    pub spec_hash: [u8; 32],
    /// The event's featured stock (`Pubkey::default()` = none).
    pub featured_mint: Pubkey,
    /// Community cups only: the host's share of the house fee.
    pub host_share_bps: u16,
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct CreateCup<'info> {
    /// Organizer and sponsor: pays the guarantee (if any).
    #[account(mut)]
    pub organizer: Signer<'info>,
    /// Pays the cup's rent (the organizer, or the house keeper).
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    pub ruleset: Box<Account<'info, Ruleset>>,
    #[account(seeds = [MINT_SEED, allowed_mint.mint.as_ref()], bump = allowed_mint.bump)]
    pub allowed_mint: Box<Account<'info, AllowedMint>>,
    #[account(
        init,
        payer = payer,
        space = 8 + Cup::INIT_SPACE,
        seeds = [CUP_SEED, &id.to_le_bytes()],
        bump
    )]
    pub cup: Box<Account<'info, Cup>>,
    /// An admin-approved host's record (the path with no token).
    #[account(seeds = [HOST_SEED, organizer.key().as_ref()], bump = host.bump)]
    pub host: Option<Box<Account<'info, Host>>>,
    /// A token-holding host's project-token account.
    pub holder_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    /// The cup's associated token account; create it earlier in the transaction.
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub organizer_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_cup(ctx: Context<CreateCup>, id: u64, params: CupParams) -> Result<()> {
    let a = &ctx.accounts;
    let now = Clock::get()?.unix_timestamp;
    let p = &params;
    let organizer = a.organizer.key();
    let cfg = &a.config;
    require!(!cfg.paused, ArenaError::Paused);
    // (the id is the creator's choice - a random one can't be taken first; the
    // cup's address, ["cup", id], must be free)
    require!(a.ruleset.enabled, ArenaError::RulesetDisabled);
    require!(a.allowed_mint.enabled, ArenaError::MintDisabled);
    require!(
        payout_table_ok(&p.payout_bps),
        ArenaError::InvalidPayoutTable
    );
    require!(
        p.ends_at > p.starts_at && p.ends_at > now && p.ends_at - now <= MAX_CUP_SECS,
        ArenaError::InvalidWindow
    );
    require!(
        p.class == CLASS_EQUAL || p.class == CLASS_GARAGE,
        ArenaError::InvalidParams
    );
    match p.kind {
        // a board: no entries, its prizes are the guarantee
        CupKind::Board => require!(
            p.entry_fee == 0 && p.max_entrants == 0 && p.guarantee > 0,
            ArenaError::InvalidParams
        ),
        CupKind::Entries if p.entry_fee > 0 => require!(
            p.entry_fee >= a.allowed_mint.min_stake && p.entry_fee <= a.allowed_mint.max_stake,
            ArenaError::StakeOutOfRange
        ),
        CupKind::Entries => require!(p.guarantee > 0, ArenaError::InvalidParams), // a free roll needs a prize
    }

    // Who may organize: the house (admin, a resolver, the prize vault the house
    // program signs for), or a community host: admin-approved, or holding enough
    // of the project token. Only a community cup has a host and a host share.
    let house =
        organizer == cfg.admin || cfg.is_resolver(&organizer) || organizer == cfg.prize_vault;
    let mut approved = false;
    if !house {
        approved = a
            .host
            .as_ref()
            .is_some_and(|h| h.wallet == organizer && h.until > now);
        let holder = a.holder_tokens.as_ref().is_some_and(|h| {
            cfg.holder_mint != Pubkey::default()
                && h.mint == cfg.holder_mint
                && h.owner == organizer
                && h.amount >= cfg.host_min
        });
        require!(approved || holder, ArenaError::Unauthorized);
        // a share of the fee only for an admin-approved host: a balance checked
        // for one instant (it could be borrowed for that instant) earns none
        require!(
            p.host_share_bps <= if approved { MAX_HOST_SHARE_BPS } else { 0 },
            ArenaError::InvalidParams
        );
    }

    let token = token_side(
        &a.allowed_mint.mint,
        &a.allowed_mint.token_program,
        a.allowed_mint.transfer_fee,
        &a.cup.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    if let Some(t) = &token {
        require!(
            validate_wager_mint(&t.mint, a.allowed_mint.transfer_fee)?
                == a.allowed_mint.transfer_fee,
            ArenaError::UnsupportedMint
        );
        if t.transfer_fee {
            check_fee_cap(&t.mint, a.allowed_mint.max_transfer_fee_bps)?;
        }
    }
    deposit(
        &a.organizer.to_account_info(),
        &a.cup.to_account_info(),
        &token,
        a.organizer_tokens.as_deref(),
        &a.system_program.to_account_info(),
        p.guarantee,
    )?;

    let ruleset = a.ruleset.key();
    let rent_payer = a.payer.key();
    let (mint, token_program, transfer_fee, max_fee) = (
        a.allowed_mint.mint,
        a.allowed_mint.token_program,
        a.allowed_mint.transfer_fee,
        a.allowed_mint.max_transfer_fee_bps,
    );
    let (fee_bps, treasury, prize_vault) =
        (a.allowed_mint.cup_fee_bps, cfg.treasury, cfg.prize_vault);
    let (results_window, challenge_window, claim_window) = (
        cfg.results_window_secs,
        cfg.challenge_window_secs,
        cfg.claim_window_secs,
    );
    let key = a.cup.key();

    let cfg = &mut ctx.accounts.config;
    cfg.cup_count = cfg.cup_count.checked_add(1).ok_or(ArenaError::Overflow)?;
    let c = &mut ctx.accounts.cup;
    c.id = id;
    c.kind = p.kind;
    c.class = p.class;
    c.organizer = organizer;
    c.sponsor = organizer;
    c.rent_payer = rent_payer;
    c.host = if house { Pubkey::default() } else { organizer };
    c.host_share_bps = if approved { p.host_share_bps } else { 0 };
    c.ruleset = ruleset;
    c.spec_hash = p.spec_hash;
    c.featured_mint = p.featured_mint;
    c.mint = mint;
    c.token_program = token_program;
    c.transfer_fee = transfer_fee;
    c.max_transfer_fee_bps = max_fee;
    c.entry_fee = p.entry_fee;
    c.fee_bps = fee_bps;
    c.treasury = treasury;
    c.prize_vault = prize_vault;
    c.payout_bps = p.payout_bps.clone();
    c.guarantee = p.guarantee;
    c.starts_at = p.starts_at;
    c.ends_at = p.ends_at;
    c.results_deadline = p.ends_at + results_window as i64;
    c.challenge_window_secs = challenge_window;
    c.claim_window_secs = claim_window;
    c.max_entrants = p.max_entrants;
    c.state = CupState::Open;
    c.bump = ctx.bumps.cup;

    emit!(CupCreated {
        cup: key,
        id,
        organizer,
        mint,
        entry_fee: c.entry_fee,
        guarantee: c.guarantee,
        featured_mint: c.featured_mint,
        ends_at: c.ends_at,
    });
    Ok(())
}

// ---- enter (Entries cups) ---------------------------------------------------------

#[derive(Accounts)]
pub struct EnterCup<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    /// Pays the entry's rent (the race server's sponsor, or the player).
    #[account(mut)]
    pub payer: Signer<'info>,
    pub gate: Option<Signer<'info>>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
    #[account(
        init,
        payer = payer,
        space = 8 + Entry::INIT_SPACE,
        seeds = [ENTRY_SEED, cup.key().as_ref(), player.key().as_ref()],
        bump
    )]
    pub entry: Box<Account<'info, Entry>>,
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub player_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

pub fn handle_enter_cup(ctx: Context<EnterCup>) -> Result<()> {
    let a = &ctx.accounts;
    let now = Clock::get()?.unix_timestamp;
    require!(!a.config.paused, ArenaError::Paused);
    if a.config.gate != Pubkey::default() {
        let g = a.gate.as_ref().ok_or(ArenaError::GateRequired)?;
        require_keys_eq!(g.key(), a.config.gate, ArenaError::GateRequired);
    }
    require!(a.cup.kind == CupKind::Entries, ArenaError::NoEntries);
    require!(
        a.cup.state == CupState::Open && now < a.cup.ends_at,
        ArenaError::CupNotOpen
    );
    require!(
        a.cup.max_entrants == 0 || a.cup.entrants < a.cup.max_entrants,
        ArenaError::CupFull
    );
    let token = token_side(
        &a.cup.mint,
        &a.cup.token_program,
        a.cup.transfer_fee,
        &a.cup.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    if let Some(t) = &token {
        if t.transfer_fee {
            check_fee_cap(&t.mint, a.cup.max_transfer_fee_bps)?;
        }
    }
    deposit(
        &a.player.to_account_info(),
        &a.cup.to_account_info(),
        &token,
        a.player_tokens.as_deref(),
        &a.system_program.to_account_info(),
        a.cup.entry_fee,
    )?;

    let player = a.player.key();
    let rent_payer = a.payer.key();
    let key = a.cup.key();
    let e = &mut ctx.accounts.entry;
    e.cup = key;
    e.player = player;
    e.rent_payer = rent_payer;
    e.refunded = false;
    e.bump = ctx.bumps.entry;
    let c = &mut ctx.accounts.cup;
    c.entrants = c.entrants.checked_add(1).ok_or(ArenaError::Overflow)?;
    emit!(CupEntered {
        cup: key,
        player,
        entrants: c.entrants,
    });
    Ok(())
}

// ---- results --------------------------------------------------------------------

#[derive(Accounts)]
pub struct PostResults<'info> {
    pub resolver: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump,
        constraint = config.is_resolver(&resolver.key()) @ ArenaError::Unauthorized)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
}

/// `places`: the number of leaves the root was built over - it must be what the
/// cup will pay (min(table, scorers)), so no paid place is left without a leaf.
pub fn handle_post_results(
    ctx: Context<PostResults>,
    results_root: [u8; 32],
    scorers: u32,
    places: u8,
    standings_hash: [u8; 32],
    results_uri: String,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let key = ctx.accounts.cup.key();
    let c = &mut ctx.accounts.cup;
    require!(c.state == CupState::Open, ArenaError::CupNotOpen);
    require!(now >= c.ends_at, ArenaError::TooEarly);
    require!(now < c.results_deadline, ArenaError::TooLate);
    require!(
        scorers >= 1 && (c.kind == CupKind::Board || scorers <= c.entrants),
        ArenaError::InvalidParams
    );
    require!(
        places as usize == core::cmp::min(c.payout_bps.len(), scorers as usize)
            && results_root != [0u8; 32],
        ArenaError::InvalidParams
    );
    require!(results_uri.len() <= MAX_URI_LEN, ArenaError::UriTooLong);
    c.results_root = results_root;
    c.scorers = scorers;
    c.standings_hash = standings_hash;
    c.results_uri = results_uri.clone();
    c.places = places;
    c.posted_at = now;
    c.challenge_ends = now + c.challenge_window_secs as i64;
    c.state = CupState::Posted;
    emit!(ResultsPosted {
        cup: key,
        results_root,
        scorers,
        standings_hash,
        results_uri,
        challenge_ends: c.challenge_ends,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct GuardCup<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
}

/// Guardian or admin, during the challenge window: the posted results are
/// wrong. The resolver may post again until `results_deadline`; after that
/// every entry can be refunded.
pub fn handle_reject_results(ctx: Context<GuardCup>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let who = ctx.accounts.authority.key();
    let cfg = &ctx.accounts.config;
    require!(
        who == cfg.guardian || who == cfg.admin,
        ArenaError::Unauthorized
    );
    let key = ctx.accounts.cup.key();
    let c = &mut ctx.accounts.cup;
    require!(c.state == CupState::Posted, ArenaError::ResultsNotPosted);
    require!(now < c.challenge_ends, ArenaError::TooLate);
    c.state = CupState::Open;
    c.results_root = [0u8; 32];
    c.scorers = 0;
    c.places = 0;
    emit!(ResultsRejected { cup: key, by: who });
    Ok(())
}

/// Admin, guardian or a resolver, any time before finalization. Cancelling
/// only ever returns money to the people who put it in.
pub fn handle_cancel_cup(ctx: Context<GuardCup>) -> Result<()> {
    let who = ctx.accounts.authority.key();
    let cfg = &ctx.accounts.config;
    require!(
        who == cfg.guardian || who == cfg.admin || cfg.is_resolver(&who),
        ArenaError::Unauthorized
    );
    let key = ctx.accounts.cup.key();
    let c = &mut ctx.accounts.cup;
    require!(
        matches!(c.state, CupState::Open | CupState::Posted),
        ArenaError::NotRefundable
    );
    c.state = CupState::Cancelled;
    emit!(CupCancelled { cup: key, by: who });
    Ok(())
}

#[derive(Accounts)]
pub struct FinalizeCup<'info> {
    #[account(mut, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
}

/// Anyone, once the challenge window has passed without a rejection: the pool
/// is fixed and the claim window opens.
pub fn handle_finalize_cup(ctx: Context<FinalizeCup>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let key = ctx.accounts.cup.key();
    let c = &mut ctx.accounts.cup;
    require!(c.state == CupState::Posted, ArenaError::ResultsNotPosted);
    require!(now >= c.challenge_ends, ArenaError::TooEarly);
    let p = cup_pool(
        c.entry_fee,
        c.entrants,
        c.fee_bps,
        c.guarantee,
        &c.payout_bps,
        c.scorers,
    )?;
    c.pool = p.pool;
    c.places = p.places;
    c.used_bps = p.used_bps;
    c.claim_until = now + c.claim_window_secs as i64;
    c.state = CupState::Finalized;
    emit!(CupFinalized {
        cup: key,
        pool: p.pool,
        places: p.places,
        claim_until: c.claim_until,
    });
    Ok(())
}

// ---- prizes ---------------------------------------------------------------------

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    /// The winner (until `claim_until`), or anyone pushing it after.
    pub claimer: Signer<'info>,
    pub vault: CupVault<'info>,
    /// Entries cups: the winner's entry (it must not have been refunded).
    pub entry: Option<Box<Account<'info, Entry>>>,
    /// CHECK: the winner's wallet (SOL) or a token account owned by the winner
    /// (SPL); checked when paid.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
}

/// Pays rank `rank` its prize, always to `wallet` (the leaf's). Until
/// `claim_until` only the winner may sign, so they can swap the prize into the
/// featured stock in the same transaction; after it anyone pushes it to them.
pub fn handle_claim_prize<'info>(
    ctx: Context<'info, ClaimPrize<'info>>,
    rank: u16,
    wallet: Pubkey,
    proof: Vec<[u8; 32]>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let v = &mut ctx.accounts.vault;
    let key = v.cup.key();
    require!(v.cup.state == CupState::Finalized, ArenaError::NotFinalized);
    require!(
        rank >= 1 && (rank as usize) <= v.cup.places as usize,
        ArenaError::InvalidRank
    );
    let bit = 1u32 << (rank - 1);
    require!(v.cup.claimed & bit == 0, ArenaError::AlreadyClaimed);
    let pushed = ctx.accounts.claimer.key() != wallet;
    require!(!pushed || now >= v.cup.claim_until, ArenaError::TooEarly);
    if v.cup.kind == CupKind::Entries {
        let entry = ctx
            .accounts
            .entry
            .as_ref()
            .ok_or(ArenaError::Unauthorized)?;
        let (want, _) =
            Pubkey::find_program_address(&[ENTRY_SEED, key.as_ref(), wallet.as_ref()], &crate::ID);
        require_keys_eq!(entry.key(), want, ArenaError::Unauthorized);
        require!(!entry.refunded, ArenaError::EntryRefunded);
    }
    require!(proof.len() <= 16, ArenaError::InvalidProof);
    require!(
        merkle::verify(
            &proof,
            &v.cup.results_root,
            merkle::leaf(&key, rank, &wallet)
        ),
        ArenaError::InvalidProof
    );
    let c = &v.cup;
    let amount = cup_prize(c.pool, c.payout_bps[rank as usize - 1], c.used_bps)?;
    let token = v.token()?;
    let (_, memo) = split_memo(ctx.remaining_accounts);
    with_escrow(&v.cup, token, memo, |e| {
        e.pay(&ctx.accounts.destination.to_account_info(), &wallet, amount)
    })?;
    v.cup.claimed |= bit;
    v.cup.paid = v.cup.paid.checked_add(amount).ok_or(ArenaError::Overflow)?;
    emit!(PrizeClaimed {
        cup: key,
        rank,
        wallet,
        amount,
        pushed,
    });
    Ok(())
}

// ---- refunds --------------------------------------------------------------------

#[derive(Accounts)]
pub struct RefundEntry<'info> {
    pub vault: CupVault<'info>,
    #[account(
        mut,
        seeds = [ENTRY_SEED, vault.cup.key().as_ref(), entry.player.as_ref()],
        bump = entry.bump
    )]
    pub entry: Box<Account<'info, Entry>>,
    /// CHECK: the entrant's wallet (SOL) or token account (SPL); checked when paid.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
}

/// Anyone: a cancelled cup, or one whose results never arrived by
/// `results_deadline`, returns every entry fee in full.
pub fn handle_refund_entry<'info>(ctx: Context<'info, RefundEntry<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let v = &mut ctx.accounts.vault;
    let key = v.cup.key();
    match v.cup.state {
        CupState::Cancelled => {}
        CupState::Open if now >= v.cup.results_deadline => {
            v.cup.state = CupState::Cancelled;
            emit!(CupCancelled {
                cup: key,
                by: Pubkey::default(),
            });
        }
        _ => return err!(ArenaError::NotRefundable),
    }
    let entry = &mut ctx.accounts.entry;
    require!(!entry.refunded, ArenaError::EntryRefunded);
    let amount = v.cup.entry_fee;
    let player = entry.player;
    let token = v.token()?;
    let (_, memo) = split_memo(ctx.remaining_accounts);
    with_escrow(&v.cup, token, memo, |e| {
        e.pay(&ctx.accounts.destination.to_account_info(), &player, amount)
    })?;
    entry.refunded = true;
    v.cup.refunded = v.cup.refunded.checked_add(1).ok_or(ArenaError::Overflow)?;
    emit!(EntryRefunded {
        cup: key,
        player,
        amount,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ReturnGuarantee<'info> {
    pub vault: CupVault<'info>,
    /// CHECK: the sponsor's wallet (SOL) or token account (SPL); checked when paid.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
}

/// Anyone: the guarantee goes back to the sponsor once the cup is cancelled, or
/// once its results deadline has passed with no results (which cancels it).
pub fn handle_return_guarantee<'info>(ctx: Context<'info, ReturnGuarantee<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let v = &mut ctx.accounts.vault;
    let key = v.cup.key();
    if v.cup.state == CupState::Open && now >= v.cup.results_deadline {
        v.cup.state = CupState::Cancelled;
        emit!(CupCancelled {
            cup: key,
            by: Pubkey::default(),
        });
    }
    require!(
        v.cup.state == CupState::Cancelled,
        ArenaError::NotRefundable
    );
    require!(!v.cup.guarantee_returned, ArenaError::NotRefundable);
    let (amount, sponsor) = (v.cup.guarantee, v.cup.sponsor);
    let token = v.token()?;
    let (_, memo) = split_memo(ctx.remaining_accounts);
    with_escrow(&v.cup, token, memo, |e| {
        e.pay(
            &ctx.accounts.destination.to_account_info(),
            &sponsor,
            amount,
        )
    })?;
    v.cup.guarantee_returned = true;
    Ok(())
}

// ---- close ----------------------------------------------------------------------

#[derive(Accounts)]
pub struct CloseCup<'info> {
    #[account(mut, close = rent_payer, seeds = [CUP_SEED, &cup.id.to_le_bytes()], bump = cup.bump)]
    pub cup: Box<Account<'info, Cup>>,
    /// CHECK: receives the rent (the cup's and its token account's); must be whoever paid it.
    #[account(mut, address = cup.rent_payer)]
    pub rent_payer: UncheckedAccount<'info>,
    /// CHECK: the sponsor's wallet / token account; checked when paid.
    #[account(mut)]
    pub sponsor_destination: UncheckedAccount<'info>,
    /// CHECK: the host's wallet / token account (community cups); checked when paid.
    #[account(mut)]
    pub host_destination: Option<UncheckedAccount<'info>>,
    /// CHECK: the treasury wallet / token account; checked when paid.
    #[account(mut)]
    pub treasury_destination: UncheckedAccount<'info>,
    /// CHECK: the prize vault: SOL left above what the cup owed goes here.
    #[account(mut, address = cup.prize_vault)]
    pub prize_vault: UncheckedAccount<'info>,
    /// CHECK: token cups: the prize vault's token account, for tokens left above
    /// what the cup owed (needed only when there are some); checked when paid.
    #[account(mut)]
    pub prize_vault_tokens: Option<UncheckedAccount<'info>>,
    /// Writable: closing a transfer-fee token's escrow harvests its withheld fees into the mint.
    #[account(mut)]
    pub mint: Option<Box<InterfaceAccount<'info, Mint>>>,
    #[account(mut)]
    pub escrow_tokens: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
}

/// Anyone, when nothing is owed to players any more: after every prize was paid
/// (the unused guarantee back to the sponsor, the house fee and dust to the
/// treasury, a host's share to the host), or after a cancellation was fully
/// refunded. Anything above that goes to the prize vault.
pub fn handle_close_cup<'info>(ctx: Context<'info, CloseCup<'info>>) -> Result<()> {
    let a = &ctx.accounts;
    let c = &a.cup;
    let token = token_side(
        &c.mint,
        &c.token_program,
        c.transfer_fee,
        &c.key(),
        a.mint.as_deref(),
        a.escrow_tokens.as_deref(),
        &a.token_program,
    )?;
    let sponsor_to = a.sponsor_destination.to_account_info();
    let treasury_to = a.treasury_destination.to_account_info();
    let rent_payer = a.rent_payer.to_account_info();
    let vault_tokens = a.prize_vault_tokens.as_ref().map(|t| t.to_account_info());
    let (_, memo) = split_memo(ctx.remaining_accounts);
    let is_sol = c.is_sol();
    let (mut to_sponsor, mut to_host, mut to_treasury) = (0, 0, 0);
    match c.state {
        CupState::Finalized => {
            // every prize paid - or, FORFEIT_SECS after the claim window, the
            // ones that could never be paid: a board cup's back to its sponsor,
            // an entries cup's left for the prize vault (never the house's)
            let forfeited = if c.all_claimed() {
                0
            } else {
                let now = Clock::get()?.unix_timestamp;
                require!(
                    now >= c.claim_until.saturating_add(FORFEIT_SECS),
                    ArenaError::NotClosable
                );
                let mut unpaid = 0u64;
                for rank in 1..=c.places as usize {
                    if c.claimed & (1u32 << (rank - 1)) == 0 {
                        unpaid = unpaid
                            .checked_add(cup_prize(c.pool, c.payout_bps[rank - 1], c.used_bps)?)
                            .ok_or(ArenaError::Overflow)?;
                    }
                }
                unpaid
            };
            let settled = c.paid.checked_add(forfeited).ok_or(ArenaError::Overflow)?;
            (to_sponsor, to_host, to_treasury) =
                cup_leftover(c.total_in()?, settled, c.guarantee, c.host_share_bps)?;
            if c.kind == CupKind::Board {
                to_sponsor = to_sponsor
                    .checked_add(forfeited)
                    .ok_or(ArenaError::Overflow)?;
            }
            let host_to = match (&a.host_destination, to_host) {
                (_, 0) => None,
                (Some(h), _) => Some(h.to_account_info()),
                (None, _) => return err!(ArenaError::PayoutAccountsMismatch),
            };
            with_escrow(c, token, memo, |e| {
                e.pay(&sponsor_to, &c.sponsor, to_sponsor)?;
                if let Some(h) = &host_to {
                    e.pay(h, &c.host, to_host)?;
                }
                e.pay(&treasury_to, &c.treasury, to_treasury)?;
                if !is_sol {
                    e.close_token_escrow(vault_tokens.as_ref(), &c.prize_vault, &rent_payer)?;
                }
                Ok(())
            })?;
        }
        CupState::Cancelled => {
            require!(
                c.refunded == c.entrants && (c.guarantee == 0 || c.guarantee_returned),
                ArenaError::NotClosable
            );
            with_escrow(c, token, memo, |e| {
                if !is_sol {
                    e.close_token_escrow(vault_tokens.as_ref(), &c.prize_vault, &rent_payer)?;
                }
                Ok(())
            })?;
        }
        _ => return err!(ArenaError::NotClosable),
    }
    let info = c.to_account_info();
    let surplus = lamports_above_rent(&info)?;
    transfer_from_pda(&info, &a.prize_vault.to_account_info(), surplus)?;
    emit!(CupClosed {
        cup: c.key(),
        to_sponsor,
        to_host,
        to_treasury,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct CloseEntry<'info> {
    #[account(mut, close = rent_payer)]
    pub entry: Box<Account<'info, Entry>>,
    /// CHECK: receives the rent; must be whoever paid it.
    #[account(mut, address = entry.rent_payer)]
    pub rent_payer: UncheckedAccount<'info>,
    /// CHECK: the entry's cup; may already be closed.
    #[account(address = entry.cup)]
    pub cup: UncheckedAccount<'info>,
}

/// Anyone: return an entry's rent once it can no longer be needed: refunded,
/// its cup closed, or every prize of a finalized cup paid.
pub fn handle_close_entry(ctx: Context<CloseEntry>) -> Result<()> {
    if ctx.accounts.entry.refunded {
        return Ok(());
    }
    let info = ctx.accounts.cup.to_account_info();
    if info.data_is_empty() {
        return Ok(());
    }
    require_keys_eq!(*info.owner, crate::ID, ArenaError::NotClosable);
    let c = Cup::try_deserialize(&mut &info.try_borrow_data()?[..])?;
    require!(
        c.state == CupState::Finalized && c.all_claimed(),
        ArenaError::NotClosable
    );
    Ok(())
}

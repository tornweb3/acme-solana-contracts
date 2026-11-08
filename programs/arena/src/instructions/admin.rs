//! Protocol administration. Nothing here can move escrowed funds: the admin sets
//! terms for FUTURE rooms and cups (within hard caps), curates rulesets, tokens
//! and hosts, and rotates keys - a new guardian, gate or resolver set only after
//! a public delay (`ROLES_TIMELOCK_SECS`), so a stolen admin key can't make
//! itself the resolver and rewrite results in flight. The guardian can only stop
//! things, at once.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{constants::*, error::ArenaError, escrow::validate_wager_mint, events::*, state::*};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigParams {
    pub guardian: Pubkey,
    pub gate: Pubkey,
    pub treasury: Pubkey,
    pub prize_vault: Pubkey,
    pub settle_window_secs: u32,
    pub challenge_window_secs: u32,
    pub results_window_secs: u32,
    pub claim_window_secs: u32,
}

/// The windows the admin may change at once (they only reach FUTURE rooms and
/// cups, which freeze their own).
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct WindowParams {
    pub settle_window_secs: u32,
    pub challenge_window_secs: u32,
    pub results_window_secs: u32,
    pub claim_window_secs: u32,
}

fn window_ok(secs: u32, max: u32) -> bool {
    (MIN_WINDOW_SECS..=max).contains(&secs)
}

fn apply_windows(cfg: &mut Config, w: &WindowParams) -> Result<()> {
    require!(
        window_ok(w.settle_window_secs, MAX_SETTLE_WINDOW_SECS)
            && window_ok(w.challenge_window_secs, MAX_CHALLENGE_WINDOW_SECS)
            && window_ok(w.results_window_secs, MAX_RESULTS_WINDOW_SECS)
            && window_ok(w.claim_window_secs, MAX_CLAIM_WINDOW_SECS),
        ArenaError::InvalidWindow
    );
    cfg.settle_window_secs = w.settle_window_secs;
    cfg.challenge_window_secs = w.challenge_window_secs;
    cfg.results_window_secs = w.results_window_secs;
    cfg.claim_window_secs = w.claim_window_secs;
    Ok(())
}

/// The house program's own vaults, nowhere else: every fee and surplus reaches
/// its split (and the split's timelock).
fn house_vault(seed: &[u8]) -> Pubkey {
    Pubkey::find_program_address(&[seed], &HOUSE_ID).0
}

fn apply_params(cfg: &mut Config, p: &ConfigParams) -> Result<()> {
    apply_windows(
        cfg,
        &WindowParams {
            settle_window_secs: p.settle_window_secs,
            challenge_window_secs: p.challenge_window_secs,
            results_window_secs: p.results_window_secs,
            claim_window_secs: p.claim_window_secs,
        },
    )?;
    require!(p.guardian != Pubkey::default(), ArenaError::InvalidParams);
    require_keys_eq!(
        p.treasury,
        house_vault(HOUSE_TREASURY_SEED),
        ArenaError::InvalidParams
    );
    require_keys_eq!(
        p.prize_vault,
        house_vault(HOUSE_PRIZE_SEED),
        ArenaError::InvalidParams
    );
    cfg.guardian = p.guardian;
    cfg.gate = p.gate;
    cfg.treasury = p.treasury;
    cfg.prize_vault = p.prize_vault;
    Ok(())
}

fn resolvers_ok(resolvers: &[Pubkey]) -> bool {
    !resolvers.is_empty()
        && resolvers.len() <= MAX_RESOLVERS
        && resolvers.iter().all(|r| *r != Pubkey::default())
        && resolvers
            .iter()
            .enumerate()
            .all(|(i, r)| !resolvers[..i].contains(r))
}

// ---- initialize ----------------------------------------------------------------

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    /// The program itself, used to reach its ProgramData record.
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ ArenaError::Unauthorized)]
    pub program: Program<'info, crate::program::Arena>,
    /// Only the upgrade authority may bootstrap the protocol. Without this the
    /// first caller after deploy would become admin and pick the treasury.
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()) @ ArenaError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(
    ctx: Context<Initialize>,
    params: ConfigParams,
    resolvers: Vec<Pubkey>,
) -> Result<()> {
    require!(resolvers_ok(&resolvers), ArenaError::InvalidParams);
    let cfg = &mut ctx.accounts.config;
    cfg.admin = ctx.accounts.admin.key();
    cfg.pending_admin = Pubkey::default();
    cfg.resolvers = resolvers;
    cfg.holder_mint = Pubkey::default();
    cfg.paused = false;
    cfg.cup_count = 0;
    cfg.pending_guardian = Pubkey::default();
    cfg.pending_gate = Pubkey::default();
    cfg.pending_resolvers = Vec::new();
    cfg.roles_after = 0;
    cfg.bump = ctx.bumps.config;
    apply_params(cfg, &params)?;
    emit!(ConfigChanged { admin: cfg.admin });
    Ok(())
}

// ---- admin-only config changes -------------------------------------------------

#[derive(Accounts)]
pub struct AdminConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
}

/// The windows of future rooms and cups (each freezes its own when it opens).
pub fn handle_update_config(ctx: Context<AdminConfig>, windows: WindowParams) -> Result<()> {
    apply_windows(&mut ctx.accounts.config, &windows)?;
    emit!(ConfigChanged {
        admin: ctx.accounts.config.admin
    });
    Ok(())
}

/// A new guardian, gate and resolver set, in force once `ROLES_TIMELOCK_SECS`
/// have passed (anyone sees it coming; proposing again starts the wait again).
pub fn handle_propose_roles(
    ctx: Context<AdminConfig>,
    guardian: Pubkey,
    gate: Pubkey,
    resolvers: Vec<Pubkey>,
) -> Result<()> {
    require!(
        guardian != Pubkey::default() && resolvers_ok(&resolvers),
        ArenaError::InvalidParams
    );
    let now = Clock::get()?.unix_timestamp;
    let cfg = &mut ctx.accounts.config;
    cfg.pending_guardian = guardian;
    cfg.pending_gate = gate;
    cfg.pending_resolvers = resolvers;
    cfg.roles_after = now + ROLES_TIMELOCK_SECS;
    emit!(RolesProposed {
        guardian,
        gate,
        after: cfg.roles_after,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ApplyRoles<'info> {
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
}

/// Anyone, once the delay has passed.
pub fn handle_apply_roles(ctx: Context<ApplyRoles>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let cfg = &mut ctx.accounts.config;
    require!(cfg.roles_after != 0, ArenaError::NothingPending);
    require!(now >= cfg.roles_after, ArenaError::TooEarly);
    cfg.guardian = cfg.pending_guardian;
    cfg.gate = cfg.pending_gate;
    cfg.resolvers = core::mem::take(&mut cfg.pending_resolvers);
    cfg.pending_guardian = Pubkey::default();
    cfg.pending_gate = Pubkey::default();
    cfg.roles_after = 0;
    emit!(ConfigChanged { admin: cfg.admin });
    Ok(())
}

/// The project token's perks. The mint is set ONCE (a token can't be swapped
/// for another later); the thresholds and the holder fee can be tuned.
pub fn handle_set_holder(
    ctx: Context<AdminConfig>,
    holder_mint: Pubkey,
    holder_min: u64,
    holder_room_fee_bps: u16,
    host_min: u64,
) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    require!(holder_mint != Pubkey::default(), ArenaError::InvalidParams);
    require!(
        cfg.holder_mint == Pubkey::default() || cfg.holder_mint == holder_mint,
        ArenaError::AlreadySet
    );
    require!(
        holder_room_fee_bps <= HARD_MAX_ROOM_FEE_BPS && holder_min > 0 && host_min > 0,
        ArenaError::InvalidParams
    );
    cfg.holder_mint = holder_mint;
    cfg.holder_min = holder_min;
    cfg.holder_room_fee_bps = holder_room_fee_bps;
    cfg.host_min = host_min;
    emit!(ConfigChanged { admin: cfg.admin });
    Ok(())
}

pub fn handle_propose_admin(ctx: Context<AdminConfig>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.config.pending_admin = new_admin;
    Ok(())
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
}

pub fn handle_accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    require!(
        cfg.pending_admin != Pubkey::default() && cfg.pending_admin == ctx.accounts.new_admin.key(),
        ArenaError::NotPendingAdmin
    );
    cfg.admin = cfg.pending_admin;
    cfg.pending_admin = Pubkey::default();
    emit!(ConfigChanged { admin: cfg.admin });
    Ok(())
}

// ---- the guardian's brakes: pause new money, revoke every resolver ----------------

#[derive(Accounts)]
pub struct Guard<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
}

/// Admin or guardian may pause; only the admin unpauses.
pub fn handle_set_paused(ctx: Context<Guard>, paused: bool) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    let who = ctx.accounts.authority.key();
    let allowed = who == cfg.admin || (paused && who == cfg.guardian);
    require!(allowed, ArenaError::Unauthorized);
    cfg.paused = paused;
    Ok(())
}

/// Emergency: a resolver key may be compromised. Empties the set, so no room
/// can be locked, settled or voided and no result posted until the admin sets
/// new keys. Every room then still ends by its deadline refund.
pub fn handle_revoke_resolvers(ctx: Context<Guard>) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    let who = ctx.accounts.authority.key();
    require!(
        who == cfg.guardian || who == cfg.admin,
        ArenaError::Unauthorized
    );
    cfg.resolvers.clear();
    emit!(ResolversRevoked { by: who });
    Ok(())
}

// ---- community cup hosts (the path with no token) ---------------------------------

#[derive(Accounts)]
#[instruction(wallet: Pubkey)]
pub struct SetHost<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + Host::INIT_SPACE,
        seeds = [HOST_SEED, wallet.as_ref()],
        bump
    )]
    pub host: Account<'info, Host>,
    pub system_program: Program<'info, System>,
}

/// Lets `wallet` organize community cups until `until` (0 = no longer).
pub fn handle_set_host(ctx: Context<SetHost>, wallet: Pubkey, until: i64) -> Result<()> {
    let h = &mut ctx.accounts.host;
    h.wallet = wallet;
    h.until = until;
    h.bump = ctx.bumps.host;
    Ok(())
}

// ---- rulesets -------------------------------------------------------------------

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct AddRuleset<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = admin,
        space = 8 + Ruleset::INIT_SPACE,
        seeds = [RULESET_SEED, &id.to_le_bytes()],
        bump
    )]
    pub ruleset: Account<'info, Ruleset>,
    pub system_program: Program<'info, System>,
}

pub fn handle_add_ruleset(
    ctx: Context<AddRuleset>,
    id: u32,
    hash: [u8; 32],
    rating_points: [u8; DEPARTMENTS],
) -> Result<()> {
    let r = &mut ctx.accounts.ruleset;
    r.id = id;
    r.hash = hash;
    r.rating_points = rating_points;
    r.enabled = true;
    r.bump = ctx.bumps.ruleset;
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateRuleset<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [RULESET_SEED, &ruleset.id.to_le_bytes()], bump = ruleset.bump)]
    pub ruleset: Account<'info, Ruleset>,
}

/// The hash and the rating points never change: a new sim version is a new id.
pub fn handle_update_ruleset(ctx: Context<UpdateRuleset>, enabled: bool) -> Result<()> {
    ctx.accounts.ruleset.enabled = enabled;
    Ok(())
}

// ---- tokens ---------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintParams {
    pub min_stake: u64,
    pub max_stake: u64,
    pub room_fee_bps: u16,
    pub cup_fee_bps: u16,
    /// Accept a transfer-fee mint (the project token), up to this fee per transfer.
    pub allow_transfer_fee: bool,
    pub max_transfer_fee_bps: u16,
    pub holder_max_stake: u64,
}

fn mint_params_ok(mint: &Pubkey, p: &MintParams) -> Result<()> {
    require!(
        p.min_stake > 0 && p.min_stake <= p.max_stake,
        ArenaError::InvalidParams
    );
    // a SOL stake below an empty account's rent floor could leave a refund that
    // can't land in a player's emptied wallet (and with it everyone's)
    if *mint == NATIVE_SOL {
        require!(
            p.min_stake >= Rent::get()?.minimum_balance(0),
            ArenaError::InvalidParams
        );
    }
    require!(
        p.room_fee_bps <= HARD_MAX_ROOM_FEE_BPS && p.cup_fee_bps <= HARD_MAX_CUP_FEE_BPS,
        ArenaError::FeeTooHigh
    );
    require!(
        p.max_transfer_fee_bps <= HARD_MAX_TRANSFER_FEE_BPS,
        ArenaError::TransferFeeTooHigh
    );
    Ok(())
}

#[derive(Accounts)]
#[instruction(mint_key: Pubkey)]
pub struct AddMint<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = admin,
        space = 8 + AllowedMint::INIT_SPACE,
        seeds = [MINT_SEED, mint_key.as_ref()],
        bump
    )]
    pub allowed_mint: Account<'info, AllowedMint>,
    /// Absent when `mint_key` is NATIVE_SOL.
    pub mint: Option<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

pub fn handle_add_mint(ctx: Context<AddMint>, mint_key: Pubkey, params: MintParams) -> Result<()> {
    mint_params_ok(&mint_key, &params)?;
    let (token_program, decimals, transfer_fee) = if mint_key == NATIVE_SOL {
        require!(
            ctx.accounts.mint.is_none(),
            ArenaError::TokenAccountsMismatch
        );
        (anchor_lang::system_program::ID, 9u8, false)
    } else {
        let mint = ctx
            .accounts
            .mint
            .as_ref()
            .ok_or(ArenaError::TokenAccountsMismatch)?;
        require_keys_eq!(mint.key(), mint_key, ArenaError::TokenAccountsMismatch);
        let info = mint.to_account_info();
        let has_fee = validate_wager_mint(&info, params.allow_transfer_fee)?;
        if has_fee {
            crate::escrow::check_fee_cap(&info, params.max_transfer_fee_bps)?;
        }
        (*info.owner, mint.decimals, has_fee)
    };
    let m = &mut ctx.accounts.allowed_mint;
    m.mint = mint_key;
    m.token_program = token_program;
    m.decimals = decimals;
    m.min_stake = params.min_stake;
    m.max_stake = params.max_stake;
    m.room_fee_bps = params.room_fee_bps;
    m.cup_fee_bps = params.cup_fee_bps;
    m.transfer_fee = transfer_fee;
    m.max_transfer_fee_bps = params.max_transfer_fee_bps;
    m.holder_max_stake = params.holder_max_stake;
    m.enabled = true;
    m.bump = ctx.bumps.allowed_mint;
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateMint<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ ArenaError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [MINT_SEED, allowed_mint.mint.as_ref()], bump = allowed_mint.bump)]
    pub allowed_mint: Account<'info, AllowedMint>,
}

/// Affects only rooms and cups created afterwards. Whether the mint has a
/// transfer fee is fixed when it is added.
pub fn handle_update_mint(
    ctx: Context<UpdateMint>,
    params: MintParams,
    enabled: bool,
) -> Result<()> {
    mint_params_ok(&ctx.accounts.allowed_mint.mint, &params)?;
    let m = &mut ctx.accounts.allowed_mint;
    m.min_stake = params.min_stake;
    m.max_stake = params.max_stake;
    m.room_fee_bps = params.room_fee_bps;
    m.cup_fee_bps = params.cup_fee_bps;
    m.max_transfer_fee_bps = params.max_transfer_fee_bps;
    m.holder_max_stake = params.holder_max_stake;
    m.enabled = enabled;
    Ok(())
}

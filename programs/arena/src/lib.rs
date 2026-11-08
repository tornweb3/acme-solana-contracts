//! ACME arena: escrow for live-race rooms and prize cups, in SOL or allowlisted
//! SPL tokens (the project token's transfer fee included), with a capped house
//! fee that goes to the house program's treasury.
//!
//! Design: `docs/solana-plan.md` (in the game repo). The rules every path keeps:
//! funds leave escrow only to the players of that room or cup, as the
//! code-computed fee to the treasury frozen on it, or (anything above what is
//! owed) to the prize vault frozen on it; terms are frozen at creation; every
//! state has a deadline after which anyone can refund; pausing and revoking the
//! resolvers never block settling, refunds, claims or closing.

pub mod constants;
pub mod error;
pub mod escrow;
pub mod events;
pub mod instructions;
pub mod merkle;
pub mod payout;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK");

#[program]
pub mod arena {
    use super::*;

    // ---- admin ----
    pub fn initialize(
        ctx: Context<Initialize>,
        params: ConfigParams,
        resolvers: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::admin::handle_initialize(ctx, params, resolvers)
    }
    pub fn update_config(ctx: Context<AdminConfig>, windows: WindowParams) -> Result<()> {
        instructions::admin::handle_update_config(ctx, windows)
    }
    pub fn propose_roles(
        ctx: Context<AdminConfig>,
        guardian: Pubkey,
        gate: Pubkey,
        resolvers: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::admin::handle_propose_roles(ctx, guardian, gate, resolvers)
    }
    pub fn apply_roles(ctx: Context<ApplyRoles>) -> Result<()> {
        instructions::admin::handle_apply_roles(ctx)
    }
    pub fn set_holder(
        ctx: Context<AdminConfig>,
        holder_mint: Pubkey,
        holder_min: u64,
        holder_room_fee_bps: u16,
        host_min: u64,
    ) -> Result<()> {
        instructions::admin::handle_set_holder(
            ctx,
            holder_mint,
            holder_min,
            holder_room_fee_bps,
            host_min,
        )
    }
    pub fn propose_admin(ctx: Context<AdminConfig>, new_admin: Pubkey) -> Result<()> {
        instructions::admin::handle_propose_admin(ctx, new_admin)
    }
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::admin::handle_accept_admin(ctx)
    }
    pub fn set_paused(ctx: Context<Guard>, paused: bool) -> Result<()> {
        instructions::admin::handle_set_paused(ctx, paused)
    }
    pub fn revoke_resolvers(ctx: Context<Guard>) -> Result<()> {
        instructions::admin::handle_revoke_resolvers(ctx)
    }
    pub fn set_host(ctx: Context<SetHost>, wallet: Pubkey, until: i64) -> Result<()> {
        instructions::admin::handle_set_host(ctx, wallet, until)
    }
    pub fn add_ruleset(
        ctx: Context<AddRuleset>,
        id: u32,
        hash: [u8; 32],
        rating_points: [u8; DEPARTMENTS],
    ) -> Result<()> {
        instructions::admin::handle_add_ruleset(ctx, id, hash, rating_points)
    }
    pub fn update_ruleset(ctx: Context<UpdateRuleset>, enabled: bool) -> Result<()> {
        instructions::admin::handle_update_ruleset(ctx, enabled)
    }
    pub fn add_mint(ctx: Context<AddMint>, mint_key: Pubkey, params: MintParams) -> Result<()> {
        instructions::admin::handle_add_mint(ctx, mint_key, params)
    }
    pub fn update_mint(ctx: Context<UpdateMint>, params: MintParams, enabled: bool) -> Result<()> {
        instructions::admin::handle_update_mint(ctx, params, enabled)
    }

    // ---- rooms ----
    pub fn create_room(ctx: Context<CreateRoom>, code: [u8; 8], params: RoomParams) -> Result<()> {
        instructions::room::handle_create_room(ctx, code, params)
    }
    pub fn join_room(ctx: Context<JoinRoom>, expected_stake: u64) -> Result<()> {
        instructions::room::handle_join_room(ctx, expected_stake)
    }
    pub fn leave_room<'info>(ctx: Context<'info, LeaveRoom<'info>>) -> Result<()> {
        instructions::room::handle_leave_room(ctx)
    }
    pub fn cancel_room<'info>(ctx: Context<'info, CancelRoom<'info>>) -> Result<()> {
        instructions::room::handle_cancel_room(ctx)
    }
    pub fn lock_room(ctx: Context<LockRoom>) -> Result<()> {
        instructions::room::handle_lock_room(ctx)
    }
    pub fn draw_seed(ctx: Context<DrawSeed>) -> Result<()> {
        instructions::room::handle_draw_seed(ctx)
    }
    pub fn settle_room<'info>(
        ctx: Context<'info, SettleRoom<'info>>,
        times_ms: Vec<u32>,
        record_hash: [u8; 32],
    ) -> Result<()> {
        instructions::room::handle_settle_room(ctx, times_ms, record_hash)
    }
    pub fn void_room<'info>(
        ctx: Context<'info, VoidRoom<'info>>,
        reason: u8,
        record_hash: [u8; 32],
    ) -> Result<()> {
        instructions::room::handle_void_room(ctx, reason, record_hash)
    }
    pub fn refund_room<'info>(ctx: Context<'info, RefundRoom<'info>>) -> Result<()> {
        instructions::room::handle_refund_room(ctx)
    }
    pub fn close_room<'info>(ctx: Context<'info, CloseRoom<'info>>) -> Result<()> {
        instructions::room::handle_close_room(ctx)
    }

    // ---- cups ----
    pub fn create_cup(ctx: Context<CreateCup>, id: u64, params: CupParams) -> Result<()> {
        instructions::cup::handle_create_cup(ctx, id, params)
    }
    pub fn enter_cup(ctx: Context<EnterCup>) -> Result<()> {
        instructions::cup::handle_enter_cup(ctx)
    }
    pub fn post_results(
        ctx: Context<PostResults>,
        results_root: [u8; 32],
        scorers: u32,
        places: u8,
        standings_hash: [u8; 32],
        results_uri: String,
    ) -> Result<()> {
        instructions::cup::handle_post_results(
            ctx,
            results_root,
            scorers,
            places,
            standings_hash,
            results_uri,
        )
    }
    pub fn reject_results(ctx: Context<GuardCup>) -> Result<()> {
        instructions::cup::handle_reject_results(ctx)
    }
    pub fn cancel_cup(ctx: Context<GuardCup>) -> Result<()> {
        instructions::cup::handle_cancel_cup(ctx)
    }
    pub fn finalize_cup(ctx: Context<FinalizeCup>) -> Result<()> {
        instructions::cup::handle_finalize_cup(ctx)
    }
    pub fn claim_prize<'info>(
        ctx: Context<'info, ClaimPrize<'info>>,
        rank: u16,
        wallet: Pubkey,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        instructions::cup::handle_claim_prize(ctx, rank, wallet, proof)
    }
    pub fn refund_entry<'info>(ctx: Context<'info, RefundEntry<'info>>) -> Result<()> {
        instructions::cup::handle_refund_entry(ctx)
    }
    pub fn return_guarantee<'info>(ctx: Context<'info, ReturnGuarantee<'info>>) -> Result<()> {
        instructions::cup::handle_return_guarantee(ctx)
    }
    pub fn close_cup<'info>(ctx: Context<'info, CloseCup<'info>>) -> Result<()> {
        instructions::cup::handle_close_cup(ctx)
    }
    pub fn close_entry(ctx: Context<CloseEntry>) -> Result<()> {
        instructions::cup::handle_close_entry(ctx)
    }
}

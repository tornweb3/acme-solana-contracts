use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, system_instruction},
    InstructionData, ToAccountMetas,
};
use anchor_spl::token_2022::spl_token_2022::{
    extension::ExtensionType, instruction as t22, state::Mint as SplMint,
};
use arena::{error::ArenaError, NATIVE_SOL};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::common::*;

#[test]
fn only_the_upgrade_authority_can_initialize() {
    let mut env = Env::deployed();
    let stranger = env.player();
    let ix = env.ix_initialize(&stranger.pubkey(), env.config_params());
    env.fails(&[ix], &[&stranger], ArenaError::Unauthorized);
    let admin = env.admin.insecure_clone();
    let ix = env.ix_initialize(&admin.pubkey(), env.config_params());
    env.ok(&[ix], &[&admin]);
    let cfg = env.config();
    assert_eq!(cfg.admin, admin.pubkey());
    assert_eq!(cfg.treasury, treasury());
    assert_eq!(cfg.prize_vault, prize_vault());
    assert_eq!(cfg.holder_mint, Pubkey::default());
}

fn windows() -> arena::WindowParams {
    arena::WindowParams {
        settle_window_secs: SETTLE_WINDOW,
        challenge_window_secs: CHALLENGE_WINDOW,
        results_window_secs: RESULTS_WINDOW,
        claim_window_secs: CLAIM_WINDOW,
    }
}

#[test]
fn the_treasury_and_prize_vault_are_the_house_programs_own() {
    let mut env = Env::deployed();
    let admin = env.admin.insecure_clone();
    for (t, v) in [
        (Keypair::new().pubkey(), prize_vault()),
        (treasury(), Keypair::new().pubkey()),
    ] {
        let mut p = env.config_params();
        p.treasury = t;
        p.prize_vault = v;
        let ix = env.ix_initialize(&admin.pubkey(), p);
        env.fails(&[ix], &[&admin], ArenaError::InvalidParams);
    }
}

#[test]
fn new_roles_wait_two_days_in_public_a_revocation_none() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let (guardian, resolver) = (env.guardian.insecure_clone(), env.resolver.insecure_clone());
    let thief = env.player();
    // a stolen admin key makes itself guardian, gate and resolver: nothing changes for two days
    let ix = env.ix_admin(arena::instruction::ProposeRoles {
        guardian: thief.pubkey(),
        gate: thief.pubkey(),
        resolvers: vec![thief.pubkey()],
    });
    env.ok(&[ix], &[&admin]);
    env.fails(&[ix_apply_roles()], &[&admin], ArenaError::TooEarly);
    let cfg = env.config();
    assert_eq!(
        (cfg.guardian, cfg.resolvers.clone()),
        (guardian.pubkey(), vec![resolver.pubkey()])
    );
    assert_eq!(cfg.roles_after, env.now() + arena::ROLES_TIMELOCK_SECS);
    // proposing again starts the wait again
    env.warp(arena::ROLES_TIMELOCK_SECS - 10);
    let ix = env.ix_admin(arena::instruction::ProposeRoles {
        guardian: guardian.pubkey(),
        gate: Pubkey::default(),
        resolvers: vec![resolver.pubkey(), thief.pubkey()],
    });
    env.ok(&[ix], &[&admin]);
    env.warp(20);
    env.fails(&[ix_apply_roles()], &[&admin], ArenaError::TooEarly);
    // the guardian revokes every resolver at once, whatever is pending
    let revoke = Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::RevokeResolvers {}.data(),
        arena::accounts::Guard {
            authority: guardian.pubkey(),
            config: config_pda(),
        }
        .to_account_metas(None),
    );
    env.ok(&[revoke], &[&guardian]);
    assert!(env.config().resolvers.is_empty());
    // once the wait is over, anyone applies what was proposed
    env.warp(arena::ROLES_TIMELOCK_SECS);
    env.ok(&[ix_apply_roles()], &[&thief]);
    assert_eq!(
        env.config().resolvers,
        vec![resolver.pubkey(), thief.pubkey()]
    );
    env.fails(&[ix_apply_roles()], &[&thief], ArenaError::NothingPending);
}

#[test]
fn windows_and_fees_have_hard_caps() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let mut w = windows();
    w.settle_window_secs = 2 * 86_400; // over 24 h
    let ix = env.ix_admin(arena::instruction::UpdateConfig { windows: w });
    env.fails(&[ix], &[&admin], ArenaError::InvalidWindow);
    let ix = env.ix_admin(arena::instruction::UpdateConfig { windows: windows() });
    env.ok(&[ix], &[&admin]);

    let stranger = env.player();
    let mut ix = env.ix_admin(arena::instruction::UpdateConfig { windows: windows() });
    ix.accounts[0].pubkey = stranger.pubkey();
    env.fails(&[ix], &[&stranger], ArenaError::Unauthorized);

    // a SOL stake can't go below an empty account's rent floor
    let mut mp = mint_params(SOL / 1_000, 100 * SOL);
    mp.min_stake = 1_000;
    let ix = Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::UpdateMint {
            params: mp,
            enabled: true,
        }
        .data(),
        arena::accounts::UpdateMint {
            admin: admin.pubkey(),
            config: config_pda(),
            allowed_mint: mint_pda(&arena::NATIVE_SOL),
        }
        .to_account_metas(None),
    );
    env.fails(&[ix], &[&admin], ArenaError::InvalidParams);

    let spl = env.spl_token();
    let mut mp = mint_params(1, 10);
    mp.room_fee_bps = 1_001;
    let ix = Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::UpdateMint {
            params: mp,
            enabled: true,
        }
        .data(),
        arena::accounts::UpdateMint {
            admin: admin.pubkey(),
            config: config_pda(),
            allowed_mint: mint_pda(&spl.mint),
        }
        .to_account_metas(None),
    );
    env.fails(&[ix], &[&admin], ArenaError::FeeTooHigh);
    let mut mp = mint_params(1, 10);
    mp.max_transfer_fee_bps = 501;
    let ix = env.ix_add_mint(&Pubkey::new_unique(), None, mp);
    env.fails_any(&[ix], &[&admin]);
}

#[test]
fn the_guardian_pauses_and_revokes_but_only_the_admin_restores() {
    let mut env = Env::new();
    let (guardian, admin, resolver) = (
        env.guardian.insecure_clone(),
        env.admin.insecure_clone(),
        env.resolver.insecure_clone(),
    );
    let a = env.player();
    let b = env.player();
    // a room in flight before the brakes
    let ixs = ix_create_room(
        &a.pubkey(),
        &a.pubkey(),
        code(1),
        room_params(SOL / 10, 2),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&a]);
    let room = room_pda(&code(1));
    env.ok(&[env.ix_join(&b.pubkey(), &room, None, PLAIN)], &[&b]);

    let guard = |which: Vec<u8>, who: &Keypair| {
        Instruction::new_with_bytes(
            arena::ID,
            &which,
            arena::accounts::Guard {
                authority: who.pubkey(),
                config: config_pda(),
            }
            .to_account_metas(None),
        )
    };
    env.ok(
        &[guard(
            arena::instruction::SetPaused { paused: true }.data(),
            &guardian,
        )],
        &[&guardian],
    );
    let c = env.player();
    let ixs = ix_create_room(
        &c.pubkey(),
        &c.pubkey(),
        code(2),
        room_params(SOL / 10, 2),
        None,
        PLAIN,
    );
    env.fails(&ixs, &[&c], ArenaError::Paused);
    // the guardian can't unpause
    env.fails(
        &[guard(
            arena::instruction::SetPaused { paused: false }.data(),
            &guardian,
        )],
        &[&guardian],
        ArenaError::Unauthorized,
    );
    // revoke every resolver: settling stops, the deadline refund still works
    env.ok(
        &[guard(
            arena::instruction::RevokeResolvers {}.data(),
            &guardian,
        )],
        &[&guardian],
    );
    assert!(env.config().resolvers.is_empty());
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![60_000, 61_000],
        [1u8; 32],
        &[a.pubkey()],
    );
    env.fails(&[ix], &[&resolver], ArenaError::Unauthorized);
    env.warp(SETTLE_WINDOW as i64 + 1);
    let before = env.lamports(&a.pubkey());
    env.ok(
        &[ix_refund_room(&room, None, &[a.pubkey(), b.pubkey()])],
        &[&c],
    );
    assert_eq!(env.lamports(&a.pubkey()), before + SOL / 10);

    // the admin restores
    env.ok(
        &[guard(
            arena::instruction::SetPaused { paused: false }.data(),
            &admin,
        )],
        &[&admin],
    );
    let ix = env.ix_admin(arena::instruction::ProposeRoles {
        guardian: env.guardian.pubkey(),
        gate: Pubkey::default(),
        resolvers: vec![resolver.pubkey()],
    });
    env.ok(&[ix], &[&admin]);
    env.warp(arena::ROLES_TIMELOCK_SECS);
    env.ok(&[ix_apply_roles()], &[&admin]);
    assert_eq!(env.config().resolvers, vec![resolver.pubkey()]);
}

#[test]
fn admin_moves_in_two_steps() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let next = env.player();
    let ix = env.ix_admin(arena::instruction::ProposeAdmin {
        new_admin: next.pubkey(),
    });
    env.ok(&[ix], &[&admin]);
    let accept = |who: &Keypair| {
        Instruction::new_with_bytes(
            arena::ID,
            &arena::instruction::AcceptAdmin {}.data(),
            arena::accounts::AcceptAdmin {
                new_admin: who.pubkey(),
                config: config_pda(),
            }
            .to_account_metas(None),
        )
    };
    let stranger = env.player();
    env.fails(
        &[accept(&stranger)],
        &[&stranger],
        ArenaError::NotPendingAdmin,
    );
    env.ok(&[accept(&next)], &[&next]);
    let cfg = env.config();
    assert_eq!(cfg.admin, next.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

#[test]
fn mints_with_dangerous_extensions_are_refused_and_a_fee_only_when_allowed() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let mint = Keypair::new();
    let program = anchor_spl::token_2022::ID;
    let len =
        ExtensionType::try_calculate_account_len::<SplMint>(&[ExtensionType::PermanentDelegate])
            .unwrap();
    let rent = env.svm.minimum_balance_for_rent_exemption(len);
    let ixs = [
        system_instruction::create_account(
            &admin.pubkey(),
            &mint.pubkey(),
            rent,
            len as u64,
            &program,
        ),
        t22::initialize_permanent_delegate(&program, &mint.pubkey(), &admin.pubkey()).unwrap(),
        t22::initialize_mint2(&program, &mint.pubkey(), &admin.pubkey(), None, 6).unwrap(),
    ];
    env.ok(&ixs, &[&admin, &mint]);
    let mut mp = mint_params(1, 10);
    mp.allow_transfer_fee = true;
    mp.max_transfer_fee_bps = 100;
    let ix = env.ix_add_mint(&mint.pubkey(), Some(mint.pubkey()), mp);
    env.fails(&[ix], &[&admin], ArenaError::UnsupportedMint);

    // a transfer-fee mint: refused without the flag, refused above the cap, taken with both
    let fee = env.fee_mint(100);
    let ix = env.ix_add_mint(&fee.mint, Some(fee.mint), mint_params(1, 10));
    env.fails(&[ix], &[&admin], ArenaError::UnsupportedMint);
    let mut mp = mint_params(1, 10);
    mp.allow_transfer_fee = true;
    mp.max_transfer_fee_bps = 50;
    let ix = env.ix_add_mint(&fee.mint, Some(fee.mint), mp);
    env.fails(&[ix], &[&admin], ArenaError::TransferFeeTooHigh);
    env.allow_fee_mint(&fee, 100);
    let m: arena::state::AllowedMint = env.load(&mint_pda(&fee.mint));
    assert!(m.transfer_fee && m.max_transfer_fee_bps == 100 && m.room_fee_bps == 250);

    // SOL takes no mint account; a token takes its own mint account
    let ix = env.ix_add_mint(&NATIVE_SOL, Some(mint.pubkey()), mint_params(1, 10));
    env.fails_any(&[ix], &[&admin]);
}

#[test]
fn the_holder_mint_is_set_once() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let token = Pubkey::new_unique();
    let set = |mint: Pubkey| arena::instruction::SetHolder {
        holder_mint: mint,
        holder_min: 1_000_000,
        holder_room_fee_bps: 250,
        host_min: 5_000_000,
    };
    env.ok(&[env.ix_admin(set(token))], &[&admin]);
    // the thresholds may change; the token may not
    let mut again = set(token);
    again.holder_min = 2_000_000;
    env.ok(&[env.ix_admin(again)], &[&admin]);
    assert_eq!(env.config().holder_min, 2_000_000);
    env.fails(
        &[env.ix_admin(set(Pubkey::new_unique()))],
        &[&admin],
        ArenaError::AlreadySet,
    );
}

#[test]
fn with_a_gate_set_new_money_needs_its_signature() {
    let mut env = Env::new();
    env.gate_on();
    let gate = env.gate.insecure_clone();
    let a = env.player();
    let b = env.player();
    let ixs = ix_create_room(
        &a.pubkey(),
        &a.pubkey(),
        code(1),
        room_params(SOL / 10, 2),
        None,
        PLAIN,
    );
    env.fails(&ixs, &[&a], ArenaError::GateRequired);
    let who = Who {
        gate: Some(gate.pubkey()),
        ..PLAIN
    };
    let ixs = ix_create_room(
        &a.pubkey(),
        &a.pubkey(),
        code(1),
        room_params(SOL / 10, 2),
        None,
        who,
    );
    env.ok(&ixs, &[&a, &gate]);
    let room = room_pda(&code(1));
    env.fails(
        &[env.ix_join(&b.pubkey(), &room, None, PLAIN)],
        &[&b],
        ArenaError::GateRequired,
    );
    // a different key in the gate's place
    let fake = env.player();
    let who = Who {
        gate: Some(fake.pubkey()),
        ..PLAIN
    };
    env.fails(
        &[env.ix_join(&b.pubkey(), &room, None, who)],
        &[&b, &fake],
        ArenaError::GateRequired,
    );
    let who = Who {
        gate: Some(gate.pubkey()),
        ..PLAIN
    };
    env.ok(&[env.ix_join(&b.pubkey(), &room, None, who)], &[&b, &gate]);
    env.draw_seed(&room);
    // the gate never blocks an exit: settle without it
    let resolver = env.resolver.insecure_clone();
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![60_000, 61_000],
        [1u8; 32],
        &[a.pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
}

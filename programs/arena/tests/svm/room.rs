use anchor_lang::{prelude::Pubkey, solana_program::instruction::AccountMeta};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        memo_transfer::instruction as memo_ix, transfer_fee::instruction as fee_ix, ExtensionType,
    },
    instruction as t22,
};
use arena::{error::ArenaError, state::*, RoomParams, DNF, MEMO_PROGRAM_ID};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::common::*;

const STAKE: u64 = SOL / 10;

/// A room of `n` players (the first creates it, the race server's sponsor pays the
/// rent), all seated; `start_mode` 0 locks it as it fills.
fn seated(
    env: &mut Env,
    n: usize,
    start_mode: u8,
    spl: Option<&Spl>,
    stake: u64,
) -> (Pubkey, Vec<Keypair>) {
    let players: Vec<Keypair> = (0..n).map(|_| env.player()).collect();
    if let Some(s) = spl {
        for p in &players {
            env.give_tokens(s, &p.pubkey(), 1_000 * 1_000_000);
        }
    }
    let sponsor = env.sponsor.insecure_clone();
    let mut params = room_params(stake, n as u8);
    params.race.start_mode = start_mode;
    let ixs = ix_create_room(
        &players[0].pubkey(),
        &sponsor.pubkey(),
        code(1),
        params,
        spl,
        PLAIN,
    );
    env.ok(&ixs, &[&players[0], &sponsor]);
    let room = room_pda(&code(1));
    for p in &players[1..] {
        env.ok(&[env.ix_join(&p.pubkey(), &room, spl, PLAIN)], &[p]);
    }
    // a room that locked as it filled: its seed, as the race server then draws it
    if env.room(&room).state == RoomState::Locked {
        env.draw_seed(&room);
    }
    (room, players)
}

fn keys(ps: &[Keypair]) -> Vec<Pubkey> {
    ps.iter().map(|p| p.pubkey()).collect()
}

#[test]
fn a_sol_room_starts_when_full_and_pays_the_winner_in_the_settlement() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let sponsor_before = env.lamports(&env.sponsor.pubkey());
    let (room, ps) = seated(&mut env, 2, 0, None, STAKE);
    let r = env.room(&room);
    assert_eq!(r.state, RoomState::Locked);
    assert_ne!(r.seed, [0u8; 32]);
    assert_eq!(r.settle_deadline, r.locked_at + SETTLE_WINDOW as i64);
    assert_eq!(r.rent_payer, env.sponsor.pubkey());
    assert_eq!(r.fee_bps, 500);
    // a third player can't get in
    let c = env.player();
    env.fails(
        &[env.ix_join(&c.pubkey(), &room, None, PLAIN)],
        &[&c],
        ArenaError::RoomNotOpen,
    );

    let (a0, t0) = (env.lamports(&ps[0].pubkey()), env.lamports(&treasury()));
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![61_000, 62_500],
        [7u8; 32],
        &[ps[0].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    assert_eq!(env.lamports(&ps[0].pubkey()), a0 + 190_000_000); // 0.2 SOL − 5%
    assert_eq!(env.lamports(&treasury()), t0 + 10_000_000);
    let r = env.room(&room);
    assert_eq!(r.state, RoomState::Settled);
    assert_eq!(
        (r.winners, r.share, r.to_treasury),
        (0b01, 190_000_000, 10_000_000)
    );
    assert_eq!(r.record_hash, [7u8; 32]);
    assert_eq!(r.times_ms, vec![61_000, 62_500]);
    assert_eq!(r.settled_by, resolver.pubkey());
    // the rent goes back to whoever paid it (the sponsor signed, but paid no fees)
    env.ok(
        &[ix_close_room(&room, &env.sponsor.pubkey(), None, false)],
        &[&c],
    );
    assert!(!env.exists(&room));
    assert_eq!(env.lamports(&env.sponsor.pubkey()), sponsor_before);
}

#[test]
fn a_host_start_room_locks_at_the_resolver_with_two_players_or_more() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let players: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let mut params = room_params(STAKE, 4);
    params.race.start_mode = 1;
    let ixs = ix_create_room(
        &players[0].pubkey(),
        &players[0].pubkey(),
        code(1),
        params,
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&players[0]]);
    let room = room_pda(&code(1));
    // one player: not yet
    env.fails(
        &[ix_lock(&resolver.pubkey(), &room)],
        &[&resolver],
        ArenaError::NotEnoughPlayers,
    );
    env.ok(
        &[env.ix_join(&players[1].pubkey(), &room, None, PLAIN)],
        &[&players[1]],
    );
    let stranger = env.player();
    env.fails(
        &[ix_lock(&stranger.pubkey(), &room)],
        &[&stranger],
        ArenaError::Unauthorized,
    );
    env.ok(&[ix_lock(&resolver.pubkey(), &room)], &[&resolver]);
    assert_eq!(env.room(&room).state, RoomState::Locked);
    env.fails(
        &[env.ix_join(&players[2].pubkey(), &room, None, PLAIN)],
        &[&players[2]],
        ArenaError::RoomNotOpen,
    );
    env.fails(
        &[ix_leave(&players[1].pubkey(), &room, None)],
        &[&players[1]],
        ArenaError::RoomNotOpen,
    );
}

#[test]
fn a_settlement_takes_only_the_winners_accounts_and_checks_each() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let spl = env.spl_token();
    let stake = 5_000_000;
    let (room, ps) = seated(&mut env, 3, 0, Some(&spl), stake);
    // the loser who didn't finish empties and closes their token account: the
    // settlement needs nothing of theirs
    let (gone, left) = (
        spl.ata(&ps[2].pubkey()),
        env.tokens(&spl.ata(&ps[2].pubkey())),
    );
    let ixs = [
        t22::transfer_checked(
            &spl.program,
            &gone,
            &spl.mint,
            &spl.ata(&ps[0].pubkey()),
            &ps[2].pubkey(),
            &[],
            left,
            6,
        )
        .unwrap(),
        t22::close_account(&spl.program, &gone, &ps[2].pubkey(), &ps[2].pubkey(), &[]).unwrap(),
    ];
    env.ok(&ixs, &[&ps[2]]);
    assert!(!env.exists(&gone));
    let times = vec![90_000, 80_000, DNF];
    // the wrong count, a loser in the winner's place: refused
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&spl),
        times.clone(),
        [1; 32],
        &[],
    );
    env.fails(&[ix], &[&resolver], ArenaError::PayoutAccountsMismatch);
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&spl),
        times.clone(),
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.fails(&[ix], &[&resolver], ArenaError::PayoutAccountsMismatch);
    // the treasury's account must be the treasury's
    let mut ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&spl),
        times.clone(),
        [1; 32],
        &[ps[1].pubkey()],
    );
    ix.accounts[6].pubkey = spl.ata(&ps[0].pubkey());
    env.fails(&[ix], &[&resolver], ArenaError::PayoutAccountsMismatch);
    let t0 = env.tokens(&spl.ata(&treasury()));
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&spl),
        times,
        [1; 32],
        &[ps[1].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    // pot 15, fee 0.75: the winner gets 14.25
    assert_eq!(
        env.tokens(&spl.ata(&ps[1].pubkey())),
        1_000_000_000 - stake + 14_250_000
    );
    assert_eq!(env.tokens(&spl.ata(&treasury())), t0 + 750_000);
    assert_eq!(env.tokens(&spl.ata(&room)), 0);
    // the escrow's token account closes with the room
    let c = env.player();
    env.ok(
        &[ix_close_room(
            &room,
            &env.sponsor.pubkey(),
            Some(&spl),
            false,
        )],
        &[&c],
    );
    assert!(!env.exists(&spl.ata(&room)) && !env.exists(&room));
}

#[test]
fn ties_for_first_split_and_nobody_ahead_refunds_without_a_fee() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    // everyone tied: every stake back, no fee
    let (room, ps) = seated(&mut env, 2, 0, None, STAKE);
    let before: Vec<u64> = ps.iter().map(|p| env.lamports(&p.pubkey())).collect();
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![60_000, 60_000],
        [2; 32],
        &keys(&ps),
    );
    env.ok(&[ix], &[&resolver]);
    for (p, b) in ps.iter().zip(before) {
        assert_eq!(env.lamports(&p.pubkey()), b + STAKE);
    }
    let r = env.room(&room);
    assert_eq!((r.state, r.void_reason), (RoomState::Refunded, VOID_TIED));

    // two of three tied for first: they split the pot less the fee
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let (room, ps) = seated(&mut env, 3, 0, None, STAKE);
    let (a, b) = (env.lamports(&ps[0].pubkey()), env.lamports(&ps[2].pubkey()));
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![60_000, 70_000, 60_000],
        [3; 32],
        &[ps[0].pubkey(), ps[2].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    assert_eq!(env.lamports(&ps[0].pubkey()), a + 142_500_000);
    assert_eq!(env.lamports(&ps[2].pubkey()), b + 142_500_000);
}

#[test]
fn every_failure_refunds_void_by_the_resolver_and_deadlines_by_anyone() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let stranger = env.player();
    // the race couldn't produce a result: the resolver voids it
    let (room, ps) = seated(&mut env, 2, 0, None, STAKE);
    let before: Vec<u64> = ps.iter().map(|p| env.lamports(&p.pubkey())).collect();
    env.fails(
        &[ix_void(
            &stranger.pubkey(),
            &room,
            None,
            VOID_RACE,
            &keys(&ps),
        )],
        &[&stranger],
        ArenaError::Unauthorized,
    );
    env.ok(
        &[ix_void(
            &resolver.pubkey(),
            &room,
            None,
            VOID_RACE,
            &keys(&ps),
        )],
        &[&resolver],
    );
    for (p, b) in ps.iter().zip(before) {
        assert_eq!(env.lamports(&p.pubkey()), b + STAKE);
    }
    assert_eq!(env.room(&room).void_reason, VOID_RACE);

    // an open room nobody completes: anyone refunds after the join deadline
    let mut env = Env::new();
    let a = env.player();
    let ixs = ix_create_room(
        &a.pubkey(),
        &a.pubkey(),
        code(1),
        room_params(STAKE, 3),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&a]);
    let room = room_pda(&code(1));
    let s = env.player();
    env.fails(
        &[ix_refund_room(&room, None, &[a.pubkey()])],
        &[&s],
        ArenaError::TooEarly,
    );
    env.warp(JOIN_WINDOW as i64);
    env.fails(
        &[env.ix_join(&s.pubkey(), &room, None, PLAIN)],
        &[&s],
        ArenaError::JoinClosed,
    );
    let before = env.lamports(&a.pubkey());
    env.ok(&[ix_refund_room(&room, None, &[a.pubkey()])], &[&s]);
    assert_eq!(env.lamports(&a.pubkey()), before + STAKE);
    assert_eq!(env.room(&room).void_reason, VOID_EXPIRED);

    // a locked room the server never settles: anyone refunds after the settle
    // deadline, and the resolver can no longer settle it
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let (room, ps) = seated(&mut env, 2, 0, None, STAKE);
    let s = env.player();
    env.warp(SETTLE_WINDOW as i64 - 1);
    env.fails(
        &[ix_refund_room(&room, None, &keys(&ps))],
        &[&s],
        ArenaError::TooEarly,
    );
    env.warp(1);
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![1, 2],
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.fails(&[ix], &[&resolver], ArenaError::TooLate);
    env.ok(&[ix_refund_room(&room, None, &keys(&ps))], &[&s]);
    assert_eq!(env.room(&room).state, RoomState::Refunded);
    // nothing can be done twice
    env.fails(
        &[ix_refund_room(&room, None, &keys(&ps))],
        &[&s],
        ArenaError::NotRefundable,
    );
}

#[test]
fn players_leave_open_rooms_and_creators_cancel_them() {
    let mut env = Env::new();
    let a = env.player();
    let b = env.player();
    let ixs = ix_create_room(
        &a.pubkey(),
        &a.pubkey(),
        code(1),
        room_params(STAKE, 3),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&a]);
    let room = room_pda(&code(1));
    env.ok(&[env.ix_join(&b.pubkey(), &room, None, PLAIN)], &[&b]);
    env.fails(
        &[env.ix_join(&b.pubkey(), &room, None, PLAIN)],
        &[&b],
        ArenaError::AlreadyJoined,
    );
    env.fails(
        &[ix_leave(&a.pubkey(), &room, None)],
        &[&a],
        ArenaError::CreatorCannotLeave,
    );
    let before = env.lamports(&b.pubkey());
    env.ok(&[ix_leave(&b.pubkey(), &room, None)], &[&b]);
    assert_eq!(env.lamports(&b.pubkey()), before + STAKE - 5_000);
    let r = env.room(&room);
    assert_eq!((r.players.len(), r.levels.len()), (1, 1));
    // only the creator cancels, with every stake back
    env.fails(
        &[ix_cancel(&b.pubkey(), &room, None, &[a.pubkey()])],
        &[&b],
        ArenaError::Unauthorized,
    );
    let before = env.lamports(&a.pubkey());
    env.ok(&[ix_cancel(&a.pubkey(), &room, None, &[a.pubkey()])], &[&a]);
    assert_eq!(env.lamports(&a.pubkey()), before + STAKE - 5_000);
    assert_eq!(env.room(&room).void_reason, VOID_CANCELLED);
}

#[test]
fn garage_rooms_snapshot_each_players_levels_and_enforce_the_cap() {
    let mut env = Env::new();
    let creator = env.player();
    let full = env.player();
    let stock = env.player();
    let fake = env.player();
    env.set_upgrades(&creator.pubkey(), [1, 1, 1, 0, 0, 0]); // 700 + 10 + 15 + 19 = 744
    env.set_upgrades(&full.pubkey(), [5; 6]); // 1000
    let mut params = room_params(STAKE, 4);
    params.class = CLASS_GARAGE;
    params.max_rating = 850;
    let up = Who {
        upgrades: true,
        ..PLAIN
    };
    // GARAGE needs the creator's upgrades account
    let ixs = ix_create_room(
        &creator.pubkey(),
        &creator.pubkey(),
        code(1),
        params.clone(),
        None,
        PLAIN,
    );
    env.fails(&ixs, &[&creator], ArenaError::BadUpgrades);
    let ixs = ix_create_room(
        &creator.pubkey(),
        &creator.pubkey(),
        code(1),
        params,
        None,
        up,
    );
    env.ok(&ixs, &[&creator]);
    let room = room_pda(&code(1));
    // a full car is over the 85.0 cap
    env.fails(
        &[env.ix_join(&full.pubkey(), &room, None, up)],
        &[&full],
        ArenaError::RatingTooHigh,
    );
    // a player with no upgrades account yet races stock
    env.ok(&[env.ix_join(&stock.pubkey(), &room, None, up)], &[&stock]);
    // forged upgrades: another program's account, another discriminator, another owner inside
    env.set_upgrades_raw(
        &fake.pubkey(),
        [0; 6],
        Pubkey::new_unique(),
        arena::UPGRADES_DISCRIMINATOR,
        fake.pubkey(),
    );
    env.fails(
        &[env.ix_join(&fake.pubkey(), &room, None, up)],
        &[&fake],
        ArenaError::BadUpgrades,
    );
    env.set_upgrades_raw(
        &fake.pubkey(),
        [0; 6],
        arena::HOUSE_ID,
        [0; 8],
        fake.pubkey(),
    );
    env.fails(
        &[env.ix_join(&fake.pubkey(), &room, None, up)],
        &[&fake],
        ArenaError::BadUpgrades,
    );
    env.set_upgrades_raw(
        &fake.pubkey(),
        [0; 6],
        arena::HOUSE_ID,
        arena::UPGRADES_DISCRIMINATOR,
        creator.pubkey(),
    );
    env.fails(
        &[env.ix_join(&fake.pubkey(), &room, None, up)],
        &[&fake],
        ArenaError::BadUpgrades,
    );
    // levels above the top one
    env.set_upgrades(&fake.pubkey(), [6, 0, 0, 0, 0, 0]);
    env.fails(
        &[env.ix_join(&fake.pubkey(), &room, None, up)],
        &[&fake],
        ArenaError::BadUpgrades,
    );
    // another player's (real) upgrades account in the player's place
    let mut ix = env.ix_join(&fake.pubkey(), &room, None, up);
    ix.accounts[5].pubkey = upgrades_pda(&creator.pubkey());
    env.fails(&[ix], &[&fake], ArenaError::BadUpgrades);
    let r = env.room(&room);
    assert_eq!(r.levels, vec![[1, 1, 1, 0, 0, 0], [0; 6]]);
}

#[test]
fn a_room_opened_by_a_holder_gets_the_holder_fee_for_everyone() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let token = env.fee_mint(100);
    let ix = env.ix_admin(arena::instruction::SetHolder {
        holder_mint: token.mint,
        holder_min: 400_000_000,
        holder_room_fee_bps: 250,
        host_min: 1_000_000_000,
    });
    env.ok(&[ix], &[&admin]);
    let holder = env.player();
    let poor = env.player();
    env.give_tokens(&token, &holder.pubkey(), 400_000_000);
    env.give_tokens(&token, &poor.pubkey(), 399_999_999);
    let who = Who {
        holder: Some(&token),
        ..PLAIN
    };
    let ixs = ix_create_room(
        &poor.pubkey(),
        &poor.pubkey(),
        code(1),
        room_params(STAKE, 2),
        None,
        who,
    );
    env.fails(&ixs, &[&poor], ArenaError::NotAHolder);
    // someone else's holding doesn't count
    let mut ixs = ix_create_room(
        &poor.pubkey(),
        &poor.pubkey(),
        code(1),
        room_params(STAKE, 2),
        None,
        who,
    );
    ixs[0].accounts[8].pubkey = token.ata(&holder.pubkey());
    env.fails(&ixs, &[&poor], ArenaError::NotAHolder);
    let ixs = ix_create_room(
        &holder.pubkey(),
        &holder.pubkey(),
        code(2),
        room_params(STAKE, 2),
        None,
        who,
    );
    env.ok(&ixs, &[&holder]);
    assert_eq!(env.room(&room_pda(&code(2))).fee_bps, 250);
    // no account given: the token's own fee
    let ixs = ix_create_room(
        &holder.pubkey(),
        &holder.pubkey(),
        code(3),
        room_params(STAKE, 2),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&holder]);
    assert_eq!(env.room(&room_pda(&code(3))).fee_bps, 500);
}

#[test]
fn a_transfer_fee_token_room_holds_exact_stakes_and_pays_net_of_the_fee() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let acme = env.fee_mint(100); // 1%, as the project token
    env.allow_fee_mint(&acme, 100);
    let stake = 10_000_000; // 10 tokens
    let (room, ps) = seated(&mut env, 2, 0, Some(&acme), stake);
    // each player was charged the stake plus 1%; the escrow holds exactly the stakes
    assert_eq!(env.tokens(&acme.ata(&room)), 2 * stake);
    assert_eq!(
        env.tokens(&acme.ata(&ps[0].pubkey())),
        1_000_000_000 - 10_101_011
    );
    let r = env.room(&room);
    assert_eq!(r.fee_bps, 250);
    assert!(r.transfer_fee);
    let (w0, t0) = (
        env.tokens(&acme.ata(&ps[0].pubkey())),
        env.tokens(&acme.ata(&treasury())),
    );
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&acme),
        vec![60_000, 61_000],
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    // pot 20, house 2.5% = 0.5; the winner is sent 19.5 and nets it less 1%
    assert_eq!(
        env.tokens(&acme.ata(&ps[0].pubkey())),
        w0 + 19_500_000 - 195_000
    );
    assert_eq!(env.tokens(&acme.ata(&treasury())), t0 + 500_000 - 5_000);
    assert_eq!(env.tokens(&acme.ata(&room)), 0);
    // the fees withheld in the escrow are harvested to the mint, then it closes
    let c = env.player();
    env.ok(
        &[ix_close_room(
            &room,
            &env.sponsor.pubkey(),
            Some(&acme),
            false,
        )],
        &[&c],
    );
    assert!(!env.exists(&acme.ata(&room)));
}

#[test]
fn a_higher_scheduled_transfer_fee_stops_new_stakes_never_payouts() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let acme = env.fee_mint(100);
    env.allow_fee_mint(&acme, 100);
    let stake = 10_000_000;
    let players: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    for p in &players {
        env.give_tokens(&acme, &p.pubkey(), 100_000_000);
    }
    let mut params = room_params(stake, 3);
    params.race.start_mode = 1; // (its creator starts it: the resolver locks it with two in)
    let ixs = ix_create_room(
        &players[0].pubkey(),
        &players[0].pubkey(),
        code(1),
        params,
        Some(&acme),
        PLAIN,
    );
    env.ok(&ixs, &[&players[0]]);
    let room = room_pda(&code(1));
    env.ok(
        &[env.ix_join(&players[1].pubkey(), &room, Some(&acme), PLAIN)],
        &[&players[1]],
    );
    // the fee authority schedules 3%: new stakes stop at once
    let ix = fee_ix::set_transfer_fee(
        &acme.program,
        &acme.mint,
        &admin.pubkey(),
        &[],
        300,
        u64::MAX,
    )
    .unwrap();
    env.ok(&[ix], &[&admin]);
    env.fails(
        &[env.ix_join(&players[2].pubkey(), &room, Some(&acme), PLAIN)],
        &[&players[2]],
        ArenaError::TransferFeeTooHigh,
    );
    // the room still ends: the resolver locks and settles it, at the new fee
    env.warp_epochs(3);
    env.ok(&[ix_lock(&resolver.pubkey(), &room)], &[&resolver]);
    env.draw_seed(&room);
    let w = env.tokens(&acme.ata(&players[1].pubkey()));
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&acme),
        vec![61_000, 60_000],
        [1; 32],
        &[players[1].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    // pot 20 − 2.5% = 19.5 sent; 3% of it withheld
    assert_eq!(
        env.tokens(&acme.ata(&players[1].pubkey())),
        w + 19_500_000 - 585_000
    );
}

#[test]
fn anything_above_what_a_room_owed_goes_to_the_prize_vault_at_close() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let (room, ps) = seated(&mut env, 2, 0, None, STAKE);
    // a reward pushed to the escrow (as StonkFun pushes SOL to token holders)
    env.svm.airdrop(&room, 3_000_000).unwrap();
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![60_000, 61_000],
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    let v = env.lamports(&prize_vault());
    let c = env.player();
    env.ok(
        &[ix_close_room(&room, &env.sponsor.pubkey(), None, false)],
        &[&c],
    );
    assert_eq!(env.lamports(&prize_vault()), v + 3_000_000);

    // tokens left in a token room's escrow go to the vault's token account
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let spl = env.spl_token();
    let (room, ps) = seated(&mut env, 2, 0, Some(&spl), 5_000_000);
    env.give_tokens(&spl, &room, 42);
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&spl),
        vec![60_000, 61_000],
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.ok(&[ix], &[&resolver]);
    let c = env.player();
    // the vault's token account is needed when something is left
    env.fails(
        &[ix_close_room(
            &room,
            &env.sponsor.pubkey(),
            Some(&spl),
            false,
        )],
        &[&c],
        ArenaError::TokenAccountsMismatch,
    );
    let v = env.tokens(&spl.ata(&prize_vault()));
    env.ok(
        &[ix_close_room(
            &room,
            &env.sponsor.pubkey(),
            Some(&spl),
            true,
        )],
        &[&c],
    );
    assert_eq!(env.tokens(&spl.ata(&prize_vault())), v + 42);
}

/// A small deterministic generator for the sweeps.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn rooms_never_pay_out_more_than_their_pot() {
    let mut rng = Lcg(2026);
    for round in 0..12u32 {
        let mut env = Env::new();
        let resolver = env.resolver.insecure_clone();
        let n = 2 + (rng.next() % 7) as usize;
        let stake = SOL / 1_000 + rng.next() % (SOL / 10);
        let (room, ps) = seated(&mut env, n, 0, None, stake);
        let times: Vec<u32> = (0..n)
            .map(|_| match rng.next() % 5 {
                0 => DNF,
                _ => 60_000 + (rng.next() % 3) as u32,
            })
            .collect();
        let before: u64 =
            ps.iter().map(|p| env.lamports(&p.pubkey())).sum::<u64>() + env.lamports(&treasury());
        let room_before = env.lamports(&room);
        let r = arena::payout::room_outcome(stake, 500, &times).unwrap();
        let payees: Vec<Pubkey> = if r.refund_all {
            keys(&ps)
        } else {
            ps.iter()
                .enumerate()
                .filter(|(i, _)| r.winners & (1 << i) != 0)
                .map(|(_, p)| p.pubkey())
                .collect()
        };
        let ix = ix_settle(
            &resolver.pubkey(),
            &room,
            None,
            times.clone(),
            [round as u8; 32],
            &payees,
        );
        env.ok(&[ix], &[&resolver]);
        let after: u64 =
            ps.iter().map(|p| env.lamports(&p.pubkey())).sum::<u64>() + env.lamports(&treasury());
        // everything that left the escrow arrived at a player or the treasury, and
        // exactly the pot left it
        assert_eq!(after - before, stake * n as u64, "round {round}: {times:?}");
        assert_eq!(room_before - env.lamports(&room), stake * n as u64);
    }
}

#[test]
fn the_seed_is_drawn_after_the_lock_from_a_slot_nobody_could_see() {
    let mut env = Env::new();
    for _ in 0..10 {
        env.next_slot(); // (a chain some slots in)
    }
    let resolver = env.resolver.insecure_clone();
    let sponsor = env.sponsor.insecure_clone();
    let ps: Vec<Keypair> = (0..2).map(|_| env.player()).collect();
    let ixs = ix_create_room(
        &ps[0].pubkey(),
        &sponsor.pubkey(),
        code(1),
        room_params(STAKE, 2),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&ps[0], &sponsor]);
    let room = room_pda(&code(1));
    // a join commits to the stake it saw: a room opened again under its code
    // with another stake can't take it
    env.fails(
        &[ix_join_at(&ps[1].pubkey(), &room, None, PLAIN, STAKE * 2)],
        &[&ps[1]],
        ArenaError::StakeChanged,
    );
    // the last join locks the room, but draws no seed: whoever sends it can't see one
    env.ok(
        &[env.ix_join(&ps[1].pubkey(), &room, None, PLAIN)],
        &[&ps[1]],
    );
    let r = env.room(&room);
    assert_eq!((r.state, r.seed), (RoomState::Locked, [0u8; 32]));
    let settle = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        vec![61_000, 62_000],
        [1; 32],
        &[ps[0].pubkey()],
    );
    env.fails(
        std::slice::from_ref(&settle),
        &[&resolver],
        ArenaError::SeedNotDrawn,
    );
    // in the lock's own slot, that slot's hash doesn't exist yet
    let anyone = env.player();
    env.slot_hashes(&[(r.locked_slot - 1, [3u8; 32])]);
    env.fails(&[ix_draw_seed(&room)], &[&anyone], ArenaError::TooEarly);
    // once it is over, anyone draws it: hash(room ‖ that slot's hash ‖ the slot)
    env.slot_hashes(&[(r.locked_slot, [5u8; 32]), (r.locked_slot - 1, [3u8; 32])]);
    env.next_slot();
    env.ok(&[ix_draw_seed(&room)], &[&anyone]);
    let want =
        solana_sha256_hasher::hashv(&[room.as_ref(), &[5u8; 32], &r.locked_slot.to_le_bytes()]);
    assert_eq!(env.room(&room).seed, want.to_bytes());
    env.fails(&[ix_draw_seed(&room)], &[&anyone], ArenaError::SeedDrawn);
    env.ok(&[settle], &[&resolver]);

    // a lock's slot out of the sysvar's reach: no seed ever; the room is voided or refunded
    let mut env = Env::new();
    let (room, _) = {
        let ps: Vec<Keypair> = (0..2).map(|_| env.player()).collect();
        let ixs = ix_create_room(
            &ps[0].pubkey(),
            &ps[0].pubkey(),
            code(2),
            room_params(STAKE, 2),
            None,
            PLAIN,
        );
        env.ok(&ixs, &[&ps[0]]);
        let room = room_pda(&code(2));
        env.ok(
            &[env.ix_join(&ps[1].pubkey(), &room, None, PLAIN)],
            &[&ps[1]],
        );
        (room, ps)
    };
    let locked = env.room(&room).locked_slot;
    env.slot_hashes(&[(locked + 600, [1u8; 32])]);
    for _ in 0..601 {
        env.next_slot();
    }
    let anyone = env.player();
    env.fails(&[ix_draw_seed(&room)], &[&anyone], ArenaError::SeedExpired);
}

#[test]
fn the_resolver_locks_only_a_room_its_creator_starts_and_before_its_deadline() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let ps: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    // a room that starts when full, two of three in: the resolver can't start it early
    let ixs = ix_create_room(
        &ps[0].pubkey(),
        &ps[0].pubkey(),
        code(1),
        room_params(STAKE, 3),
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&ps[0]]);
    let room = room_pda(&code(1));
    env.ok(
        &[env.ix_join(&ps[1].pubkey(), &room, None, PLAIN)],
        &[&ps[1]],
    );
    env.fails(
        &[ix_lock(&resolver.pubkey(), &room)],
        &[&resolver],
        ArenaError::NotLockable,
    );
    // a room its creator starts: not after its join deadline (its refund is due then)
    let mut params = room_params(STAKE, 3);
    params.race.start_mode = 1;
    let ixs = ix_create_room(
        &ps[0].pubkey(),
        &ps[0].pubkey(),
        code(2),
        params,
        None,
        PLAIN,
    );
    env.ok(&ixs, &[&ps[0]]);
    let room = room_pda(&code(2));
    env.ok(
        &[env.ix_join(&ps[2].pubkey(), &room, None, PLAIN)],
        &[&ps[2]],
    );
    env.warp(env.room(&room).join_deadline - env.now());
    env.fails(
        &[ix_lock(&resolver.pubkey(), &room)],
        &[&resolver],
        ArenaError::NotLockable,
    );
}

#[test]
fn a_rooms_race_is_one_the_race_server_can_run() {
    let mut env = Env::new();
    let p = env.player();
    let bads: [fn(&mut RoomParams); 6] = [
        |r| r.race.laps = 0,
        |r| r.race.laps = 21,
        |r| r.race.format = 2,
        |r| r.race.time = 3,
        |r| r.race.sky = 6,
        |r| r.race.ai = 20,
    ];
    for bad in bads {
        let mut params = room_params(STAKE, 2);
        bad(&mut params);
        let ixs = ix_create_room(&p.pubkey(), &p.pubkey(), code(1), params, None, PLAIN);
        env.fails(&ixs, &[&p], ArenaError::InvalidParams);
    }
}

#[test]
fn the_holder_fee_needs_the_creators_own_token_account() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let token = env.spl_token();
    let ix = env.ix_admin(arena::instruction::SetHolder {
        holder_mint: token.mint,
        holder_min: 400_000_000,
        holder_room_fee_bps: 250,
        host_min: 1_000_000_000,
    });
    env.ok(&[ix], &[&admin]);
    let holder = env.player();
    // the tokens in an account of the holder's that isn't their associated one (its
    // owner could be moved for one transaction): no holder fee
    let other = litesvm_token::CreateAccount::new(&mut env.svm, &admin, &token.mint)
        .owner(&holder.pubkey())
        .send()
        .unwrap();
    litesvm_token::MintTo::new(&mut env.svm, &admin, &token.mint, &other, 500_000_000)
        .send()
        .unwrap();
    let who = Who {
        holder: Some(&token),
        ..PLAIN
    };
    let mut ixs = ix_create_room(
        &holder.pubkey(),
        &holder.pubkey(),
        code(1),
        room_params(STAKE, 2),
        None,
        who,
    );
    ixs[0].accounts[8].pubkey = other;
    env.fails(&ixs, &[&holder], ArenaError::NotAHolder);
    env.give_tokens(&token, &holder.pubkey(), 500_000_000);
    let ixs = ix_create_room(
        &holder.pubkey(),
        &holder.pubkey(),
        code(1),
        room_params(STAKE, 2),
        None,
        who,
    );
    env.ok(&ixs, &[&holder]);
    assert_eq!(env.room(&room_pda(&code(1))).fee_bps, 250);
}

#[test]
fn a_player_who_requires_memos_cant_block_a_refund_its_paid_with_one() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let acme = env.fee_mint(100);
    env.allow_fee_mint(&acme, 100);
    let stake = 10_000_000;
    let (room, ps) = seated(&mut env, 2, 1, Some(&acme), stake);
    // after joining, the second player turns required memos on for their account
    let b = &ps[1];
    let b_tokens = acme.ata(&b.pubkey());
    let turn_on = [
        t22::reallocate(
            &acme.program,
            &b_tokens,
            &b.pubkey(),
            &b.pubkey(),
            &[],
            &[ExtensionType::MemoTransfer],
        )
        .unwrap(),
        memo_ix::enable_required_transfer_memos(&acme.program, &b_tokens, &b.pubkey(), &[])
            .unwrap(),
    ];
    env.ok(&turn_on, &[b]);
    let void = || ix_void(&resolver.pubkey(), &room, Some(&acme), 3, &keys(&ps));
    // a payout to it needs a memo: without the memo program, the refund can't go
    env.fails(&[void()], &[&resolver], ArenaError::MemoRequired);
    // with it (the last remaining account), everyone is paid, that player too
    let mut ix = void();
    ix.accounts
        .push(AccountMeta::new_readonly(MEMO_PROGRAM_ID, false));
    let (a0, b0) = (
        env.tokens(&acme.ata(&ps[0].pubkey())),
        env.tokens(&b_tokens),
    );
    env.ok(&[ix], &[&resolver]);
    assert_eq!(env.room(&room).state, RoomState::Refunded);
    // (each nets the stake less the 1 % tax)
    assert_eq!(
        env.tokens(&acme.ata(&ps[0].pubkey())),
        a0 + stake - stake / 100
    );
    assert_eq!(env.tokens(&b_tokens), b0 + stake - stake / 100);
}

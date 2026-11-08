use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, system_program},
    InstructionData, ToAccountMetas,
};
use arena::{error::ArenaError, state::*};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::common::*;

const WEEK: i64 = 7 * 86_400;

fn keys(ps: &[Keypair]) -> Vec<Pubkey> {
    ps.iter().map(|p| p.pubkey()).collect()
}

fn ix_set_host(admin: &Pubkey, wallet: &Pubkey, until: i64) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::SetHost {
            wallet: *wallet,
            until,
        }
        .data(),
        arena::accounts::SetHost {
            admin: *admin,
            config: config_pda(),
            host: host_pda(wallet),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

/// Past the cup's end, results posted for `winners` (rank order) and finalized.
fn finish(env: &mut Env, cup: &Pubkey, winners: &[Pubkey], scorers: u32) -> Vec<Vec<[u8; 32]>> {
    let resolver = env.resolver.insecure_clone();
    let (root, proofs) = results_tree(cup, winners);
    env.warp(WEEK);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), cup, root, scorers)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(cup)], &[&resolver]);
    proofs
}

#[test]
fn a_board_cup_pays_the_weeks_top_three_from_its_guarantee() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let usdc = env.spl_token();
    env.give_tokens(&usdc, &admin.pubkey(), 1_000_000_000);
    let now = env.now();
    let cup = cup_pda(0);
    // a board has no entry fee; at most 90 days
    let p = cup_params(CupKind::Board, 0, 100_000_000, now);
    let mut bad = p.clone();
    bad.entry_fee = 1_000_000;
    env.fails(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            bad,
            Some(&usdc),
            false,
            None,
        ),
        &[&admin],
        ArenaError::InvalidParams,
    );
    let mut bad = p.clone();
    bad.ends_at = now + 91 * 86_400;
    env.fails(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            bad,
            Some(&usdc),
            false,
            None,
        ),
        &[&admin],
        ArenaError::InvalidWindow,
    );
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            p.clone(),
            Some(&usdc),
            false,
            None,
        ),
        &[&admin],
    );
    // an id is the creator's choice (a random one can't be taken first), but one in use is refused
    env.fails_any(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            p,
            Some(&usdc),
            false,
            None,
        ),
        &[&admin],
    );
    let c = env.cup(&cup);
    assert_eq!(c.featured_mint, Pubkey::new_from_array([42; 32]));
    assert_eq!(
        (c.host, c.sponsor, c.rent_payer),
        (Pubkey::default(), admin.pubkey(), admin.pubkey())
    );
    assert_eq!(env.tokens(&usdc.ata(&cup)), 100_000_000);
    assert_eq!(env.config().cup_count, 1);

    // a board takes no entries
    let stranger = env.player();
    env.ensure_ata(&usdc, &stranger.pubkey());
    let ix = ix_enter(
        &stranger.pubkey(),
        &stranger.pubkey(),
        &cup,
        Some(&usdc),
        None,
    );
    env.fails(&[ix], &[&stranger], ArenaError::NoEntries);

    // results only once the week is over, from a resolver
    let winners: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let w = keys(&winners);
    for k in &w {
        env.ensure_ata(&usdc, k);
    }
    let (root, proofs) = results_tree(&cup, &w);
    env.fails(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 57)],
        &[&resolver],
        ArenaError::TooEarly,
    );
    env.warp(WEEK);
    env.fails(
        &[env.ix_post(&stranger.pubkey(), &cup, root, 57)],
        &[&stranger],
        ArenaError::Unauthorized,
    );
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 57)],
        &[&resolver],
    ); // 57 drivers set a time
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            proofs[0].clone(),
            Some(&usdc),
            false,
        )],
        &[&winners[0]],
        ArenaError::NotFinalized,
    );
    env.fails(&[ix_finalize(&cup)], &[&stranger], ArenaError::TooEarly);
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&stranger]);
    let c = env.cup(&cup);
    assert_eq!(
        (c.pool, c.places, c.claim_until),
        (100_000_000, 3, env.now() + CLAIM_WINDOW as i64)
    );

    // the winner claims (and may swap the prize into the week's stock in the same transaction)
    env.ok(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            proofs[0].clone(),
            Some(&usdc),
            false,
        )],
        &[&winners[0]],
    );
    assert_eq!(env.tokens(&usdc.ata(&w[0])), 50_000_000);
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            proofs[0].clone(),
            Some(&usdc),
            false,
        )],
        &[&winners[0]],
        ArenaError::AlreadyClaimed,
    );
    env.fails(
        &[ix_claim(
            &w[1],
            &cup,
            4,
            &w[1],
            proofs[1].clone(),
            Some(&usdc),
            false,
        )],
        &[&winners[1]],
        ArenaError::InvalidRank,
    );
    // paid only to the leaf's wallet
    let mut ix = ix_claim(&w[1], &cup, 2, &w[1], proofs[1].clone(), Some(&usdc), false);
    ix.accounts[6].pubkey = usdc.ata(&stranger.pubkey());
    env.fails(&[ix], &[&winners[1]], ArenaError::PayoutAccountsMismatch);
    // until the claim window ends, only the winner may claim
    env.fails(
        &[ix_claim(
            &stranger.pubkey(),
            &cup,
            2,
            &w[1],
            proofs[1].clone(),
            Some(&usdc),
            false,
        )],
        &[&stranger],
        ArenaError::TooEarly,
    );
    env.ok(
        &[ix_claim(
            &w[1],
            &cup,
            2,
            &w[1],
            proofs[1].clone(),
            Some(&usdc),
            false,
        )],
        &[&winners[1]],
    );
    // nothing closes while a prize is owed
    env.fails(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            Some(&usdc),
        )],
        &[&stranger],
        ArenaError::NotClosable,
    );
    // after it, anyone pushes the prize to its winner
    env.warp(CLAIM_WINDOW as i64);
    env.ok(
        &[ix_claim(
            &stranger.pubkey(),
            &cup,
            3,
            &w[2],
            proofs[2].clone(),
            Some(&usdc),
            false,
        )],
        &[&stranger],
    );
    assert_eq!(env.tokens(&usdc.ata(&w[2])), 20_000_000);
    assert_eq!(env.cup(&cup).paid, 100_000_000);

    // all paid: the cup and its token account close, their rent back to the payer
    let rent = env.lamports(&cup) + env.lamports(&usdc.ata(&cup));
    let before = env.lamports(&admin.pubkey());
    env.ok(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            Some(&usdc),
        )],
        &[&stranger],
    );
    assert!(!env.exists(&cup) && !env.exists(&usdc.ata(&cup)));
    assert_eq!(env.lamports(&admin.pubkey()), before + rent);
}

#[test]
fn claims_need_the_proof_of_that_rank_and_that_wallet() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let now = env.now();
    let cup = cup_pda(0);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            cup_params(CupKind::Board, 0, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    let winners: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let w = keys(&winners);
    let proofs = finish(&mut env, &cup, &w, 3);
    let stranger = env.player();
    // rank 1's proof for rank 2; rank 1's wallet with rank 2's proof; a wallet not in the results
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            2,
            &w[0],
            proofs[0].clone(),
            None,
            false,
        )],
        &[&winners[0]],
        ArenaError::InvalidProof,
    );
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            proofs[1].clone(),
            None,
            false,
        )],
        &[&winners[0]],
        ArenaError::InvalidProof,
    );
    env.fails(
        &[ix_claim(
            &w[1],
            &cup,
            1,
            &w[1],
            proofs[0].clone(),
            None,
            false,
        )],
        &[&winners[1]],
        ArenaError::InvalidProof,
    );
    let s = stranger.pubkey();
    env.fails(
        &[ix_claim(&s, &cup, 1, &s, proofs[0].clone(), None, false)],
        &[&stranger],
        ArenaError::InvalidProof,
    );
    env.fails(
        &[ix_claim(&s, &cup, 1, &s, vec![[0u8; 32]; 17], None, false)],
        &[&stranger],
        ArenaError::InvalidProof,
    );
    for (i, k) in winners.iter().enumerate() {
        let before = env.lamports(&k.pubkey());
        env.ok(
            &[ix_claim(
                &k.pubkey(),
                &cup,
                i as u16 + 1,
                &k.pubkey(),
                proofs[i].clone(),
                None,
                false,
            )],
            &[k],
        );
        assert_eq!(
            env.lamports(&k.pubkey()),
            before + [SOL / 2, SOL * 3 / 10, SOL / 5][i] - 5_000
        );
    }

    // a board where one driver set a time: the whole pool is theirs
    let now = env.now();
    let cup = cup_pda(1);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            1,
            cup_params(CupKind::Board, 0, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    let proofs = finish(&mut env, &cup, &w[..1], 1);
    assert_eq!(env.cup(&cup).places, 1);
    env.fails(
        &[ix_claim(&w[1], &cup, 2, &w[1], vec![], None, false)],
        &[&winners[1]],
        ArenaError::InvalidRank,
    );
    let before = env.lamports(&w[0]);
    env.ok(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            proofs[0].clone(),
            None,
            false,
        )],
        &[&winners[0]],
    );
    assert_eq!(env.lamports(&w[0]), before + SOL - 5_000);
}

#[test]
fn an_entries_cup_pays_its_pool_and_its_fee_goes_to_the_treasury() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let sponsor = env.sponsor.insecure_clone();
    let now = env.now();
    let cup = cup_pda(0);
    let mut p = cup_params(CupKind::Entries, SOL / 10, 0, now);
    p.max_entrants = 5;
    env.ok(
        &ix_create_cup(&admin.pubkey(), &admin.pubkey(), 0, p, None, false, None),
        &[&admin],
    );
    let ps: Vec<Keypair> = (0..5).map(|_| env.player()).collect();
    for k in &ps {
        // the race server's sponsor pays each entry's rent
        env.ok(
            &[ix_enter(&k.pubkey(), &sponsor.pubkey(), &cup, None, None)],
            &[k, &sponsor],
        );
    }
    let late = env.player();
    env.fails(
        &[ix_enter(&late.pubkey(), &late.pubkey(), &cup, None, None)],
        &[&late],
        ArenaError::CupFull,
    );
    env.fails_any(
        &[ix_enter(&ps[0].pubkey(), &ps[0].pubkey(), &cup, None, None)],
        &[&ps[0]],
    );
    assert_eq!(env.cup(&cup).entrants, 5);
    assert_eq!(
        env.lamports(&cup)
            - env
                .svm
                .minimum_balance_for_rent_exemption(env.svm.get_account(&cup).unwrap().data.len()),
        SOL / 2
    );

    let order = [ps[3].pubkey(), ps[0].pubkey(), ps[4].pubkey()];
    let (root, proofs) = results_tree(&cup, &order);
    env.warp(WEEK);
    // more scorers than entrants
    env.fails(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 6)],
        &[&resolver],
        ArenaError::InvalidParams,
    );
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 5)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&resolver]);
    // pool: 5 × 0.1 less the 10 % fee = 0.45; 50 / 30 / 20 %
    assert_eq!(env.cup(&cup).pool, 450_000_000);
    // an entries cup pays only an entrant: their own entry is needed
    env.fails(
        &[ix_claim(
            &order[0],
            &cup,
            1,
            &order[0],
            proofs[0].clone(),
            None,
            false,
        )],
        &[&ps[3]],
        ArenaError::Unauthorized,
    );
    let mut ix = ix_claim(&order[0], &cup, 1, &order[0], proofs[0].clone(), None, true);
    ix.accounts[5].pubkey = entry_pda(&cup, &ps[1].pubkey());
    env.fails(&[ix], &[&ps[3]], ArenaError::Unauthorized);
    // an entry can't be closed while prizes are owed
    env.fails(
        &[ix_close_entry(&cup, &ps[1].pubkey(), &sponsor.pubkey())],
        &[&ps[1]],
        ArenaError::NotClosable,
    );
    let prizes = [225_000_000, 135_000_000, 90_000_000];
    for (i, k) in [&ps[3], &ps[0], &ps[4]].into_iter().enumerate() {
        let before = env.lamports(&k.pubkey());
        env.ok(
            &[ix_claim(
                &k.pubkey(),
                &cup,
                i as u16 + 1,
                &k.pubkey(),
                proofs[i].clone(),
                None,
                true,
            )],
            &[k],
        );
        assert_eq!(env.lamports(&k.pubkey()), before + prizes[i] - 5_000);
    }
    // the fee to the treasury; the rent back to the payer
    let (t0, a0) = (env.lamports(&treasury()), env.lamports(&admin.pubkey()));
    let rent = env.lamports(&cup) - SOL / 20;
    env.ok(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            None,
        )],
        &[&resolver],
    );
    assert_eq!(env.lamports(&treasury()), t0 + SOL / 20);
    assert_eq!(env.lamports(&admin.pubkey()), a0 + rent);
    // each entry's rent back to the sponsor that paid it, never to the player
    env.fails_any(
        &[ix_close_entry(&cup, &ps[1].pubkey(), &ps[1].pubkey())],
        &[&ps[1]],
    );
    let entry_rent = env.lamports(&entry_pda(&cup, &ps[0].pubkey()));
    let s0 = env.lamports(&sponsor.pubkey());
    for k in &ps {
        env.ok(
            &[ix_close_entry(&cup, &k.pubkey(), &sponsor.pubkey())],
            &[&resolver],
        );
    }
    assert_eq!(env.lamports(&sponsor.pubkey()), s0 + 5 * entry_rent);
}

#[test]
fn wrong_results_are_rejected_in_the_challenge_window_and_posted_again() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let guardian = env.guardian.insecure_clone();
    let now = env.now();
    let cup = cup_pda(0);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            cup_params(CupKind::Board, 0, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    let winners: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let w = keys(&winners);
    let (wrong, old_proofs) = results_tree(&cup, &w);
    env.warp(WEEK);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, wrong, 3)],
        &[&resolver],
    );
    // only the guardian or the admin rejects (not a resolver, not anyone)
    let stranger = env.player();
    env.fails(
        &[ix_guard("reject", &stranger.pubkey(), &cup)],
        &[&stranger],
        ArenaError::Unauthorized,
    );
    env.fails(
        &[ix_guard("reject", &resolver.pubkey(), &cup)],
        &[&resolver],
        ArenaError::Unauthorized,
    );
    env.ok(
        &[ix_guard("reject", &guardian.pubkey(), &cup)],
        &[&guardian],
    );
    assert_eq!(env.cup(&cup).state, CupState::Open);
    env.fails(
        &[ix_finalize(&cup)],
        &[&stranger],
        ArenaError::ResultsNotPosted,
    );
    // posted again, right; the window has passed: it stands
    let right = [w[2], w[0], w[1]];
    let (root, proofs) = results_tree(&cup, &right);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 3)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.fails(
        &[ix_guard("reject", &guardian.pubkey(), &cup)],
        &[&guardian],
        ArenaError::TooLate,
    );
    env.ok(&[ix_finalize(&cup)], &[&stranger]);
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            1,
            &w[0],
            old_proofs[0].clone(),
            None,
            false,
        )],
        &[&winners[0]],
        ArenaError::InvalidProof,
    );
    let before = env.lamports(&w[2]);
    env.ok(
        &[ix_claim(
            &w[2],
            &cup,
            1,
            &w[2],
            proofs[0].clone(),
            None,
            false,
        )],
        &[&winners[2]],
    );
    assert_eq!(env.lamports(&w[2]), before + SOL / 2 - 5_000);
    // finalized: no cancelling now
    env.fails(
        &[ix_guard("cancel", &guardian.pubkey(), &cup)],
        &[&guardian],
        ArenaError::NotRefundable,
    );
}

#[test]
fn a_cancelled_cup_returns_every_entry_and_the_guarantee() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let guardian = env.guardian.insecure_clone();
    let now = env.now();
    let cup = cup_pda(0);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            cup_params(CupKind::Entries, SOL / 10, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    let ps: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    for k in &ps {
        env.ok(
            &[ix_enter(&k.pubkey(), &k.pubkey(), &cup, None, None)],
            &[k],
        );
    }
    let stranger = env.player();
    env.fails(
        &[ix_guard("cancel", &stranger.pubkey(), &cup)],
        &[&stranger],
        ArenaError::Unauthorized,
    );
    env.fails(
        &[ix_refund_entry(&cup, &ps[0].pubkey(), None)],
        &[&stranger],
        ArenaError::NotRefundable,
    );
    env.ok(
        &[ix_guard("cancel", &guardian.pubkey(), &cup)],
        &[&guardian],
    );
    env.fails(
        &[ix_enter(
            &stranger.pubkey(),
            &stranger.pubkey(),
            &cup,
            None,
            None,
        )],
        &[&stranger],
        ArenaError::CupNotOpen,
    );
    env.fails(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            None,
        )],
        &[&stranger],
        ArenaError::NotClosable,
    );
    // anyone sends every entry back, in full, once
    for k in &ps {
        let before = env.lamports(&k.pubkey());
        env.ok(&[ix_refund_entry(&cup, &k.pubkey(), None)], &[&stranger]);
        assert_eq!(env.lamports(&k.pubkey()), before + SOL / 10);
    }
    env.fails(
        &[ix_refund_entry(&cup, &ps[0].pubkey(), None)],
        &[&stranger],
        ArenaError::EntryRefunded,
    );
    let before = env.lamports(&admin.pubkey());
    env.ok(
        &[ix_return_guarantee(&cup, &admin.pubkey(), None)],
        &[&stranger],
    );
    assert_eq!(env.lamports(&admin.pubkey()), before + SOL);
    env.fails(
        &[ix_return_guarantee(&cup, &admin.pubkey(), None)],
        &[&stranger],
        ArenaError::NotRefundable,
    );
    env.ok(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            None,
        )],
        &[&stranger],
    );
    // a refunded entry closes at once
    env.ok(
        &[ix_close_entry(&cup, &ps[0].pubkey(), &ps[0].pubkey())],
        &[&stranger],
    );

    // results that never arrive: after the deadline anyone refunds, which cancels the cup
    let resolver = env.resolver.insecure_clone();
    let now = env.now();
    let cup = cup_pda(1);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            1,
            cup_params(CupKind::Entries, SOL / 10, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    env.ok(
        &[ix_enter(&ps[1].pubkey(), &ps[1].pubkey(), &cup, None, None)],
        &[&ps[1]],
    );
    env.warp(WEEK + RESULTS_WINDOW as i64);
    let (root, _) = results_tree(&cup, &[ps[1].pubkey()]);
    env.fails(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 1)],
        &[&resolver],
        ArenaError::TooLate,
    );
    let before = env.lamports(&ps[1].pubkey());
    env.ok(
        &[ix_refund_entry(&cup, &ps[1].pubkey(), None)],
        &[&stranger],
    );
    assert_eq!(env.lamports(&ps[1].pubkey()), before + SOL / 10);
    assert_eq!(env.cup(&cup).state, CupState::Cancelled);
    env.ok(
        &[ix_return_guarantee(&cup, &admin.pubkey(), None)],
        &[&stranger],
    );
}

#[test]
fn community_hosts_need_approval_or_a_holding_and_get_their_share_of_the_fee() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let host = env.player();
    let now = env.now();
    let mut p = cup_params(CupKind::Entries, SOL / 10, 0, now);
    p.host_share_bps = 2_500;
    // a stranger can't organize
    env.fails(
        &ix_create_cup(
            &host.pubkey(),
            &host.pubkey(),
            0,
            p.clone(),
            None,
            false,
            None,
        ),
        &[&host],
        ArenaError::Unauthorized,
    );
    // an approved host can, for as long as approved, with at most half the fee
    env.ok(
        &[ix_set_host(
            &admin.pubkey(),
            &host.pubkey(),
            now + 30 * 86_400,
        )],
        &[&admin],
    );
    let mut greedy = p.clone();
    greedy.host_share_bps = 5_001;
    env.fails(
        &ix_create_cup(&host.pubkey(), &host.pubkey(), 0, greedy, None, true, None),
        &[&host],
        ArenaError::InvalidParams,
    );
    env.ok(
        &ix_create_cup(
            &host.pubkey(),
            &host.pubkey(),
            0,
            p.clone(),
            None,
            true,
            None,
        ),
        &[&host],
    );
    let cup = cup_pda(0);
    let c = env.cup(&cup);
    assert_eq!((c.host, c.host_share_bps), (host.pubkey(), 2_500));
    let ps: Vec<Keypair> = (0..4).map(|_| env.player()).collect();
    for k in &ps {
        env.ok(
            &[ix_enter(&k.pubkey(), &k.pubkey(), &cup, None, None)],
            &[k],
        );
    }
    let order = keys(&ps[..3]);
    let proofs = finish(&mut env, &cup, &order, 4);
    for (i, k) in ps[..3].iter().enumerate() {
        env.ok(
            &[ix_claim(
                &k.pubkey(),
                &cup,
                i as u16 + 1,
                &k.pubkey(),
                proofs[i].clone(),
                None,
                true,
            )],
            &[k],
        );
    }
    // the house fee is 0.04 SOL: a quarter to the host, the rest to the treasury
    let stranger = env.player();
    env.fails(
        &[ix_close_cup(
            &cup,
            &host.pubkey(),
            &host.pubkey(),
            None,
            None,
        )],
        &[&stranger],
        ArenaError::PayoutAccountsMismatch,
    );
    let (t0, h0) = (env.lamports(&treasury()), env.lamports(&host.pubkey()));
    let rent = env.lamports(&cup) - SOL / 25;
    env.ok(
        &[ix_close_cup(
            &cup,
            &host.pubkey(),
            &host.pubkey(),
            Some(&host.pubkey()),
            None,
        )],
        &[&stranger],
    );
    assert_eq!(env.lamports(&treasury()), t0 + 30_000_000);
    assert_eq!(env.lamports(&host.pubkey()), h0 + 10_000_000 + rent);

    // the approval runs out
    env.warp(30 * 86_400);
    let now = env.now();
    let mut p = cup_params(CupKind::Entries, SOL / 10, 0, now);
    p.host_share_bps = 2_500;
    env.fails(
        &ix_create_cup(
            &host.pubkey(),
            &host.pubkey(),
            1,
            p.clone(),
            None,
            true,
            None,
        ),
        &[&host],
        ArenaError::Unauthorized,
    );

    // or hold enough of the project token
    let token = env.fee_mint(100);
    let ix = env.ix_admin(arena::instruction::SetHolder {
        holder_mint: token.mint,
        holder_min: 400_000_000,
        holder_room_fee_bps: 250,
        host_min: 1_000_000_000,
    });
    env.ok(&[ix], &[&admin]);
    let holder = env.player();
    let small = env.player();
    env.give_tokens(&token, &holder.pubkey(), 1_000_000_000);
    env.give_tokens(&token, &small.pubkey(), 999_999_999);
    env.fails(
        &ix_create_cup(
            &small.pubkey(),
            &small.pubkey(),
            1,
            p.clone(),
            None,
            false,
            Some(&token),
        ),
        &[&small],
        ArenaError::Unauthorized,
    );
    // a holder hosts, but takes no share of the fee: a balance checked for one
    // instant could have been borrowed for it
    env.fails(
        &ix_create_cup(
            &holder.pubkey(),
            &holder.pubkey(),
            1,
            p.clone(),
            None,
            false,
            Some(&token),
        ),
        &[&holder],
        ArenaError::InvalidParams,
    );
    p.host_share_bps = 0;
    env.ok(
        &ix_create_cup(
            &holder.pubkey(),
            &holder.pubkey(),
            1,
            p,
            None,
            false,
            Some(&token),
        ),
        &[&holder],
    );
    let c = env.cup(&cup_pda(1));
    assert_eq!((c.host, c.host_share_bps), (holder.pubkey(), 0));

    // a house cup has no host and no host share, whatever it asks for
    let mut p = cup_params(CupKind::Board, 0, SOL, env.now());
    p.host_share_bps = 2_500;
    env.ok(
        &ix_create_cup(&admin.pubkey(), &admin.pubkey(), 2, p, None, false, None),
        &[&admin],
    );
    let c = env.cup(&cup_pda(2));
    assert_eq!((c.host, c.host_share_bps), (Pubkey::default(), 0));
}

#[test]
fn a_cup_in_the_project_token_pays_prizes_net_of_its_transfer_fee() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let acme = env.fee_mint(100);
    env.allow_fee_mint(&acme, 100);
    env.give_tokens(&acme, &admin.pubkey(), 1_000_000_000);
    let now = env.now();
    let cup = cup_pda(0);
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            cup_params(CupKind::Board, 0, 100_000_000, now),
            Some(&acme),
            false,
            None,
        ),
        &[&admin],
    );
    // the sponsor paid the guarantee plus the tax; the cup holds exactly the guarantee
    assert_eq!(env.tokens(&acme.ata(&cup)), 100_000_000);
    assert_eq!(
        env.tokens(&acme.ata(&admin.pubkey())),
        1_000_000_000 - 101_010_102
    );
    let winners: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let w = keys(&winners);
    for k in &w {
        env.ensure_ata(&acme, k);
    }
    let proofs = finish(&mut env, &cup, &w, 3);
    for (i, k) in winners.iter().enumerate() {
        env.ok(
            &[ix_claim(
                &w[i],
                &cup,
                i as u16 + 1,
                &w[i],
                proofs[i].clone(),
                Some(&acme),
                false,
            )],
            &[k],
        );
    }
    // each nets their prize less 1 %
    assert_eq!(env.tokens(&acme.ata(&w[0])), 49_500_000);
    assert_eq!(env.tokens(&acme.ata(&w[1])), 29_700_000);
    assert_eq!(env.tokens(&acme.ata(&w[2])), 19_800_000);
    // the withheld fees are harvested to the mint and the escrow closes
    let stranger = env.player();
    env.ok(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            Some(&acme),
        )],
        &[&stranger],
    );
    assert!(!env.exists(&acme.ata(&cup)) && !env.exists(&cup));
}

#[test]
fn results_state_their_paid_places_and_a_place_nobody_takes_doesnt_hold_the_rest() {
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let now = env.now();
    // a board cup in SOL: 1 SOL for three places (50 / 30 / 20)
    env.ok(
        &ix_create_cup(
            &admin.pubkey(),
            &admin.pubkey(),
            0,
            cup_params(CupKind::Board, 0, SOL, now),
            None,
            false,
            None,
        ),
        &[&admin],
    );
    let cup = cup_pda(0);
    let winners: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let (root, proofs) = results_tree(&cup, &keys(&winners));
    env.warp(WEEK);
    // the places posted must be the ones the cup pays (min(table, scorers)), over a real root
    env.fails(
        &[ix_post_places(&resolver.pubkey(), &cup, root, 10, 2)],
        &[&resolver],
        ArenaError::InvalidParams,
    );
    env.fails(
        &[ix_post_places(&resolver.pubkey(), &cup, [0u8; 32], 10, 3)],
        &[&resolver],
        ArenaError::InvalidParams,
    );
    env.ok(
        &[ix_post_places(&resolver.pubkey(), &cup, root, 10, 3)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&resolver]);
    // two winners take theirs; the third never can (as if their wallet couldn't take it)
    for i in 0..2 {
        env.ok(
            &[ix_claim(
                &winners[i].pubkey(),
                &cup,
                i as u16 + 1,
                &winners[i].pubkey(),
                proofs[i].clone(),
                None,
                false,
            )],
            &[&winners[i]],
        );
    }
    let anyone = env.player();
    let close = ix_close_cup(&cup, &admin.pubkey(), &admin.pubkey(), None, None);
    env.fails(
        std::slice::from_ref(&close),
        &[&anyone],
        ArenaError::NotClosable,
    );
    env.warp(CLAIM_WINDOW as i64);
    env.fails(
        std::slice::from_ref(&close),
        &[&anyone],
        ArenaError::NotClosable,
    );
    // a week after the claim window it closes anyway: the unpaid prize back to its sponsor
    env.warp(arena::FORFEIT_SECS);
    let s0 = env.lamports(&admin.pubkey());
    let rent = env.lamports(&cup) - SOL / 5;
    env.ok(&[close], &[&anyone]);
    assert!(!env.exists(&cup));
    assert_eq!(env.lamports(&admin.pubkey()), s0 + SOL / 5 + rent);
}

// A season's league on the cups as they are (docs/solana-plan.md "Seasons"; server/chain/season.js): its drivers' table is
// one cup, its teams' table another, whose places are halves - a team's place is two ranks in a row, one for each of
// its drivers, and a team of one driver holds both. The program computes every amount from those tables; the race
// server posts only who finished where.
use anchor_lang::prelude::Pubkey;
use arena::{error::ArenaError, state::*};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::common::*;

const SEASON: i64 = 14 * 86_400;
/// The teams' table (src/ui/data/copy.js, FORMATS' season teamBps) as half-places.
const TEAM_HALVES: [u16; 10] = [2_000, 2_000, 1_250, 1_250, 850, 850, 550, 550, 350, 350];

fn season_cup(
    env: &mut Env,
    id: u64,
    kind: CupKind,
    fee: u64,
    guarantee: u64,
    spl: Option<&Spl>,
) -> Pubkey {
    let admin = env.admin.insecure_clone();
    let now = env.now();
    let mut p = cup_params(kind, fee, guarantee, now);
    p.payout_bps = TEAM_HALVES.to_vec();
    p.ends_at = now + SEASON;
    p.featured_mint = Pubkey::default();
    env.ok(
        &ix_create_cup(&admin.pubkey(), &admin.pubkey(), id, p, spl, false, None),
        &[&admin],
    );
    cup_pda(id)
}

fn share(pool: u64, bps: &[u16], places: usize, rank: usize) -> u64 {
    let used: u64 = bps[..places].iter().map(|&b| b as u64).sum();
    pool * bps[rank - 1] as u64 / used
}

#[test]
fn a_teams_place_is_two_halves_and_a_team_of_one_holds_both() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let sponsor = env.sponsor.insecure_clone();
    let cup = season_cup(&mut env, 11, CupKind::Entries, SOL / 10, 0, None);
    let ds: Vec<Keypair> = (0..6).map(|_| env.player()).collect();
    for k in &ds {
        env.ok(
            &[ix_enter(&k.pubkey(), &sponsor.pubkey(), &cup, None, None)],
            &[k, &sponsor],
        );
    }
    let w: Vec<Pubkey> = ds.iter().map(|k| k.pubkey()).collect();
    // the teams that placed: w0 and w1's, w2's (a team of one: both halves), w3 and w4's; w5 raced for nobody's team
    let order = [w[0], w[1], w[2], w[2], w[3], w[4]];
    let (root, proofs) = results_tree(&cup, &order);
    env.warp(SEASON);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, order.len() as u32)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&resolver]);
    let c = env.cup(&cup);
    assert_eq!((c.pool, c.places), (540_000_000, 6));
    // the team of one claims both its halves in one transaction
    let before = env.lamports(&w[2]);
    env.ok(
        &[
            ix_claim(&w[2], &cup, 3, &w[2], proofs[2].clone(), None, true),
            ix_claim(&w[2], &cup, 4, &w[2], proofs[3].clone(), None, true),
        ],
        &[&ds[2]],
    );
    let half = share(c.pool, &TEAM_HALVES, 6, 3);
    assert_eq!(env.lamports(&w[2]), before + 2 * half - 5_000);
    // a half is paid once; a driver can't take their teammate's half
    env.fails(
        &[ix_claim(
            &w[2],
            &cup,
            3,
            &w[2],
            proofs[2].clone(),
            None,
            true,
        )],
        &[&ds[2]],
        ArenaError::AlreadyClaimed,
    );
    env.fails(
        &[ix_claim(
            &w[0],
            &cup,
            2,
            &w[0],
            proofs[1].clone(),
            None,
            true,
        )],
        &[&ds[0]],
        ArenaError::InvalidProof,
    );
    for (i, k) in [(0usize, &ds[0]), (1, &ds[1]), (4, &ds[3]), (5, &ds[4])] {
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
    let done = env.cup(&cup);
    assert!(done.all_claimed());
    // every half of a place the same: the two drivers of a team get the same
    let paid: u64 = (1..=6).map(|r| share(c.pool, &TEAM_HALVES, 6, r)).sum();
    assert_eq!(done.paid, paid);
    assert!(done.paid <= done.pool);
}

#[test]
fn an_entries_cup_names_no_more_places_than_it_has_entrants() {
    // three drivers: a team of two and a team of one place; the halves would be four leaves for three entrants
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let sponsor = env.sponsor.insecure_clone();
    let cup = season_cup(&mut env, 12, CupKind::Entries, SOL / 10, 0, None);
    let ds: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    for k in &ds {
        env.ok(
            &[ix_enter(&k.pubkey(), &sponsor.pubkey(), &cup, None, None)],
            &[k, &sponsor],
        );
    }
    let w: Vec<Pubkey> = ds.iter().map(|k| k.pubkey()).collect();
    let all = [w[0], w[1], w[2], w[2]];
    let (root, _) = results_tree(&cup, &all);
    env.warp(SEASON);
    env.fails(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 4)],
        &[&resolver],
        ArenaError::InvalidParams,
    );
    // so the race server names as many halves as there are entrants: the team of one holds one, and the shares are
    // spread over the three places reached (server/chain/season.js teamLeaves; the menus show the same numbers)
    let three = [w[0], w[1], w[2]];
    let (root, proofs) = results_tree(&cup, &three);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 3)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&resolver]);
    let c = env.cup(&cup);
    assert_eq!((c.pool, c.places, c.used_bps), (270_000_000, 3, 5_250));
    let before = env.lamports(&w[2]);
    env.ok(
        &[ix_claim(
            &w[2],
            &cup,
            3,
            &w[2],
            proofs[2].clone(),
            None,
            true,
        )],
        &[&ds[2]],
    );
    assert_eq!(
        env.lamports(&w[2]),
        before + 270_000_000 * 1_250 / 5_250 - 5_000
    );
}

#[test]
fn a_free_leagues_board_cup_names_both_halves_of_a_team_of_one() {
    // the Club league: no entries on-chain, its tables paid from what the house put in (a board cup has no entrants
    // to count against)
    let mut env = Env::new();
    let admin = env.admin.insecure_clone();
    let resolver = env.resolver.insecure_clone();
    let usdc = env.spl_token();
    env.give_tokens(&usdc, &admin.pubkey(), 1_000_000_000);
    let cup = season_cup(&mut env, 13, CupKind::Board, 0, 12_000_000, Some(&usdc));
    let ds: Vec<Keypair> = (0..3).map(|_| env.player()).collect();
    let w: Vec<Pubkey> = ds.iter().map(|k| k.pubkey()).collect();
    for x in &w {
        env.ensure_ata(&usdc, x);
    }
    let order = [w[0], w[1], w[2], w[2]];
    let (root, proofs) = results_tree(&cup, &order);
    env.warp(SEASON);
    env.ok(
        &[env.ix_post(&resolver.pubkey(), &cup, root, 4)],
        &[&resolver],
    );
    env.warp(CHALLENGE_WINDOW as i64);
    env.ok(&[ix_finalize(&cup)], &[&resolver]);
    let c = env.cup(&cup);
    assert_eq!((c.pool, c.places, c.used_bps), (12_000_000, 4, 6_500));
    env.ok(
        &[
            ix_claim(&w[2], &cup, 3, &w[2], proofs[2].clone(), Some(&usdc), false),
            ix_claim(&w[2], &cup, 4, &w[2], proofs[3].clone(), Some(&usdc), false),
        ],
        &[&ds[2]],
    );
    assert_eq!(
        env.balance(Some(&usdc), &w[2]),
        2 * (12_000_000 * 1_250 / 6_500)
    );
    // nobody else claims: after the claim window the server pushes them, then the cup closes, the odd units back to
    // the house that put the guarantee in
    env.warp(CLAIM_WINDOW as i64);
    for i in 0..2 {
        env.ok(
            &[ix_claim(
                &resolver.pubkey(),
                &cup,
                i as u16 + 1,
                &w[i],
                proofs[i].clone(),
                Some(&usdc),
                false,
            )],
            &[&resolver],
        );
    }
    let paid = env.cup(&cup).paid;
    let s0 = env.balance(Some(&usdc), &admin.pubkey());
    env.ok(
        &[ix_close_cup(
            &cup,
            &admin.pubkey(),
            &admin.pubkey(),
            None,
            Some(&usdc),
        )],
        &[&resolver],
    );
    assert_eq!(
        env.balance(Some(&usdc), &admin.pubkey()),
        s0 + 12_000_000 - paid
    );
}

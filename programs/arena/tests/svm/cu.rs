//! Compute units and wire sizes of the heaviest paths: an 8-player room in a
//! transfer-fee token (the project token), voided and settled seven ways, and a
//! cup's claims. `cargo test -p arena --test svm cu -- --nocapture` prints them.

use anchor_lang::{prelude::Pubkey, solana_program::instruction::Instruction};
use arena::{state::*, DNF};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;

use crate::common::*;

/// The default per-instruction budget, and the largest transaction a packet carries.
const BUDGET: u64 = 200_000;
const PACKET: usize = 1_232;

fn keys(ps: &[Keypair]) -> Vec<Pubkey> {
    ps.iter().map(|p| p.pubkey()).collect()
}

fn wire_size(env: &Env, ixs: &[Instruction], payer: &Pubkey) -> usize {
    let msg = Message::new_with_blockhash(ixs, Some(payer), &env.svm.latest_blockhash());
    1 + 64 * msg.header.num_required_signatures as usize + msg.serialize().len()
}

/// Sends, prints the cost, and checks it fits the default budget and one packet.
fn measure(env: &mut Env, label: &str, ixs: &[Instruction], signers: &[&Keypair]) -> u64 {
    let size = wire_size(env, ixs, &signers[0].pubkey());
    let units = match env.send(ixs, signers) {
        Ok(meta) => meta.compute_units_consumed,
        Err(e) => panic!("{label}: {:?}\n{}", e.err, e.meta.logs.join("\n")),
    };
    println!("{label:<48} {units:>7} CU {size:>5} bytes");
    assert!(
        units < BUDGET,
        "{label}: {units} CU is over the default budget"
    );
    assert!(
        size <= PACKET,
        "{label}: {size} bytes is over the packet limit"
    );
    units
}

/// An 8-player room in `spl` (or SOL), full and therefore locked, its seed drawn.
fn full_room(env: &mut Env, n: u32, spl: Option<&Spl>) -> (Pubkey, Vec<Keypair>) {
    let sponsor = env.sponsor.insecure_clone();
    let ps: Vec<Keypair> = (0..8).map(|_| env.player()).collect();
    if let Some(s) = spl {
        for p in &ps {
            env.give_tokens(s, &p.pubkey(), 100_000_000);
        }
    }
    let stake = if spl.is_some() { 10_000_000 } else { SOL / 10 };
    let ixs = ix_create_room(
        &ps[0].pubkey(),
        &sponsor.pubkey(),
        code(n),
        room_params(stake, 8),
        spl,
        PLAIN,
    );
    let label = if spl.is_some() {
        "create_room (fee token, escrow created)"
    } else {
        "create_room (SOL)"
    };
    measure(env, label, &ixs, &[&ps[0], &sponsor]);
    let room = room_pda(&code(n));
    for (i, p) in ps[1..].iter().enumerate() {
        let ix = env.ix_join(&p.pubkey(), &room, spl, PLAIN);
        if i == 6 {
            measure(env, "join_room (the 8th: locks it)", &[ix], &[p]);
        } else {
            env.ok(&[ix], &[p]);
        }
    }
    // the seed, from the hash of the slot it locked in (a full SlotHashes: 512 entries to search)
    let locked = env.room(&room).locked_slot;
    let entries: Vec<(u64, [u8; 32])> = (0..512).map(|i| (locked + 511 - i, [7u8; 32])).collect();
    env.slot_hashes(&entries);
    env.next_slot();
    measure(
        env,
        "draw_seed (512 slot hashes)",
        &[ix_draw_seed(&room)],
        &[&sponsor],
    );
    (room, ps)
}

#[test]
fn the_heaviest_paths_fit_the_default_budget_and_one_packet() {
    let mut env = Env::new();
    let resolver = env.resolver.insecure_clone();
    let sponsor = env.sponsor.insecure_clone();
    let acme = env.fee_mint(100);
    env.allow_fee_mint(&acme, 100);

    // every stake back, in one transaction
    let (room, ps) = full_room(&mut env, 1, Some(&acme));
    let ix = ix_void(
        &resolver.pubkey(),
        &room,
        Some(&acme),
        VOID_RACE,
        &keys(&ps),
    );
    measure(
        &mut env,
        "void_room (8 refunds, fee token)",
        &[ix],
        &[&resolver],
    );
    let ix = ix_close_room(&room, &sponsor.pubkey(), Some(&acme), false);
    measure(
        &mut env,
        "close_room (harvest, close the escrow)",
        &[ix],
        &[&resolver],
    );

    // seven tied winners and the fee: eight payments
    let (room, ps) = full_room(&mut env, 2, Some(&acme));
    let mut times = vec![60_000; 7];
    times.push(DNF);
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        Some(&acme),
        times,
        [1; 32],
        &keys(&ps[..7]),
    );
    measure(
        &mut env,
        "settle_room (7 tied winners + fee, fee token)",
        &[ix],
        &[&resolver],
    );

    // the usual case: one winner, SOL
    let (room, ps) = full_room(&mut env, 3, None);
    let mut times = vec![DNF; 8];
    times[5] = 61_000;
    let ix = ix_settle(
        &resolver.pubkey(),
        &room,
        None,
        times,
        [1; 32],
        &[ps[5].pubkey()],
    );
    measure(
        &mut env,
        "settle_room (one winner, SOL)",
        &[ix],
        &[&resolver],
    );

    // a week's cup in the fee token: posted, finalized, claimed
    let admin = env.admin.insecure_clone();
    env.give_tokens(&acme, &admin.pubkey(), 1_000_000_000);
    let now = env.now();
    let mut p = cup_params(CupKind::Board, 0, 100_000_000, now);
    p.payout_bps = vec![1_000; 10];
    let ixs = ix_create_cup(
        &admin.pubkey(),
        &admin.pubkey(),
        0,
        p,
        Some(&acme),
        false,
        None,
    );
    measure(&mut env, "create_cup (board, fee token)", &ixs, &[&admin]);
    let cup = cup_pda(0);
    let winners: Vec<Keypair> = (0..10).map(|_| env.player()).collect();
    for w in &winners {
        env.ensure_ata(&acme, &w.pubkey());
    }
    let (root, proofs) = results_tree(&cup, &keys(&winners));
    env.warp(7 * 86_400);
    let post = env.ix_post(&resolver.pubkey(), &cup, root, 500);
    measure(&mut env, "post_results", &[post], &[&resolver]);
    env.warp(CHALLENGE_WINDOW as i64);
    measure(&mut env, "finalize_cup", &[ix_finalize(&cup)], &[&resolver]);
    for (i, w) in winners.iter().enumerate() {
        let ix = ix_claim(
            &w.pubkey(),
            &cup,
            i as u16 + 1,
            &w.pubkey(),
            proofs[i].clone(),
            Some(&acme),
            false,
        );
        if i == 9 {
            measure(
                &mut env,
                "claim_prize (rank 10 of 10, fee token)",
                &[ix],
                &[w],
            );
        } else {
            env.ok(&[ix], &[w]);
        }
    }
    let ix = ix_close_cup(&cup, &admin.pubkey(), &admin.pubkey(), None, Some(&acme));
    measure(
        &mut env,
        "close_cup (harvest, close the escrow)",
        &[ix],
        &[&resolver],
    );
}

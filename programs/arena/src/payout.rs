//! Pure money math, shared by every settlement path so they can never disagree.
//! No accounts here: unit-tested directly (`cargo test -p arena --lib`).

use anchor_lang::prelude::*;

use crate::{constants::*, error::ArenaError};

/// How a room's pot is split.
#[derive(Debug, PartialEq, Eq)]
pub struct RoomOutcome {
    /// Everyone gets their stake back and no fee is taken: nobody finished, or
    /// every player tied.
    pub refund_all: bool,
    /// Bit i set = players[i] wins `share`.
    pub winners: u8,
    pub share: u64,
    /// Fee plus rounding dust.
    pub to_treasury: u64,
}

/// The fastest valid time takes the pot minus the fee; tied best times split it.
pub fn room_outcome(stake: u64, fee_bps: u16, times_ms: &[u32]) -> Result<RoomOutcome> {
    let n = times_ms.len();
    require!(
        (MIN_ROOM_PLAYERS as usize..=MAX_ROOM_PLAYERS as usize).contains(&n),
        ArenaError::InvalidResults
    );
    require!(times_ms.iter().all(|&t| t > 0), ArenaError::InvalidResults);
    require!(fee_bps <= HARD_MAX_ROOM_FEE_BPS, ArenaError::FeeTooHigh);

    let refund = RoomOutcome {
        refund_all: true,
        winners: 0,
        share: 0,
        to_treasury: 0,
    };
    let best = match times_ms.iter().copied().filter(|&t| t != DNF).min() {
        Some(b) => b,
        None => return Ok(refund),
    };
    let mut winners = 0u8;
    for (i, &t) in times_ms.iter().enumerate() {
        if t == best {
            winners |= 1 << i;
        }
    }
    let w = winners.count_ones() as u128;
    if w as usize == n {
        return Ok(refund);
    }

    let pot = (stake as u128) * (n as u128);
    let fee = pot * (fee_bps as u128) / (BPS_DENOMINATOR as u128);
    let net = pot - fee;
    let share = net / w;
    let dust = net - share * w;
    Ok(RoomOutcome {
        refund_all: false,
        winners,
        share: u64::try_from(share).map_err(|_| error!(ArenaError::Overflow))?,
        to_treasury: u64::try_from(fee + dust).map_err(|_| error!(ArenaError::Overflow))?,
    })
}

/// Payout table: 1..=32 places, each > 0, summing to exactly 10000 bps.
pub fn payout_table_ok(bps: &[u16]) -> bool {
    !bps.is_empty()
        && bps.len() <= MAX_PAYOUT_PLACES
        && bps.iter().all(|&b| b > 0)
        && bps.iter().map(|&b| b as u64).sum::<u64>() == BPS_DENOMINATOR
}

/// What `finalize` fixes for a cup.
#[derive(Debug, PartialEq, Eq)]
pub struct CupPool {
    pub pool: u64,
    pub places: u8,
    pub used_bps: u32,
}

/// pool = max(guarantee, entries − fee); paid places = min(table, scorers).
/// The shares of places nobody reached are spread over the places that were.
pub fn cup_pool(
    entry_fee: u64,
    entrants: u32,
    fee_bps: u16,
    guarantee: u64,
    payout_bps: &[u16],
    scorers: u32,
) -> Result<CupPool> {
    require!(scorers > 0, ArenaError::InvalidParams);
    let entries = (entry_fee as u128) * (entrants as u128);
    let fee = entries * (fee_bps as u128) / (BPS_DENOMINATOR as u128);
    let pool = core::cmp::max(guarantee as u128, entries - fee);
    let places = core::cmp::min(payout_bps.len(), scorers as usize);
    let used_bps: u32 = payout_bps[..places].iter().map(|&b| b as u32).sum();
    Ok(CupPool {
        pool: u64::try_from(pool).map_err(|_| error!(ArenaError::Overflow))?,
        places: places as u8,
        used_bps,
    })
}

/// floor(pool × bps[rank] / used_bps). The sum over all paid places never exceeds `pool`.
pub fn cup_prize(pool: u64, rank_bps: u16, used_bps: u32) -> Result<u64> {
    require!(used_bps > 0, ArenaError::InvalidParams);
    let prize = (pool as u128) * (rank_bps as u128) / (used_bps as u128);
    u64::try_from(prize).map_err(|_| error!(ArenaError::Overflow))
}

/// After every prize is paid: up to the guarantee goes back to the sponsor; the
/// rest (the fee + rounding dust) is the house's, of which a community cup's host
/// gets `host_share_bps`. Returns (to_sponsor, to_host, to_treasury).
pub fn cup_leftover(
    total_in: u64,
    paid: u64,
    guarantee: u64,
    host_share_bps: u16,
) -> Result<(u64, u64, u64)> {
    let left = total_in
        .checked_sub(paid)
        .ok_or_else(|| error!(ArenaError::InsufficientEscrow))?;
    let to_sponsor = core::cmp::min(left, guarantee);
    let house = left - to_sponsor;
    let to_host =
        u64::try_from((house as u128) * (host_share_bps as u128) / (BPS_DENOMINATOR as u128))
            .map_err(|_| error!(ArenaError::Overflow))?;
    Ok((to_sponsor, to_host, house - to_host))
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 100_000_000; // 0.1 SOL

    #[test]
    fn room_winner_takes_pot_minus_fee() {
        let o = room_outcome(S, 500, &[61_000, 62_500]).unwrap();
        assert_eq!(o.winners, 0b01);
        assert_eq!(o.share, 190_000_000); // 0.2 SOL − 5%
        assert_eq!(o.to_treasury, 10_000_000);
        assert!(!o.refund_all);
    }

    #[test]
    fn room_tie_for_first_splits_and_dust_goes_to_treasury() {
        let o = room_outcome(1_001, 500, &[60_000, 60_000, 61_000]).unwrap();
        // pot 3003, fee 150, net 2853 -> 1426 each, dust 1
        assert_eq!(o.winners, 0b011);
        assert_eq!(o.share, 1_426);
        assert_eq!(o.to_treasury, 151);
        assert_eq!(o.share * 2 + o.to_treasury, 3_003);
    }

    #[test]
    fn room_everyone_tied_or_nobody_finished_refunds_without_fee() {
        assert!(room_outcome(S, 500, &[60_000, 60_000]).unwrap().refund_all);
        assert!(room_outcome(S, 500, &[DNF, DNF, DNF]).unwrap().refund_all);
        let o = room_outcome(S, 500, &[DNF, 70_000]).unwrap();
        assert_eq!(o.winners, 0b10);
    }

    #[test]
    fn room_rejects_bad_results() {
        assert!(room_outcome(S, 500, &[60_000]).is_err());
        assert!(room_outcome(S, 500, &[0, 60_000]).is_err());
        assert!(room_outcome(S, 1_001, &[1, 2]).is_err());
        assert!(room_outcome(S, 500, &[1; 9]).is_err());
    }

    #[test]
    fn room_never_pays_more_than_the_pot() {
        for n in 2..=8usize {
            for fee in [0u16, 1, 333, 500, 999, 1_000] {
                for stake in [1u64, 7, 999, 1_000_000_007, u64::MAX / 8] {
                    for winners in 1..n {
                        let mut t = vec![90_000u32; n];
                        for x in t.iter_mut().take(winners) {
                            *x = 80_000;
                        }
                        let o = room_outcome(stake, fee, &t).unwrap();
                        let paid = o.share as u128 * winners as u128 + o.to_treasury as u128;
                        assert_eq!(paid, stake as u128 * n as u128);
                    }
                }
            }
        }
    }

    #[test]
    fn payout_tables() {
        assert!(payout_table_ok(&[5_000, 3_000, 2_000]));
        assert!(payout_table_ok(&[10_000]));
        assert!(!payout_table_ok(&[]));
        assert!(!payout_table_ok(&[5_000, 3_000]));
        assert!(!payout_table_ok(&[10_000, 0]));
        assert!(!payout_table_ok(&[1; 33]));
    }

    // The competition cases from acme-kit's ledger tests.
    #[test]
    fn cup_50_30_20() {
        let p = cup_pool(100, 3, 1_000, 0, &[5_000, 3_000, 2_000], 3).unwrap();
        assert_eq!(
            p,
            CupPool {
                pool: 270,
                places: 3,
                used_bps: 10_000
            }
        );
        let prizes: Vec<u64> = [5_000u16, 3_000, 2_000]
            .iter()
            .map(|&b| cup_prize(p.pool, b, p.used_bps).unwrap())
            .collect();
        assert_eq!(prizes, vec![135, 81, 54]);
        assert_eq!(cup_leftover(300, 270, 0, 0).unwrap(), (0, 0, 30));
    }

    #[test]
    fn cup_fewer_scorers_than_places_spreads_unused_shares() {
        let p = cup_pool(100, 5, 1_000, 0, &[5_000, 3_000, 2_000], 2).unwrap();
        assert_eq!((p.pool, p.places, p.used_bps), (450, 2, 8_000));
        assert_eq!(cup_prize(p.pool, 5_000, p.used_bps).unwrap(), 281);
        assert_eq!(cup_prize(p.pool, 3_000, p.used_bps).unwrap(), 168);
        // 449 paid, 1 dust + 50 fee to the treasury
        assert_eq!(cup_leftover(500, 449, 0, 0).unwrap(), (0, 0, 51));
    }

    #[test]
    fn cup_guarantee_tops_up_and_returns_unused() {
        // guarantee 1000 > entries 200 − fee 20
        let p = cup_pool(100, 2, 1_000, 1_000, &[10_000], 1).unwrap();
        assert_eq!(p.pool, 1_000);
        assert_eq!(cup_prize(p.pool, 10_000, p.used_bps).unwrap(), 1_000);
        // in: 1200, paid 1000 -> 200 left, all of it back to the sponsor (≤ guarantee)
        assert_eq!(cup_leftover(1_200, 1_000, 1_000, 0).unwrap(), (200, 0, 0));
        // big field: guarantee irrelevant, the sponsor gets it all back
        let p = cup_pool(100, 100, 1_000, 1_000, &[10_000], 50).unwrap();
        assert_eq!(p.pool, 9_000);
        assert_eq!(
            cup_leftover(11_000, 9_000, 1_000, 0).unwrap(),
            (1_000, 0, 1_000)
        );
    }

    #[test]
    fn cup_needs_a_scorer() {
        assert!(cup_pool(100, 3, 1_000, 0, &[10_000], 0).is_err());
    }

    #[test]
    fn cup_host_gets_its_share_of_the_house_fee() {
        // 40 entries of 5 at 10%: 200 in, 180 pool paid; 20 left: a 25% host share is 5
        assert_eq!(cup_leftover(200, 180, 0, 2_500).unwrap(), (0, 5, 15));
        // the guarantee comes back first; the host shares only the house's part
        assert_eq!(
            cup_leftover(1_200, 1_000, 1_000, 2_500).unwrap(),
            (200, 0, 0)
        );
        assert_eq!(
            cup_leftover(11_000, 9_000, 1_000, 10_000).unwrap(),
            (1_000, 1_000, 0)
        );
    }
}

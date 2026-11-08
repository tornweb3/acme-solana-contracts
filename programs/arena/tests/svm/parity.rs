// A season's prize maths in the menus and the server (src/ui/lib/prizes.js cupPayout: credits leagues and every
// projection) against the program's own (payout.rs cup_pool and cup_prize): every cup of 500 random leagues in the
// fixture tests/ui/seasonParity.test.js writes, to the base unit.
use arena::payout::{cup_pool, cup_prize};

const FIXTURE: &str = include_str!("../fixtures/season_parity.txt");

fn numbers<T: std::str::FromStr>(s: &str) -> Vec<T>
where
    T::Err: std::fmt::Debug,
{
    s.split(',').map(|x| x.parse().unwrap()).collect()
}

#[test]
fn the_menus_and_the_server_compute_what_the_program_pays() {
    let mut cups = 0;
    for line in FIXTURE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (given, want) = line.split_once(" | ").unwrap();
        let g: Vec<&str> = given.split(' ').collect();
        let w: Vec<&str> = want.split(' ').collect();
        let (fee, entrants, fee_bps, guarantee, scorers) = (
            g[0].parse::<u64>().unwrap(),
            g[1].parse::<u32>().unwrap(),
            g[2].parse::<u16>().unwrap(),
            g[3].parse::<u64>().unwrap(),
            g[4].parse::<u32>().unwrap(),
        );
        let bps: Vec<u16> = numbers(g[5]);
        let p = cup_pool(fee, entrants, fee_bps, guarantee, &bps, scorers).unwrap();
        assert_eq!(
            (p.pool, p.places as usize, p.used_bps),
            (
                w[0].parse::<u64>().unwrap(),
                w[1].parse::<usize>().unwrap(),
                w[2].parse::<u32>().unwrap()
            ),
            "{line}"
        );
        let prizes: Vec<u64> = numbers(w[3]);
        assert_eq!(prizes.len(), p.places as usize, "{line}");
        for (i, want) in prizes.iter().enumerate() {
            assert_eq!(
                cup_prize(p.pool, bps[i], p.used_bps).unwrap(),
                *want,
                "{line}"
            );
        }
        cups += 1;
    }
    assert!(cups > 700, "{cups} cups");
}

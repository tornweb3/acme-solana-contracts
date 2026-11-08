// LiteSVM tests for the house program, with the arena deployed beside it (the
// arena's own harness, reused): the split, the burn, the shop, the cross-program
// upgrades read, funding cups, the project token's timelock and the buyback
// auction. `anchor build` first (they load target/deploy/{arena,house}.so),
// then `cargo test -p house`.

#[path = "../../arena/tests/svm/common.rs"]
mod common;

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
    Discriminator, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent,
    token_2022::spl_token_2022::extension::transfer_fee::instruction as fee_ix,
};
use arena::{error::ArenaError, state::*};
use common::*;
use house::{
    error::HouseError, Auction, HouseConfig, HouseParams, Split, Totals, AUCTION_EVERY_SECS,
    NATIVE_SOL, TOKEN_TIMELOCK_SECS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SPLIT: Split = Split {
    ops_bps: 5_000,
    prize_bps: 3_000,
    buyback_bps: 2_000,
};
const FUND_EVERY: u32 = 6 * 86_400;

fn hpda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &house::ID).0
}
fn house_pda() -> Pubkey {
    hpda(&[b"house"])
}
fn buyback() -> Pubkey {
    hpda(&[b"buyback"])
}
fn price_pda(mint: &Pubkey) -> Pubkey {
    hpda(&[b"price", mint.as_ref()])
}
fn token_pda() -> Pubkey {
    hpda(&[b"token"])
}
fn auction_pda(lot_mint: &Pubkey) -> Pubkey {
    hpda(&[b"auction", lot_mint.as_ref()])
}
fn totals_pda(mint: &Pubkey) -> Pubkey {
    hpda(&[b"totals", mint.as_ref()])
}
fn house_program_data() -> Pubkey {
    Pubkey::find_program_address(&[house::ID.as_ref()], &bpf_loader_upgradeable::ID).0
}

fn house_err(e: HouseError) -> String {
    format!(
        "Custom({})",
        anchor_lang::error::ERROR_CODE_OFFSET + e as u32
    )
}

// ---- instruction builders ----------------------------------------------------------

fn ix_init(admin: &Pubkey, params: HouseParams) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::Initialize { params }.data(),
        house::accounts::Initialize {
            admin: *admin,
            house: house_pda(),
            treasury: treasury(),
            prize: prize_vault(),
            buyback: buyback(),
            program: house::ID,
            program_data: house_program_data(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn ix_admin<T: InstructionData>(admin: &Pubkey, data: T) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &data.data(),
        house::accounts::AdminHouse {
            admin: *admin,
            house: house_pda(),
        }
        .to_account_metas(None),
    )
}

fn ix_apply_split() -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::ApplySplit {}.data(),
        house::accounts::ApplySplit { house: house_pda() }.to_account_metas(None),
    )
}

fn ix_add_mint(admin: &Pubkey, mint_key: &Pubkey, mint: Option<Pubkey>) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::AddMint {
            mint_key: *mint_key,
        }
        .data(),
        house::accounts::AddMint {
            admin: *admin,
            house: house_pda(),
            price_table: price_pda(mint_key),
            totals: totals_pda(mint_key),
            mint,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn ix_propose_token(admin: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::ProposeTokenMint {}.data(),
        house::accounts::ProposeTokenMint {
            admin: *admin,
            house: house_pda(),
            pending: token_pda(),
            mint: *mint,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn ix_apply_token(mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::ApplyTokenMint {}.data(),
        house::accounts::ApplyTokenMint {
            house: house_pda(),
            pending: token_pda(),
            mint: *mint,
        }
        .to_account_metas(None),
    )
}

/// The admin's lowest rate for auctions of a token (`None`: SOL).
fn ix_set_floor(
    admin: &Pubkey,
    lot_token: Option<&Spl>,
    min_lot: u64,
    min_price: u64,
) -> Instruction {
    let lot_mint = lot_token.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::SetAuctionFloor {
            lot_mint,
            min_lot,
            min_price,
        }
        .data(),
        house::accounts::SetAuctionFloor {
            admin: *admin,
            house: house_pda(),
            auction: auction_pda(&lot_mint),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

/// The keeper's auction of `lot` of a token (`None`: SOL) from the buyback vault.
fn ix_start(
    keeper: &Pubkey,
    lot_token: Option<&Spl>,
    lot: u64,
    start_price: u64,
    floor_price: u64,
    decline_secs: u32,
    run_secs: u32,
) -> Instruction {
    let lot_mint = lot_token.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::StartAuction {
            lot_mint,
            lot,
            start_price,
            floor_price,
            decline_secs,
            run_secs,
        }
        .data(),
        house::accounts::StartAuction {
            keeper: *keeper,
            house: house_pda(),
            auction: auction_pda(&lot_mint),
            buyback: buyback(),
            buyback_tokens: lot_token.map(|s| s.ata(&buyback())),
        }
        .to_account_metas(None),
    )
}

/// `buyer` buys the running lot of a token (`None`: SOL), burning `token`.
fn ix_buy_lot(
    buyer: &Pubkey,
    token: &Spl,
    lot_token: Option<&Spl>,
    max_price: u64,
    min_lot: u64,
) -> Instruction {
    let lot_mint = lot_token.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::BuyLot { max_price, min_lot }.data(),
        house::accounts::BuyLot {
            buyer: *buyer,
            house: house_pda(),
            auction: auction_pda(&lot_mint),
            token_totals: totals_pda(&token.mint),
            token_mint: token.mint,
            buyer_tokens: token.ata(buyer),
            token_program: token.program,
            buyback: buyback(),
            lot_mint: lot_token.map(|s| s.mint),
            buyback_tokens: lot_token.map(|s| s.ata(&buyback())),
            buyer_lot_tokens: lot_token.map(|s| s.ata(buyer)),
            lot_token_program: lot_token.map(|s| s.program),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

type Prices = [[u64; 5]; 6];

/// dept d, level l costs unit × (d + 1) × l.
fn prices(unit: u64) -> Prices {
    let mut t = [[0u64; 5]; 6];
    for (d, row) in t.iter_mut().enumerate() {
        for (l, p) in row.iter_mut().enumerate() {
            *p = unit * (d as u64 + 1) * (l as u64 + 1);
        }
    }
    t
}

fn ix_set_prices(
    authority: &Pubkey,
    mint_key: &Pubkey,
    prices: Prices,
    enabled: bool,
) -> Instruction {
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::SetPrices { prices, enabled }.data(),
        house::accounts::SetPrices {
            authority: *authority,
            house: house_pda(),
            price_table: price_pda(mint_key),
        }
        .to_account_metas(None),
    )
}

/// `with_buyback`: pass the buyback's token account (needed when it gets a share
/// that isn't burned).
fn ix_sweep(ops: &Pubkey, spl: Option<&Spl>, with_buyback: bool) -> Instruction {
    let mint_key = spl.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::Sweep { mint_key }.data(),
        house::accounts::Sweep {
            house: house_pda(),
            treasury: treasury(),
            prize: prize_vault(),
            buyback: buyback(),
            ops_wallet: *ops,
            totals: totals_pda(&mint_key),
            mint: spl.map(|s| s.mint),
            treasury_tokens: spl.map(|s| s.ata(&treasury())),
            ops_tokens: spl.map(|s| s.ata(ops)),
            prize_tokens: spl.map(|s| s.ata(&prize_vault())),
            buyback_tokens: spl.filter(|_| with_buyback).map(|s| s.ata(&buyback())),
            token_program: spl.map(|s| s.program),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn ix_buy(
    owner: &Pubkey,
    payer: &Pubkey,
    spl: Option<&Spl>,
    dept: u8,
    level: u8,
    max_price: u64,
) -> Instruction {
    let mint_key = spl.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    Instruction::new_with_bytes(
        house::ID,
        &house::instruction::BuyUpgrade {
            dept,
            level,
            max_price,
        }
        .data(),
        house::accounts::BuyUpgrade {
            owner: *owner,
            payer: *payer,
            house: house_pda(),
            price_table: price_pda(&mint_key),
            upgrades: upgrades_pda(owner),
            treasury: treasury(),
            mint: spl.map(|s| s.mint),
            owner_tokens: spl.map(|s| s.ata(owner)),
            treasury_tokens: spl.map(|s| s.ata(&treasury())),
            token_program: spl.map(|s| s.program),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

/// For a token cup, prepends the cup escrow's creation, as the keeper's job does.
fn ix_fund(
    keeper: &Pubkey,
    id: u64,
    params: arena::CupParams,
    spl: Option<&Spl>,
) -> Vec<Instruction> {
    let cup = cup_pda(id);
    let mint_key = spl.map(|s| s.mint).unwrap_or(NATIVE_SOL);
    let mut ixs = Vec::new();
    if let Some(s) = spl {
        ixs.push(create_associated_token_account_idempotent(
            keeper, &cup, &s.mint, &s.program,
        ));
    }
    ixs.push(Instruction::new_with_bytes(
        house::ID,
        &house::instruction::FundCup { id, params }.data(),
        house::accounts::FundCup {
            keeper: *keeper,
            house: house_pda(),
            prize: prize_vault(),
            totals: totals_pda(&mint_key),
            arena_config: config_pda(),
            ruleset: ruleset_pda(RULESET_ID),
            allowed_mint: mint_pda(&mint_key),
            cup,
            mint: spl.map(|s| s.mint),
            escrow_tokens: spl.map(|s| s.ata(&cup)),
            prize_tokens: spl.map(|s| s.ata(&prize_vault())),
            token_program: spl.map(|s| s.program),
            arena_program: arena::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    ));
    ixs
}

// ---- the house beside the arena ---------------------------------------------------------

struct House {
    env: Env,
    keeper: Keypair,
    ops: Keypair,
}

impl House {
    /// The arena deployed and initialized (its treasury and prize vault these
    /// PDAs), the house deployed but not initialized; the admin is the upgrade
    /// authority of both.
    fn deployed() -> House {
        let mut env = Env::new();
        let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/house.so"));
        env.svm.add_program(house::ID, bytes).unwrap();
        let pd = house_program_data();
        let mut acct = env.svm.get_account(&pd).unwrap();
        acct.data[12] = 1;
        acct.data[13..45].copy_from_slice(env.admin.pubkey().as_ref());
        env.svm.set_account(pd, acct).unwrap();
        let (keeper, ops) = (env.player(), env.player());
        House { env, keeper, ops }
    }

    fn params(&self) -> HouseParams {
        HouseParams {
            keeper: self.keeper.pubkey(),
            ops_wallet: self.ops.pubkey(),
            split: SPLIT,
            fund_max_bps: 2_000,
            fund_every_secs: FUND_EVERY,
        }
    }

    /// Initialized, selling in SOL (no prices yet).
    fn new() -> House {
        let mut h = House::deployed();
        let admin = h.env.admin.insecure_clone();
        let ix = ix_init(&admin.pubkey(), h.params());
        h.env.ok(&[ix], &[&admin]);
        h.env.ok(
            &[ix_add_mint(&admin.pubkey(), &NATIVE_SOL, None)],
            &[&admin],
        );
        h
    }

    fn admin(&mut self, ixs: &[Instruction]) {
        let admin = self.env.admin.insecure_clone();
        self.env.ok(ixs, &[&admin]);
    }

    fn add_token(&mut self, spl: &Spl) {
        let a = self.env.admin.pubkey();
        self.admin(&[ix_add_mint(&a, &spl.mint, Some(spl.mint))]);
    }

    /// The project token, proposed and applied once its timelock has passed.
    fn set_token(&mut self, mint: &Pubkey) {
        let a = self.env.admin.pubkey();
        self.admin(&[ix_propose_token(&a, mint)]);
        self.env.warp(TOKEN_TIMELOCK_SECS);
        self.admin(&[ix_apply_token(mint)]);
    }

    fn auction(&self, lot_mint: &Pubkey) -> Auction {
        self.env.load(&auction_pda(lot_mint))
    }

    fn fails(&mut self, ixs: &[Instruction], signers: &[&Keypair], code: HouseError) {
        let want = house_err(code);
        match self.env.send(ixs, signers) {
            Ok(_) => panic!("expected {want}, tx succeeded"),
            Err(e) => {
                let got = format!("{:?}", e.err);
                assert!(
                    got.contains(&want),
                    "expected {want}, got {got}\n{}",
                    e.meta.logs.join("\n")
                );
            }
        }
    }

    fn config(&self) -> HouseConfig {
        self.env.load(&house_pda())
    }

    fn totals(&self, mint: &Pubkey) -> Totals {
        self.env.load(&totals_pda(mint))
    }
}

fn supply(env: &Env, mint: &Pubkey) -> u64 {
    let a = env.svm.get_account(mint).unwrap();
    u64::from_le_bytes(a.data[36..44].try_into().unwrap())
}

// ---- tests ----------------------------------------------------------------------------

#[test]
fn the_arena_reads_the_shops_accounts_by_the_same_layout() {
    assert_eq!(arena::HOUSE_ID, house::ID);
    assert_eq!(
        <house::Upgrades as Discriminator>::DISCRIMINATOR,
        &arena::UPGRADES_DISCRIMINATOR[..]
    );
    assert_eq!(house::UPGRADES_VERSION, arena::UPGRADES_VERSION);
    assert_eq!(arena::UPGRADES_LEVELS_AT, 8 + 1 + 32);
}

#[test]
fn only_the_upgrade_authority_initializes_and_every_vault_gets_its_rent() {
    let mut h = House::deployed();
    let admin = h.env.admin.insecure_clone();
    let stranger = h.env.player();
    let ix = ix_init(&stranger.pubkey(), h.params());
    h.fails(&[ix], &[&stranger], HouseError::Unauthorized);
    let mut bad = h.params();
    bad.split.buyback_bps = 1_999;
    h.fails(
        &[ix_init(&admin.pubkey(), bad)],
        &[&admin],
        HouseError::InvalidSplit,
    );
    let mut bad = h.params();
    bad.fund_max_bps = 5_001;
    h.fails(
        &[ix_init(&admin.pubkey(), bad)],
        &[&admin],
        HouseError::InvalidParams,
    );
    let ix = ix_init(&admin.pubkey(), h.params());
    h.env.ok(&[ix], &[&admin]);
    let floor = h.env.svm.minimum_balance_for_rent_exemption(0);
    for v in [treasury(), prize_vault(), buyback()] {
        assert_eq!(h.env.lamports(&v), floor);
    }
    let c = h.config();
    assert_eq!(
        (c.split, c.token_mint, c.keeper),
        (SPLIT, Pubkey::default(), h.keeper.pubkey())
    );
    // once only
    let ix = ix_init(&admin.pubkey(), h.params());
    h.env.fails_any(&[ix], &[&admin]);
}

#[test]
fn a_sweep_splits_sol_and_the_buyback_share_goes_to_prizes_until_a_token_exists() {
    let mut h = House::new();
    let anyone = h.env.player();
    let ops = h.ops.pubkey();
    let floor = h.env.svm.minimum_balance_for_rent_exemption(0);
    // fees arrive in the treasury: a settled room's cut, an upgrade sale
    h.env.svm.airdrop(&treasury(), SOL).unwrap();
    let (o0, p0, b0) = (
        h.env.lamports(&ops),
        h.env.lamports(&prize_vault()),
        h.env.lamports(&buyback()),
    );
    h.env.ok(&[ix_sweep(&ops, None, false)], &[&anyone]);
    assert_eq!(h.env.lamports(&ops), o0 + SOL / 2);
    assert_eq!(h.env.lamports(&prize_vault()), p0 + SOL / 2);
    assert_eq!(h.env.lamports(&buyback()), b0);
    assert_eq!(h.env.lamports(&treasury()), floor);
    let t = h.totals(&NATIVE_SOL);
    assert_eq!(
        (t.swept_ops, t.swept_prize, t.swept_buyback, t.burned),
        (SOL / 2, SOL / 2, 0, 0)
    );
    // nothing left: nothing happens; the rent floor is never touched
    h.env.ok(&[ix_sweep(&ops, None, false)], &[&anyone]);
    assert_eq!(h.env.lamports(&treasury()), floor);
    // rounding dust goes to prizes
    h.env.svm.airdrop(&treasury(), 999).unwrap();
    let (o0, p0) = (h.env.lamports(&ops), h.env.lamports(&prize_vault()));
    h.env.ok(&[ix_sweep(&ops, None, false)], &[&anyone]);
    assert_eq!(
        (h.env.lamports(&ops), h.env.lamports(&prize_vault())),
        (o0 + 499, p0 + 500)
    );
    // the studio's share goes to the studio's wallet only
    h.env.svm.airdrop(&treasury(), SOL).unwrap();
    h.env
        .fails_any(&[ix_sweep(&anyone.pubkey(), None, false)], &[&anyone]);

    // once the project token is set, the buyback share builds up for it
    let token = h.env.fee_mint(100);
    h.set_token(&token.mint);
    let (o0, p0, b0) = (
        h.env.lamports(&ops),
        h.env.lamports(&prize_vault()),
        h.env.lamports(&buyback()),
    );
    h.env.ok(&[ix_sweep(&ops, None, false)], &[&anyone]);
    assert_eq!(h.env.lamports(&ops), o0 + SOL / 2);
    assert_eq!(h.env.lamports(&prize_vault()), p0 + SOL * 3 / 10);
    assert_eq!(h.env.lamports(&buyback()), b0 + SOL / 5);
}

#[test]
fn a_token_sweep_pays_each_share_and_burns_the_project_tokens_buyback_share() {
    let mut h = House::new();
    let anyone = h.env.player();
    let ops = h.ops.pubkey();
    let usdc = h.env.spl_token();
    h.add_token(&usdc);
    h.env.ensure_ata(&usdc, &ops);
    h.env.ensure_ata(&usdc, &buyback());
    h.env.give_tokens(&usdc, &treasury(), 1_000_001);
    h.env.ok(&[ix_sweep(&ops, Some(&usdc), false)], &[&anyone]);
    // no project token yet: its share and the dust go to prizes
    assert_eq!(h.env.tokens(&usdc.ata(&ops)), 500_000);
    assert_eq!(h.env.tokens(&usdc.ata(&prize_vault())), 500_001);
    assert_eq!(h.env.tokens(&usdc.ata(&treasury())), 0);

    // the project token: a StonkFun-style mint, 1 % on every transfer
    let acme = h.env.fee_mint(100);
    h.add_token(&acme);
    let admin = h.env.admin.insecure_clone();
    let propose = |mint: Pubkey| ix_propose_token(&admin.pubkey(), &mint);
    // a real mint only
    h.env.fails_any(&[propose(Pubkey::default())], &[&admin]);
    h.set_token(&acme.mint);
    h.fails(&[propose(usdc.mint)], &[&admin], HouseError::AlreadySet);
    h.env.ensure_ata(&acme, &ops);
    h.env.give_tokens(&acme, &treasury(), 1_000_000);
    let s0 = supply(&h.env, &acme.mint);
    h.env.ok(&[ix_sweep(&ops, Some(&acme), false)], &[&anyone]);
    // the transfers pay the token's tax (to its holders); the burn pays none
    assert_eq!(h.env.tokens(&acme.ata(&ops)), 495_000);
    assert_eq!(h.env.tokens(&acme.ata(&prize_vault())), 297_000);
    assert_eq!(supply(&h.env, &acme.mint), s0 - 200_000);
    assert_eq!(h.env.tokens(&acme.ata(&treasury())), 0);
    assert_eq!(h.totals(&acme.mint).burned, 200_000);

    // other tokens' buyback share now builds up for the buyback
    h.env.give_tokens(&usdc, &treasury(), 1_000_000);
    h.fails(
        &[ix_sweep(&ops, Some(&usdc), false)],
        &[&anyone],
        HouseError::TokenAccountsMismatch,
    );
    h.env.ok(&[ix_sweep(&ops, Some(&usdc), true)], &[&anyone]);
    assert_eq!(h.env.tokens(&usdc.ata(&buyback())), 200_000);
    assert_eq!(h.env.tokens(&usdc.ata(&prize_vault())), 500_001 + 300_000);
    // every share to its owner's account only
    h.env.give_tokens(&usdc, &treasury(), 10);
    let other = h.env.ensure_ata(&usdc, &anyone.pubkey());
    let mut ix = ix_sweep(&ops, Some(&usdc), true);
    ix.accounts[8].pubkey = other;
    h.fails(&[ix], &[&anyone], HouseError::TokenAccountsMismatch);
    let mut ix = ix_sweep(&ops, Some(&usdc), true);
    ix.accounts[9].pubkey = other;
    h.fails(&[ix], &[&anyone], HouseError::TokenAccountsMismatch);
}

#[test]
fn a_new_split_waits_a_week_and_then_anyone_applies_it() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let stranger = h.env.player();
    let next = Split {
        ops_bps: 6_000,
        prize_bps: 3_000,
        buyback_bps: 1_000,
    };
    h.fails(
        &[ix_admin(
            &stranger.pubkey(),
            house::instruction::ProposeSplit { split: next },
        )],
        &[&stranger],
        HouseError::Unauthorized,
    );
    let bad = Split {
        buyback_bps: 0,
        ..next
    };
    h.fails(
        &[ix_admin(
            &admin.pubkey(),
            house::instruction::ProposeSplit { split: bad },
        )],
        &[&admin],
        HouseError::InvalidSplit,
    );
    h.fails(
        &[ix_apply_split()],
        &[&stranger],
        HouseError::NothingPending,
    );
    h.env.ok(
        &[ix_admin(
            &admin.pubkey(),
            house::instruction::ProposeSplit { split: next },
        )],
        &[&admin],
    );
    assert_eq!(h.config().split_after, h.env.now() + 7 * 86_400);
    h.fails(&[ix_apply_split()], &[&stranger], HouseError::TooEarly);
    h.env.warp(7 * 86_400 - 1);
    h.fails(&[ix_apply_split()], &[&stranger], HouseError::TooEarly);
    // until it applies, sweeps use the old split
    h.env.svm.airdrop(&treasury(), SOL).unwrap();
    let o0 = h.env.lamports(&h.ops.pubkey());
    h.env
        .ok(&[ix_sweep(&h.ops.pubkey(), None, false)], &[&stranger]);
    assert_eq!(h.env.lamports(&h.ops.pubkey()), o0 + SOL / 2);
    h.env.warp(1);
    h.env.ok(&[ix_apply_split()], &[&stranger]);
    let c = h.config();
    assert_eq!((c.split, c.split_after), (next, 0));
    h.fails(
        &[ix_apply_split()],
        &[&stranger],
        HouseError::NothingPending,
    );
    h.env.svm.airdrop(&treasury(), SOL).unwrap();
    let o0 = h.env.lamports(&h.ops.pubkey());
    h.env
        .ok(&[ix_sweep(&h.ops.pubkey(), None, false)], &[&stranger]);
    assert_eq!(h.env.lamports(&h.ops.pubkey()), o0 + SOL * 6 / 10);
}

#[test]
fn admin_and_roles_move_only_by_the_admin_and_in_two_steps() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let next = h.env.player();
    let stranger = h.env.player();
    h.fails(
        &[ix_admin(
            &stranger.pubkey(),
            house::instruction::SetRoles {
                keeper: stranger.pubkey(),
                ops_wallet: stranger.pubkey(),
            },
        )],
        &[&stranger],
        HouseError::Unauthorized,
    );
    h.fails(
        &[ix_admin(
            &admin.pubkey(),
            house::instruction::SetFundCaps {
                fund_max_bps: 5_001,
                fund_every_secs: 0,
            },
        )],
        &[&admin],
        HouseError::InvalidParams,
    );
    // tighter caps apply at once; looser ones only after the split's timelock
    let caps = |bps: u16, secs: u32| {
        ix_admin(
            &admin.pubkey(),
            house::instruction::SetFundCaps {
                fund_max_bps: bps,
                fund_every_secs: secs,
            },
        )
    };
    let apply = Instruction::new_with_bytes(
        house::ID,
        &house::instruction::ApplyFundCaps {}.data(),
        house::accounts::ApplySplit { house: house_pda() }.to_account_metas(None),
    );
    let (bps0, secs0) = (h.config().fund_max_bps, h.config().fund_every_secs);
    h.env.ok(&[caps(bps0 / 2, secs0 * 2)], &[&admin]);
    assert_eq!(
        (h.config().fund_max_bps, h.config().fund_every_secs),
        (bps0 / 2, secs0 * 2)
    );
    h.env.ok(&[caps(5_000, 0)], &[&admin]);
    assert_eq!(
        (h.config().fund_max_bps, h.config().fund_every_secs),
        (bps0 / 2, secs0 * 2)
    );
    h.fails(
        std::slice::from_ref(&apply),
        &[&stranger],
        HouseError::TooEarly,
    );
    h.env.warp(7 * 86_400);
    h.env.ok(std::slice::from_ref(&apply), &[&stranger]);
    assert_eq!(
        (h.config().fund_max_bps, h.config().fund_every_secs),
        (5_000, 0)
    );
    h.fails(&[apply], &[&stranger], HouseError::NothingPending);
    h.env.ok(
        &[ix_admin(
            &admin.pubkey(),
            house::instruction::ProposeAdmin {
                new_admin: next.pubkey(),
            },
        )],
        &[&admin],
    );
    let accept = |who: &Keypair| {
        Instruction::new_with_bytes(
            house::ID,
            &house::instruction::AcceptAdmin {}.data(),
            house::accounts::AcceptAdmin {
                new_admin: who.pubkey(),
                house: house_pda(),
            }
            .to_account_metas(None),
        )
    };
    h.fails(
        &[accept(&stranger)],
        &[&stranger],
        HouseError::NotPendingAdmin,
    );
    h.env.ok(&[accept(&next)], &[&next]);
    assert_eq!(h.config().admin, next.pubkey());
    h.fails(
        &[ix_admin(
            &admin.pubkey(),
            house::instruction::SetPaused { paused: true },
        )],
        &[&admin],
        HouseError::Unauthorized,
    );
}

#[test]
fn upgrades_are_bought_a_level_at_a_time_at_the_listed_price() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let keeper = h.keeper.insecure_clone();
    let sponsor = h.env.sponsor.insecure_clone();
    let buyer = h.env.player();
    let stranger = h.env.player();
    let unit = SOL / 100;
    // not for sale until priced and enabled; prices by the admin only (never a server key)
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 1, SOL)],
        &[&buyer],
        HouseError::NotForSale,
    );
    h.fails(
        &[ix_set_prices(
            &stranger.pubkey(),
            &NATIVE_SOL,
            prices(unit),
            true,
        )],
        &[&stranger],
        HouseError::Unauthorized,
    );
    h.fails(
        &[ix_set_prices(
            &keeper.pubkey(),
            &NATIVE_SOL,
            prices(unit),
            true,
        )],
        &[&keeper],
        HouseError::Unauthorized,
    );
    h.env.ok(
        &[ix_set_prices(
            &admin.pubkey(),
            &NATIVE_SOL,
            prices(unit),
            true,
        )],
        &[&admin],
    );

    // the first level; the race server's sponsor pays the account's rent
    let t0 = h.env.lamports(&treasury());
    let b0 = h.env.lamports(&buyer.pubkey());
    h.env.ok(
        &[ix_buy(&buyer.pubkey(), &sponsor.pubkey(), None, 0, 1, unit)],
        &[&buyer, &sponsor],
    );
    assert_eq!(h.env.lamports(&treasury()), t0 + unit);
    assert_eq!(h.env.lamports(&buyer.pubkey()), b0 - unit - 10_000);
    // the account is what the arena reads: discriminator, version, owner, levels
    let acct = h
        .env
        .svm
        .get_account(&upgrades_pda(&buyer.pubkey()))
        .unwrap();
    assert_eq!(acct.owner, house::ID);
    assert_eq!(&acct.data[..8], &arena::UPGRADES_DISCRIMINATOR[..]);
    assert_eq!(acct.data[8], arena::UPGRADES_VERSION);
    assert_eq!(&acct.data[9..41], buyer.pubkey().as_ref());
    assert_eq!(&acct.data[41..47], &[1, 0, 0, 0, 0, 0]);

    // only the next level: a repeated or skipped purchase fails rather than charging
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 1, SOL)],
        &[&buyer],
        HouseError::WrongLevel,
    );
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 3, SOL)],
        &[&buyer],
        HouseError::WrongLevel,
    );
    // the buyer's limit holds against a price change
    h.fails(
        &[ix_buy(
            &buyer.pubkey(),
            &buyer.pubkey(),
            None,
            0,
            2,
            2 * unit - 1,
        )],
        &[&buyer],
        HouseError::PriceTooHigh,
    );
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 6, 1, SOL)],
        &[&buyer],
        HouseError::InvalidParams,
    );
    // a level priced 0 isn't for sale
    let mut p = prices(unit);
    p[0][1] = 0;
    h.env.ok(
        &[ix_set_prices(&admin.pubkey(), &NATIVE_SOL, p, true)],
        &[&admin],
    );
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 2, SOL)],
        &[&buyer],
        HouseError::NotForSale,
    );
    h.env.ok(
        &[ix_set_prices(
            &admin.pubkey(),
            &NATIVE_SOL,
            prices(unit),
            true,
        )],
        &[&admin],
    );
    // paused: no sales, sweeps go on
    h.admin(&[ix_admin(
        &admin.pubkey(),
        house::instruction::SetPaused { paused: true },
    )]);
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 2, SOL)],
        &[&buyer],
        HouseError::Paused,
    );
    h.env
        .ok(&[ix_sweep(&h.ops.pubkey(), None, false)], &[&stranger]);
    h.admin(&[ix_admin(
        &admin.pubkey(),
        house::instruction::SetPaused { paused: false },
    )]);
    for level in 2..=5u8 {
        h.env.ok(
            &[ix_buy(
                &buyer.pubkey(),
                &buyer.pubkey(),
                None,
                0,
                level,
                SOL,
            )],
            &[&buyer],
        );
    }
    h.fails(
        &[ix_buy(&buyer.pubkey(), &buyer.pubkey(), None, 0, 6, SOL)],
        &[&buyer],
        HouseError::WrongLevel,
    );
    let up: house::Upgrades = h.env.load(&upgrades_pda(&buyer.pubkey()));
    assert_eq!(up.levels, [5, 0, 0, 0, 0, 0, 0, 0]);

    // in a token: paid into the treasury's account of it
    let usdc = h.env.spl_token();
    h.add_token(&usdc);
    h.env.ok(
        &[ix_set_prices(
            &admin.pubkey(),
            &usdc.mint,
            prices(400_000),
            true,
        )],
        &[&admin],
    );
    h.env.give_tokens(&usdc, &buyer.pubkey(), 10_000_000);
    h.env.ok(
        &[ix_buy(
            &buyer.pubkey(),
            &buyer.pubkey(),
            Some(&usdc),
            1,
            1,
            800_000,
        )],
        &[&buyer],
    );
    assert_eq!(h.env.tokens(&usdc.ata(&treasury())), 800_000);
    assert_eq!(h.env.tokens(&usdc.ata(&buyer.pubkey())), 9_200_000);
    // from the buyer's own account, to the treasury's
    let other = h.env.give_tokens(&usdc, &stranger.pubkey(), 10_000_000);
    let mut ix = ix_buy(&buyer.pubkey(), &buyer.pubkey(), Some(&usdc), 1, 2, SOL);
    ix.accounts[7].pubkey = other;
    h.fails(&[ix], &[&buyer], HouseError::TokenAccountsMismatch);
    let mut ix = ix_buy(&buyer.pubkey(), &buyer.pubkey(), Some(&usdc), 1, 2, SOL);
    ix.accounts[8].pubkey = other;
    h.fails(&[ix], &[&buyer], HouseError::TokenAccountsMismatch);
}

#[test]
fn a_garage_room_races_the_levels_bought_in_the_shop() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    h.env.ok(
        &[ix_set_prices(
            &admin.pubkey(),
            &NATIVE_SOL,
            prices(SOL / 100),
            true,
        )],
        &[&admin],
    );
    let buyer = h.env.player();
    for (dept, level) in [(0u8, 1u8), (1, 1), (2, 1), (0, 2)] {
        h.env.ok(
            &[ix_buy(
                &buyer.pubkey(),
                &buyer.pubkey(),
                None,
                dept,
                level,
                SOL,
            )],
            &[&buyer],
        );
    }
    // 700 + 2 × 10 + 15 + 19 = 754
    let mut params = room_params(SOL / 10, 2);
    params.class = CLASS_GARAGE;
    params.max_rating = 750;
    let up = Who {
        upgrades: true,
        ..PLAIN
    };
    let ixs = ix_create_room(
        &buyer.pubkey(),
        &buyer.pubkey(),
        code(1),
        params.clone(),
        None,
        up,
    );
    h.env.fails(&ixs, &[&buyer], ArenaError::RatingTooHigh);
    params.max_rating = 760;
    let ixs = ix_create_room(&buyer.pubkey(), &buyer.pubkey(), code(1), params, None, up);
    h.env.ok(&ixs, &[&buyer]);
    assert_eq!(
        h.env.room(&room_pda(&code(1))).levels,
        vec![[2, 1, 1, 0, 0, 0]]
    );
}

#[test]
fn the_keeper_funds_a_weekly_cup_from_the_prize_vault_within_its_caps() {
    let mut h = House::new();
    let keeper = h.keeper.insecure_clone();
    let guardian = h.env.guardian.insecure_clone();
    let stranger = h.env.player();
    h.env.svm.airdrop(&prize_vault(), 10 * SOL).unwrap();
    let now = h.env.now();
    // at most 20 % of what the vault holds
    let ixs = ix_fund(
        &keeper.pubkey(),
        0,
        cup_params(CupKind::Board, 0, 2 * SOL + 1, now),
        None,
    );
    h.fails(&ixs, &[&keeper], HouseError::FundTooLarge);
    let ixs = ix_fund(
        &stranger.pubkey(),
        0,
        cup_params(CupKind::Board, 0, SOL, now),
        None,
    );
    h.fails(&ixs, &[&stranger], HouseError::Unauthorized);
    // a free board cup only: no entries a leaked keeper could shape for an accomplice
    let ixs = ix_fund(
        &keeper.pubkey(),
        0,
        cup_params(CupKind::Entries, SOL / 100, SOL, now),
        None,
    );
    h.fails(&ixs, &[&keeper], HouseError::NotABoardCup);
    // a paused house funds nothing
    let admin = h.env.admin.insecure_clone();
    let pause = |on: bool| {
        ix_admin(
            &admin.pubkey(),
            house::instruction::SetPaused { paused: on },
        )
    };
    h.env.ok(&[pause(true)], &[&admin]);
    let ixs = ix_fund(
        &keeper.pubkey(),
        0,
        cup_params(CupKind::Board, 0, 2 * SOL, now),
        None,
    );
    h.fails(&ixs, &[&keeper], HouseError::Paused);
    h.env.ok(&[pause(false)], &[&admin]);
    h.env.ok(&ixs, &[&keeper]);
    // an arena cup, organized and sponsored by the vault, its rent paid by the keeper
    let c = h.env.cup(&cup_pda(0));
    assert_eq!(
        (c.organizer, c.sponsor, c.rent_payer),
        (prize_vault(), prize_vault(), keeper.pubkey())
    );
    assert_eq!(
        (c.guarantee, c.featured_mint, c.host),
        (2 * SOL, Pubkey::new_from_array([42; 32]), Pubkey::default())
    );
    let floor = h.env.svm.minimum_balance_for_rent_exemption(0);
    assert_eq!(h.env.lamports(&prize_vault()), floor + 8 * SOL);
    assert_eq!(h.totals(&NATIVE_SOL).funded, 2 * SOL);
    // at most once per period
    let now = h.env.now();
    let ixs = ix_fund(
        &keeper.pubkey(),
        1,
        cup_params(CupKind::Board, 0, SOL / 10, now),
        None,
    );
    h.fails(&ixs, &[&keeper], HouseError::FundTooSoon);
    h.env.warp(FUND_EVERY as i64);
    let now = h.env.now();
    let ixs = ix_fund(
        &keeper.pubkey(),
        1,
        cup_params(CupKind::Board, 0, 8 * SOL / 5 + 1, now),
        None,
    );
    h.fails(&ixs, &[&keeper], HouseError::FundTooLarge);
    let ixs = ix_fund(
        &keeper.pubkey(),
        1,
        cup_params(CupKind::Board, 0, 8 * SOL / 5, now),
        None,
    );
    h.env.ok(&ixs, &[&keeper]);
    // nobody else can spend the vault: a cup that names it as organizer needs the
    // house's signature for it
    let mut ixs = ix_create_cup(
        &prize_vault(),
        &stranger.pubkey(),
        2,
        cup_params(CupKind::Board, 0, SOL, now),
        None,
        false,
        None,
    );
    ixs[0].accounts[0].is_signer = false;
    h.env.fails_any(&ixs, &[&stranger]);

    // a cancelled cup's guarantee goes back to the vault, its rent to the keeper
    let cup = cup_pda(1);
    h.env.ok(
        &[ix_guard("cancel", &guardian.pubkey(), &cup)],
        &[&guardian],
    );
    let v0 = h.env.lamports(&prize_vault());
    h.env.ok(
        &[ix_return_guarantee(&cup, &prize_vault(), None)],
        &[&stranger],
    );
    assert_eq!(h.env.lamports(&prize_vault()), v0 + 8 * SOL / 5);
    let (k0, rent) = (h.env.lamports(&keeper.pubkey()), h.env.lamports(&cup));
    h.env.ok(
        &[ix_close_cup(
            &cup,
            &keeper.pubkey(),
            &prize_vault(),
            None,
            None,
        )],
        &[&stranger],
    );
    assert_eq!(h.env.lamports(&keeper.pubkey()), k0 + rent);
}

#[test]
fn the_keeper_funds_a_usdc_cup_from_the_vaults_usdc() {
    let mut h = House::new();
    let keeper = h.keeper.insecure_clone();
    let usdc = h.env.spl_token();
    h.add_token(&usdc);
    h.env.give_tokens(&usdc, &prize_vault(), 1_000_000_000);
    let now = h.env.now();
    // the cap is on the vault's USDC, not its SOL
    let ixs = ix_fund(
        &keeper.pubkey(),
        0,
        cup_params(CupKind::Board, 0, 200_000_001, now),
        Some(&usdc),
    );
    h.fails(&ixs, &[&keeper], HouseError::FundTooLarge);
    let ixs = ix_fund(
        &keeper.pubkey(),
        0,
        cup_params(CupKind::Board, 0, 100_000_000, now),
        Some(&usdc),
    );
    h.env.ok(&ixs, &[&keeper]);
    assert_eq!(h.env.tokens(&usdc.ata(&cup_pda(0))), 100_000_000);
    assert_eq!(h.env.tokens(&usdc.ata(&prize_vault())), 900_000_000);
    assert_eq!(h.totals(&usdc.mint).funded, 100_000_000);
    // the week's top three claim it in arena
    let winners: Vec<Keypair> = (0..3).map(|_| h.env.player()).collect();
    let w: Vec<Pubkey> = winners.iter().map(|k| k.pubkey()).collect();
    for k in &w {
        h.env.ensure_ata(&usdc, k);
    }
    let resolver = h.env.resolver.insecure_clone();
    let cup = cup_pda(0);
    let (root, proofs) = results_tree(&cup, &w);
    h.env.warp(7 * 86_400);
    let post = h.env.ix_post(&resolver.pubkey(), &cup, root, 40);
    h.env.ok(&[post], &[&resolver]);
    h.env.warp(CHALLENGE_WINDOW as i64);
    h.env.ok(&[ix_finalize(&cup)], &[&resolver]);
    for (i, k) in winners.iter().enumerate() {
        h.env.ok(
            &[ix_claim(
                &w[i],
                &cup,
                i as u16 + 1,
                &w[i],
                proofs[i].clone(),
                Some(&usdc),
                false,
            )],
            &[k],
        );
    }
    assert_eq!(h.env.tokens(&usdc.ata(&w[0])), 50_000_000);
    let (k0, rent) = (
        h.env.lamports(&keeper.pubkey()),
        h.env.lamports(&cup) + h.env.lamports(&usdc.ata(&cup)),
    );
    h.env.ok(
        &[ix_close_cup(
            &cup,
            &keeper.pubkey(),
            &prize_vault(),
            None,
            Some(&usdc),
        )],
        &[&resolver],
    );
    assert_eq!(h.env.lamports(&keeper.pubkey()), k0 + rent);
}

#[test]
fn the_project_token_takes_effect_only_after_its_timelock() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let anyone = h.env.player();
    let acme = h.env.fee_mint(100);
    let other = h.env.fee_mint(100);
    // nothing proposed: nothing to apply
    h.env.fails_any(&[ix_apply_token(&acme.mint)], &[&anyone]);
    // the admin alone proposes; a token taking more than 5 % a transfer is refused
    h.fails(
        &[ix_propose_token(&anyone.pubkey(), &acme.mint)],
        &[&anyone],
        HouseError::Unauthorized,
    );
    let greedy = h.env.fee_mint(600);
    h.fails(
        &[ix_propose_token(&admin.pubkey(), &greedy.mint)],
        &[&admin],
        HouseError::TransferFeeTooHigh,
    );
    h.env
        .ok(&[ix_propose_token(&admin.pubkey(), &acme.mint)], &[&admin]);
    // not before its time, and only the proposed mint
    h.fails(
        &[ix_apply_token(&acme.mint)],
        &[&anyone],
        HouseError::TooEarly,
    );
    h.fails(
        &[ix_apply_token(&other.mint)],
        &[&anyone],
        HouseError::NotTheToken,
    );
    assert_eq!(h.config().token_mint, Pubkey::default());
    // a new proposal starts the wait again
    h.env.warp(TOKEN_TIMELOCK_SECS - 60);
    h.env
        .ok(&[ix_propose_token(&admin.pubkey(), &other.mint)], &[&admin]);
    h.env.warp(120);
    h.fails(
        &[ix_apply_token(&other.mint)],
        &[&anyone],
        HouseError::TooEarly,
    );
    h.env.warp(TOKEN_TIMELOCK_SECS);
    h.env.ok(&[ix_apply_token(&other.mint)], &[&anyone]);
    assert_eq!(h.config().token_mint, other.mint);
    // once: never again
    h.fails(
        &[ix_propose_token(&admin.pubkey(), &acme.mint)],
        &[&admin],
        HouseError::AlreadySet,
    );
    h.fails(
        &[ix_apply_token(&other.mint)],
        &[&anyone],
        HouseError::AlreadySet,
    );
}

#[test]
fn the_shop_sells_in_a_fee_token_only_within_the_houses_cap() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let buyer = h.env.player();
    let acme = h.env.fee_mint(100);
    h.add_token(&acme);
    h.admin(&[ix_set_prices(
        &admin.pubkey(),
        &acme.mint,
        prices(1_000),
        true,
    )]);
    h.env.give_tokens(&acme, &buyer.pubkey(), 100_000);
    // 1 %: sold, the treasury netting the price less the tax
    h.env.ok(
        &[ix_buy(
            &buyer.pubkey(),
            &buyer.pubkey(),
            Some(&acme),
            0,
            1,
            1_000,
        )],
        &[&buyer],
    );
    assert_eq!(h.env.tokens(&acme.ata(&treasury())), 990);
    // its fee authority schedules 6 %: no more sales in it, even before it applies
    let raise = fee_ix::set_transfer_fee(
        &acme.program,
        &acme.mint,
        &admin.pubkey(),
        &[],
        600,
        u64::MAX,
    )
    .unwrap();
    h.env.ok(&[raise], &[&admin]);
    h.fails(
        &[ix_buy(
            &buyer.pubkey(),
            &buyer.pubkey(),
            Some(&acme),
            0,
            2,
            2_000,
        )],
        &[&buyer],
        HouseError::TransferFeeTooHigh,
    );
}

#[test]
fn the_buyback_vault_is_sold_in_dutch_auctions_and_the_price_burned() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let keeper = h.keeper.insecure_clone();
    let k = keeper.pubkey();
    let buyer = h.env.player();
    let usdc = h.env.spl_token();
    h.add_token(&usdc);
    let acme = h.env.fee_mint(100);
    h.add_token(&acme);
    // 1,000 USDC in the buyback vault (as sweeps leave it)
    h.env.ensure_ata(&usdc, &buyback());
    h.env.give_tokens(&usdc, &buyback(), 1_000_000_000);
    let start = |lot: u64, start: u64, floor: u64, decline: u32, run: u32| {
        ix_start(&k, Some(&usdc), lot, start, floor, decline, run)
    };
    // no lowest rate from the admin yet: no auction (its account doesn't exist)
    h.env.fails_any(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 7_200)],
        &[&keeper],
    );
    // the admin alone sets it: 1 ACME a USDC at the lowest
    h.fails(
        &[ix_set_floor(
            &buyer.pubkey(),
            Some(&usdc),
            1_000_000,
            1_000_000,
        )],
        &[&buyer],
        HouseError::Unauthorized,
    );
    h.fails(
        &[ix_set_floor(&admin.pubkey(), Some(&usdc), 1_000_000, 0)],
        &[&admin],
        HouseError::InvalidParams,
    );
    h.admin(&[ix_set_floor(
        &admin.pubkey(),
        Some(&usdc),
        1_000_000,
        1_000_000,
    )]);
    let a = h.auction(&usdc.mint);
    assert_eq!((a.min_lot, a.min_price, a.lot), (1_000_000, 1_000_000, 0));
    // no project token yet: no auctions
    h.fails(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 7_200)],
        &[&keeper],
        HouseError::NoToken,
    );
    h.set_token(&acme.mint);
    // a floor below the admin's rate: refused
    h.fails(
        &[start(100_000_000, 400_000_000, 99_999_999, 3_600, 7_200)],
        &[&keeper],
        HouseError::FloorTooLow,
    );

    // the keeper alone; a quarter of the vault at most; sane prices and times
    h.fails(
        &[ix_start(
            &buyer.pubkey(),
            Some(&usdc),
            100_000_000,
            400_000_000,
            100_000_000,
            3_600,
            7_200,
        )],
        &[&buyer],
        HouseError::Unauthorized,
    );
    h.fails(
        &[start(250_000_001, 400_000_000, 100_000_000, 3_600, 7_200)],
        &[&keeper],
        HouseError::LotTooLarge,
    );
    for bad in [
        start(0, 400_000_000, 100_000_000, 3_600, 7_200),
        start(100_000_000, 400_000_000, 0, 3_600, 7_200),
        start(100_000_000, 99_999_999, 100_000_000, 3_600, 7_200),
        start(100_000_000, 400_000_000, 100_000_000, 3_599, 7_200),
        start(100_000_000, 400_000_000, 100_000_000, 3_600, 3_599),
        start(
            100_000_000,
            400_000_000,
            100_000_000,
            7 * 86_400 + 1,
            14 * 86_400,
        ),
        start(
            100_000_000,
            400_000_000,
            100_000_000,
            3_600,
            14 * 86_400 + 1,
        ),
    ] {
        h.fails(&[bad], &[&keeper], HouseError::InvalidParams);
    }
    // the project token itself is never sold: no rate for it, so no auction
    h.env.ensure_ata(&acme, &buyback());
    h.fails(
        &[ix_set_floor(&admin.pubkey(), Some(&acme), 1, 1)],
        &[&admin],
        HouseError::InvalidParams,
    );
    h.env.fails_any(
        &[ix_start(&k, Some(&acme), 1, 2, 1, 3_600, 3_600)],
        &[&keeper],
    );

    // 100 USDC: 400 ACME at first, falling to 100 over an hour, then held an hour
    h.env.ok(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 7_200)],
        &[&keeper],
    );
    h.fails(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 7_200)],
        &[&keeper],
        HouseError::AuctionRunning,
    );
    h.env.give_tokens(&acme, &buyer.pubkey(), 1_000_000_000);
    h.env.ensure_ata(&usdc, &buyer.pubkey());
    // half way down: 250 ACME; a quote's guards on the price and the lot
    h.env.warp(1_800);
    let buy = |max_price: u64, min_lot: u64| {
        ix_buy_lot(&buyer.pubkey(), &acme, Some(&usdc), max_price, min_lot)
    };
    h.fails(
        &[buy(249_999_999, 100_000_000)],
        &[&buyer],
        HouseError::PriceTooHigh,
    );
    h.fails(
        &[buy(250_000_000, 100_000_001)],
        &[&buyer],
        HouseError::LotTooSmall,
    );
    // only the buyer's own account of the project token is burned from
    let mut stolen = buy(250_000_000, 100_000_000);
    stolen.accounts[5].pubkey = h.env.ensure_ata(&acme, &admin.pubkey());
    h.fails(&[stolen], &[&buyer], HouseError::TokenAccountsMismatch);
    let (s0, b0) = (
        supply(&h.env, &acme.mint),
        h.env.tokens(&acme.ata(&buyer.pubkey())),
    );
    h.env.ok(&[buy(250_000_000, 100_000_000)], &[&buyer]);
    // burned in full (a burn pays no tax), and the lot paid
    assert_eq!(supply(&h.env, &acme.mint), s0 - 250_000_000);
    assert_eq!(h.env.tokens(&acme.ata(&buyer.pubkey())), b0 - 250_000_000);
    assert_eq!(h.env.tokens(&usdc.ata(&buyer.pubkey())), 100_000_000);
    assert_eq!(h.env.tokens(&usdc.ata(&buyback())), 900_000_000);
    assert_eq!(h.totals(&acme.mint).burned, 250_000_000);
    let a = h.auction(&usdc.mint);
    assert_eq!(
        (
            a.lot,
            a.last_lot,
            a.last_price,
            a.sold,
            a.paid_out,
            a.burned
        ),
        (0, 100_000_000, 250_000_000, 1, 100_000_000, 250_000_000)
    );
    // sold: nothing more to buy
    h.fails(&[buy(400_000_000, 0)], &[&buyer], HouseError::NoAuction);

    // the next auction: a day after the last one started, its floor at least
    // half the last sale's rate (2.5 ACME a USDC: 200 USDC at 250 ACME or more)
    h.fails(
        &[start(200_000_000, 600_000_000, 250_000_000, 3_600, 3_600)],
        &[&keeper],
        HouseError::AuctionTooSoon,
    );
    h.env.warp(AUCTION_EVERY_SECS);
    h.fails(
        &[start(200_000_000, 600_000_000, 249_999_999, 3_600, 3_600)],
        &[&keeper],
        HouseError::FloorTooLow,
    );
    h.env.ok(
        &[start(200_000_000, 600_000_000, 250_000_000, 3_600, 3_600)],
        &[&keeper],
    );
    // nobody buys: it ends at its end; the next one a day after it started
    h.env.warp(3_600);
    h.fails(&[buy(600_000_000, 0)], &[&buyer], HouseError::NoAuction);
    h.fails(
        &[start(200_000_000, 600_000_000, 250_000_000, 3_600, 3_600)],
        &[&keeper],
        HouseError::AuctionTooSoon,
    );
    h.env.warp(AUCTION_EVERY_SECS);
    h.env.ok(
        &[start(200_000_000, 600_000_000, 250_000_000, 3_600, 3_600)],
        &[&keeper],
    );
    // paused: no sales and no new auctions
    h.admin(&[ix_admin(
        &admin.pubkey(),
        house::instruction::SetPaused { paused: true },
    )]);
    h.fails(&[buy(600_000_000, 0)], &[&buyer], HouseError::Paused);
    h.admin(&[ix_admin(
        &admin.pubkey(),
        house::instruction::SetPaused { paused: false },
    )]);
    h.env.ok(&[buy(600_000_000, 200_000_000)], &[&buyer]);
    assert_eq!(h.env.tokens(&usdc.ata(&buyback())), 700_000_000);
    // the last sale (3 ACME a USDC) holds the floor to half of it for a week;
    // after that only the admin's rate does (the market may have moved)
    h.env.warp(AUCTION_EVERY_SECS);
    h.fails(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 3_600)],
        &[&keeper],
        HouseError::FloorTooLow,
    );
    h.env.warp(7 * 86_400);
    h.env.ok(
        &[start(100_000_000, 400_000_000, 100_000_000, 3_600, 3_600)],
        &[&keeper],
    );
}

#[test]
fn a_sol_lot_is_paid_from_the_vaults_lamports_above_its_rent() {
    let mut h = House::new();
    let admin = h.env.admin.insecure_clone();
    let keeper = h.keeper.insecure_clone();
    let payer = h.env.player();
    let buyer = h.env.player();
    let acme = h.env.fee_mint(100);
    h.add_token(&acme);
    // the admin's lowest rate, 1 ACME a SOL - set before the project token (SOL's "mint" is the
    // key a house without one has as its token_mint)
    h.admin(&[ix_set_floor(&admin.pubkey(), None, SOL, 1_000_000)]);
    h.set_token(&acme.mint);
    h.env.svm.airdrop(&buyback(), 4 * SOL).unwrap();
    let floor = h.env.svm.minimum_balance_for_rent_exemption(0);
    let held = h.env.lamports(&buyback()) - floor;
    // a quarter of what it holds above its rent at most
    h.fails(
        &[ix_start(
            &keeper.pubkey(),
            None,
            held / 4 + 1,
            5_000_000,
            1_000_000,
            3_600,
            3_600,
        )],
        &[&keeper],
        HouseError::LotTooLarge,
    );
    h.env.ok(
        &[ix_start(
            &keeper.pubkey(),
            None,
            SOL,
            5_000_000,
            1_000_000,
            3_600,
            3_600,
        )],
        &[&keeper],
    );
    h.env.give_tokens(&acme, &buyer.pubkey(), 10_000_000);
    // at its start: the start price (the server pays the fee: the buyer's SOL moves by the lot alone)
    let (l0, v0, s0) = (
        h.env.lamports(&buyer.pubkey()),
        h.env.lamports(&buyback()),
        supply(&h.env, &acme.mint),
    );
    h.env.ok(
        &[ix_buy_lot(&buyer.pubkey(), &acme, None, 5_000_000, SOL)],
        &[&payer, &buyer],
    );
    assert_eq!(h.env.lamports(&buyer.pubkey()), l0 + SOL);
    assert_eq!(h.env.lamports(&buyback()), v0 - SOL);
    assert_eq!(supply(&h.env, &acme.mint), s0 - 5_000_000);
    // token accounts for a SOL lot are refused
    h.env.warp(AUCTION_EVERY_SECS);
    h.env.ok(
        &[ix_start(
            &keeper.pubkey(),
            None,
            SOL / 2,
            5_000_000,
            2_500_000,
            3_600,
            3_600,
        )],
        &[&keeper],
    );
    let usdc = h.env.spl_token();
    h.env.ensure_ata(&usdc, &buyback());
    let mut ix = ix_buy_lot(&buyer.pubkey(), &acme, None, 5_000_000, 0);
    let with_tokens = ix_buy_lot(&buyer.pubkey(), &acme, Some(&usdc), 5_000_000, 0);
    ix.accounts[9] = with_tokens.accounts[9].clone();
    h.fails(&[ix], &[&payer, &buyer], HouseError::TokenAccountsMismatch);
}

//! Test harness: a LiteSVM with the arena deployed and initialized (its treasury
//! and prize vault the house program's PDAs, given their rent as the house's
//! `initialize` does), plus instruction builders that mirror what the race
//! server's transaction builder sends.

#![allow(dead_code)]

use anchor_lang::{
    prelude::{Clock, Pubkey},
    solana_program::{
        instruction::AccountMeta, instruction::Instruction, system_instruction, system_program,
    },
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address_with_program_id,
        spl_associated_token_account::instruction::create_associated_token_account_idempotent,
    },
    token_2022::spl_token_2022::{
        extension::{transfer_fee::instruction as fee_ix, ExtensionType},
        instruction as t22,
        state::Mint as SplMint,
    },
};
use arena::{
    instructions::{ConfigParams, CupParams, MintParams, RoomParams},
    state::*,
    HOUSE_ID, NATIVE_SOL,
};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

pub const SOL: u64 = 1_000_000_000;
pub const RULESET_ID: u32 = 1;
pub const SETTLE_WINDOW: u32 = 4 * 3_600;
pub const CHALLENGE_WINDOW: u32 = 86_400;
pub const RESULTS_WINDOW: u32 = 7 * 86_400;
pub const CLAIM_WINDOW: u32 = 7 * 86_400;
pub const JOIN_WINDOW: u32 = 3_600;
/// The game's rating points per level (src/game/upgrades.js): pu, aero, tyre,
/// brake, chassis, dur.
pub const POINTS: [u8; 6] = [10, 15, 19, 6, 6, 4];

pub fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &arena::ID).0
}
pub fn config_pda() -> Pubkey {
    pda(&[b"config"])
}
pub fn ruleset_pda(id: u32) -> Pubkey {
    pda(&[b"ruleset", &id.to_le_bytes()])
}
pub fn mint_pda(mint: &Pubkey) -> Pubkey {
    pda(&[b"mint", mint.as_ref()])
}
pub fn room_pda(code: &[u8; 8]) -> Pubkey {
    pda(&[b"room", code])
}
pub fn cup_pda(id: u64) -> Pubkey {
    pda(&[b"cup", &id.to_le_bytes()])
}
pub fn entry_pda(cup: &Pubkey, player: &Pubkey) -> Pubkey {
    pda(&[b"entry", cup.as_ref(), player.as_ref()])
}
pub fn host_pda(wallet: &Pubkey) -> Pubkey {
    pda(&[b"host", wallet.as_ref()])
}
pub fn house_pda(seed: &[u8]) -> Pubkey {
    Pubkey::find_program_address(&[seed], &HOUSE_ID).0
}
pub fn treasury() -> Pubkey {
    house_pda(b"treasury")
}
pub fn prize_vault() -> Pubkey {
    house_pda(b"prize")
}
pub fn upgrades_pda(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"upgrades", owner.as_ref()], &HOUSE_ID).0
}

/// A room code from a number: "ROOM0001", ...
pub fn code(n: u32) -> [u8; 8] {
    let s = format!("ROOM{n:04}");
    s.as_bytes().try_into().unwrap()
}

pub fn race(start_mode: u8) -> RaceSpec {
    RaceSpec {
        track: 0,
        format: 0,
        laps: 3,
        ai: 0,
        time: 0,
        sky: 0,
        start_mode,
    }
}

/// An EQUAL room that starts when full.
pub fn room_params(stake: u64, max_players: u8) -> RoomParams {
    RoomParams {
        stake,
        max_players,
        join_window_secs: JOIN_WINDOW,
        class: CLASS_EQUAL,
        max_rating: 0,
        access: ACCESS_OPEN,
        race: race(0),
    }
}

pub fn mint_params(min: u64, max: u64) -> MintParams {
    MintParams {
        min_stake: min,
        max_stake: max,
        room_fee_bps: 500,
        cup_fee_bps: 1_000,
        allow_transfer_fee: false,
        max_transfer_fee_bps: 0,
        holder_max_stake: max,
    }
}

/// The token a room or cup is in. `None` everywhere below means SOL.
#[derive(Clone, Copy)]
pub struct Spl {
    pub mint: Pubkey,
    pub program: Pubkey,
}

impl Spl {
    pub fn ata(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(owner, &self.mint, &self.program)
    }
}

pub fn err_code(e: arena::error::ArenaError) -> String {
    format!(
        "Custom({})",
        anchor_lang::error::ERROR_CODE_OFFSET + e as u32
    )
}

pub struct Env {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub guardian: Keypair,
    pub gate: Keypair,
    pub resolver: Keypair,
    /// The race server's fee and rent sponsor.
    pub sponsor: Keypair,
}

impl Env {
    /// Deployed, but not initialized.
    pub fn deployed() -> Env {
        let mut svm = LiteSVM::new();
        // a real date (LiteSVM's clock starts at 0, which no chain ever shows)
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp = 1_792_368_000; // 2026-10-19
        svm.set_sysvar(&clock);
        let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/arena.so"));
        svm.add_program(arena::ID, bytes).unwrap();
        let (admin, guardian, gate, resolver, sponsor) = (
            Keypair::new(),
            Keypair::new(),
            Keypair::new(),
            Keypair::new(),
            Keypair::new(),
        );
        for k in [&admin, &guardian, &gate, &resolver, &sponsor] {
            svm.airdrop(&k.pubkey(), 100 * SOL).unwrap();
        }
        // the house's vaults, as its initialize leaves them: plain system accounts
        // holding their rent
        for v in [treasury(), prize_vault()] {
            svm.airdrop(&v, svm.minimum_balance_for_rent_exemption(0))
                .unwrap();
        }
        // LiteSVM deploys with no upgrade authority; make the admin the upgrade
        // authority, as a real deploy would. ProgramData: u32 tag, u64 slot,
        // Option<Pubkey> authority.
        let pd = program_data();
        let mut acct = svm.get_account(&pd).unwrap();
        acct.data[12] = 1;
        acct.data[13..45].copy_from_slice(admin.pubkey().as_ref());
        svm.set_account(pd, acct).unwrap();
        Env {
            svm,
            admin,
            guardian,
            gate,
            resolver,
            sponsor,
        }
    }

    /// Ungated (a test turns the gate on with `gate_on`).
    pub fn config_params(&self) -> ConfigParams {
        ConfigParams {
            guardian: self.guardian.pubkey(),
            gate: Pubkey::default(),
            treasury: treasury(),
            prize_vault: prize_vault(),
            settle_window_secs: SETTLE_WINDOW,
            challenge_window_secs: CHALLENGE_WINDOW,
            results_window_secs: RESULTS_WINDOW,
            claim_window_secs: CLAIM_WINDOW,
        }
    }

    pub fn ix_initialize(&self, signer: &Pubkey, params: ConfigParams) -> Instruction {
        Instruction::new_with_bytes(
            arena::ID,
            &arena::instruction::Initialize {
                params,
                resolvers: vec![self.resolver.pubkey()],
            }
            .data(),
            arena::accounts::Initialize {
                admin: *signer,
                config: config_pda(),
                program: arena::ID,
                program_data: program_data(),
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        )
    }

    /// Initialized, with ruleset 1 and SOL enabled (0.001..=100 SOL).
    pub fn new() -> Env {
        let mut env = Env::deployed();
        let admin = env.admin.insecure_clone();
        let ix = env.ix_initialize(&admin.pubkey(), env.config_params());
        env.ok(&[ix], &[&admin]);
        let ix = Instruction::new_with_bytes(
            arena::ID,
            &arena::instruction::AddRuleset {
                id: RULESET_ID,
                hash: [7u8; 32],
                rating_points: POINTS,
            }
            .data(),
            arena::accounts::AddRuleset {
                admin: admin.pubkey(),
                config: config_pda(),
                ruleset: ruleset_pda(RULESET_ID),
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        );
        env.ok(&[ix], &[&admin]);
        let ix = env.ix_add_mint(&NATIVE_SOL, None, mint_params(SOL / 1_000, 100 * SOL));
        env.ok(&[ix], &[&admin]);
        env
    }

    pub fn ix_admin<T: InstructionData>(&self, data: T) -> Instruction {
        Instruction::new_with_bytes(
            arena::ID,
            &data.data(),
            arena::accounts::AdminConfig {
                admin: self.admin.pubkey(),
                config: config_pda(),
            }
            .to_account_metas(None),
        )
    }

    /// Every new room, join and entry needs the gate's signature from now on
    /// (proposed, and in force once the roles' timelock has passed).
    pub fn gate_on(&mut self) {
        let ix = self.ix_admin(arena::instruction::ProposeRoles {
            guardian: self.guardian.pubkey(),
            gate: self.gate.pubkey(),
            resolvers: vec![self.resolver.pubkey()],
        });
        let admin = self.admin.insecure_clone();
        self.ok(&[ix], &[&admin]);
        self.warp(arena::ROLES_TIMELOCK_SECS);
        self.ok(&[ix_apply_roles()], &[&admin]);
    }

    /// A join, its expected stake the room's own.
    pub fn ix_join(
        &self,
        player: &Pubkey,
        room: &Pubkey,
        spl: Option<&Spl>,
        who: Who,
    ) -> Instruction {
        ix_join_at(player, room, spl, who, self.room(room).stake)
    }

    /// The room's seed, drawn the way the chain allows it: the slot it locked in
    /// over (in the SlotHashes sysvar), anyone sends draw_seed.
    pub fn draw_seed(&mut self, room: &Pubkey) {
        let slot = self.room(room).locked_slot;
        self.slot_hashes(&[(slot, [9u8; 32])]);
        self.next_slot();
        let anyone = self.player();
        self.ok(&[ix_draw_seed(room)], &[&anyone]);
    }

    /// The next slot, the same second.
    pub fn next_slot(&mut self) {
        let mut c = self.svm.get_sysvar::<Clock>();
        c.slot += 1;
        self.svm.set_sysvar(&c);
    }

    /// The SlotHashes sysvar's entries (newest first): (slot, hash).
    pub fn slot_hashes(&mut self, entries: &[(u64, [u8; 32])]) {
        let mut data = (entries.len() as u64).to_le_bytes().to_vec();
        for (slot, hash) in entries {
            data.extend_from_slice(&slot.to_le_bytes());
            data.extend_from_slice(hash);
        }
        let id = solana_sdk_ids::sysvar::slot_hashes::ID;
        let mut acct = self.svm.get_account(&id).unwrap_or_default();
        acct.lamports = acct
            .lamports
            .max(self.svm.minimum_balance_for_rent_exemption(data.len()));
        acct.data = data;
        acct.owner = solana_sdk_ids::sysvar::ID;
        self.svm.set_account(id, acct).unwrap();
    }

    /// A result post, its places what the cup will pay (min(table, scorers)).
    pub fn ix_post(
        &self,
        resolver: &Pubkey,
        cup: &Pubkey,
        root: [u8; 32],
        scorers: u32,
    ) -> Instruction {
        let places = core::cmp::min(self.cup(cup).payout_bps.len(), scorers as usize) as u8;
        ix_post_places(resolver, cup, root, scorers, places)
    }

    pub fn ix_add_mint(
        &self,
        mint: &Pubkey,
        mint_account: Option<Pubkey>,
        params: MintParams,
    ) -> Instruction {
        Instruction::new_with_bytes(
            arena::ID,
            &arena::instruction::AddMint {
                mint_key: *mint,
                params,
            }
            .data(),
            arena::accounts::AddMint {
                admin: self.admin.pubkey(),
                config: config_pda(),
                allowed_mint: mint_pda(mint),
                mint: mint_account,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        )
    }

    /// A classic SPL token (6 decimals), enabled for stakes of 1..=1000 tokens.
    pub fn spl_token(&mut self) -> Spl {
        let admin = self.admin.insecure_clone();
        let program = anchor_spl::token::ID;
        let mint = litesvm_token::CreateMint::new(&mut self.svm, &admin)
            .decimals(6)
            .send()
            .unwrap();
        let spl = Spl { mint, program };
        let ix = self.ix_add_mint(&mint, Some(mint), mint_params(1_000_000, 1_000_000_000));
        self.ok(&[ix], &[&admin]);
        self.ensure_ata(&spl, &treasury());
        self.ensure_ata(&spl, &prize_vault());
        spl
    }

    /// A Token-2022 mint with a transfer fee of `bps` (StonkFun's project token
    /// is 100 bps), the admin its fee authority. Not yet allowed in the arena.
    pub fn fee_mint(&mut self, bps: u16) -> Spl {
        let admin = self.admin.insecure_clone();
        let mint = Keypair::new();
        let program = anchor_spl::token_2022::ID;
        let len = ExtensionType::try_calculate_account_len::<SplMint>(&[
            ExtensionType::TransferFeeConfig,
        ])
        .unwrap();
        let rent = self.svm.minimum_balance_for_rent_exemption(len);
        let ixs = [
            system_instruction::create_account(
                &admin.pubkey(),
                &mint.pubkey(),
                rent,
                len as u64,
                &program,
            ),
            fee_ix::initialize_transfer_fee_config(
                &program,
                &mint.pubkey(),
                Some(&admin.pubkey()),
                Some(&admin.pubkey()),
                bps,
                u64::MAX,
            )
            .unwrap(),
            t22::initialize_mint2(&program, &mint.pubkey(), &admin.pubkey(), None, 6).unwrap(),
        ];
        self.ok(&ixs, &[&admin, &mint]);
        let spl = Spl {
            mint: mint.pubkey(),
            program,
        };
        self.ensure_ata(&spl, &treasury());
        self.ensure_ata(&spl, &prize_vault());
        spl
    }

    /// The fee mint allowed in the arena (stakes 1..=100_000 tokens; fee cap `cap_bps`).
    pub fn allow_fee_mint(&mut self, spl: &Spl, cap_bps: u16) {
        let admin = self.admin.insecure_clone();
        let mut p = mint_params(1_000_000, 100_000_000_000);
        p.allow_transfer_fee = true;
        p.max_transfer_fee_bps = cap_bps;
        p.room_fee_bps = 250;
        let ix = self.ix_add_mint(&spl.mint, Some(spl.mint), p);
        self.ok(&[ix], &[&admin]);
    }

    /// Fund `owner` with `amount` base units of `spl` in their ATA.
    pub fn give_tokens(&mut self, spl: &Spl, owner: &Pubkey, amount: u64) -> Pubkey {
        let admin = self.admin.insecure_clone();
        let ata = self.ensure_ata(spl, owner);
        let ix = t22::mint_to(&spl.program, &spl.mint, &ata, &admin.pubkey(), &[], amount).unwrap();
        self.ok(&[ix], &[&admin]);
        ata
    }

    pub fn ensure_ata(&mut self, spl: &Spl, owner: &Pubkey) -> Pubkey {
        let admin = self.admin.insecure_clone();
        let ix = create_associated_token_account_idempotent(
            &admin.pubkey(),
            owner,
            &spl.mint,
            &spl.program,
        );
        self.ok(&[ix], &[&admin]);
        spl.ata(owner)
    }

    /// Writes the house program's upgrades account for `owner`, as its shop would
    /// leave it (owner, discriminator, version, levels, bump).
    pub fn set_upgrades(&mut self, owner: &Pubkey, levels: [u8; 6]) {
        self.set_upgrades_raw(
            owner,
            levels,
            HOUSE_ID,
            arena::UPGRADES_DISCRIMINATOR,
            *owner,
        );
    }

    pub fn set_upgrades_raw(
        &mut self,
        owner: &Pubkey,
        levels: [u8; 6],
        program: Pubkey,
        disc: [u8; 8],
        recorded_owner: Pubkey,
    ) {
        let (key, bump) = Pubkey::find_program_address(&[b"upgrades", owner.as_ref()], &HOUSE_ID);
        let mut data = Vec::with_capacity(8 + 1 + 32 + 8 + 8 + 1);
        data.extend_from_slice(&disc);
        data.push(1);
        data.extend_from_slice(recorded_owner.as_ref());
        data.extend_from_slice(&levels);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&0i64.to_le_bytes());
        data.push(bump);
        let lamports = self.svm.minimum_balance_for_rent_exemption(data.len());
        self.svm
            .set_account(
                key,
                solana_account::Account {
                    lamports,
                    data,
                    owner: program,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
    }

    pub fn player(&mut self) -> Keypair {
        let k = Keypair::new();
        self.svm.airdrop(&k.pubkey(), 100 * SOL).unwrap();
        k
    }

    // ---- transactions ----

    // LiteSVM's own result type; its large Err variant is not ours to box.
    #[allow(clippy::result_large_err)]
    pub fn send(&mut self, ixs: &[Instruction], signers: &[&Keypair]) -> TransactionResult {
        let msg = Message::new_with_blockhash(
            ixs,
            Some(&signers[0].pubkey()),
            &self.svm.latest_blockhash(),
        );
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
        let res = self.svm.send_transaction(tx);
        self.svm.expire_blockhash();
        res
    }

    pub fn ok(&mut self, ixs: &[Instruction], signers: &[&Keypair]) {
        if let Err(e) = self.send(ixs, signers) {
            panic!("tx failed: {:?}\n{}", e.err, e.meta.logs.join("\n"));
        }
    }

    pub fn fails(
        &mut self,
        ixs: &[Instruction],
        signers: &[&Keypair],
        code: arena::error::ArenaError,
    ) {
        let want = err_code(code);
        match self.send(ixs, signers) {
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

    /// Fails with anything (account constraint, missing account, ...).
    pub fn fails_any(&mut self, ixs: &[Instruction], signers: &[&Keypair]) {
        assert!(
            self.send(ixs, signers).is_err(),
            "expected failure, tx succeeded"
        );
    }

    // ---- clock and state ----

    pub fn now(&self) -> i64 {
        self.svm.get_sysvar::<Clock>().unix_timestamp
    }

    pub fn warp(&mut self, secs: i64) {
        let mut c = self.svm.get_sysvar::<Clock>();
        c.unix_timestamp += secs;
        c.slot += 1;
        self.svm.set_sysvar(&c);
    }

    /// Into a later epoch (a scheduled transfer fee takes effect two epochs on).
    pub fn warp_epochs(&mut self, n: u64) {
        let mut c = self.svm.get_sysvar::<Clock>();
        c.epoch += n;
        c.slot += 432_000 * n;
        self.svm.set_sysvar(&c);
    }

    pub fn lamports(&self, pk: &Pubkey) -> u64 {
        self.svm.get_account(pk).map(|a| a.lamports).unwrap_or(0)
    }

    pub fn tokens(&self, pk: &Pubkey) -> u64 {
        let a = self.svm.get_account(pk).unwrap();
        u64::from_le_bytes(a.data[64..72].try_into().unwrap())
    }

    pub fn exists(&self, pk: &Pubkey) -> bool {
        self.svm
            .get_account(pk)
            .map(|a| a.lamports > 0)
            .unwrap_or(false)
    }

    pub fn load<T: AccountDeserialize>(&self, pk: &Pubkey) -> T {
        let a = self.svm.get_account(pk).unwrap();
        T::try_deserialize(&mut &a.data[..]).unwrap()
    }

    pub fn room(&self, pk: &Pubkey) -> Room {
        self.load(pk)
    }
    pub fn cup(&self, pk: &Pubkey) -> Cup {
        self.load(pk)
    }
    pub fn config(&self) -> Config {
        self.load(&config_pda())
    }

    /// Where a payout to `owner` lands: the wallet for SOL, the ATA for SPL.
    pub fn dest(&self, spl: Option<&Spl>, owner: &Pubkey) -> Pubkey {
        match spl {
            None => *owner,
            Some(s) => s.ata(owner),
        }
    }
    pub fn balance(&self, spl: Option<&Spl>, owner: &Pubkey) -> u64 {
        match spl {
            None => self.lamports(owner),
            Some(s) => self.tokens(&s.ata(owner)),
        }
    }
}

pub fn program_data() -> Pubkey {
    Pubkey::find_program_address(
        &[arena::ID.as_ref()],
        &anchor_lang::solana_program::bpf_loader_upgradeable::ID,
    )
    .0
}

fn token_opts(spl: Option<&Spl>, pda: &Pubkey) -> (Option<Pubkey>, Option<Pubkey>, Option<Pubkey>) {
    match spl {
        None => (None, None, None),
        Some(s) => (Some(s.mint), Some(s.ata(pda)), Some(s.program)),
    }
}

fn payouts(metas: &mut Vec<AccountMeta>, spl: Option<&Spl>, players: &[Pubkey]) {
    for p in players {
        let dest = match spl {
            None => *p,
            Some(s) => s.ata(p),
        };
        metas.push(AccountMeta::new(dest, false));
    }
}

// ---- room instruction builders -------------------------------------------------

#[derive(Clone, Copy)]
pub struct Who<'a> {
    pub upgrades: bool,
    pub holder: Option<&'a Spl>,
    pub gate: Option<Pubkey>,
}

pub const PLAIN: Who = Who {
    upgrades: false,
    holder: None,
    gate: None,
};

/// For SPL, prepends the idempotent creation of the room's escrow ATA, as the
/// race server's transaction builder does.
pub fn ix_create_room(
    creator: &Pubkey,
    payer: &Pubkey,
    room_code: [u8; 8],
    params: RoomParams,
    spl: Option<&Spl>,
    who: Who,
) -> Vec<Instruction> {
    let room = room_pda(&room_code);
    let (mint, escrow, program) = token_opts(spl, &room);
    let mut ixs = Vec::new();
    if let Some(s) = spl {
        ixs.push(create_associated_token_account_idempotent(
            payer, &room, &s.mint, &s.program,
        ));
    }
    ixs.push(Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::CreateRoom {
            code: room_code,
            params,
        }
        .data(),
        arena::accounts::CreateRoom {
            creator: *creator,
            payer: *payer,
            gate: who.gate,
            config: config_pda(),
            ruleset: ruleset_pda(RULESET_ID),
            allowed_mint: mint_pda(&spl.map(|s| s.mint).unwrap_or(NATIVE_SOL)),
            room,
            upgrades: who.upgrades.then(|| upgrades_pda(creator)),
            holder_tokens: who.holder.map(|h| h.ata(creator)),
            mint,
            escrow_tokens: escrow,
            creator_tokens: spl.map(|s| s.ata(creator)),
            token_program: program,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    ));
    ixs
}

pub fn ix_join_at(
    player: &Pubkey,
    room: &Pubkey,
    spl: Option<&Spl>,
    who: Who,
    expected_stake: u64,
) -> Instruction {
    let (mint, escrow, program) = token_opts(spl, room);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::JoinRoom { expected_stake }.data(),
        arena::accounts::JoinRoom {
            player: *player,
            gate: who.gate,
            config: config_pda(),
            room: *room,
            ruleset: ruleset_pda(RULESET_ID),
            upgrades: who.upgrades.then(|| upgrades_pda(player)),
            mint,
            escrow_tokens: escrow,
            player_tokens: spl.map(|s| s.ata(player)),
            token_program: program,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn ix_leave(player: &Pubkey, room: &Pubkey, spl: Option<&Spl>) -> Instruction {
    let (mint, escrow, program) = token_opts(spl, room);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::LeaveRoom {}.data(),
        arena::accounts::LeaveRoom {
            player: *player,
            room: *room,
            mint,
            escrow_tokens: escrow,
            player_tokens: spl.map(|s| s.ata(player)),
            token_program: program,
        }
        .to_account_metas(None),
    )
}

fn room_payout(room: &Pubkey, spl: Option<&Spl>) -> arena::accounts::RoomPayout {
    let (mint, escrow, program) = token_opts(spl, room);
    arena::accounts::RoomPayout {
        room: *room,
        mint,
        escrow_tokens: escrow,
        token_program: program,
    }
}

pub fn ix_cancel(
    creator: &Pubkey,
    room: &Pubkey,
    spl: Option<&Spl>,
    players: &[Pubkey],
) -> Instruction {
    let mut metas = arena::accounts::CancelRoom {
        creator: *creator,
        payout: room_payout(room, spl),
    }
    .to_account_metas(None);
    payouts(&mut metas, spl, players);
    Instruction::new_with_bytes(arena::ID, &arena::instruction::CancelRoom {}.data(), metas)
}

pub fn ix_lock(resolver: &Pubkey, room: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::LockRoom {}.data(),
        arena::accounts::LockRoom {
            resolver: *resolver,
            config: config_pda(),
            room: *room,
        }
        .to_account_metas(None),
    )
}

pub fn ix_draw_seed(room: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::DrawSeed {}.data(),
        arena::accounts::DrawSeed {
            room: *room,
            slot_hashes: solana_sdk_ids::sysvar::slot_hashes::ID,
        }
        .to_account_metas(None),
    )
}

pub fn ix_apply_roles() -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::ApplyRoles {}.data(),
        arena::accounts::ApplyRoles {
            config: config_pda(),
        }
        .to_account_metas(None),
    )
}

/// `payees`: the winners in seat order (every player, when it refunds).
pub fn ix_settle(
    resolver: &Pubkey,
    room: &Pubkey,
    spl: Option<&Spl>,
    times_ms: Vec<u32>,
    record_hash: [u8; 32],
    payees: &[Pubkey],
) -> Instruction {
    let mut metas = arena::accounts::SettleRoom {
        resolver: *resolver,
        config: config_pda(),
        payout: room_payout(room, spl),
        treasury: match spl {
            None => treasury(),
            Some(s) => s.ata(&treasury()),
        },
    }
    .to_account_metas(None);
    payouts(&mut metas, spl, payees);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::SettleRoom {
            times_ms,
            record_hash,
        }
        .data(),
        metas,
    )
}

pub fn ix_void(
    resolver: &Pubkey,
    room: &Pubkey,
    spl: Option<&Spl>,
    reason: u8,
    players: &[Pubkey],
) -> Instruction {
    let mut metas = arena::accounts::VoidRoom {
        resolver: *resolver,
        config: config_pda(),
        payout: room_payout(room, spl),
    }
    .to_account_metas(None);
    payouts(&mut metas, spl, players);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::VoidRoom {
            reason,
            record_hash: [9u8; 32],
        }
        .data(),
        metas,
    )
}

pub fn ix_refund_room(room: &Pubkey, spl: Option<&Spl>, players: &[Pubkey]) -> Instruction {
    let mut metas = arena::accounts::RefundRoom {
        payout: room_payout(room, spl),
    }
    .to_account_metas(None);
    payouts(&mut metas, spl, players);
    Instruction::new_with_bytes(arena::ID, &arena::instruction::RefundRoom {}.data(), metas)
}

pub fn ix_close_room(
    room: &Pubkey,
    rent_payer: &Pubkey,
    spl: Option<&Spl>,
    vault_tokens: bool,
) -> Instruction {
    let (mint, escrow, program) = token_opts(spl, room);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::CloseRoom {}.data(),
        arena::accounts::CloseRoom {
            room: *room,
            rent_payer: *rent_payer,
            prize_vault: prize_vault(),
            prize_vault_tokens: match (spl, vault_tokens) {
                (Some(s), true) => Some(s.ata(&prize_vault())),
                _ => None,
            },
            mint,
            escrow_tokens: escrow,
            token_program: program,
        }
        .to_account_metas(None),
    )
}

// ---- cup instruction builders --------------------------------------------------

pub fn cup_params(kind: CupKind, entry_fee: u64, guarantee: u64, now: i64) -> CupParams {
    CupParams {
        kind,
        class: CLASS_EQUAL,
        entry_fee,
        payout_bps: vec![5_000, 3_000, 2_000],
        guarantee,
        starts_at: now,
        ends_at: now + 7 * 86_400,
        max_entrants: 0,
        spec_hash: [3u8; 32],
        featured_mint: Pubkey::new_from_array([42u8; 32]),
        host_share_bps: 0,
    }
}

pub fn ix_create_cup(
    organizer: &Pubkey,
    payer: &Pubkey,
    id: u64,
    params: CupParams,
    spl: Option<&Spl>,
    host: bool,
    holder: Option<&Spl>,
) -> Vec<Instruction> {
    let cup = cup_pda(id);
    let (mint, escrow, program) = token_opts(spl, &cup);
    let mut ixs = Vec::new();
    if let Some(s) = spl {
        ixs.push(create_associated_token_account_idempotent(
            payer, &cup, &s.mint, &s.program,
        ));
    }
    ixs.push(Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::CreateCup { id, params }.data(),
        arena::accounts::CreateCup {
            organizer: *organizer,
            payer: *payer,
            config: config_pda(),
            ruleset: ruleset_pda(RULESET_ID),
            allowed_mint: mint_pda(&spl.map(|s| s.mint).unwrap_or(NATIVE_SOL)),
            cup,
            host: host.then(|| host_pda(organizer)),
            holder_tokens: holder.map(|h| h.ata(organizer)),
            mint,
            escrow_tokens: escrow,
            organizer_tokens: spl.map(|s| s.ata(organizer)),
            token_program: program,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    ));
    ixs
}

/// `payer` pays the entry's rent (the race server's sponsor, or the player).
pub fn ix_enter(
    player: &Pubkey,
    payer: &Pubkey,
    cup: &Pubkey,
    spl: Option<&Spl>,
    gate: Option<Pubkey>,
) -> Instruction {
    let (mint, escrow, program) = token_opts(spl, cup);
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::EnterCup {}.data(),
        arena::accounts::EnterCup {
            player: *player,
            payer: *payer,
            gate,
            config: config_pda(),
            cup: *cup,
            entry: entry_pda(cup, player),
            mint,
            escrow_tokens: escrow,
            player_tokens: spl.map(|s| s.ata(player)),
            token_program: program,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn ix_post_places(
    resolver: &Pubkey,
    cup: &Pubkey,
    root: [u8; 32],
    scorers: u32,
    places: u8,
) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::PostResults {
            results_root: root,
            scorers,
            places,
            standings_hash: [5u8; 32],
            results_uri: "https://example.org/standings.json".into(),
        }
        .data(),
        arena::accounts::PostResults {
            resolver: *resolver,
            config: config_pda(),
            cup: *cup,
        }
        .to_account_metas(None),
    )
}

pub fn ix_guard(which: &str, authority: &Pubkey, cup: &Pubkey) -> Instruction {
    let data = match which {
        "reject" => arena::instruction::RejectResults {}.data(),
        _ => arena::instruction::CancelCup {}.data(),
    };
    Instruction::new_with_bytes(
        arena::ID,
        &data,
        arena::accounts::GuardCup {
            authority: *authority,
            config: config_pda(),
            cup: *cup,
        }
        .to_account_metas(None),
    )
}

pub fn ix_finalize(cup: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::FinalizeCup {}.data(),
        arena::accounts::FinalizeCup { cup: *cup }.to_account_metas(None),
    )
}

fn vault(cup: &Pubkey, spl: Option<&Spl>) -> arena::accounts::CupVault {
    let (mint, escrow, program) = token_opts(spl, cup);
    arena::accounts::CupVault {
        cup: *cup,
        mint,
        escrow_tokens: escrow,
        token_program: program,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ix_claim(
    claimer: &Pubkey,
    cup: &Pubkey,
    rank: u16,
    wallet: &Pubkey,
    proof: Vec<[u8; 32]>,
    spl: Option<&Spl>,
    entry: bool,
) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::ClaimPrize {
            rank,
            wallet: *wallet,
            proof,
        }
        .data(),
        arena::accounts::ClaimPrize {
            claimer: *claimer,
            vault: vault(cup, spl),
            entry: entry.then(|| entry_pda(cup, wallet)),
            destination: match spl {
                None => *wallet,
                Some(s) => s.ata(wallet),
            },
        }
        .to_account_metas(None),
    )
}

pub fn ix_refund_entry(cup: &Pubkey, player: &Pubkey, spl: Option<&Spl>) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::RefundEntry {}.data(),
        arena::accounts::RefundEntry {
            vault: vault(cup, spl),
            entry: entry_pda(cup, player),
            destination: match spl {
                None => *player,
                Some(s) => s.ata(player),
            },
        }
        .to_account_metas(None),
    )
}

pub fn ix_return_guarantee(cup: &Pubkey, sponsor: &Pubkey, spl: Option<&Spl>) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::ReturnGuarantee {}.data(),
        arena::accounts::ReturnGuarantee {
            vault: vault(cup, spl),
            destination: match spl {
                None => *sponsor,
                Some(s) => s.ata(sponsor),
            },
        }
        .to_account_metas(None),
    )
}

pub fn ix_close_cup(
    cup: &Pubkey,
    rent_payer: &Pubkey,
    sponsor: &Pubkey,
    host: Option<&Pubkey>,
    spl: Option<&Spl>,
) -> Instruction {
    let (mint, escrow, program) = token_opts(spl, cup);
    let to = |owner: &Pubkey| match spl {
        None => *owner,
        Some(s) => s.ata(owner),
    };
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::CloseCup {}.data(),
        arena::accounts::CloseCup {
            cup: *cup,
            rent_payer: *rent_payer,
            sponsor_destination: to(sponsor),
            host_destination: host.map(to),
            treasury_destination: to(&treasury()),
            prize_vault: prize_vault(),
            prize_vault_tokens: spl.map(|s| s.ata(&prize_vault())),
            mint,
            escrow_tokens: escrow,
            token_program: program,
        }
        .to_account_metas(None),
    )
}

pub fn ix_close_entry(cup: &Pubkey, player: &Pubkey, rent_payer: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        arena::ID,
        &arena::instruction::CloseEntry {}.data(),
        arena::accounts::CloseEntry {
            entry: entry_pda(cup, player),
            rent_payer: *rent_payer,
            cup: *cup,
        }
        .to_account_metas(None),
    )
}

// ---- Merkle results, as the race server builds them -------------------------------

pub fn results_tree(cup: &Pubkey, winners: &[Pubkey]) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
    let leaves: Vec<[u8; 32]> = winners
        .iter()
        .enumerate()
        .map(|(i, w)| arena::merkle::leaf(cup, (i + 1) as u16, w))
        .collect();
    let mut proofs: Vec<Vec<[u8; 32]>> = vec![vec![]; leaves.len()];
    let mut level: Vec<[u8; 32]> = leaves.clone();
    let mut index: Vec<usize> = (0..leaves.len()).collect();
    while level.len() > 1 {
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            next.push(if pair.len() == 2 {
                arena::merkle::node(&pair[0], &pair[1])
            } else {
                pair[0]
            });
        }
        for (leaf, pos) in index.iter_mut().enumerate() {
            let sib = *pos ^ 1;
            if sib < level.len() {
                proofs[leaf].push(level[sib]);
            }
            *pos /= 2;
        }
        level = next;
    }
    (level[0], proofs)
}

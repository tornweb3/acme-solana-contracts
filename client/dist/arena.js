import { BN, BorshCoder, EventParser, Program, utils } from "./anchor.js";
import { createAssociatedTokenAccountIdempotentInstruction } from "@solana/spl-token";
import { PublicKey, SystemProgram, SYSVAR_SLOT_HASHES_PUBKEY, } from "@solana/web3.js";
import { ARENA_PROGRAM_ID, CLASS_GARAGE, MEMO_PROGRAM_ID, NATIVE_SOL, ROOM_OFFSETS, ROOM_STATE, TOKEN_2022_ID } from "./constants.js";
import { arenaIdl } from "./idl/arena.js";
import { roomOutcome, winnerSeats } from "./payout.js";
import { allowedMintPda, ata, configPda, cupPda, entryPda, hostPda, roomCode, roomPda, rulesetPda, upgradesPda, } from "./pda.js";
/** A cup id nobody can guess before it is used (a cup's address is ["cup", id]): 8 random bytes. */
export function randomCupId() {
    const b = new Uint8Array(8);
    globalThis.crypto.getRandomValues(b);
    return new DataView(b.buffer).getBigUint64(0, true);
}
/** Anchor enums decode as `{ open: {} }`; this returns `"open"`. */
export const stateName = (s) => Object.keys(s)[0];
export const bn = (x) => (BN.isBN(x) ? x : new BN(x.toString()));
export const bytes32 = (b) => {
    const a = Array.from(b);
    if (a.length !== 32)
        throw new Error("expected 32 bytes");
    return a;
};
const mintParams = (p) => ({
    minStake: bn(p.minStake),
    maxStake: bn(p.maxStake),
    roomFeeBps: p.roomFeeBps,
    cupFeeBps: p.cupFeeBps,
    allowTransferFee: p.allowTransferFee ?? false,
    maxTransferFeeBps: p.maxTransferFeeBps ?? 0,
    holderMaxStake: bn(p.holderMaxStake ?? p.maxStake),
});
export const cupParams = (p) => ({
    kind: p.kind === "board" ? { board: {} } : { entries: {} },
    class: p.class ?? 0,
    entryFee: bn(p.entryFee ?? 0),
    payoutBps: p.payoutBps,
    guarantee: bn(p.guarantee ?? 0),
    startsAt: bn(p.startsAt),
    endsAt: bn(p.endsAt),
    maxEntrants: p.maxEntrants ?? 0,
    specHash: bytes32(p.specHash ?? new Uint8Array(32)),
    featuredMint: p.featuredMint ?? PublicKey.default,
    hostShareBps: p.hostShareBps ?? 0,
});
const RACE = { track: 0, format: 0, laps: 3, ai: 0, time: 0, sky: 0, startMode: 0 };
/**
 * Builds arena instructions and reads arena accounts. Every method returns
 * instructions; signing and sending stay with the caller (the race server's
 * keys, or the player's wallet). Token rooms and cups need their escrow and
 * payout token accounts, which the methods create idempotently.
 */
export class ArenaClient {
    connection;
    program;
    programId;
    constructor(connection, programId = ARENA_PROGRAM_ID) {
        this.connection = connection;
        this.programId = programId;
        this.program = new Program({ ...arenaIdl, address: programId.toBase58() }, { connection });
    }
    // ---- reads ----------------------------------------------------------------
    pda = {
        config: () => configPda(this.programId),
        ruleset: (id) => rulesetPda(id, this.programId),
        allowedMint: (mint) => allowedMintPda(mint, this.programId),
        room: (code) => roomPda(code, this.programId),
        cup: (id) => cupPda(id, this.programId),
        entry: (cup, player) => entryPda(cup, player, this.programId),
        host: (wallet) => hostPda(wallet, this.programId),
    };
    fetchConfig() {
        return this.program.account.config.fetch(this.pda.config());
    }
    fetchRoom(room) {
        return this.program.account.room.fetch(room);
    }
    fetchRoomByCode(code) {
        return this.program.account.room.fetchNullable(this.pda.room(code));
    }
    fetchCup(cup) {
        return this.program.account.cup.fetch(cup);
    }
    fetchEntry(cup, player) {
        return this.program.account.entry.fetchNullable(this.pda.entry(cup, player));
    }
    fetchHost(wallet) {
        return this.program.account.host.fetchNullable(this.pda.host(wallet));
    }
    fetchAllowedMint(mint) {
        return this.program.account.allowedMint.fetchNullable(this.pda.allowedMint(mint));
    }
    fetchAllowedMints() {
        return this.program.account.allowedMint.all();
    }
    fetchRuleset(id) {
        return this.program.account.ruleset.fetch(this.pda.ruleset(id));
    }
    fetchRulesets() {
        return this.program.account.ruleset.all();
    }
    /** Rooms, filtered on-chain by state and token (the lobby's view). */
    fetchRooms(f = {}) {
        const filters = [];
        if (f.state)
            filters.push({ memcmp: { offset: ROOM_OFFSETS.state, bytes: utils.bytes.bs58.encode([ROOM_STATE[f.state]]) } });
        if (f.mint)
            filters.push({ memcmp: { offset: ROOM_OFFSETS.mint, bytes: f.mint.toBase58() } });
        return this.program.account.room.all(filters);
    }
    async fetchCups(state) {
        const all = await this.program.account.cup.all();
        return state ? all.filter((c) => stateName(c.account.state) === state) : all;
    }
    /** Decode arena events from a transaction's log messages. */
    parseEvents(logs) {
        const parser = new EventParser(this.programId, new BorshCoder(this.program.idl));
        return [...parser.parseLogs(logs)];
    }
    // ---- token plumbing ---------------------------------------------------------
    tokenAccounts(owner, mint, tokenProgram) {
        if (mint.equals(NATIVE_SOL))
            return { mint: null, escrowTokens: null, tokenProgram: null };
        return { mint, escrowTokens: ata(owner, mint, tokenProgram), tokenProgram };
    }
    /** Where a payment to `owner` goes: the wallet (SOL) or its ATA (SPL). */
    payoutAccount(owner, mint, tokenProgram) {
        return mint.equals(NATIVE_SOL) ? owner : ata(owner, mint, tokenProgram);
    }
    /**
     * Creations for the token accounts among `owners`' that don't exist yet, so
     * every token payment has somewhere to land (only the missing ones: eight
     * idempotent creations would not fit in one transaction beside a settlement).
     */
    async ensureAtas(payer, owners, mint, tokenProgram) {
        if (mint.equals(NATIVE_SOL))
            return [];
        const unique = [...new Map(owners.map((o) => [o.toBase58(), o])).values()];
        const atas = unique.map((o) => ata(o, mint, tokenProgram));
        const infos = await this.connection.getMultipleAccountsInfo(atas, "confirmed");
        return unique.flatMap((o, i) => infos[i] ? [] : [createAssociatedTokenAccountIdempotentInstruction(payer, atas[i], o, mint, tokenProgram)]);
    }
    /** The payout accounts as remaining accounts; for a Token-2022 coin the memo program after them (an account whose
     *  owner turned on required memos is paid after a memo). */
    payouts(owners, mint, tokenProgram) {
        return [
            ...owners.map((p) => ({ pubkey: this.payoutAccount(p, mint, tokenProgram), isSigner: false, isWritable: true })),
            ...this.memo(tokenProgram),
        ];
    }
    /** The memo program as the last remaining account, for payouts in a Token-2022 coin. */
    memo(tokenProgram) {
        return tokenProgram.equals(TOKEN_2022_ID) ? [{ pubkey: MEMO_PROGRAM_ID, isSigner: false, isWritable: false }] : [];
    }
    async stakeToken(mint) {
        if (mint.equals(NATIVE_SOL))
            return { mint, tokenProgram: SystemProgram.programId };
        const m = await this.fetchAllowedMint(mint);
        if (!m)
            throw new Error(`token ${mint.toBase58()} is not enabled in the arena`);
        return { mint, tokenProgram: m.tokenProgram };
    }
    /** The project token account that claims the holder fee for `owner` (null: no project token yet). */
    async holderAccount(owner) {
        const cfg = await this.fetchConfig();
        if (cfg.holderMint.equals(PublicKey.default))
            return null;
        const info = await this.connection.getAccountInfo(cfg.holderMint, "confirmed");
        return info ? ata(owner, cfg.holderMint, info.owner) : null;
    }
    // ---- rooms ------------------------------------------------------------------
    /**
     * Open a room and take the first seat. `code` is the race server's room id;
     * `payer` pays the room's rent and gets it back at close (the server's
     * sponsor, or the creator). GARAGE rooms snapshot each player's levels from
     * the house's upgrades account; `holderTokens` claims the holder fee.
     */
    async createRoom(p) {
        const payer = p.payer ?? p.creator;
        const w = await this.stakeToken(p.mint ?? NATIVE_SOL);
        const room = this.pda.room(p.code);
        const t = this.tokenAccounts(room, w.mint, w.tokenProgram);
        const cls = p.class ?? 0;
        const ix = await this.program.methods
            .createRoom(Array.from(roomCode(p.code)), {
            stake: bn(p.stake),
            maxPlayers: p.maxPlayers ?? 2,
            joinWindowSecs: p.joinWindowSecs ?? 900,
            class: cls,
            maxRating: p.maxRating ?? 0,
            access: p.access ?? 0,
            race: { ...RACE, ...p.race },
        })
            .accountsStrict({
            creator: p.creator,
            payer,
            gate: p.gate ?? null,
            config: this.pda.config(),
            ruleset: this.pda.ruleset(p.ruleset ?? 1),
            allowedMint: this.pda.allowedMint(w.mint),
            room,
            upgrades: cls === CLASS_GARAGE ? upgradesPda(p.creator) : null,
            holderTokens: p.holderTokens ?? null,
            ...t,
            creatorTokens: t.mint ? ata(p.creator, w.mint, w.tokenProgram) : null,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        return { ixs: [...(await this.ensureAtas(payer, [room], w.mint, w.tokenProgram)), ix], room };
    }
    /**
     * A player's seat. `expectedStake`: the stake the player was shown (the join
     * fails if the room's isn't that: a room opened again under its code with
     * another stake can't take theirs); the room's own when not given.
     */
    async joinRoom(p) {
        const r = p.roomAccount ?? (await this.fetchRoom(p.room));
        const t = this.tokenAccounts(p.room, r.mint, r.tokenProgram);
        const ix = await this.program.methods
            .joinRoom(bn(p.expectedStake ?? r.stake))
            .accountsStrict({
            player: p.player,
            gate: p.gate ?? null,
            config: this.pda.config(),
            room: p.room,
            ruleset: r.ruleset,
            upgrades: r.class === CLASS_GARAGE ? upgradesPda(p.player) : null,
            ...t,
            playerTokens: t.mint ? ata(p.player, r.mint, r.tokenProgram) : null,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        return [ix];
    }
    async leaveRoom(p) {
        const r = await this.fetchRoom(p.room);
        const t = this.tokenAccounts(p.room, r.mint, r.tokenProgram);
        const ix = await this.program.methods
            .leaveRoom()
            .accountsStrict({
            player: p.player,
            room: p.room,
            ...t,
            playerTokens: t.mint ? ata(p.player, r.mint, r.tokenProgram) : null,
        })
            .remainingAccounts(this.memo(r.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.player, [p.player], r.mint, r.tokenProgram)), ix];
    }
    payoutGroup(room, r) {
        return { room, ...this.tokenAccounts(room, r.mint, r.tokenProgram) };
    }
    /** The creator, while the room is open: every stake back. */
    async cancelRoom(p) {
        const r = await this.fetchRoom(p.room);
        const ix = await this.program.methods
            .cancelRoom()
            .accountsStrict({ creator: p.creator, payout: this.payoutGroup(p.room, r) })
            .remainingAccounts(this.payouts(r.players, r.mint, r.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.creator, r.players, r.mint, r.tokenProgram)), ix];
    }
    /** Resolver: a room its creator starts (two players or more, before its join deadline) locks; its seed comes next. */
    async lockRoom(p) {
        const ix = await this.program.methods
            .lockRoom()
            .accountsStrict({ resolver: p.resolver, config: this.pda.config(), room: p.room })
            .instruction();
        return [ix];
    }
    /**
     * Anyone, once the slot a room locked in is over (within ~512 slots): its seed,
     * from that slot's hash - which nobody could see while the locking transaction ran.
     */
    async drawSeed(p) {
        const ix = await this.program.methods
            .drawSeed()
            .accountsStrict({ room: p.room, slotHashes: SYSVAR_SLOT_HASHES_PUBKEY })
            .instruction();
        return [ix];
    }
    /**
     * Resolver: the race's result (one time per seat, DNF for none) and the hash
     * of its full record. Pays the winners in this transaction; only their
     * accounts are passed (every player's when it comes out as a refund).
     */
    async settleRoom(p) {
        const r = p.roomAccount ?? (await this.fetchRoom(p.room));
        const o = roomOutcome(BigInt(r.stake.toString()), r.feeBps, p.timesMs);
        const payees = o.refundAll ? r.players : winnerSeats(o, r.players.length).map((i) => r.players[i]);
        const ix = await this.program.methods
            .settleRoom(p.timesMs, bytes32(p.recordHash))
            .accountsStrict({
            resolver: p.resolver,
            config: this.pda.config(),
            payout: this.payoutGroup(p.room, r),
            treasury: this.payoutAccount(r.treasury, r.mint, r.tokenProgram),
        })
            .remainingAccounts(this.payouts(payees, r.mint, r.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer ?? p.resolver, [...payees, r.treasury], r.mint, r.tokenProgram)), ix];
    }
    /** Resolver: no result (the server stopped, too few on the grid): every stake back. */
    async voidRoom(p) {
        const r = await this.fetchRoom(p.room);
        const ix = await this.program.methods
            .voidRoom(p.reason, bytes32(p.recordHash ?? new Uint8Array(32)))
            .accountsStrict({ resolver: p.resolver, config: this.pda.config(), payout: this.payoutGroup(p.room, r) })
            .remainingAccounts(this.payouts(r.players, r.mint, r.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer ?? p.resolver, r.players, r.mint, r.tokenProgram)), ix];
    }
    /** Anyone, after the join or settle deadline: every stake back, no fee. */
    async refundRoom(p) {
        const r = await this.fetchRoom(p.room);
        const ix = await this.program.methods
            .refundRoom()
            .accountsStrict({ payout: this.payoutGroup(p.room, r) })
            .remainingAccounts(this.payouts(r.players, r.mint, r.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer, r.players, r.mint, r.tokenProgram)), ix];
    }
    /** Anyone, once settled or refunded: rent back to its payer, any surplus to the prize vault. */
    async closeRoom(p) {
        const r = await this.fetchRoom(p.room);
        const sol = r.mint.equals(NATIVE_SOL);
        const ix = await this.program.methods
            .closeRoom()
            .accountsStrict({
            room: p.room,
            rentPayer: r.rentPayer,
            prizeVault: r.prizeVault,
            prizeVaultTokens: sol ? null : ata(r.prizeVault, r.mint, r.tokenProgram),
            ...this.tokenAccounts(p.room, r.mint, r.tokenProgram),
        })
            .instruction();
        return [...(await this.ensureAtas(p.payer, [r.prizeVault], r.mint, r.tokenProgram)), ix];
    }
    // ---- cups -------------------------------------------------------------------
    /**
     * The house (admin, a resolver, the prize vault through the house program) or
     * a community host (`host`: an approved one; `holderTokens`: holding enough
     * of the project token). `id`: the cup's (a random one by default: nobody can take it first).
     */
    async createCup(p) {
        const payer = p.payer ?? p.organizer;
        const id = p.id ?? randomCupId();
        const w = await this.stakeToken(p.mint ?? NATIVE_SOL);
        const cup = this.pda.cup(id);
        const t = this.tokenAccounts(cup, w.mint, w.tokenProgram);
        const ix = await this.program.methods
            .createCup(bn(id), cupParams(p))
            .accountsStrict({
            organizer: p.organizer,
            payer,
            config: this.pda.config(),
            ruleset: this.pda.ruleset(p.ruleset ?? 1),
            allowedMint: this.pda.allowedMint(w.mint),
            cup,
            host: p.host ? this.pda.host(p.organizer) : null,
            holderTokens: p.holderTokens ?? null,
            ...t,
            organizerTokens: t.mint ? ata(p.organizer, w.mint, w.tokenProgram) : null,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        // (the organizer's token account too: the guarantee is deposited from it, even when it is nothing)
        return { ixs: [...(await this.ensureAtas(payer, [cup, p.organizer], w.mint, w.tokenProgram)), ix], cup, id };
    }
    /** `payer` pays the entry's rent and gets it back (the server's sponsor, or the player). */
    async enterCup(p) {
        const c = await this.fetchCup(p.cup);
        const t = this.tokenAccounts(p.cup, c.mint, c.tokenProgram);
        const ix = await this.program.methods
            .enterCup()
            .accountsStrict({
            player: p.player,
            payer: p.payer ?? p.player,
            gate: p.gate ?? null,
            config: this.pda.config(),
            cup: p.cup,
            entry: this.pda.entry(p.cup, p.player),
            ...t,
            playerTokens: t.mint ? ata(p.player, c.mint, c.tokenProgram) : null,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        return [ix];
    }
    /**
     * Resolver, after `endsAt`. Build `resultsRoot` with `buildResultsTree` over the paid places; `places` is how
     * many leaves it has, and must be what the cup pays: min(its table's length, scorers).
     */
    async postResults(p) {
        const ix = await this.program.methods
            .postResults(bytes32(p.resultsRoot), p.scorers, p.places, bytes32(p.standingsHash), p.resultsUri)
            .accountsStrict({ resolver: p.resolver, config: this.pda.config(), cup: p.cup })
            .instruction();
        return [ix];
    }
    /** Guardian or admin, during the challenge window. */
    async rejectResults(p) {
        const ix = await this.program.methods
            .rejectResults()
            .accountsStrict({ authority: p.authority, config: this.pda.config(), cup: p.cup })
            .instruction();
        return [ix];
    }
    /** Admin, guardian or a resolver, before finalization. */
    async cancelCup(p) {
        const ix = await this.program.methods
            .cancelCup()
            .accountsStrict({ authority: p.authority, config: this.pda.config(), cup: p.cup })
            .instruction();
        return [ix];
    }
    /** Anyone, after the challenge window: the pool is fixed and claims open. */
    async finalizeCup(p) {
        return [await this.program.methods.finalizeCup().accountsStrict({ cup: p.cup }).instruction()];
    }
    vault(cup, c) {
        return { cup, ...this.tokenAccounts(cup, c.mint, c.tokenProgram) };
    }
    /**
     * Pays rank `rank` to `wallet`. Until `claimUntil` only the winner may sign
     * (and may swap the prize into the featured stock in the same transaction);
     * after it anyone pushes it to them.
     */
    async claimPrize(p) {
        const c = p.cupAccount ?? (await this.fetchCup(p.cup));
        const ix = await this.program.methods
            .claimPrize(p.rank, p.wallet, p.proof.map(bytes32))
            .accountsStrict({
            claimer: p.claimer,
            vault: this.vault(p.cup, c),
            entry: stateName(c.kind) === "entries" ? this.pda.entry(p.cup, p.wallet) : null,
            destination: this.payoutAccount(p.wallet, c.mint, c.tokenProgram),
        })
            .remainingAccounts(this.memo(c.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer ?? p.claimer, [p.wallet], c.mint, c.tokenProgram)), ix];
    }
    /** Anyone: a cancelled cup, or one with no results by its deadline, returns an entry fee. */
    async refundEntry(p) {
        const c = await this.fetchCup(p.cup);
        const ix = await this.program.methods
            .refundEntry()
            .accountsStrict({
            vault: this.vault(p.cup, c),
            entry: this.pda.entry(p.cup, p.player),
            destination: this.payoutAccount(p.player, c.mint, c.tokenProgram),
        })
            .remainingAccounts(this.memo(c.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer, [p.player], c.mint, c.tokenProgram)), ix];
    }
    async returnGuarantee(p) {
        const c = await this.fetchCup(p.cup);
        const ix = await this.program.methods
            .returnGuarantee()
            .accountsStrict({ vault: this.vault(p.cup, c), destination: this.payoutAccount(c.sponsor, c.mint, c.tokenProgram) })
            .remainingAccounts(this.memo(c.tokenProgram))
            .instruction();
        return [...(await this.ensureAtas(p.payer, [c.sponsor], c.mint, c.tokenProgram)), ix];
    }
    /** Anyone, once nothing is owed: the unused guarantee to the sponsor, the fee to the treasury (a host's share to the host). */
    async closeCup(p) {
        const c = await this.fetchCup(p.cup);
        const hosted = !c.host.equals(PublicKey.default);
        const sol = c.mint.equals(NATIVE_SOL);
        const ix = await this.program.methods
            .closeCup()
            .accountsStrict({
            cup: p.cup,
            rentPayer: c.rentPayer,
            sponsorDestination: this.payoutAccount(c.sponsor, c.mint, c.tokenProgram),
            hostDestination: hosted ? this.payoutAccount(c.host, c.mint, c.tokenProgram) : null,
            treasuryDestination: this.payoutAccount(c.treasury, c.mint, c.tokenProgram),
            prizeVault: c.prizeVault,
            prizeVaultTokens: sol ? null : ata(c.prizeVault, c.mint, c.tokenProgram),
            ...this.tokenAccounts(p.cup, c.mint, c.tokenProgram),
        })
            .remainingAccounts(this.memo(c.tokenProgram))
            .instruction();
        const owners = [c.sponsor, c.treasury, c.prizeVault, ...(hosted ? [c.host] : [])];
        return [...(await this.ensureAtas(p.payer, owners, c.mint, c.tokenProgram)), ix];
    }
    /** Anyone: an entry's rent back to whoever paid it, once it can't be needed. */
    async closeEntry(p) {
        const entry = this.pda.entry(p.cup, p.player);
        const e = await this.program.account.entry.fetch(entry);
        const ix = await this.program.methods
            .closeEntry()
            .accountsStrict({ entry, rentPayer: e.rentPayer, cup: p.cup })
            .instruction();
        return [ix];
    }
    // ---- admin ------------------------------------------------------------------
    /** The upgrade authority, once after deploy. */
    async initialize(p) {
        const programData = PublicKey.findProgramAddressSync([this.programId.toBytes()], new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"))[0];
        const ix = await this.program.methods
            .initialize(p.params, p.resolvers)
            .accountsStrict({
            admin: p.admin,
            config: this.pda.config(),
            program: this.programId,
            programData,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        return [ix];
    }
    adminCfg(admin) {
        return { admin, config: this.pda.config() };
    }
    /** The windows of future rooms and cups (each freezes its own). */
    async updateConfig(p) {
        return [await this.program.methods.updateConfig(p.windows).accountsStrict(this.adminCfg(p.admin)).instruction()];
    }
    /** A new guardian, gate and resolver set, in force two days later (`applyRoles`, by anyone). */
    async proposeRoles(p) {
        return [
            await this.program.methods.proposeRoles(p.guardian, p.gate, p.resolvers).accountsStrict(this.adminCfg(p.admin)).instruction(),
        ];
    }
    async applyRoles() {
        return [await this.program.methods.applyRoles().accountsStrict({ config: this.pda.config() }).instruction()];
    }
    /** The project token (once) and the holder thresholds, in its base units. */
    async setHolder(p) {
        return [
            await this.program.methods
                .setHolder(p.holderMint, bn(p.holderMin), p.holderRoomFeeBps, bn(p.hostMin))
                .accountsStrict(this.adminCfg(p.admin))
                .instruction(),
        ];
    }
    async proposeAdmin(p) {
        return [await this.program.methods.proposeAdmin(p.newAdmin).accountsStrict(this.adminCfg(p.admin)).instruction()];
    }
    async acceptAdmin(p) {
        return [
            await this.program.methods
                .acceptAdmin()
                .accountsStrict({ newAdmin: p.newAdmin, config: this.pda.config() })
                .instruction(),
        ];
    }
    /** Admin or guardian may pause; only the admin unpauses. */
    async setPaused(p) {
        return [
            await this.program.methods
                .setPaused(p.paused)
                .accountsStrict({ authority: p.authority, config: this.pda.config() })
                .instruction(),
        ];
    }
    /** Guardian or admin: every resolver key off at once (refunds by deadline still work). */
    async revokeResolvers(p) {
        return [
            await this.program.methods
                .revokeResolvers()
                .accountsStrict({ authority: p.authority, config: this.pda.config() })
                .instruction(),
        ];
    }
    /** Lets `wallet` organize community cups until `until` (0: no longer). */
    async setHost(p) {
        return [
            await this.program.methods
                .setHost(p.wallet, bn(p.until))
                .accountsStrict({
                ...this.adminCfg(p.admin),
                host: this.pda.host(p.wallet),
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    /** A ruleset: the game build's hash and its rating points per department level (tenths). */
    async addRuleset(p) {
        return [
            await this.program.methods
                .addRuleset(p.id, bytes32(p.hash), p.ratingPoints)
                .accountsStrict({
                ...this.adminCfg(p.admin),
                ruleset: this.pda.ruleset(p.id),
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    async updateRuleset(p) {
        return [
            await this.program.methods
                .updateRuleset(p.enabled)
                .accountsStrict({ ...this.adminCfg(p.admin), ruleset: this.pda.ruleset(p.id) })
                .instruction(),
        ];
    }
    /** Enable a token (`NATIVE_SOL` for SOL) for rooms and cups. */
    async addMint(p) {
        const sol = p.mint.equals(NATIVE_SOL);
        return [
            await this.program.methods
                .addMint(p.mint, mintParams(p))
                .accountsStrict({
                ...this.adminCfg(p.admin),
                allowedMint: this.pda.allowedMint(p.mint),
                mint: sol ? null : p.mint,
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    async updateMint(p) {
        return [
            await this.program.methods
                .updateMint(mintParams(p), p.enabled)
                .accountsStrict({ ...this.adminCfg(p.admin), allowedMint: this.pda.allowedMint(p.mint) })
                .instruction(),
        ];
    }
}

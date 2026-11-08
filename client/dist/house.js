import { BN, BorshCoder, EventParser, Program } from "./anchor.js";
import { createAssociatedTokenAccountIdempotentInstruction } from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { ArenaClient, bn, cupParams, randomCupId } from "./arena.js";
import { ARENA_PROGRAM_ID, DEPARTMENTS, HOUSE_PROGRAM_ID, MAX_LEVEL, NATIVE_SOL } from "./constants.js";
import { houseIdl } from "./idl/house.js";
import { allowedMintPda, ata, auctionPda, buybackPda, configPda, cupPda, housePda, pendingTokenPda, pricePda, prizeVaultPda, rulesetPda, totalsPda, treasuryPda, upgradesPda, } from "./pda.js";
/** The whole lot's price in the project token at `now` (unix seconds), as the house computes it: falling evenly from
 *  the start price to the floor over the decline, held at the floor after (rounded up: the house's side). */
export function auctionPrice(a, now) {
    const start = BigInt(a.startPrice.toString()), floor = BigInt(a.floorPrice.toString());
    const elapsed = BigInt(Math.max(0, now - Number(a.startAt.toString())));
    const decline = BigInt(a.declineSecs);
    if (decline === 0n || elapsed >= decline)
        return floor;
    return start - ((start - floor) * elapsed) / decline;
}
/** An auction with something for sale at `now`. */
export const auctionRunning = (a, now) => BigInt(a.lot.toString()) > 0n && now < Number(a.endAt.toString());
/**
 * Builds house instructions and reads its accounts: the treasury's split, the
 * prize vault's funded cups and the upgrade shop. Returns instructions only.
 */
export class HouseClient {
    connection;
    arenaProgramId;
    program;
    programId;
    /** The arena the house funds cups in (its config gives the next cup's id). */
    arena;
    constructor(connection, programId = HOUSE_PROGRAM_ID, arenaProgramId = ARENA_PROGRAM_ID) {
        this.connection = connection;
        this.arenaProgramId = arenaProgramId;
        this.programId = programId;
        this.program = new Program({ ...houseIdl, address: programId.toBase58() }, { connection });
        this.arena = new ArenaClient(connection, arenaProgramId);
    }
    pda = {
        house: () => housePda(this.programId),
        treasury: () => treasuryPda(this.programId),
        prizeVault: () => prizeVaultPda(this.programId),
        buyback: () => buybackPda(this.programId),
        price: (mint) => pricePda(mint, this.programId),
        totals: (mint) => totalsPda(mint, this.programId),
        upgrades: (owner) => upgradesPda(owner, this.programId),
        pendingToken: () => pendingTokenPda(this.programId),
        auction: (lotMint) => auctionPda(lotMint, this.programId),
    };
    // ---- reads ----------------------------------------------------------------
    fetchHouse() {
        return this.program.account.houseConfig.fetch(this.pda.house());
    }
    fetchPriceTable(mint) {
        return this.program.account.priceTable.fetchNullable(this.pda.price(mint));
    }
    fetchPriceTables() {
        return this.program.account.priceTable.all();
    }
    fetchTotals(mint) {
        return this.program.account.totals.fetchNullable(this.pda.totals(mint));
    }
    fetchAllTotals() {
        return this.program.account.totals.all();
    }
    /** The project token proposed and when it can be applied (null: none proposed). */
    fetchPendingToken() {
        return this.program.account.pendingToken.fetchNullable(this.pda.pendingToken());
    }
    /** The buyback auction of `lotMint` (`NATIVE_SOL`: SOL), null before its first. */
    fetchAuction(lotMint) {
        return this.program.account.auction.fetchNullable(this.pda.auction(lotMint));
    }
    /** A wallet's six levels (all 0 before its first purchase: the stock car). */
    async levelsOf(owner) {
        const u = await this.program.account.upgrades.fetchNullable(this.pda.upgrades(owner));
        return u ? u.levels.slice(0, DEPARTMENTS.length) : DEPARTMENTS.map(() => 0);
    }
    parseEvents(logs) {
        const parser = new EventParser(this.programId, new BorshCoder(this.program.idl));
        return [...parser.parseLogs(logs)];
    }
    async tokenProgramOf(mint) {
        const t = await this.fetchPriceTable(mint);
        if (!t)
            throw new Error(`token ${mint.toBase58()} is not added to the house`);
        return t.tokenProgram;
    }
    /** Creations for the missing token accounts among `owners`'. */
    async ensureAtas(payer, owners, mint, tokenProgram) {
        const unique = [...new Map(owners.map((o) => [o.toBase58(), o])).values()];
        const atas = unique.map((o) => ata(o, mint, tokenProgram));
        const infos = await this.connection.getMultipleAccountsInfo(atas, "confirmed");
        return unique.flatMap((o, i) => infos[i] ? [] : [createAssociatedTokenAccountIdempotentInstruction(payer, atas[i], o, mint, tokenProgram)]);
    }
    // ---- money ----------------------------------------------------------------
    /**
     * Anyone: split what the treasury holds in `mint` (default SOL) by the split.
     * The project token's buyback share is burned; other tokens' builds up in the
     * buyback vault once the project token exists (before that it goes to prizes).
     */
    async sweep(p) {
        const h = await this.fetchHouse();
        const mint = p.mint ?? NATIVE_SOL;
        const base = {
            house: this.pda.house(),
            treasury: this.pda.treasury(),
            prize: this.pda.prizeVault(),
            buyback: this.pda.buyback(),
            opsWallet: h.opsWallet,
            totals: this.pda.totals(mint),
            systemProgram: SystemProgram.programId,
        };
        if (mint.equals(NATIVE_SOL)) {
            const ix = await this.program.methods
                .sweep(mint)
                .accountsStrict({
                ...base,
                mint: null,
                treasuryTokens: null,
                opsTokens: null,
                prizeTokens: null,
                buybackTokens: null,
                tokenProgram: null,
            })
                .instruction();
            return [ix];
        }
        const program = await this.tokenProgramOf(mint);
        const hasToken = !h.tokenMint.equals(PublicKey.default);
        const buybackShare = hasToken && !mint.equals(h.tokenMint) && h.split.buybackBps > 0;
        const owners = [h.opsWallet, this.pda.prizeVault(), ...(buybackShare ? [this.pda.buyback()] : [])];
        const ix = await this.program.methods
            .sweep(mint)
            .accountsStrict({
            ...base,
            mint,
            treasuryTokens: ata(this.pda.treasury(), mint, program),
            opsTokens: ata(h.opsWallet, mint, program),
            prizeTokens: ata(this.pda.prizeVault(), mint, program),
            buybackTokens: buybackShare ? ata(this.pda.buyback(), mint, program) : null,
            tokenProgram: program,
        })
            .instruction();
        return [...(await this.ensureAtas(p.payer, owners, mint, program)), ix];
    }
    /**
     * The keeper funds a cup (the weekly time trial) from the prize vault: the
     * cup is arena's, organized and sponsored by the vault, so what isn't won
     * comes back to it. Capped on-chain by share of the vault and frequency.
     */
    async fundCup(p) {
        const mint = p.mint ?? NATIVE_SOL;
        const id = p.id ?? randomCupId();
        const cup = cupPda(id, this.arenaProgramId);
        const sol = mint.equals(NATIVE_SOL);
        const program = sol ? null : await this.tokenProgramOf(mint);
        const ix = await this.program.methods
            .fundCup(new BN(id.toString()), cupParams(p))
            .accountsStrict({
            keeper: p.keeper,
            house: this.pda.house(),
            prize: this.pda.prizeVault(),
            totals: this.pda.totals(mint),
            arenaConfig: configPda(this.arenaProgramId),
            ruleset: rulesetPda(p.ruleset ?? 1, this.arenaProgramId),
            allowedMint: allowedMintPda(mint, this.arenaProgramId),
            cup,
            mint: sol ? null : mint,
            escrowTokens: program ? ata(cup, mint, program) : null,
            prizeTokens: program ? ata(this.pda.prizeVault(), mint, program) : null,
            tokenProgram: program,
            arenaProgram: this.arenaProgramId,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        const pre = program ? await this.ensureAtas(p.keeper, [cup], mint, program) : [];
        return { ixs: [...pre, ix], cup, id };
    }
    // ---- the buyback auction -------------------------------------------------------
    /**
     * The keeper: `lot` of `lotMint` (`NATIVE_SOL`: SOL) from the buyback vault, for the project token - `startPrice` for
     * the whole lot now, falling evenly to `floorPrice` over `declineSecs`, held there until `runSecs` from now. At most a
     * quarter of the vault, a day after the last one started, its floor's rate at least the admin's lowest
     * (`setAuctionFloor`) and, for a week after a sale, half that sale's.
     */
    async startAuction(p) {
        const sol = p.lotMint.equals(NATIVE_SOL);
        const program = sol ? null : await this.tokenProgramOf(p.lotMint);
        const ix = await this.program.methods
            .startAuction(p.lotMint, bn(p.lot), bn(p.startPrice), bn(p.floorPrice), p.declineSecs, p.runSecs)
            .accountsStrict({
            keeper: p.keeper,
            house: this.pda.house(),
            auction: this.pda.auction(p.lotMint),
            buyback: this.pda.buyback(),
            buybackTokens: program ? ata(this.pda.buyback(), p.lotMint, program) : null,
        })
            .instruction();
        return [ix];
    }
    /** The admin: the lowest rate an auction of `lotMint` (`NATIVE_SOL`: SOL) may sell at - `minPrice` of the project
     *  token for `minLot` of it. No auction of a coin starts before it is set; the keeper's floors never go below it. */
    async setAuctionFloor(p) {
        return [
            await this.program.methods
                .setAuctionFloor(p.lotMint, bn(p.minLot), bn(p.minPrice))
                .accountsStrict({
                admin: p.admin,
                house: this.pda.house(),
                auction: this.pda.auction(p.lotMint),
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    /**
     * Anyone: the running lot of `lotMint` at its price now - at most `maxPrice` of the project token, for a lot of at
     * least `minLot` - burned from the buyer's own account; the lot paid to them. `payer` makes the buyer's account for
     * a token lot if they have none (the server's sponsor, or the buyer).
     */
    async buyLot(p) {
        const h = await this.fetchHouse();
        if (h.tokenMint.equals(PublicKey.default))
            throw new Error("the house has no project token yet");
        const tokenProgram = await this.tokenProgramOf(h.tokenMint);
        const sol = p.lotMint.equals(NATIVE_SOL);
        const lotProgram = sol ? null : await this.tokenProgramOf(p.lotMint);
        const ix = await this.program.methods
            .buyLot(bn(p.maxPrice), bn(p.minLot))
            .accountsStrict({
            buyer: p.buyer,
            house: this.pda.house(),
            auction: this.pda.auction(p.lotMint),
            tokenTotals: this.pda.totals(h.tokenMint),
            tokenMint: h.tokenMint,
            buyerTokens: ata(p.buyer, h.tokenMint, tokenProgram),
            tokenProgram,
            buyback: this.pda.buyback(),
            lotMint: sol ? null : p.lotMint,
            buybackTokens: lotProgram ? ata(this.pda.buyback(), p.lotMint, lotProgram) : null,
            buyerLotTokens: lotProgram ? ata(p.buyer, p.lotMint, lotProgram) : null,
            lotTokenProgram: lotProgram,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        const pre = lotProgram ? await this.ensureAtas(p.payer ?? p.buyer, [p.buyer], p.lotMint, lotProgram) : [];
        return [...pre, ix];
    }
    // ---- the shop ---------------------------------------------------------------
    /**
     * Buys the next level of `dept` (0-5) at the listed price, at most `maxPrice`
     * (a quote's guard). `payer` pays the upgrades account's rent on a first
     * purchase (the server's sponsor, or the owner).
     */
    async buyUpgrade(p) {
        const mint = p.mint ?? NATIVE_SOL;
        const sol = mint.equals(NATIVE_SOL);
        const program = sol ? null : await this.tokenProgramOf(mint);
        const ix = await this.program.methods
            .buyUpgrade(p.dept, p.level, bn(p.maxPrice))
            .accountsStrict({
            owner: p.owner,
            payer: p.payer ?? p.owner,
            house: this.pda.house(),
            priceTable: this.pda.price(mint),
            upgrades: this.pda.upgrades(p.owner),
            treasury: this.pda.treasury(),
            mint: sol ? null : mint,
            ownerTokens: program ? ata(p.owner, mint, program) : null,
            treasuryTokens: program ? ata(this.pda.treasury(), mint, program) : null,
            tokenProgram: program,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        const pre = program ? await this.ensureAtas(p.payer ?? p.owner, [this.pda.treasury()], mint, program) : [];
        return [...pre, ix];
    }
    // ---- admin ------------------------------------------------------------------
    /** The upgrade authority, once after deploy. Gives the three vaults their rent. */
    async initialize(p) {
        const programData = PublicKey.findProgramAddressSync([this.programId.toBytes()], new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"))[0];
        const ix = await this.program.methods
            .initialize({
            keeper: p.keeper,
            opsWallet: p.opsWallet,
            split: p.split,
            fundMaxBps: p.fundMaxBps,
            fundEverySecs: p.fundEverySecs,
        })
            .accountsStrict({
            admin: p.admin,
            house: this.pda.house(),
            treasury: this.pda.treasury(),
            prize: this.pda.prizeVault(),
            buyback: this.pda.buyback(),
            program: this.programId,
            programData,
            systemProgram: SystemProgram.programId,
        })
            .instruction();
        return [ix];
    }
    admin(admin) {
        return { admin, house: this.pda.house() };
    }
    async setRoles(p) {
        return [await this.program.methods.setRoles(p.keeper, p.opsWallet).accountsStrict(this.admin(p.admin)).instruction()];
    }
    async setFundCaps(p) {
        return [
            await this.program.methods.setFundCaps(p.fundMaxBps, p.fundEverySecs).accountsStrict(this.admin(p.admin)).instruction(),
        ];
    }
    /** A new split, in force a week later (anyone applies it then). */
    async proposeSplit(p) {
        return [await this.program.methods.proposeSplit(p.split).accountsStrict(this.admin(p.admin)).instruction()];
    }
    async applySplit() {
        return [await this.program.methods.applySplit().accountsStrict({ house: this.pda.house() }).instruction()];
    }
    /** The project token, proposed (checked as the arena checks a mint, its fee within 5 %): anyone applies it once
     *  `TOKEN_TIMELOCK_SECS` have passed. Only while the house has none: it is set once. */
    async proposeTokenMint(p) {
        return [
            await this.program.methods
                .proposeTokenMint()
                .accountsStrict({
                admin: p.admin,
                house: this.pda.house(),
                pending: this.pda.pendingToken(),
                mint: p.tokenMint,
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    /** Anyone, once the proposal's timelock has passed: the project token takes effect. */
    async applyTokenMint(p) {
        return [
            await this.program.methods
                .applyTokenMint()
                .accountsStrict({ house: this.pda.house(), pending: this.pda.pendingToken(), mint: p.tokenMint })
                .instruction(),
        ];
    }
    /** Looser funding caps, once their timelock has passed (anyone). */
    async applyFundCaps() {
        return [await this.program.methods.applyFundCaps().accountsStrict({ house: this.pda.house() }).instruction()];
    }
    async setPaused(p) {
        return [await this.program.methods.setPaused(p.paused).accountsStrict(this.admin(p.admin)).instruction()];
    }
    async proposeAdmin(p) {
        return [await this.program.methods.proposeAdmin(p.newAdmin).accountsStrict(this.admin(p.admin)).instruction()];
    }
    async acceptAdmin(p) {
        return [
            await this.program.methods.acceptAdmin().accountsStrict({ newAdmin: p.newAdmin, house: this.pda.house() }).instruction(),
        ];
    }
    /** A token the house sells in and keeps totals for (`NATIVE_SOL` for SOL). */
    async addMint(p) {
        const sol = p.mint.equals(NATIVE_SOL);
        return [
            await this.program.methods
                .addMint(p.mint)
                .accountsStrict({
                admin: p.admin,
                house: this.pda.house(),
                priceTable: this.pda.price(p.mint),
                totals: this.pda.totals(p.mint),
                mint: sol ? null : p.mint,
                systemProgram: SystemProgram.programId,
            })
                .instruction(),
        ];
    }
    /** Admin or keeper: prices for future sales (6 departments × 5 levels). */
    async setPrices(p) {
        if (p.prices.length !== DEPARTMENTS.length || p.prices.some((r) => r.length !== MAX_LEVEL))
            throw new Error("prices: 6 departments × 5 levels");
        return [
            await this.program.methods
                .setPrices(p.prices.map((r) => r.map(bn)), p.enabled)
                .accountsStrict({ authority: p.authority, house: this.pda.house(), priceTable: this.pda.price(p.mint) })
                .instruction(),
        ];
    }
}

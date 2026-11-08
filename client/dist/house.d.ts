import { Program, type IdlAccounts, type IdlEvents, type IdlTypes } from "./anchor.js";
import { type Connection, PublicKey, type TransactionInstruction } from "@solana/web3.js";
import { type Amount, ArenaClient, type CupInput } from "./arena.js";
import { type House } from "./idl/house.js";
export type HouseConfig = IdlAccounts<House>["houseConfig"];
export type PriceTable = IdlAccounts<House>["priceTable"];
export type Totals = IdlAccounts<House>["totals"];
export type Upgrades = IdlAccounts<House>["upgrades"];
export type PendingToken = IdlAccounts<House>["pendingToken"];
export type Auction = IdlAccounts<House>["auction"];
/** The whole lot's price in the project token at `now` (unix seconds), as the house computes it: falling evenly from
 *  the start price to the floor over the decline, held at the floor after (rounded up: the house's side). */
export declare function auctionPrice(a: Pick<Auction, "startPrice" | "floorPrice" | "startAt" | "declineSecs">, now: number): bigint;
/** An auction with something for sale at `now`. */
export declare const auctionRunning: (a: Pick<Auction, "lot" | "endAt">, now: number) => boolean;
export type Split = IdlTypes<House>["split"];
export type HouseEvent = {
    name: keyof IdlEvents<House>;
    data: IdlEvents<House>[keyof IdlEvents<House>];
};
/** Prices per department (src/game/upgrades.js order) and level 1-5, in base units; 0 = not for sale. */
export type Prices = Amount[][];
/**
 * Builds house instructions and reads its accounts: the treasury's split, the
 * prize vault's funded cups and the upgrade shop. Returns instructions only.
 */
export declare class HouseClient {
    readonly connection: Connection;
    readonly arenaProgramId: PublicKey;
    readonly program: Program<House>;
    readonly programId: PublicKey;
    /** The arena the house funds cups in (its config gives the next cup's id). */
    readonly arena: ArenaClient;
    constructor(connection: Connection, programId?: PublicKey, arenaProgramId?: PublicKey);
    pda: {
        house: () => PublicKey;
        treasury: () => PublicKey;
        prizeVault: () => PublicKey;
        buyback: () => PublicKey;
        price: (mint: PublicKey) => PublicKey;
        totals: (mint: PublicKey) => PublicKey;
        upgrades: (owner: PublicKey) => PublicKey;
        pendingToken: () => PublicKey;
        auction: (lotMint: PublicKey) => PublicKey;
    };
    fetchHouse(): Promise<{
        admin: PublicKey;
        pendingAdmin: PublicKey;
        keeper: PublicKey;
        opsWallet: PublicKey;
        split: {
            opsBps: number;
            prizeBps: number;
            buybackBps: number;
        };
        pendingSplit: {
            opsBps: number;
            prizeBps: number;
            buybackBps: number;
        };
        splitAfter: import("bn.js");
        tokenMint: PublicKey;
        fundMaxBps: number;
        fundEverySecs: number;
        pendingFundMaxBps: number;
        pendingFundEverySecs: number;
        fundCapsAfter: import("bn.js");
        lastFundAt: import("bn.js");
        paused: boolean;
        treasuryBump: number;
        prizeBump: number;
        buybackBump: number;
        bump: number;
    }>;
    fetchPriceTable(mint: PublicKey): Promise<{
        mint: PublicKey;
        tokenProgram: PublicKey;
        decimals: number;
        prices: import("bn.js")[][];
        enabled: boolean;
        updatedAt: import("bn.js");
        bump: number;
    } | null>;
    fetchPriceTables(): Promise<import("@anchor-lang/core").ProgramAccount<{
        mint: PublicKey;
        tokenProgram: PublicKey;
        decimals: number;
        prices: import("bn.js")[][];
        enabled: boolean;
        updatedAt: import("bn.js");
        bump: number;
    }>[]>;
    fetchTotals(mint: PublicKey): Promise<{
        mint: PublicKey;
        sweptOps: import("bn.js");
        sweptPrize: import("bn.js");
        sweptBuyback: import("bn.js");
        burned: import("bn.js");
        funded: import("bn.js");
        bump: number;
    } | null>;
    fetchAllTotals(): Promise<import("@anchor-lang/core").ProgramAccount<{
        mint: PublicKey;
        sweptOps: import("bn.js");
        sweptPrize: import("bn.js");
        sweptBuyback: import("bn.js");
        burned: import("bn.js");
        funded: import("bn.js");
        bump: number;
    }>[]>;
    /** The project token proposed and when it can be applied (null: none proposed). */
    fetchPendingToken(): Promise<{
        mint: PublicKey;
        after: import("bn.js");
        bump: number;
    } | null>;
    /** The buyback auction of `lotMint` (`NATIVE_SOL`: SOL), null before its first. */
    fetchAuction(lotMint: PublicKey): Promise<{
        lotMint: PublicKey;
        lot: import("bn.js");
        startPrice: import("bn.js");
        floorPrice: import("bn.js");
        startAt: import("bn.js");
        declineSecs: number;
        endAt: import("bn.js");
        minLot: import("bn.js");
        minPrice: import("bn.js");
        lastLot: import("bn.js");
        lastPrice: import("bn.js");
        lastSoldAt: import("bn.js");
        sold: number;
        paidOut: import("bn.js");
        burned: import("bn.js");
        bump: number;
    } | null>;
    /** A wallet's six levels (all 0 before its first purchase: the stock car). */
    levelsOf(owner: PublicKey): Promise<number[]>;
    parseEvents(logs: string[]): HouseEvent[];
    private tokenProgramOf;
    /** Creations for the missing token accounts among `owners`'. */
    private ensureAtas;
    /**
     * Anyone: split what the treasury holds in `mint` (default SOL) by the split.
     * The project token's buyback share is burned; other tokens' builds up in the
     * buyback vault once the project token exists (before that it goes to prizes).
     */
    sweep(p: {
        payer: PublicKey;
        mint?: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * The keeper funds a cup (the weekly time trial) from the prize vault: the
     * cup is arena's, organized and sponsored by the vault, so what isn't won
     * comes back to it. Capped on-chain by share of the vault and frequency.
     */
    fundCup(p: CupInput & {
        keeper: PublicKey;
        mint?: PublicKey;
        ruleset?: number;
        id?: bigint;
    }): Promise<{
        ixs: TransactionInstruction[];
        cup: PublicKey;
        id: bigint;
    }>;
    /**
     * The keeper: `lot` of `lotMint` (`NATIVE_SOL`: SOL) from the buyback vault, for the project token - `startPrice` for
     * the whole lot now, falling evenly to `floorPrice` over `declineSecs`, held there until `runSecs` from now. At most a
     * quarter of the vault, a day after the last one started, its floor's rate at least the admin's lowest
     * (`setAuctionFloor`) and, for a week after a sale, half that sale's.
     */
    startAuction(p: {
        keeper: PublicKey;
        lotMint: PublicKey;
        lot: Amount;
        startPrice: Amount;
        floorPrice: Amount;
        declineSecs: number;
        runSecs: number;
    }): Promise<TransactionInstruction[]>;
    /** The admin: the lowest rate an auction of `lotMint` (`NATIVE_SOL`: SOL) may sell at - `minPrice` of the project
     *  token for `minLot` of it. No auction of a coin starts before it is set; the keeper's floors never go below it. */
    setAuctionFloor(p: {
        admin: PublicKey;
        lotMint: PublicKey;
        minLot: Amount;
        minPrice: Amount;
    }): Promise<TransactionInstruction[]>;
    /**
     * Anyone: the running lot of `lotMint` at its price now - at most `maxPrice` of the project token, for a lot of at
     * least `minLot` - burned from the buyer's own account; the lot paid to them. `payer` makes the buyer's account for
     * a token lot if they have none (the server's sponsor, or the buyer).
     */
    buyLot(p: {
        buyer: PublicKey;
        lotMint: PublicKey;
        maxPrice: Amount;
        minLot: Amount;
        payer?: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * Buys the next level of `dept` (0-5) at the listed price, at most `maxPrice`
     * (a quote's guard). `payer` pays the upgrades account's rent on a first
     * purchase (the server's sponsor, or the owner).
     */
    buyUpgrade(p: {
        owner: PublicKey;
        payer?: PublicKey;
        dept: number;
        level: number;
        maxPrice: Amount;
        mint?: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** The upgrade authority, once after deploy. Gives the three vaults their rent. */
    initialize(p: {
        admin: PublicKey;
        keeper: PublicKey;
        opsWallet: PublicKey;
        split: Split;
        fundMaxBps: number;
        fundEverySecs: number;
    }): Promise<TransactionInstruction[]>;
    private admin;
    setRoles(p: {
        admin: PublicKey;
        keeper: PublicKey;
        opsWallet: PublicKey;
    }): Promise<TransactionInstruction[]>;
    setFundCaps(p: {
        admin: PublicKey;
        fundMaxBps: number;
        fundEverySecs: number;
    }): Promise<TransactionInstruction[]>;
    /** A new split, in force a week later (anyone applies it then). */
    proposeSplit(p: {
        admin: PublicKey;
        split: Split;
    }): Promise<TransactionInstruction[]>;
    applySplit(): Promise<TransactionInstruction[]>;
    /** The project token, proposed (checked as the arena checks a mint, its fee within 5 %): anyone applies it once
     *  `TOKEN_TIMELOCK_SECS` have passed. Only while the house has none: it is set once. */
    proposeTokenMint(p: {
        admin: PublicKey;
        tokenMint: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone, once the proposal's timelock has passed: the project token takes effect. */
    applyTokenMint(p: {
        tokenMint: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Looser funding caps, once their timelock has passed (anyone). */
    applyFundCaps(): Promise<TransactionInstruction[]>;
    setPaused(p: {
        admin: PublicKey;
        paused: boolean;
    }): Promise<TransactionInstruction[]>;
    proposeAdmin(p: {
        admin: PublicKey;
        newAdmin: PublicKey;
    }): Promise<TransactionInstruction[]>;
    acceptAdmin(p: {
        newAdmin: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** A token the house sells in and keeps totals for (`NATIVE_SOL` for SOL). */
    addMint(p: {
        admin: PublicKey;
        mint: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Admin or keeper: prices for future sales (6 departments × 5 levels). */
    setPrices(p: {
        authority: PublicKey;
        mint: PublicKey;
        prices: Prices;
        enabled: boolean;
    }): Promise<TransactionInstruction[]>;
}

import { BN, Program, type IdlAccounts, type IdlEvents, type IdlTypes } from "./anchor.js";
import { type AccountMeta, type Connection, PublicKey, type TransactionInstruction } from "@solana/web3.js";
import { type Arena } from "./idl/arena.js";
export type ArenaConfig = IdlAccounts<Arena>["config"];
export type Room = IdlAccounts<Arena>["room"];
export type Cup = IdlAccounts<Arena>["cup"];
export type Entry = IdlAccounts<Arena>["entry"];
export type Host = IdlAccounts<Arena>["host"];
export type AllowedMint = IdlAccounts<Arena>["allowedMint"];
export type Ruleset = IdlAccounts<Arena>["ruleset"];
export type ConfigParams = IdlTypes<Arena>["configParams"];
export type WindowParams = IdlTypes<Arena>["windowParams"];
/** A cup id nobody can guess before it is used (a cup's address is ["cup", id]): 8 random bytes. */
export declare function randomCupId(): bigint;
export type RaceSpec = IdlTypes<Arena>["raceSpec"];
export type ArenaEvent = {
    name: keyof IdlEvents<Arena>;
    data: IdlEvents<Arena>[keyof IdlEvents<Arena>];
};
export type RoomStateName = "open" | "locked" | "settled" | "refunded";
export type CupStateName = "open" | "posted" | "finalized" | "cancelled";
export type CupKindName = "entries" | "board";
/** Anchor enums decode as `{ open: {} }`; this returns `"open"`. */
export declare const stateName: <T extends string>(s: object) => T;
export type Amount = bigint | number | BN;
export declare const bn: (x: Amount) => BN;
export declare const bytes32: (b: Uint8Array | number[]) => number[];
/** A token's limits and fees, as `addMint` / `updateMint` take them (base units). */
export interface MintInput {
    minStake: Amount;
    maxStake: Amount;
    roomFeeBps: number;
    cupFeeBps: number;
    /** Token-2022 transfer-fee mints only (the project token). */
    allowTransferFee?: boolean;
    maxTransferFeeBps?: number;
    /** The holder fee applies to stakes up to this. Default: `maxStake`. */
    holderMaxStake?: Amount;
}
/** A cup's terms, as `createCup` (and the house's `fundCup`) take them. */
export interface CupInput {
    kind: CupKindName;
    payoutBps: number[];
    startsAt: number;
    endsAt: number;
    entryFee?: Amount;
    guarantee?: Amount;
    maxEntrants?: number;
    class?: number;
    /** sha256 of the event's published race spec. */
    specHash?: Uint8Array;
    /** The event's featured stock (winners may swap their prize into it as they claim). */
    featuredMint?: PublicKey;
    /** Community cups: the host's share of the house fee (at most 5000). */
    hostShareBps?: number;
}
export declare const cupParams: (p: CupInput) => {
    kind: {
        board: {};
        entries?: undefined;
    } | {
        entries: {};
        board?: undefined;
    };
    class: number;
    entryFee: import("bn.js");
    payoutBps: number[];
    guarantee: import("bn.js");
    startsAt: import("bn.js");
    endsAt: import("bn.js");
    maxEntrants: number;
    specHash: number[];
    featuredMint: PublicKey;
    hostShareBps: number;
};
/**
 * Builds arena instructions and reads arena accounts. Every method returns
 * instructions; signing and sending stay with the caller (the race server's
 * keys, or the player's wallet). Token rooms and cups need their escrow and
 * payout token accounts, which the methods create idempotently.
 */
export declare class ArenaClient {
    readonly connection: Connection;
    readonly program: Program<Arena>;
    readonly programId: PublicKey;
    constructor(connection: Connection, programId?: PublicKey);
    pda: {
        config: () => PublicKey;
        ruleset: (id: number) => PublicKey;
        allowedMint: (mint: PublicKey) => PublicKey;
        room: (code: string | Uint8Array) => PublicKey;
        cup: (id: bigint) => PublicKey;
        entry: (cup: PublicKey, player: PublicKey) => PublicKey;
        host: (wallet: PublicKey) => PublicKey;
    };
    fetchConfig(): Promise<{
        admin: PublicKey;
        pendingAdmin: PublicKey;
        guardian: PublicKey;
        gate: PublicKey;
        resolvers: PublicKey[];
        treasury: PublicKey;
        prizeVault: PublicKey;
        holderMint: PublicKey;
        holderMin: import("bn.js");
        holderRoomFeeBps: number;
        hostMin: import("bn.js");
        settleWindowSecs: number;
        challengeWindowSecs: number;
        resultsWindowSecs: number;
        claimWindowSecs: number;
        paused: boolean;
        cupCount: import("bn.js");
        pendingGuardian: PublicKey;
        pendingGate: PublicKey;
        pendingResolvers: PublicKey[];
        rolesAfter: import("bn.js");
        bump: number;
    }>;
    fetchRoom(room: PublicKey): Promise<{
        state: ({
            open?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            refunded: Record<string, never>;
        }) | ({
            refunded?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            open: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            settled?: undefined;
        } & {
            locked: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            locked?: undefined;
        } & {
            settled: Record<string, never>;
        });
        access: number;
        class: number;
        mint: PublicKey;
        stake: import("bn.js");
        maxPlayers: number;
        maxRating: number;
        code: number[];
        creator: PublicKey;
        ruleset: PublicKey;
        race: {
            track: number;
            format: number;
            laps: number;
            ai: number;
            time: number;
            sky: number;
            startMode: number;
        };
        tokenProgram: PublicKey;
        feeBps: number;
        transferFee: boolean;
        maxTransferFeeBps: number;
        treasury: PublicKey;
        prizeVault: PublicKey;
        rentPayer: PublicKey;
        players: PublicKey[];
        levels: number[][];
        seed: number[];
        timesMs: number[];
        winners: number;
        share: import("bn.js");
        toTreasury: import("bn.js");
        recordHash: number[];
        voidReason: number;
        settledBy: PublicKey;
        createdAt: import("bn.js");
        joinDeadline: import("bn.js");
        settleWindowSecs: number;
        lockedAt: import("bn.js");
        lockedSlot: import("bn.js");
        settleDeadline: import("bn.js");
        bump: number;
    }>;
    fetchRoomByCode(code: string): Promise<{
        state: ({
            open?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            refunded: Record<string, never>;
        }) | ({
            refunded?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            open: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            settled?: undefined;
        } & {
            locked: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            locked?: undefined;
        } & {
            settled: Record<string, never>;
        });
        access: number;
        class: number;
        mint: PublicKey;
        stake: import("bn.js");
        maxPlayers: number;
        maxRating: number;
        code: number[];
        creator: PublicKey;
        ruleset: PublicKey;
        race: {
            track: number;
            format: number;
            laps: number;
            ai: number;
            time: number;
            sky: number;
            startMode: number;
        };
        tokenProgram: PublicKey;
        feeBps: number;
        transferFee: boolean;
        maxTransferFeeBps: number;
        treasury: PublicKey;
        prizeVault: PublicKey;
        rentPayer: PublicKey;
        players: PublicKey[];
        levels: number[][];
        seed: number[];
        timesMs: number[];
        winners: number;
        share: import("bn.js");
        toTreasury: import("bn.js");
        recordHash: number[];
        voidReason: number;
        settledBy: PublicKey;
        createdAt: import("bn.js");
        joinDeadline: import("bn.js");
        settleWindowSecs: number;
        lockedAt: import("bn.js");
        lockedSlot: import("bn.js");
        settleDeadline: import("bn.js");
        bump: number;
    } | null>;
    fetchCup(cup: PublicKey): Promise<{
        id: import("bn.js");
        kind: ({
            board?: undefined;
        } & {
            entries: Record<string, never>;
        }) | ({
            entries?: undefined;
        } & {
            board: Record<string, never>;
        });
        class: number;
        organizer: PublicKey;
        sponsor: PublicKey;
        rentPayer: PublicKey;
        host: PublicKey;
        hostShareBps: number;
        ruleset: PublicKey;
        specHash: number[];
        featuredMint: PublicKey;
        mint: PublicKey;
        tokenProgram: PublicKey;
        transferFee: boolean;
        entryFee: import("bn.js");
        feeBps: number;
        treasury: PublicKey;
        prizeVault: PublicKey;
        maxTransferFeeBps: number;
        payoutBps: number[];
        guarantee: import("bn.js");
        startsAt: import("bn.js");
        endsAt: import("bn.js");
        resultsDeadline: import("bn.js");
        challengeWindowSecs: number;
        claimWindowSecs: number;
        maxEntrants: number;
        entrants: number;
        refunded: number;
        state: ({
            posted?: undefined;
            finalized?: undefined;
            cancelled?: undefined;
        } & {
            open: Record<string, never>;
        }) | ({
            open?: undefined;
            finalized?: undefined;
            cancelled?: undefined;
        } & {
            posted: Record<string, never>;
        }) | ({
            open?: undefined;
            posted?: undefined;
            cancelled?: undefined;
        } & {
            finalized: Record<string, never>;
        }) | ({
            open?: undefined;
            posted?: undefined;
            finalized?: undefined;
        } & {
            cancelled: Record<string, never>;
        });
        resultsRoot: number[];
        scorers: number;
        standingsHash: number[];
        resultsUri: string;
        postedAt: import("bn.js");
        challengeEnds: import("bn.js");
        claimUntil: import("bn.js");
        pool: import("bn.js");
        places: number;
        usedBps: number;
        claimed: number;
        paid: import("bn.js");
        guaranteeReturned: boolean;
        bump: number;
    }>;
    fetchEntry(cup: PublicKey, player: PublicKey): Promise<{
        cup: PublicKey;
        player: PublicKey;
        rentPayer: PublicKey;
        refunded: boolean;
        bump: number;
    } | null>;
    fetchHost(wallet: PublicKey): Promise<{
        wallet: PublicKey;
        until: import("bn.js");
        bump: number;
    } | null>;
    fetchAllowedMint(mint: PublicKey): Promise<{
        mint: PublicKey;
        tokenProgram: PublicKey;
        decimals: number;
        minStake: import("bn.js");
        maxStake: import("bn.js");
        roomFeeBps: number;
        cupFeeBps: number;
        transferFee: boolean;
        maxTransferFeeBps: number;
        holderMaxStake: import("bn.js");
        enabled: boolean;
        bump: number;
    } | null>;
    fetchAllowedMints(): Promise<import("@anchor-lang/core").ProgramAccount<{
        mint: PublicKey;
        tokenProgram: PublicKey;
        decimals: number;
        minStake: import("bn.js");
        maxStake: import("bn.js");
        roomFeeBps: number;
        cupFeeBps: number;
        transferFee: boolean;
        maxTransferFeeBps: number;
        holderMaxStake: import("bn.js");
        enabled: boolean;
        bump: number;
    }>[]>;
    fetchRuleset(id: number): Promise<{
        id: number;
        hash: number[];
        ratingPoints: number[];
        enabled: boolean;
        bump: number;
    }>;
    fetchRulesets(): Promise<import("@anchor-lang/core").ProgramAccount<{
        id: number;
        hash: number[];
        ratingPoints: number[];
        enabled: boolean;
        bump: number;
    }>[]>;
    /** Rooms, filtered on-chain by state and token (the lobby's view). */
    fetchRooms(f?: {
        state?: RoomStateName;
        mint?: PublicKey;
    }): Promise<import("@anchor-lang/core").ProgramAccount<{
        state: ({
            open?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            refunded: Record<string, never>;
        }) | ({
            refunded?: undefined;
            locked?: undefined;
            settled?: undefined;
        } & {
            open: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            settled?: undefined;
        } & {
            locked: Record<string, never>;
        }) | ({
            refunded?: undefined;
            open?: undefined;
            locked?: undefined;
        } & {
            settled: Record<string, never>;
        });
        access: number;
        class: number;
        mint: PublicKey;
        stake: import("bn.js");
        maxPlayers: number;
        maxRating: number;
        code: number[];
        creator: PublicKey;
        ruleset: PublicKey;
        race: {
            track: number;
            format: number;
            laps: number;
            ai: number;
            time: number;
            sky: number;
            startMode: number;
        };
        tokenProgram: PublicKey;
        feeBps: number;
        transferFee: boolean;
        maxTransferFeeBps: number;
        treasury: PublicKey;
        prizeVault: PublicKey;
        rentPayer: PublicKey;
        players: PublicKey[];
        levels: number[][];
        seed: number[];
        timesMs: number[];
        winners: number;
        share: import("bn.js");
        toTreasury: import("bn.js");
        recordHash: number[];
        voidReason: number;
        settledBy: PublicKey;
        createdAt: import("bn.js");
        joinDeadline: import("bn.js");
        settleWindowSecs: number;
        lockedAt: import("bn.js");
        lockedSlot: import("bn.js");
        settleDeadline: import("bn.js");
        bump: number;
    }>[]>;
    fetchCups(state?: CupStateName): Promise<import("@anchor-lang/core").ProgramAccount<{
        id: import("bn.js");
        kind: ({
            board?: undefined;
        } & {
            entries: Record<string, never>;
        }) | ({
            entries?: undefined;
        } & {
            board: Record<string, never>;
        });
        class: number;
        organizer: PublicKey;
        sponsor: PublicKey;
        rentPayer: PublicKey;
        host: PublicKey;
        hostShareBps: number;
        ruleset: PublicKey;
        specHash: number[];
        featuredMint: PublicKey;
        mint: PublicKey;
        tokenProgram: PublicKey;
        transferFee: boolean;
        entryFee: import("bn.js");
        feeBps: number;
        treasury: PublicKey;
        prizeVault: PublicKey;
        maxTransferFeeBps: number;
        payoutBps: number[];
        guarantee: import("bn.js");
        startsAt: import("bn.js");
        endsAt: import("bn.js");
        resultsDeadline: import("bn.js");
        challengeWindowSecs: number;
        claimWindowSecs: number;
        maxEntrants: number;
        entrants: number;
        refunded: number;
        state: ({
            posted?: undefined;
            finalized?: undefined;
            cancelled?: undefined;
        } & {
            open: Record<string, never>;
        }) | ({
            open?: undefined;
            finalized?: undefined;
            cancelled?: undefined;
        } & {
            posted: Record<string, never>;
        }) | ({
            open?: undefined;
            posted?: undefined;
            cancelled?: undefined;
        } & {
            finalized: Record<string, never>;
        }) | ({
            open?: undefined;
            posted?: undefined;
            finalized?: undefined;
        } & {
            cancelled: Record<string, never>;
        });
        resultsRoot: number[];
        scorers: number;
        standingsHash: number[];
        resultsUri: string;
        postedAt: import("bn.js");
        challengeEnds: import("bn.js");
        claimUntil: import("bn.js");
        pool: import("bn.js");
        places: number;
        usedBps: number;
        claimed: number;
        paid: import("bn.js");
        guaranteeReturned: boolean;
        bump: number;
    }>[]>;
    /** Decode arena events from a transaction's log messages. */
    parseEvents(logs: string[]): ArenaEvent[];
    private tokenAccounts;
    /** Where a payment to `owner` goes: the wallet (SOL) or its ATA (SPL). */
    payoutAccount(owner: PublicKey, mint: PublicKey, tokenProgram: PublicKey): PublicKey;
    /**
     * Creations for the token accounts among `owners`' that don't exist yet, so
     * every token payment has somewhere to land (only the missing ones: eight
     * idempotent creations would not fit in one transaction beside a settlement).
     */
    ensureAtas(payer: PublicKey, owners: PublicKey[], mint: PublicKey, tokenProgram: PublicKey): Promise<TransactionInstruction[]>;
    /** The payout accounts as remaining accounts; for a Token-2022 coin the memo program after them (an account whose
     *  owner turned on required memos is paid after a memo). */
    private payouts;
    /** The memo program as the last remaining account, for payouts in a Token-2022 coin. */
    memo(tokenProgram: PublicKey): AccountMeta[];
    private stakeToken;
    /** The project token account that claims the holder fee for `owner` (null: no project token yet). */
    holderAccount(owner: PublicKey): Promise<PublicKey | null>;
    /**
     * Open a room and take the first seat. `code` is the race server's room id;
     * `payer` pays the room's rent and gets it back at close (the server's
     * sponsor, or the creator). GARAGE rooms snapshot each player's levels from
     * the house's upgrades account; `holderTokens` claims the holder fee.
     */
    createRoom(p: {
        creator: PublicKey;
        payer?: PublicKey;
        code: string;
        stake: Amount;
        mint?: PublicKey;
        maxPlayers?: number;
        joinWindowSecs?: number;
        class?: number;
        maxRating?: number;
        access?: number;
        race?: Partial<RaceSpec>;
        gate?: PublicKey;
        holderTokens?: PublicKey | null;
        ruleset?: number;
    }): Promise<{
        ixs: TransactionInstruction[];
        room: PublicKey;
    }>;
    /**
     * A player's seat. `expectedStake`: the stake the player was shown (the join
     * fails if the room's isn't that: a room opened again under its code with
     * another stake can't take theirs); the room's own when not given.
     */
    joinRoom(p: {
        player: PublicKey;
        room: PublicKey;
        gate?: PublicKey;
        roomAccount?: Room;
        expectedStake?: Amount;
    }): Promise<TransactionInstruction[]>;
    leaveRoom(p: {
        player: PublicKey;
        room: PublicKey;
    }): Promise<TransactionInstruction[]>;
    private payoutGroup;
    /** The creator, while the room is open: every stake back. */
    cancelRoom(p: {
        creator: PublicKey;
        room: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Resolver: a room its creator starts (two players or more, before its join deadline) locks; its seed comes next. */
    lockRoom(p: {
        resolver: PublicKey;
        room: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * Anyone, once the slot a room locked in is over (within ~512 slots): its seed,
     * from that slot's hash - which nobody could see while the locking transaction ran.
     */
    drawSeed(p: {
        room: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * Resolver: the race's result (one time per seat, DNF for none) and the hash
     * of its full record. Pays the winners in this transaction; only their
     * accounts are passed (every player's when it comes out as a refund).
     */
    settleRoom(p: {
        resolver: PublicKey;
        room: PublicKey;
        timesMs: number[];
        recordHash: Uint8Array;
        payer?: PublicKey;
        roomAccount?: Room;
    }): Promise<TransactionInstruction[]>;
    /** Resolver: no result (the server stopped, too few on the grid): every stake back. */
    voidRoom(p: {
        resolver: PublicKey;
        room: PublicKey;
        reason: number;
        recordHash?: Uint8Array;
        payer?: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone, after the join or settle deadline: every stake back, no fee. */
    refundRoom(p: {
        room: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone, once settled or refunded: rent back to its payer, any surplus to the prize vault. */
    closeRoom(p: {
        room: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * The house (admin, a resolver, the prize vault through the house program) or
     * a community host (`host`: an approved one; `holderTokens`: holding enough
     * of the project token). `id`: the cup's (a random one by default: nobody can take it first).
     */
    createCup(p: CupInput & {
        organizer: PublicKey;
        payer?: PublicKey;
        mint?: PublicKey;
        host?: boolean;
        holderTokens?: PublicKey | null;
        ruleset?: number;
        id?: bigint;
    }): Promise<{
        ixs: TransactionInstruction[];
        cup: PublicKey;
        id: bigint;
    }>;
    /** `payer` pays the entry's rent and gets it back (the server's sponsor, or the player). */
    enterCup(p: {
        player: PublicKey;
        cup: PublicKey;
        payer?: PublicKey;
        gate?: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /**
     * Resolver, after `endsAt`. Build `resultsRoot` with `buildResultsTree` over the paid places; `places` is how
     * many leaves it has, and must be what the cup pays: min(its table's length, scorers).
     */
    postResults(p: {
        resolver: PublicKey;
        cup: PublicKey;
        resultsRoot: Uint8Array;
        scorers: number;
        places: number;
        standingsHash: Uint8Array;
        resultsUri: string;
    }): Promise<TransactionInstruction[]>;
    /** Guardian or admin, during the challenge window. */
    rejectResults(p: {
        authority: PublicKey;
        cup: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Admin, guardian or a resolver, before finalization. */
    cancelCup(p: {
        authority: PublicKey;
        cup: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone, after the challenge window: the pool is fixed and claims open. */
    finalizeCup(p: {
        cup: PublicKey;
    }): Promise<TransactionInstruction[]>;
    private vault;
    /**
     * Pays rank `rank` to `wallet`. Until `claimUntil` only the winner may sign
     * (and may swap the prize into the featured stock in the same transaction);
     * after it anyone pushes it to them.
     */
    claimPrize(p: {
        claimer: PublicKey;
        cup: PublicKey;
        rank: number;
        wallet: PublicKey;
        proof: Uint8Array[];
        payer?: PublicKey;
        cupAccount?: Cup;
    }): Promise<TransactionInstruction[]>;
    /** Anyone: a cancelled cup, or one with no results by its deadline, returns an entry fee. */
    refundEntry(p: {
        cup: PublicKey;
        player: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]>;
    returnGuarantee(p: {
        cup: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone, once nothing is owed: the unused guarantee to the sponsor, the fee to the treasury (a host's share to the host). */
    closeCup(p: {
        cup: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Anyone: an entry's rent back to whoever paid it, once it can't be needed. */
    closeEntry(p: {
        cup: PublicKey;
        player: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** The upgrade authority, once after deploy. */
    initialize(p: {
        admin: PublicKey;
        params: ConfigParams;
        resolvers: PublicKey[];
    }): Promise<TransactionInstruction[]>;
    private adminCfg;
    /** The windows of future rooms and cups (each freezes its own). */
    updateConfig(p: {
        admin: PublicKey;
        windows: WindowParams;
    }): Promise<TransactionInstruction[]>;
    /** A new guardian, gate and resolver set, in force two days later (`applyRoles`, by anyone). */
    proposeRoles(p: {
        admin: PublicKey;
        guardian: PublicKey;
        gate: PublicKey;
        resolvers: PublicKey[];
    }): Promise<TransactionInstruction[]>;
    applyRoles(): Promise<TransactionInstruction[]>;
    /** The project token (once) and the holder thresholds, in its base units. */
    setHolder(p: {
        admin: PublicKey;
        holderMint: PublicKey;
        holderMin: Amount;
        holderRoomFeeBps: number;
        hostMin: Amount;
    }): Promise<TransactionInstruction[]>;
    proposeAdmin(p: {
        admin: PublicKey;
        newAdmin: PublicKey;
    }): Promise<TransactionInstruction[]>;
    acceptAdmin(p: {
        newAdmin: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Admin or guardian may pause; only the admin unpauses. */
    setPaused(p: {
        authority: PublicKey;
        paused: boolean;
    }): Promise<TransactionInstruction[]>;
    /** Guardian or admin: every resolver key off at once (refunds by deadline still work). */
    revokeResolvers(p: {
        authority: PublicKey;
    }): Promise<TransactionInstruction[]>;
    /** Lets `wallet` organize community cups until `until` (0: no longer). */
    setHost(p: {
        admin: PublicKey;
        wallet: PublicKey;
        until: number;
    }): Promise<TransactionInstruction[]>;
    /** A ruleset: the game build's hash and its rating points per department level (tenths). */
    addRuleset(p: {
        admin: PublicKey;
        id: number;
        hash: Uint8Array;
        ratingPoints: number[];
    }): Promise<TransactionInstruction[]>;
    updateRuleset(p: {
        admin: PublicKey;
        id: number;
        enabled: boolean;
    }): Promise<TransactionInstruction[]>;
    /** Enable a token (`NATIVE_SOL` for SOL) for rooms and cups. */
    addMint(p: MintInput & {
        admin: PublicKey;
        mint: PublicKey;
    }): Promise<TransactionInstruction[]>;
    updateMint(p: MintInput & {
        admin: PublicKey;
        mint: PublicKey;
        enabled: boolean;
    }): Promise<TransactionInstruction[]>;
}

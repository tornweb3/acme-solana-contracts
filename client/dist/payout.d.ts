export interface RoomOutcome {
    /** Every stake back, no fee: nobody finished, or everyone tied. */
    refundAll: boolean;
    /** Bit i set = players[i] wins `share`. */
    winners: number;
    share: bigint;
    /** The house fee plus rounding dust. */
    toTreasury: bigint;
}
/** The fastest time takes the pot less the fee; tied best times split it. */
export declare function roomOutcome(stake: bigint, feeBps: number, timesMs: number[]): RoomOutcome;
/** Seat indexes of the winners, in seat order. */
export declare const winnerSeats: (o: RoomOutcome, players: number) => number[];
/** pool = max(guarantee, entries − fee); paid places = min(table, scorers). */
export declare function cupPool(p: {
    entryFee: bigint;
    entrants: number;
    feeBps: number;
    guarantee: bigint;
    payoutBps: number[];
    scorers: number;
}): {
    pool: bigint;
    places: number;
    usedBps: number;
};
/** Rank `rank`'s prize (1-based). Shares of places nobody reached go to those who did. */
export declare const cupPrize: (pool: bigint, payoutBps: number[], usedBps: number, rank: number) => bigint;
/** A GARAGE car's rating in tenths: 700 + Σ level × the ruleset's points. */
export declare const rating: (levels: number[], points: number[]) => number;

// The programs' money math (programs/arena/src/payout.rs), for showing amounts
// before they happen and for picking a settlement's payout accounts. The
// programs compute every amount themselves; these must agree with them.

import { DNF, MAX_ROOM_PLAYERS, MIN_ROOM_PLAYERS, STOCK_RATING } from "./constants.js";

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
export function roomOutcome(stake: bigint, feeBps: number, timesMs: number[]): RoomOutcome {
  const n = timesMs.length;
  if (n < MIN_ROOM_PLAYERS || n > MAX_ROOM_PLAYERS || timesMs.some((t) => t <= 0)) throw new Error("invalid results");
  const refund = { refundAll: true, winners: 0, share: 0n, toTreasury: 0n };
  const finished = timesMs.filter((t) => t !== DNF);
  if (finished.length === 0) return refund;
  const best = Math.min(...finished);
  let winners = 0;
  timesMs.forEach((t, i) => {
    if (t === best) winners |= 1 << i;
  });
  const w = BigInt(timesMs.filter((t) => t === best).length);
  if (w === BigInt(n)) return refund;
  const pot = stake * BigInt(n);
  const fee = (pot * BigInt(feeBps)) / 10_000n;
  const net = pot - fee;
  const share = net / w;
  return { refundAll: false, winners, share, toTreasury: fee + (net - share * w) };
}

/** Seat indexes of the winners, in seat order. */
export const winnerSeats = (o: RoomOutcome, players: number): number[] =>
  [...Array(players).keys()].filter((i) => o.winners & (1 << i));

/** pool = max(guarantee, entries − fee); paid places = min(table, scorers). */
export function cupPool(p: {
  entryFee: bigint;
  entrants: number;
  feeBps: number;
  guarantee: bigint;
  payoutBps: number[];
  scorers: number;
}): { pool: bigint; places: number; usedBps: number } {
  const entries = p.entryFee * BigInt(p.entrants);
  const net = entries - (entries * BigInt(p.feeBps)) / 10_000n;
  const pool = p.guarantee > net ? p.guarantee : net;
  const places = Math.min(p.payoutBps.length, p.scorers);
  const usedBps = p.payoutBps.slice(0, places).reduce((a, b) => a + b, 0);
  return { pool, places, usedBps };
}

/** Rank `rank`'s prize (1-based). Shares of places nobody reached go to those who did. */
export const cupPrize = (pool: bigint, payoutBps: number[], usedBps: number, rank: number): bigint =>
  (pool * BigInt(payoutBps[rank - 1])) / BigInt(usedBps);

/** A GARAGE car's rating in tenths: 700 + Σ level × the ruleset's points. */
export const rating = (levels: number[], points: number[]): number =>
  levels.reduce((r, l, i) => r + l * (points[i] ?? 0), STOCK_RATING);

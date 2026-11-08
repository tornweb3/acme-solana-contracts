import type { Connection, PublicKey } from "@solana/web3.js";
/**
 * A token's transfer fee (Token-2022's TransferFeeConfig, e.g. a StonkFun
 * token's 1 %): basis points, and the most it takes from one transfer.
 */
export interface TransferFee {
    bps: number;
    maxFee: bigint;
}
export declare const NO_FEE: TransferFee;
/** What Token-2022 withholds from a transfer of `amount`: ceil(amount × bps), capped. */
export declare function transferFee(amount: bigint, fee: TransferFee): bigint;
/** What arrives when `amount` is sent. */
export declare const netOf: (amount: bigint, fee: TransferFee) => bigint;
/**
 * What must be sent so that exactly `net` arrives: the arena grosses every
 * stake and entry up this way, so a player pays the stake plus the tax.
 */
export declare function grossForNet(net: bigint, fee: TransferFee): bigint;
/** A mint's transfer fee in an epoch (default: the current one). None for SOL and plain tokens. */
export declare function fetchTransferFee(connection: Connection, mint: PublicKey, epoch?: number): Promise<TransferFee>;
/**
 * The higher of the fee in force and the scheduled one. The arena refuses new
 * stakes in a token when this is above the cap set for it (its fee authority,
 * not us, can raise it); payouts are never refused.
 */
export declare function fetchTransferFeeCeiling(connection: Connection, mint: PublicKey): Promise<number>;

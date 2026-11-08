import { type Connection, type PublicKey, type TransactionInstruction, VersionedTransaction } from "@solana/web3.js";
/**
 * A v0 transaction with a fresh blockhash, ready for the wallet (or game
 * wallet) to sign. Simulate it before asking for a signature:
 * `connection.simulateTransaction(tx, { sigVerify: false })`.
 */
export declare function buildTransaction(connection: Connection, payer: PublicKey, ixs: TransactionInstruction[]): Promise<{
    tx: VersionedTransaction;
    lastValidBlockHeight: number;
}>;

import { TransactionMessage, VersionedTransaction, } from "@solana/web3.js";
/**
 * A v0 transaction with a fresh blockhash, ready for the wallet (or game
 * wallet) to sign. Simulate it before asking for a signature:
 * `connection.simulateTransaction(tx, { sigVerify: false })`.
 */
export async function buildTransaction(connection, payer, ixs) {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash("confirmed");
    const msg = new TransactionMessage({ payerKey: payer, recentBlockhash: blockhash, instructions: ixs }).compileToV0Message();
    return { tx: new VersionedTransaction(msg), lastValidBlockHeight };
}

import type { PublicKey } from "@solana/web3.js";
export declare const sha: (...parts: Uint8Array[]) => Uint8Array<ArrayBufferLike>;
export declare function resultsLeaf(cup: PublicKey, rank: number, wallet: PublicKey): Uint8Array;
export declare function resultsNode(a: Uint8Array, b: Uint8Array): Uint8Array;
/**
 * The results tree for `ranked` wallets (index 0 = rank 1, one entry per PAID
 * place). Returns the root to post and each rank's proof for `claimPrize`.
 */
export declare function buildResultsTree(cup: PublicKey, ranked: PublicKey[]): {
    root: Uint8Array;
    proofs: Uint8Array[][];
};
export declare function verifyResultsProof(root: Uint8Array, leaf: Uint8Array, proof: Uint8Array[]): boolean;

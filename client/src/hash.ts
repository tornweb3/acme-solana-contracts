import { sha256 } from "@noble/hashes/sha256";
import type { PublicKey } from "@solana/web3.js";

function concat(parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

function compare(a: Uint8Array, b: Uint8Array): number {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

export const sha = (...parts: Uint8Array[]) => sha256(concat(parts));

// ---- a cup's results tree (mirrors programs/arena/src/merkle.rs) ----

export function resultsLeaf(cup: PublicKey, rank: number, wallet: PublicKey): Uint8Array {
  const r = new Uint8Array(2);
  new DataView(r.buffer).setUint16(0, rank, true);
  return sha(Uint8Array.of(0), cup.toBytes(), r, wallet.toBytes());
}

export function resultsNode(a: Uint8Array, b: Uint8Array): Uint8Array {
  const [lo, hi] = compare(a, b) <= 0 ? [a, b] : [b, a];
  return sha(Uint8Array.of(1), lo, hi);
}

/**
 * The results tree for `ranked` wallets (index 0 = rank 1, one entry per PAID
 * place). Returns the root to post and each rank's proof for `claimPrize`.
 */
export function buildResultsTree(cup: PublicKey, ranked: PublicKey[]): { root: Uint8Array; proofs: Uint8Array[][] } {
  if (ranked.length === 0) throw new Error("no ranked players");
  let level = ranked.map((p, i) => resultsLeaf(cup, i + 1, p));
  const proofs: Uint8Array[][] = ranked.map(() => []);
  const pos = ranked.map((_, i) => i);
  while (level.length > 1) {
    pos.forEach((p, i) => {
      const sib = p ^ 1;
      if (sib < level.length) proofs[i].push(level[sib]);
      pos[i] = p >> 1;
    });
    const next: Uint8Array[] = [];
    for (let i = 0; i < level.length; i += 2) next.push(i + 1 < level.length ? resultsNode(level[i], level[i + 1]) : level[i]);
    level = next;
  }
  return { root: level[0], proofs };
}

export function verifyResultsProof(root: Uint8Array, leaf: Uint8Array, proof: Uint8Array[]): boolean {
  return compare(proof.reduce((acc, p) => resultsNode(acc, p), leaf), root) === 0;
}

//! Cup results: a Merkle root over one leaf per paid place (rank, wallet).
//! Sorted-pair hashing with distinct leaf/node prefixes, so a leaf can never be
//! passed off as an inner node. The TS client builds the same tree.

use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::constants::*;

pub fn leaf(cup: &Pubkey, rank: u16, wallet: &Pubkey) -> [u8; 32] {
    hashv(&[
        LEAF_PREFIX,
        cup.as_ref(),
        &rank.to_le_bytes(),
        wallet.as_ref(),
    ])
    .to_bytes()
}

pub fn node(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    hashv(&[NODE_PREFIX, lo, hi]).to_bytes()
}

pub fn verify(proof: &[[u8; 32]], root: &[u8; 32], leaf: [u8; 32]) -> bool {
    proof.iter().fold(leaf, |acc, p| node(&acc, p)) == *root
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build all levels for a list of leaves (odd node carried up), like the TS client.
    fn root_and_proofs(leaves: &[[u8; 32]]) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
        let mut proofs = vec![Vec::new(); leaves.len()];
        let mut idx: Vec<usize> = (0..leaves.len()).collect();
        let mut level = leaves.to_vec();
        while level.len() > 1 {
            let mut next = Vec::new();
            for pair in level.chunks(2) {
                next.push(if pair.len() == 2 {
                    node(&pair[0], &pair[1])
                } else {
                    pair[0]
                });
            }
            for (leaf_i, pos) in idx.iter_mut().enumerate() {
                let sib = *pos ^ 1;
                if sib < level.len() {
                    proofs[leaf_i].push(level[sib]);
                }
                *pos /= 2;
            }
            level = next;
        }
        (level[0], proofs)
    }

    #[test]
    fn proofs_verify_and_tampering_fails() {
        let t = Pubkey::new_unique();
        let players: Vec<Pubkey> = (0..7).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<[u8; 32]> = players
            .iter()
            .enumerate()
            .map(|(i, p)| leaf(&t, i as u16 + 1, p))
            .collect();
        let (root, proofs) = root_and_proofs(&leaves);
        for (i, p) in players.iter().enumerate() {
            assert!(verify(&proofs[i], &root, leaf(&t, i as u16 + 1, p)));
            // wrong rank, wrong player, wrong tournament
            assert!(!verify(&proofs[i], &root, leaf(&t, i as u16 + 2, p)));
            assert!(!verify(
                &proofs[i],
                &root,
                leaf(&t, i as u16 + 1, &Pubkey::new_unique())
            ));
            assert!(!verify(
                &proofs[i],
                &root,
                leaf(&Pubkey::new_unique(), i as u16 + 1, p)
            ));
        }
        // a single-leaf tree: root == leaf, empty proof
        let one = leaf(&t, 1, &players[0]);
        assert!(verify(&[], &one, one));
    }
}

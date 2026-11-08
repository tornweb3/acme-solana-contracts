import { PublicKey } from "@solana/web3.js";
/**
 * A room's 8-byte code: the race server's room id (8 characters of [A-Za-z0-9_-]),
 * as bytes. Room addresses derive from it, so the server knows a room's address
 * before it exists.
 */
export declare function roomCode(id: string): Uint8Array;
export declare const configPda: (program?: PublicKey) => PublicKey;
export declare const rulesetPda: (id: number, program?: PublicKey) => PublicKey;
export declare const allowedMintPda: (mint: PublicKey, program?: PublicKey) => PublicKey;
export declare const roomPda: (code: string | Uint8Array, program?: PublicKey) => PublicKey;
export declare const cupPda: (id: bigint, program?: PublicKey) => PublicKey;
export declare const entryPda: (cup: PublicKey, player: PublicKey, program?: PublicKey) => PublicKey;
export declare const hostPda: (wallet: PublicKey, program?: PublicKey) => PublicKey;
export declare const housePda: (program?: PublicKey) => PublicKey;
/** Every fee and sale lands here (arena's `config.treasury`). */
export declare const treasuryPda: (program?: PublicKey) => PublicKey;
/** Funds cups and takes escrow surpluses (arena's `config.prizeVault`). */
export declare const prizeVaultPda: (program?: PublicKey) => PublicKey;
export declare const buybackPda: (program?: PublicKey) => PublicKey;
export declare const pricePda: (mint: PublicKey, program?: PublicKey) => PublicKey;
export declare const totalsPda: (mint: PublicKey, program?: PublicKey) => PublicKey;
export declare const upgradesPda: (owner: PublicKey, program?: PublicKey) => PublicKey;
/** The project token, proposed and waiting out its timelock. */
export declare const pendingTokenPda: (program?: PublicKey) => PublicKey;
/** The buyback auction of one token (`NATIVE_SOL`: SOL) for the project token. */
export declare const auctionPda: (lotMint: PublicKey, program?: PublicKey) => PublicKey;
/** Associated token account, allowing PDA owners (room / cup escrows, the house's vaults). */
export declare const ata: (owner: PublicKey, mint: PublicKey, tokenProgram: PublicKey) => PublicKey;

import { PublicKey } from "@solana/web3.js";
import { ARENA_PROGRAM_ID, HOUSE_PROGRAM_ID } from "./constants.js";
const enc = (s) => new TextEncoder().encode(s);
const u32 = (n) => {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, n, true);
    return b;
};
const u64 = (n) => {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigUint64(0, n, true);
    return b;
};
const find = (seeds, program) => PublicKey.findProgramAddressSync(seeds, program)[0];
/**
 * A room's 8-byte code: the race server's room id (8 characters of [A-Za-z0-9_-]),
 * as bytes. Room addresses derive from it, so the server knows a room's address
 * before it exists.
 */
export function roomCode(id) {
    if (!/^[A-Za-z0-9_-]{8}$/.test(id))
        throw new Error(`a room code is 8 characters: ${id}`);
    return enc(id);
}
// ---- arena ----
export const configPda = (program = ARENA_PROGRAM_ID) => find([enc("config")], program);
export const rulesetPda = (id, program = ARENA_PROGRAM_ID) => find([enc("ruleset"), u32(id)], program);
export const allowedMintPda = (mint, program = ARENA_PROGRAM_ID) => find([enc("mint"), mint.toBytes()], program);
export const roomPda = (code, program = ARENA_PROGRAM_ID) => find([enc("room"), typeof code === "string" ? roomCode(code) : code], program);
export const cupPda = (id, program = ARENA_PROGRAM_ID) => find([enc("cup"), u64(id)], program);
export const entryPda = (cup, player, program = ARENA_PROGRAM_ID) => find([enc("entry"), cup.toBytes(), player.toBytes()], program);
export const hostPda = (wallet, program = ARENA_PROGRAM_ID) => find([enc("host"), wallet.toBytes()], program);
// ---- house ----
export const housePda = (program = HOUSE_PROGRAM_ID) => find([enc("house")], program);
/** Every fee and sale lands here (arena's `config.treasury`). */
export const treasuryPda = (program = HOUSE_PROGRAM_ID) => find([enc("treasury")], program);
/** Funds cups and takes escrow surpluses (arena's `config.prizeVault`). */
export const prizeVaultPda = (program = HOUSE_PROGRAM_ID) => find([enc("prize")], program);
export const buybackPda = (program = HOUSE_PROGRAM_ID) => find([enc("buyback")], program);
export const pricePda = (mint, program = HOUSE_PROGRAM_ID) => find([enc("price"), mint.toBytes()], program);
export const totalsPda = (mint, program = HOUSE_PROGRAM_ID) => find([enc("totals"), mint.toBytes()], program);
export const upgradesPda = (owner, program = HOUSE_PROGRAM_ID) => find([enc("upgrades"), owner.toBytes()], program);
/** The project token, proposed and waiting out its timelock. */
export const pendingTokenPda = (program = HOUSE_PROGRAM_ID) => find([enc("token")], program);
/** The buyback auction of one token (`NATIVE_SOL`: SOL) for the project token. */
export const auctionPda = (lotMint, program = HOUSE_PROGRAM_ID) => find([enc("auction"), lotMint.toBytes()], program);
/** Associated token account, allowing PDA owners (room / cup escrows, the house's vaults). */
export const ata = (owner, mint, tokenProgram) => find([owner.toBytes(), tokenProgram.toBytes(), mint.toBytes()], new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"));

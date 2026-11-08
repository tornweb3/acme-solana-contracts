import { PublicKey } from "@solana/web3.js";

/** Same ids on localnet and devnet. Mainnet will get NEW program ids. */
export const ARENA_PROGRAM_ID = new PublicKey("G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK");
export const HOUSE_PROGRAM_ID = new PublicKey("5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw");

/** A room, cup, price table or sweep whose mint is this address is in native SOL. */
export const NATIVE_SOL = PublicKey.default;

/** SPL Memo v3: a payout into a Token-2022 account that requires memos goes after one (the arena does it, given this
 *  program as the last remaining account). */
export const MEMO_PROGRAM_ID = new PublicKey("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
export const TOKEN_2022_ID = new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/** A seat's time meaning "no result". Any other time is milliseconds, lower wins. */
export const DNF = 0xffffffff;

/** The house: the project token takes effect this long after it is proposed; a transfer-fee token above this fee is
 *  refused as the project token and for sales. */
export const TOKEN_TIMELOCK_SECS = 2 * 86_400;
export const HARD_MAX_TOKEN_FEE_BPS = 500;
/** The buyback auction's bounds (house): a lot at most this share of the vault, a day between auctions, the price's
 *  fall over 1 h to 7 days, an auction 14 days at most, its floor at least half the last sale's rate. */
export const AUCTION_MAX_LOT_BPS = 2_500;
export const AUCTION_EVERY_SECS = 86_400;
export const MIN_DECLINE_SECS = 3_600;
export const MAX_DECLINE_SECS = 7 * 86_400;
export const MAX_AUCTION_SECS = 14 * 86_400;
export const FLOOR_ANCHOR_BPS = 5_000;
export const ANCHOR_SECS = 7 * 86_400;

/** Hard ceilings in the arena program; no setting can go above them. */
export const HARD_MAX_ROOM_FEE_BPS = 1_000;
export const HARD_MAX_CUP_FEE_BPS = 1_500;
export const HARD_MAX_TRANSFER_FEE_BPS = 500;
export const MAX_HOST_SHARE_BPS = 5_000;
export const MIN_ROOM_PLAYERS = 2;
export const MAX_ROOM_PLAYERS = 8;

/** Room classes: EQUAL races the full car for everyone; GARAGE each player's own levels. */
export const CLASS_EQUAL = 0;
export const CLASS_GARAGE = 1;
export const ACCESS_OPEN = 0;
export const ACCESS_INVITE = 1;
/** start_mode: 0 = the room starts (locks) as it fills; 1 = its creator starts it. */
export const START_WHEN_FULL = 0;
export const START_BY_HOST = 1;

/** Why a room ended Refunded (`room.voidReason`). */
export const VOID_NONE = 0;
export const VOID_CANCELLED = 1;
export const VOID_EXPIRED = 2;
export const VOID_RACE = 3;
export const VOID_TIED = 4;

/** Upgrades: six departments (src/game/upgrades.js order), five levels each. */
export const DEPARTMENTS = ["pu", "aero", "tyre", "brake", "chassis", "dur"] as const;
export const MAX_LEVEL = 5;
/** A car's rating in tenths: 700 stock, 1000 with every level. */
export const STOCK_RATING = 700;

/**
 * Byte offsets of a room's fixed prefix (after the 8-byte discriminator), for
 * `getProgramAccounts` memcmp filters in a lobby.
 */
export const ROOM_OFFSETS = { state: 8, access: 9, class: 10, mint: 11, stake: 43, maxPlayers: 51, maxRating: 52 } as const;
/** `room.state` as stored (the enum's index). */
export const ROOM_STATE = { open: 0, locked: 1, settled: 2, refunded: 3 } as const;

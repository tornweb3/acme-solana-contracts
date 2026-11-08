import { PublicKey } from "@solana/web3.js";
/** Same ids on localnet and devnet. Mainnet will get NEW program ids. */
export declare const ARENA_PROGRAM_ID: PublicKey;
export declare const HOUSE_PROGRAM_ID: PublicKey;
/** A room, cup, price table or sweep whose mint is this address is in native SOL. */
export declare const NATIVE_SOL: PublicKey;
/** SPL Memo v3: a payout into a Token-2022 account that requires memos goes after one (the arena does it, given this
 *  program as the last remaining account). */
export declare const MEMO_PROGRAM_ID: PublicKey;
export declare const TOKEN_2022_ID: PublicKey;
/** A seat's time meaning "no result". Any other time is milliseconds, lower wins. */
export declare const DNF = 4294967295;
/** The house: the project token takes effect this long after it is proposed; a transfer-fee token above this fee is
 *  refused as the project token and for sales. */
export declare const TOKEN_TIMELOCK_SECS: number;
export declare const HARD_MAX_TOKEN_FEE_BPS = 500;
/** The buyback auction's bounds (house): a lot at most this share of the vault, a day between auctions, the price's
 *  fall over 1 h to 7 days, an auction 14 days at most, its floor at least half the last sale's rate. */
export declare const AUCTION_MAX_LOT_BPS = 2500;
export declare const AUCTION_EVERY_SECS = 86400;
export declare const MIN_DECLINE_SECS = 3600;
export declare const MAX_DECLINE_SECS: number;
export declare const MAX_AUCTION_SECS: number;
export declare const FLOOR_ANCHOR_BPS = 5000;
export declare const ANCHOR_SECS: number;
/** Hard ceilings in the arena program; no setting can go above them. */
export declare const HARD_MAX_ROOM_FEE_BPS = 1000;
export declare const HARD_MAX_CUP_FEE_BPS = 1500;
export declare const HARD_MAX_TRANSFER_FEE_BPS = 500;
export declare const MAX_HOST_SHARE_BPS = 5000;
export declare const MIN_ROOM_PLAYERS = 2;
export declare const MAX_ROOM_PLAYERS = 8;
/** Room classes: EQUAL races the full car for everyone; GARAGE each player's own levels. */
export declare const CLASS_EQUAL = 0;
export declare const CLASS_GARAGE = 1;
export declare const ACCESS_OPEN = 0;
export declare const ACCESS_INVITE = 1;
/** start_mode: 0 = the room starts (locks) as it fills; 1 = its creator starts it. */
export declare const START_WHEN_FULL = 0;
export declare const START_BY_HOST = 1;
/** Why a room ended Refunded (`room.voidReason`). */
export declare const VOID_NONE = 0;
export declare const VOID_CANCELLED = 1;
export declare const VOID_EXPIRED = 2;
export declare const VOID_RACE = 3;
export declare const VOID_TIED = 4;
/** Upgrades: six departments (src/game/upgrades.js order), five levels each. */
export declare const DEPARTMENTS: readonly ["pu", "aero", "tyre", "brake", "chassis", "dur"];
export declare const MAX_LEVEL = 5;
/** A car's rating in tenths: 700 stock, 1000 with every level. */
export declare const STOCK_RATING = 700;
/**
 * Byte offsets of a room's fixed prefix (after the 8-byte discriminator), for
 * `getProgramAccounts` memcmp filters in a lobby.
 */
export declare const ROOM_OFFSETS: {
    readonly state: 8;
    readonly access: 9;
    readonly class: 10;
    readonly mint: 11;
    readonly stake: 43;
    readonly maxPlayers: 51;
    readonly maxRating: 52;
};
/** `room.state` as stored (the enum's index). */
export declare const ROOM_STATE: {
    readonly open: 0;
    readonly locked: 1;
    readonly settled: 2;
    readonly refunded: 3;
};

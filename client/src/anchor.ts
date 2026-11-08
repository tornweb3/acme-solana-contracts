// One import point for @anchor-lang/core that works everywhere. Node's ESM
// loader sees its CommonJS build, where everything hangs off the default
// export; bundlers (Vite) see its ESM build, which has named exports.
import * as ns from "@anchor-lang/core";
import type { Idl } from "@anchor-lang/core";

const anchor = ((ns as unknown as { default?: typeof ns }).default ?? ns) as typeof ns;

export const BN = anchor.BN;
export type BN = InstanceType<typeof anchor.BN>;
export const Program = anchor.Program;
export type Program<T extends Idl = Idl> = ns.Program<T>;
export const BorshCoder = anchor.BorshCoder;
export const EventParser = anchor.EventParser;
export const utils = anchor.utils;
export type { Idl, IdlAccounts, IdlEvents, IdlTypes } from "@anchor-lang/core";

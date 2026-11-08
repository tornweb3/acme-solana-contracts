// One import point for @anchor-lang/core that works everywhere. Node's ESM
// loader sees its CommonJS build, where everything hangs off the default
// export; bundlers (Vite) see its ESM build, which has named exports.
import * as ns from "@anchor-lang/core";
const anchor = (ns.default ?? ns);
export const BN = anchor.BN;
export const Program = anchor.Program;
export const BorshCoder = anchor.BorshCoder;
export const EventParser = anchor.EventParser;
export const utils = anchor.utils;

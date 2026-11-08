import { TOKEN_2022_PROGRAM_ID, getEpochFee, getTransferFeeConfig, unpackMint } from "@solana/spl-token";
export const NO_FEE = { bps: 0, maxFee: 0n };
/** What Token-2022 withholds from a transfer of `amount`: ceil(amount × bps), capped. */
export function transferFee(amount, fee) {
    if (fee.bps === 0 || amount === 0n)
        return 0n;
    const raw = (amount * BigInt(fee.bps) + 9999n) / 10000n;
    return raw < fee.maxFee ? raw : fee.maxFee;
}
/** What arrives when `amount` is sent. */
export const netOf = (amount, fee) => amount - transferFee(amount, fee);
/**
 * What must be sent so that exactly `net` arrives: the arena grosses every
 * stake and entry up this way, so a player pays the stake plus the tax.
 */
export function grossForNet(net, fee) {
    if (fee.bps === 0 || net === 0n)
        return net;
    let gross;
    if (fee.bps >= 10_000)
        gross = net + fee.maxFee;
    else {
        const d = BigInt(10_000 - fee.bps);
        const raw = (net * 10000n + d - 1n) / d;
        gross = raw - net >= fee.maxFee ? net + fee.maxFee : raw;
    }
    for (let i = 0; i < 2; i++, gross++)
        if (gross - transferFee(gross, fee) === net)
            return gross;
    throw new Error(`no exact gross amount for ${net}`);
}
async function feeConfig(connection, mint) {
    const info = await connection.getAccountInfo(mint, "confirmed");
    if (!info || !info.owner.equals(TOKEN_2022_PROGRAM_ID))
        return null;
    return getTransferFeeConfig(unpackMint(mint, info, TOKEN_2022_PROGRAM_ID));
}
/** A mint's transfer fee in an epoch (default: the current one). None for SOL and plain tokens. */
export async function fetchTransferFee(connection, mint, epoch) {
    const cfg = await feeConfig(connection, mint);
    if (!cfg)
        return NO_FEE;
    const e = epoch ?? (await connection.getEpochInfo("confirmed")).epoch;
    const f = getEpochFee(cfg, BigInt(e));
    return { bps: f.transferFeeBasisPoints, maxFee: f.maximumFee };
}
/**
 * The higher of the fee in force and the scheduled one. The arena refuses new
 * stakes in a token when this is above the cap set for it (its fee authority,
 * not us, can raise it); payouts are never refused.
 */
export async function fetchTransferFeeCeiling(connection, mint) {
    const cfg = await feeConfig(connection, mint);
    if (!cfg)
        return 0;
    return Math.max(cfg.olderTransferFee.transferFeeBasisPoints, cfg.newerTransferFee.transferFeeBasisPoints);
}

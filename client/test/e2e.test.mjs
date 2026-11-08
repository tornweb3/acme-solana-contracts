// End-to-end: the built client (../dist) against both programs on a throwaway
// local validator. Run by `anchor test --validator legacy` (see Anchor.toml),
// which deploys arena + house as upgradeable programs and sets ANCHOR_WALLET (their
// upgrade authority) and ANCHOR_PROVIDER_URL. The flows: a USDC room, a 1 % ACME
// room, an upgrade raced in a GARAGE room, a sweep, the weekly cup.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { before, test } from "node:test";

import {
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  createMint,
  createMintToInstruction,
  getAccount,
  getMintLen,
} from "@solana/spl-token";
import { Connection, Keypair, LAMPORTS_PER_SOL, SystemProgram, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";

import {
  ArenaClient,
  CLASS_GARAGE,
  DNF,
  HouseClient,
  NATIVE_SOL,
  ata,
  buildResultsTree,
  fetchTransferFee,
  grossForNet,
  netOf,
  prizeVaultPda,
  stateName,
  treasuryPda,
} from "../dist/index.js";

const connection = new Connection(process.env.ANCHOR_PROVIDER_URL ?? "http://127.0.0.1:8899", "confirmed");
const admin = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(process.env.ANCHOR_WALLET, "utf8"))));
const arena = new ArenaClient(connection);
const house = new HouseClient(connection);
const [resolver, keeper, ops, sponsor] = [Keypair.generate(), Keypair.generate(), Keypair.generate(), Keypair.generate()];
const SOL = BigInt(LAMPORTS_PER_SOL);
const treasury = treasuryPda();
const vault = prizeVaultPda();
let usdc; // a plain SPL token standing in for USDC
let acme; // a Token-2022 mint with a 1 % transfer fee, like a StonkFun token

async function send(ixs, signers) {
  return sendAndConfirmTransaction(connection, new Transaction().add(...ixs), signers, { commitment: "confirmed" });
}
async function fund(pubkey, sol) {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  const bh = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
}
async function player(sol = 5) {
  const k = Keypair.generate();
  await fund(k.publicKey, sol);
  return k;
}
const lamports = async (pk) => BigInt(await connection.getBalance(pk, "confirmed"));
const tokens = async (pk, program = TOKEN_PROGRAM_ID) => (await getAccount(connection, pk, "confirmed", program)).amount;
async function give(mint, program, owner, amount) {
  const to = ata(owner, mint, program);
  await send(
    [
      createAssociatedTokenAccountIdempotentInstruction(admin.publicKey, to, owner, mint, program),
      createMintToInstruction(mint, to, admin.publicKey, amount, [], program),
    ],
    [admin],
  );
  return to;
}
async function chainTime() {
  return connection.getBlockTime(await connection.getSlot("confirmed"));
}
async function waitUntil(unix) {
  while ((await chainTime()) <= unix) await new Promise((r) => setTimeout(r, 500));
}
const hash = (n) => new Uint8Array(32).fill(n);
/** A locked room's seed, drawn once the slot it locked in is over (anyone may: the resolver here). */
async function drawn(room) {
  for (let i = 0; ; i++) {
    try {
      return await send(await arena.drawSeed({ room }), [resolver]);
    } catch (e) {
      if (i > 20 || !/TooEarly|too early/i.test(String(e.logs ?? e.message))) throw e;
      await new Promise((r) => setTimeout(r, 400));
    }
  }
}

before(async () => {
  for (const k of [resolver, keeper, ops, sponsor]) await fund(k.publicKey, 10);
  await send(
    await house.initialize({
      admin: admin.publicKey,
      keeper: keeper.publicKey,
      opsWallet: ops.publicKey,
      split: { opsBps: 5_000, prizeBps: 3_000, buybackBps: 2_000 },
      fundMaxBps: 2_000,
      fundEverySecs: 0,
    }),
    [admin],
  );
  await send(
    await arena.initialize({
      admin: admin.publicKey,
      params: {
        guardian: admin.publicKey,
        gate: NATIVE_SOL,
        treasury,
        prizeVault: vault,
        settleWindowSecs: 120,
        challengeWindowSecs: 10,
        resultsWindowSecs: 300,
        claimWindowSecs: 10,
      },
      resolvers: [resolver.publicKey],
    }),
    [admin],
  );
  await send(
    await arena.addRuleset({ admin: admin.publicKey, id: 1, hash: hash(7), ratingPoints: [10, 15, 19, 6, 6, 4] }),
    [admin],
  );
  await send(
    await arena.addMint({ admin: admin.publicKey, mint: NATIVE_SOL, minStake: SOL / 1000n, maxStake: 10n * SOL, roomFeeBps: 500, cupFeeBps: 1_000 }),
    [admin],
  );
  await send(await house.addMint({ admin: admin.publicKey, mint: NATIVE_SOL }), [admin]);

  usdc = await createMint(connection, admin, admin.publicKey, null, 6);
  await send(
    await arena.addMint({ admin: admin.publicKey, mint: usdc, minStake: 1_000_000, maxStake: 1_000_000_000, roomFeeBps: 500, cupFeeBps: 1_000 }),
    [admin],
  );
  await send(await house.addMint({ admin: admin.publicKey, mint: usdc }), [admin]);

  const mint = Keypair.generate();
  const space = getMintLen([ExtensionType.TransferFeeConfig]);
  await send(
    [
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: mint.publicKey,
        space,
        lamports: await connection.getMinimumBalanceForRentExemption(space),
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferFeeConfigInstruction(mint.publicKey, admin.publicKey, admin.publicKey, 100, 2n ** 64n - 1n, TOKEN_2022_PROGRAM_ID),
      createInitializeMintInstruction(mint.publicKey, 6, admin.publicKey, null, TOKEN_2022_PROGRAM_ID),
    ],
    [admin, mint],
  );
  acme = mint.publicKey;
  await send(
    await arena.addMint({
      admin: admin.publicKey,
      mint: acme,
      minStake: 1_000_000,
      maxStake: 100_000_000_000,
      roomFeeBps: 250,
      cupFeeBps: 1_000,
      allowTransferFee: true,
      maxTransferFeeBps: 100,
    }),
    [admin],
  );
  await send(await house.addMint({ admin: admin.publicKey, mint: acme }), [admin]);
});

test("both programs are set up and point at each other", async () => {
  const cfg = await arena.fetchConfig();
  assert.ok(cfg.admin.equals(admin.publicKey));
  assert.ok(cfg.treasury.equals(treasury) && cfg.prizeVault.equals(vault));
  assert.equal(cfg.resolvers.length, 1);
  const h = await house.fetchHouse();
  assert.deepEqual(h.split, { opsBps: 5_000, prizeBps: 3_000, buybackBps: 2_000 });
  assert.ok(h.keeper.equals(keeper.publicKey));
});

test("a USDC room: two players, the race's record hash, the winner paid in the settlement", async () => {
  const [a, b] = [await player(), await player()];
  for (const p of [a, b]) await give(usdc, TOKEN_PROGRAM_ID, p.publicKey, 50_000_000n);
  const stake = 5_000_000n;
  // the race server's sponsor pays the room's rent
  const { ixs, room } = await arena.createRoom({ creator: a.publicKey, payer: sponsor.publicKey, code: "USDC0001", stake, mint: usdc });
  await send(ixs, [a, sponsor]);
  await send(await arena.joinRoom({ player: b.publicKey, room, expectedStake: stake }), [b]);
  let r = await arena.fetchRoom(room);
  assert.equal(stateName(r.state), "locked"); // full: it locked itself; its seed comes from a later slot
  assert.ok(r.seed.every((x) => x === 0));
  await drawn(room);
  assert.ok((await arena.fetchRoom(room)).seed.some((x) => x !== 0));
  assert.equal((await arena.fetchRooms({ state: "locked", mint: usdc })).filter((x) => x.publicKey.equals(room)).length, 1);

  const t0 = (await connection.getAccountInfo(ata(treasury, usdc, TOKEN_PROGRAM_ID))) ? await tokens(ata(treasury, usdc, TOKEN_PROGRAM_ID)) : 0n;
  const sig = await send(
    await arena.settleRoom({ resolver: resolver.publicKey, room, timesMs: [92_000, 90_500], recordHash: hash(3) }),
    [resolver],
  );
  assert.equal(await tokens(ata(b.publicKey, usdc, TOKEN_PROGRAM_ID)), 45_000_000n + 9_500_000n);
  assert.equal(await tokens(ata(treasury, usdc, TOKEN_PROGRAM_ID)), t0 + 500_000n);
  r = await arena.fetchRoom(room);
  assert.equal(stateName(r.state), "settled");
  assert.deepEqual(Array.from(r.recordHash), Array.from(hash(3)));
  const tx = await connection.getTransaction(sig, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
  assert.ok(arena.parseEvents(tx.meta.logMessages).some((e) => e.name === "roomSettled"));

  const s0 = await lamports(sponsor.publicKey);
  await send(await arena.closeRoom({ room, payer: resolver.publicKey }), [resolver]);
  assert.equal(await connection.getAccountInfo(room), null);
  assert.ok((await lamports(sponsor.publicKey)) > s0); // its rent came back
});

test("a 1 % ACME room: stakes grossed up by the tax, payouts net of it, the escrow harvested at close", async () => {
  const fee = await fetchTransferFee(connection, acme);
  assert.equal(fee.bps, 100);
  const [a, b] = [await player(), await player()];
  for (const p of [a, b]) await give(acme, TOKEN_2022_PROGRAM_ID, p.publicKey, 1_000_000_000n);
  const stake = 10_000_000n;
  const { ixs, room } = await arena.createRoom({ creator: a.publicKey, code: "ACME0001", stake, mint: acme });
  await send(ixs, [a]);
  await send(await arena.joinRoom({ player: b.publicKey, room }), [b]);
  await drawn(room);
  const escrow = ata(room, acme, TOKEN_2022_PROGRAM_ID);
  assert.equal(await tokens(escrow, TOKEN_2022_PROGRAM_ID), 2n * stake);
  assert.equal(await tokens(ata(a.publicKey, acme, TOKEN_2022_PROGRAM_ID), TOKEN_2022_PROGRAM_ID), 1_000_000_000n - grossForNet(stake, fee));
  const r = await arena.fetchRoom(room);
  assert.equal(r.feeBps, 250); // the token's own room fee

  await send(await arena.settleRoom({ resolver: resolver.publicKey, room, timesMs: [60_000, DNF], recordHash: hash(4) }), [resolver]);
  const share = 2n * stake - (2n * stake * 250n) / 10_000n;
  assert.equal(
    await tokens(ata(a.publicKey, acme, TOKEN_2022_PROGRAM_ID), TOKEN_2022_PROGRAM_ID),
    1_000_000_000n - grossForNet(stake, fee) + netOf(share, fee),
  );
  await send(await arena.closeRoom({ room, payer: resolver.publicKey }), [resolver]);
  assert.equal(await connection.getAccountInfo(escrow), null);
});

test("an 8-player ACME room the race couldn't finish: every stake back in one transaction, less the tax", async () => {
  const fee = await fetchTransferFee(connection, acme);
  const players = [];
  for (let i = 0; i < 8; i++) players.push(await player(1));
  for (const p of players) await give(acme, TOKEN_2022_PROGRAM_ID, p.publicKey, 100_000_000n);
  const stake = 1_000_000n;
  const { ixs, room } = await arena.createRoom({ creator: players[0].publicKey, code: "ACME0008", stake, mint: acme, maxPlayers: 8 });
  await send(ixs, [players[0]]);
  for (const p of players.slice(1)) await send(await arena.joinRoom({ player: p.publicKey, room }), [p]);
  assert.equal(stateName((await arena.fetchRoom(room)).state), "locked");
  await send(await arena.voidRoom({ resolver: resolver.publicKey, room, reason: 3 }), [resolver]);
  const r = await arena.fetchRoom(room);
  assert.equal(stateName(r.state), "refunded");
  for (const p of players) {
    // the tax was paid on the way in and again on the way out; the room took no fee
    const left = await tokens(ata(p.publicKey, acme, TOKEN_2022_PROGRAM_ID), TOKEN_2022_PROGRAM_ID);
    assert.equal(left, 100_000_000n - grossForNet(stake, fee) + netOf(stake, fee));
  }
});

test("an upgrade bought in the shop races in a GARAGE room", async () => {
  const unit = SOL / 100n;
  const prices = [0, 1, 2, 3, 4, 5].map((d) => [1n, 2n, 3n, 4n, 5n].map((l) => unit * BigInt(d + 1) * l));
  // (prices are the admin's alone)
  await assert.rejects(send(await house.setPrices({ authority: keeper.publicKey, mint: NATIVE_SOL, prices, enabled: true }), [keeper]));
  await send(await house.setPrices({ authority: admin.publicKey, mint: NATIVE_SOL, prices, enabled: true }), [admin]);
  const buyer = await player();
  const t0 = await lamports(treasury);
  await send(await house.buyUpgrade({ owner: buyer.publicKey, payer: sponsor.publicKey, dept: 0, level: 1, maxPrice: unit }), [buyer, sponsor]);
  await send(await house.buyUpgrade({ owner: buyer.publicKey, dept: 2, level: 1, maxPrice: 3n * unit }), [buyer]);
  assert.equal(await lamports(treasury), t0 + unit + 3n * unit);
  assert.deepEqual(await house.levelsOf(buyer.publicKey), [1, 0, 1, 0, 0, 0]);
  // the same purchase twice fails instead of charging twice
  await assert.rejects(send(await house.buyUpgrade({ owner: buyer.publicKey, dept: 0, level: 1, maxPrice: unit }), [buyer]));

  // 700 + 10 + 19 = 729: under an 80.0 cap
  const { ixs, room } = await arena.createRoom({
    creator: buyer.publicKey,
    code: "GARAGE01",
    stake: SOL / 100n,
    class: CLASS_GARAGE,
    maxRating: 800,
  });
  await send(ixs, [buyer]);
  assert.deepEqual((await arena.fetchRoom(room)).levels, [[1, 0, 1, 0, 0, 0]]);
});

test("a sweep splits the treasury: half to the studio, the rest to prizes until the token exists", async () => {
  const floor = BigInt(await connection.getMinimumBalanceForRentExemption(0));
  const [o0, v0, held] = [await lamports(ops.publicKey), await lamports(vault), (await lamports(treasury)) - floor];
  assert.ok(held > 0n);
  await send(await house.sweep({ payer: resolver.publicKey }), [resolver]);
  const toOps = (held * 5_000n) / 10_000n;
  assert.equal(await lamports(ops.publicKey), o0 + toOps);
  assert.equal(await lamports(vault), v0 + held - toOps);
  assert.equal(await lamports(treasury), floor);

  const u = await tokens(ata(treasury, usdc, TOKEN_PROGRAM_ID));
  await send(await house.sweep({ payer: resolver.publicKey, mint: usdc }), [resolver]);
  assert.equal(await tokens(ata(ops.publicKey, usdc, TOKEN_PROGRAM_ID)), u / 2n);
  const t = await house.fetchTotals(usdc);
  assert.equal(BigInt(t.sweptOps.toString()), u / 2n);
});

test("the weekly cup: the keeper funds it from the prize vault, its winners claim", async () => {
  // the vault's USDC (a studio pool to start with)
  await give(usdc, TOKEN_PROGRAM_ID, vault, 1_000_000_000n);
  const v0 = await tokens(ata(vault, usdc, TOKEN_PROGRAM_ID));
  const now = await chainTime();
  const featured = Keypair.generate().publicKey; // the week's stock
  const { ixs, cup } = await house.fundCup({
    keeper: keeper.publicKey,
    mint: usdc,
    kind: "board",
    payoutBps: [5_000, 3_000, 2_000],
    guarantee: 100_000_000n,
    startsAt: now,
    endsAt: now + 6,
    featuredMint: featured,
  });
  await send(ixs, [keeper]);
  let c = await arena.fetchCup(cup);
  assert.ok(c.organizer.equals(vault) && c.featuredMint.equals(featured) && c.rentPayer.equals(keeper.publicKey));
  assert.equal(await tokens(ata(vault, usdc, TOKEN_PROGRAM_ID)), v0 - 100_000_000n);

  const winners = [await player(1), await player(1), await player(1)];
  const ranked = winners.map((w) => w.publicKey);
  const { root, proofs } = buildResultsTree(cup, ranked);
  await waitUntil(now + 6);
  await send(
    await arena.postResults({
      resolver: resolver.publicKey,
      cup,
      resultsRoot: root,
      scorers: 41,
      places: 3,
      standingsHash: hash(5),
      resultsUri: "https://acme.example/trial/2026-43.json",
    }),
    [resolver],
  );
  c = await arena.fetchCup(cup);
  await waitUntil(c.challengeEnds.toNumber());
  await send(await arena.finalizeCup({ cup }), [resolver]);
  // the winners claim themselves (where the swap into the week's stock would go)
  for (const i of [0, 1]) {
    const w = winners[i];
    await send(await arena.claimPrize({ claimer: w.publicKey, cup, rank: i + 1, wallet: w.publicKey, proof: proofs[i] }), [w]);
  }
  // after the claim window, the server pushes what's left to its winner
  c = await arena.fetchCup(cup);
  await waitUntil(c.claimUntil.toNumber());
  await send(
    await arena.claimPrize({ claimer: resolver.publicKey, cup, rank: 3, wallet: ranked[2], proof: proofs[2] }),
    [resolver],
  );
  assert.deepEqual(
    await Promise.all(ranked.map((w) => tokens(ata(w, usdc, TOKEN_PROGRAM_ID)))),
    [50_000_000n, 30_000_000n, 20_000_000n],
  );
  const k0 = await lamports(keeper.publicKey);
  await send(await arena.closeCup({ cup, payer: resolver.publicKey }), [resolver]);
  assert.equal(await connection.getAccountInfo(cup), null);
  assert.ok((await lamports(keeper.publicKey)) > k0); // the rent went back to the keeper
});

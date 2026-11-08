# @acme/client

TypeScript client for ACME's on-chain programs (`../README.md`):

- **arena** (`G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK`): live-race rooms and prize cups in SOL or allowed tokens, with a capped house fee.
- **house** (`5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw`): the treasury split, the prize vault's funded cups, the upgrade shop.

Program ids are the same on localnet and devnet. Mainnet will get new ones.

Every method returns `TransactionInstruction[]` and never signs: the race server builds a transaction, pre-signs it as gate and sponsor, and the player's wallet signs it. Every amount is in **base units** (`bigint`, `number` or `BN`): lamports for SOL, the mint's smallest unit for tokens. `NATIVE_SOL` (`PublicKey.default`) means SOL wherever a mint is expected. Token payouts need a token account to land in; the methods add creations for the ones that don't exist yet.

## Install

```bash
# built and packed in the game repo: cd solana && npm run pull (after a build)
npm i ./solana/client/acme-client-0.2.0.tgz
npm i @anchor-lang/core@1.1.2 @solana/web3.js@^1.98 @solana/spl-token@^0.4.13
```

## Use

```ts
import { Connection } from "@solana/web3.js";
import { ArenaClient, HouseClient, buildTransaction, buildResultsTree, DNF, NATIVE_SOL, stateName } from "@acme/client";

const connection = new Connection(RPC_URL, "confirmed");
const arena = new ArenaClient(connection);
const house = new HouseClient(connection);

const { tx } = await buildTransaction(connection, sponsor.publicKey, ixs); // v0, fresh blockhash
tx.sign([sponsor]);                                                    // then the wallet adds its signature
```

### Rooms

```ts
// the server's room id is the room's code; the sponsor pays the rent and gets it back at close
const { ixs, room } = await arena.createRoom({ creator, payer: sponsor, code: "AB12CD34", stake: 5_000_000n, mint: USDC, maxPlayers: 4 });
await arena.joinRoom({ player, room });                 // a start-when-full room locks as it fills
const open = await arena.fetchRooms({ state: "open", mint: USDC });  // memcmp-filtered lobby

// after the race and its re-run: times in seat order (DNF = no result), the record's sha256
await arena.settleRoom({ resolver, room, timesMs: [92_000, 90_500, DNF, 91_200], recordHash });
await arena.voidRoom({ resolver, room, reason: VOID_RACE });  // no result: every stake back
await arena.closeRoom({ room, payer: resolver });
```

A room moves `open` → `locked` → `settled` or `refunded` (`room.voidReason` says why). `roomOutcome(stake, feeBps, times)` gives the split before it happens. GARAGE rooms (`class: CLASS_GARAGE`, optional `maxRating`) read each player's levels from the house; `holderTokens: await arena.holderAccount(creator)` claims the holder fee.

### Cups

```ts
// the weekly time trial, funded from the prize vault by the keeper
const { ixs, cup } = await house.fundCup({ keeper, mint: USDC, kind: "board", payoutBps: [5_000, 3_000, 2_000],
  guarantee: 100_000_000n, startsAt, endsAt, featuredMint: NVDA });
const { root, proofs } = buildResultsTree(cup, [first, second, third]);
await arena.postResults({ resolver, cup, resultsRoot: root, scorers: 41, standingsHash, resultsUri });
await arena.finalizeCup({ cup });                       // after the challenge window
await arena.claimPrize({ claimer: first, cup, rank: 1, wallet: first, proof: proofs[0] }); // + the swap, in one tx
```

Until `claimUntil` only the winner can claim; after it anyone pushes the prize to the winner. Community hosts create Entries cups with `arena.createCup({ organizer, host: true | holderTokens, hostShareBps, … })`.

### The shop and the split

```ts
await house.buyUpgrade({ owner, payer: sponsor, dept: 0, level: 1, maxPrice: quote });  // the next level only
const levels = await house.levelsOf(owner);             // [pu, aero, tyre, brake, chassis, dur]
await house.sweep({ payer, mint: USDC });                // anyone: studio / prize vault / buyback
```

### Transfer-fee tokens

`fetchTransferFee(connection, mint)` reads a token's fee for the current epoch (`{ bps, maxFee }`, none for SOL and plain tokens); `grossForNet(stake, fee)` is what a player pays to stake, `netOf(amount, fee)` what a payout delivers; `fetchTransferFeeCeiling` is the higher of the fee in force and the scheduled one (the arena refuses new stakes above a token's cap).

### Admin (multisig later)

`arena.updateConfig / setResolvers / setHolder / setHost / setPaused / revokeResolvers / addRuleset / addMint / updateMint / cancelCup / rejectResults` and `house.setRoles / setFundCaps / proposeSplit / applySplit / setTokenMint / setPaused / addMint / setPrices`. Each takes the signer's public key and returns instructions, so they can go into a Squads proposal unchanged.

### Events

`arena.parseEvents(tx.meta.logMessages)` and `house.parseEvents(...)` decode the programs' events (`roomSettled`, `prizeClaimed`, `swept`, `upgradeBought`, …).

## Build

From `solana/` in the game repo: `bash scripts/build-copy.sh push`, then in `~/acme-solana` run `npm run check` and `npm run pack`. Back in the repo, `bash scripts/build-copy.sh pull` brings `dist/` and the `.tgz` back. `test/e2e.test.mjs` exercises this package against both programs on a local validator.

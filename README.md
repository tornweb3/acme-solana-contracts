# ACME — Solana programs

This Anchor workspace holds the game's two programs and the TypeScript client its server uses. The design and its reasoning are in [`../docs/solana-plan.md`](../docs/solana-plan.md), version 3 (2026-10-19).

| | Program id (localnet + devnet) | Keypair |
|---|---|---|
| **arena**: player money: live-race rooms and prize cups | `G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK` | `~/.config/solana/acme/arena.json` |
| **house**: the house's money: the treasury split, the prize vault, the upgrade shop | `5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw` | `~/.config/solana/acme/house.json` |
| Deployer / test wallet | `DZX2YhDQGvWdZyWZR81qqgCHoRoGCYuiWL8qD9UQkxpx` | `~/.config/solana/acme/deployer.json` |

`garage` (NFT cars) was deleted on 2026-10-19; its keypair `garage.json` stays where it is (keys are never deleted).

**Toolchain (pinned):** Anchor 1.1.2, exactly, for the CLI, `anchor-lang`/`anchor-spl` and `@anchor-lang/core`; Agave 3.1.10; platform-tools v1.52 (rustc 1.89.0).

**Keypairs never live in this directory, and mainnet gets NEW program keypairs.**

## arena

The rules every instruction keeps:
- Money leaves escrow only to that room's or cup's own players, the frozen treasury (the coded fee plus rounding dust), the frozen prize vault (a surplus only) or whoever paid the rent.
- Terms are frozen when a room or cup is created.
- Every state has a deadline after which anyone can refund.
- Pausing stops new money coming in. It never stops settling, refunds, claims or closing.

**Room** (a live race's pot, 2–8 players, equal stakes in SOL or an allowed token):
1. `create_room(code, params)`: `code` is the race server's 8-character room id. The creator stakes and takes seat 0; `payer` (the server's sponsor, or the creator) pays the rent and gets it back. A creator holding at least `holder_min` of the project token gets the holder fee for the whole room. With a gate set, the gate co-signs.
2. `join_room` until full. A room that starts when full locks as the last seat fills; a host-started one is locked by the resolver (`lock_room`, two players or more). Locking draws `seed = hash(room ‖ latest slot hash ‖ time)`.
3. The race runs on the race server, which re-runs it from its record.
4. `settle_room(times_ms, record_hash)` (resolver, before the settle deadline) pays in the same transaction: the fastest takes the pot less the fee; tied best times split it, dust to the treasury; everyone tied or nobody finishing (every time `DNF` = `u32::MAX`) refunds all, no fee. Only the winners' accounts are passed.
5. `void_room(reason, record_hash)` (resolver): no result, every stake back, no fee.
6. `refund_room` (anyone): an open room after its join deadline, a locked one after its settle deadline.
7. While a room is open, a non-creator can `leave_room` and the creator can `cancel_room`.
8. `close_room` (anyone): harvests withheld transfer fees, closes the escrow, rent to its payer, any surplus to the prize vault.

**Classes:** EQUAL races the full car for everyone. GARAGE snapshots each player's levels from the house's `["upgrades", player]` account when they join (no account yet = the stock car), under an optional rating cap (`700 + Σ level × the ruleset's points`, in tenths).

**Cup** (a prize event over a window):
1. `create_cup(id, params)`: by the house (admin, a resolver, or the prize vault through `house::fund_cup`) or a community host (approved with `set_host`, or holding at least `host_min`). **Entries** cups take entries; **Board** cups (the free weekly time trial) take none and pay from the guarantee. `featured_mint` names the event's stock.
2. `enter_cup` (Entries) until `ends_at`.
3. `post_results` (resolver): a Merkle root over `(rank, wallet)` for the paid places, the number of scorers, the standings' URI and hash.
4. The challenge window: the guardian or admin can `reject_results`, and the resolver posts again.
5. `finalize_cup` (anyone) fixes the pool: max(guarantee, entries − fee); `claim_until` = now + the claim window.
6. `claim_prize(rank, wallet, proof)`: until `claim_until` only the winner (who can swap the prize into the featured stock in the same transaction); after it anyone pushes it. Always paid to the leaf's wallet.
7. `close_cup`: the unused guarantee to the sponsor, the fee and dust to the treasury, a community host's share to the host, any surplus to the prize vault, rent to its payer.
8. No results by `results_deadline`, or cancelled: `refund_entry` per entrant, `return_guarantee`, then close.

**Admin:** `initialize` (the upgrade authority only); `update_config` (windows within hard bounds); `set_resolvers` (up to 4); `set_holder` (the project token, once, and its thresholds); `set_host`; `add_ruleset` / `update_ruleset` (a build's hash and rating points); `add_mint` / `update_mint` (SOL is `NATIVE_SOL` = all zeros; Token-2022 mints with a transfer hook, permanent delegate, pause or other non-metadata extension are refused, and a transfer fee is accepted only with the flag and under its cap); `propose_admin` / `accept_admin`. **Guardian:** `set_paused` (only the admin unpauses), `revoke_resolvers`, `reject_results`, `cancel_cup`.

## house

- `initialize` (the upgrade authority only): the split, the keeper, the studio's ops wallet, the funding caps; gives the `treasury`, `prize` and `buyback` PDAs their rent.
- `sweep(mint)` (anyone): splits the treasury's balance by the split (5000 / 3000 / 2000 bps: studio / prize vault / buyback). No project token yet: the buyback share goes to prizes. The project token: its share is burned. Other tokens: it builds up for the buyback auction.
- The buyback auction (P5): `set_auction_floor(lot_mint, min_lot, min_price)` (admin: a coin's lowest rate, needed before its first auction; never the project token), `start_auction(lot_mint, lot, start_price, floor_price, decline_secs, run_secs)` (keeper: a quarter of the vault at most, a day apart, the floor no lower than the admin's rate nor, for a week after a sale, half its rate), `buy_lot(max_price, min_lot)` (anyone: the price, falling from start to floor, burned from the buyer's own account of the project token; the lot paid from the vault).
- `propose_split` (admin) and `apply_split` (anyone, a week later).
- `fund_cup` (keeper): creates an arena cup by CPI, the prize vault as organizer and sponsor; at most `fund_max_bps` of the vault's balance in that token (hard cap 50 %), at most once per `fund_every_secs`.
- The shop: `add_mint` (a price table and totals per token), `set_prices` (admin or keeper), `buy_upgrade(dept, level, max_price)`: the next level only, paid into the treasury. The levels live in `["upgrades", owner]`, in the fixed layout arena reads.
- `propose_token_mint` (admin, while there is none; a token taking at most 5 % a transfer) and `apply_token_mint` (anyone, two days later): the project token, once. `set_roles`, `set_fund_caps`, `set_paused` (sales and auctions only), `propose_admin` / `accept_admin`. A token taking more than 5 % a transfer isn't sold in.

## Build and test

Rust doesn't build at a usable speed on `/mnt/c` under WSL, so the workspace builds from a copy at `~/acme-solana`. Move source there **only** with the script. It checks the copy is byte-identical and installs both program keypairs as `target/deploy/<program>-keypair.json`.

```bash
bash scripts/build-copy.sh push       # repo -> ~/acme-solana, verified
cd ~/acme-solana
npm run check                         # anchor build + fmt + clippy -D warnings + Rust tests + client build + e2e on a local validator (~2 min)
npm run pack                          # client/acme-client-<version>.tgz
cd -
bash scripts/build-copy.sh pull       # lockfiles, idl/, client/dist and the .tgz back into the repo
```

| Tests | What |
|---|---|
| `programs/arena/src/{payout,merkle}.rs` (unit) | The room split, cup prizes (50/30/20, fewer scorers than places, the guarantee, the host share), "never pays more than the pot", Merkle proofs |
| `programs/arena/tests/svm/{admin,room,cup,cu}.rs` (LiteSVM, clock control) | Init gating, hard caps, pause and revoke, two-step admin, unsafe mints, the holder mint once, the gate; rooms in SOL, SPL and a 1 % token (exact stakes, net payouts, a scheduled fee rise, harvest at close), winners-only payouts with a closed loser account, wrong payees, ties, every deadline exit, GARAGE snapshots and forged upgrades, the holder fee, surpluses, a randomized solvency sweep; cups (board and entries, proofs, the claim window and push, reject and repost, cancel and refunds, hosts and holders, the host share, a 1 % token); compute units and wire sizes of the heaviest paths |
| `programs/house/tests/svm.rs` (LiteSVM, both programs) | Init gating and vault rent, SOL and token sweeps, dust, the burn, the split's timelock, two-step admin, the shop (levels in order, `max_price`, pause, token accounts, the account arena reads), a GARAGE room racing bought levels, `fund_cup` caps through the CPI in SOL and USDC |
| `client/test/e2e.test.mjs` (`anchor test --validator legacy`) | The built client against both programs on a validator: a USDC room, a 1 % ACME room, an 8-player refund in one transaction, an upgrade raced in a GARAGE room, a sweep, the weekly cup funded by the house and claimed |

Notes:
- `anchor test` loads both programs as **upgradeable with the provider wallet as upgrade authority** (`[[test.genesis]]` in `Anchor.toml`). Anchor's default load has no authority, and `initialize` would refuse it.
- The LiteSVM harness starts its clock at 2026-10-19 (LiteSVM's starts at 0, which no chain shows).
- Large `Accounts` structs are `Box`ed, to stay inside the 4 KB SBF stack frame.
- Program sizes: arena ≈ 780 KB, house ≈ 455 KB, so about 5.6 and 3.2 SOL of rent at exact size.
- The heaviest path, a settlement paying 7 tied winners and the fee in a fee token, takes 72 k of the default 200 k compute units and 675 of 1,232 bytes; the client adds creations only for token accounts that don't exist.
- Adding a crate that has an `idl-build` feature means adding it to the program's `idl-build` feature list too (house lists `arena/idl-build`).

## Devnet

Fund the deployer with about 8 SOL: the programs' rent (house 2.4 SOL, arena 4.1 SOL; devnet's rent, 2026-10) and
1.7 SOL for the race server's keys. https://faucet.solana.com gives 5 SOL a request; the CLI's airdrop is refused from
this machine. Then, from the build copy, each program with a buffer key of its own, so an upload the RPC cuts short
resumes from where it stopped (and its SOL is never stranded); not with `--use-rpc`, which the public RPC throttles:

```bash
K=~/.config/solana/acme
for p in house arena; do
  [ -f $K/devnet/$p-buffer.json ] || solana-keygen new --no-bip39-passphrase --silent --outfile $K/devnet/$p-buffer.json
  solana program deploy target/deploy/$p.so --url https://api.devnet.solana.com --buffer $K/devnet/$p-buffer.json \
    --program-id $K/$p.json --keypair $K/deployer.json --upgrade-authority $K/deployer.json
done
```

`solana program dump <id> out.so` and a sha256 of its first `stat -c %s target/deploy/<p>.so` bytes show the deployed
program is the build's. An upgrade is the same command (a build larger than the deployed one extends its program
account first, by itself, at that size's rent; the buffer's rent comes back).

**On devnet (2026-10-19):** both programs, the builds after the security review (`c00262a`), each checked byte for
byte against its build - house `5rV9…` (upgraded, slot 512745763, sha256 `2335203564a5db8f…`), arena `G8hR…` (slot
512746136, `ae6248542e546684…`); the deployer is the upgrade authority. `setup.mjs --yes --usdc own --pool 500` ran on 2026-10-19: the house and the arena set up, ruleset 1
(`f2-rules-6`), SOL and our own test USDC (`Bd54UKFSiCJ3wf8pUMEuTSzzYP6kgpqhJeEVtAEFWQEA`, 6 decimals, the faucet key
mints it), the workshop's USDC prices, the desk's test stocks (NVDAx `HrVc…ixgn`, TSLAx `FwvR…EAV`, AAPLx `5vv4…w3HE`,
SPYx `5mbZ…u75h`, MSFTx `Bzku…ib9C`; all in `src/ui/lib/chainIds.js`), 500 test USDC in the prize vault. The race
server's keys are in `~/.config/solana/acme/devnet/` (resolver `A7sd…kAUM`, gate `Cf8j…Tx4T`, sponsor
`7Cgs…cC2k`, keeper `A21i…VaqV`, faucet `FoBe…YYfs4`, desk `APfw…TmWcz`). `devnet-smoke.mjs` then raced a USDC room
end to end on devnet: faucet, room `Y4PCW15T` opened and joined (locked, seed drawn), raced and checked, the winner
paid 1.9 USDC in the settlement with the record's hash
([settle](https://explorer.solana.com/tx/46euB3aQrtRhSoUwNs11yZ9FWU33E1xwtLQWGY13jeRn56wr2vt3ucwiAF71SNHBM5dQdRvKZrzJkTW6fV2tdMbF?cluster=devnet)),
the room closed.

Then, from the repo, the rest in one command (`tools/chain/setup.mjs`; the deployer, the upgrade authority, signs):

```bash
node tools/chain/setup.mjs                 # what it would do: nothing is sent or written
node tools/chain/setup.mjs --yes [--pool 500] [--usdc circle|own|<mint>]
```

**P5 on devnet (2026-10-20):** `setup.mjs --usdc own --acme --yes` made the test ACME
`8eVbsZDLeLSPoeSZGA6DfrtnFWccspVe7BEU5vjUzwJA` ("ACME (devnet test)": Token-2022, 6 decimals, 1 B, 1 % a transfer up to
the whole supply, its fee authorities the simulator key `stonkfun.json`, no freeze authority)
([the mint](https://explorer.solana.com/tx/4NB9eA19KBRPjyZJJdnFB4ccmMobDMg1wyX2EkRPxmrUbwqPiWivaTdKeQhFCdkrx9qGh3zo97zGjDogRzjz3WaY?cluster=devnet);
[its supply to the faucet, its mint authority revoked](https://explorer.solana.com/tx/5t4mdLhZResTi1443f3qXn9xBWd1UDaSnNBnRg5EiEqzrLecpZPsRyaDjEpNVmJKJSi8Hr6xD5NtXUtwEXzFjGg1?cluster=devnet)),
then ACME in the arena (stakes 2,500 to 250,000, rooms at 2.5 %, its tax taken up to 1 %:
[tx](https://explorer.solana.com/tx/3DMThVsgs9QQ7S9TJ8cpS5oaDBqSSwdF7LKaMig7jKB9LMwjvyJjiJDDr7LTkEvkrvHCrR8Qa4u4dCxaRvtzTrwd?cluster=devnet)),
in the house ([tx](https://explorer.solana.com/tx/337ZvVDD4a2Q6oDMkCxrsDudsfvHZsT2znot8MzdVD11VWErwgy2JiMqjMHBczqpj47C2AZ7AKsnVjrMW8po6nZs?cluster=devnet)),
the workshop's ACME prices ([tx](https://explorer.solana.com/tx/4czYG17QfxxKFkxRmGJ8fNFvLvcUpAjEY9tztXsjhow4eMkx82frDtRGWUktBdVX5UacfR2mBEE7Abfp8CJETf9Z?cluster=devnet))
and the holder perks at 1 M ACME, about $400 at the devnet $0.0004
([tx](https://explorer.solana.com/tx/eNpvqFNFr556ibSesazVkfDEyQaTiYDHE7W5KcQhzQeeMpmsuxYH6gRwDGbkcNnrmYkeNqFcB7N19txESJchEZi?cluster=devnet)).
`devnet-smoke.mjs --vote` then gave two wallets test ACME with their USDC
([drip](https://explorer.solana.com/tx/EfLzdi6ee7c49g9nQaSTyKnpQuYj1Ny4sckdvK4wjXkFurQnCcei9QnywwmtjGpnHY79qJsusKGjGdSmMZ5PdQL?cluster=devnet)),
raced a USDC room ([settled](https://explorer.solana.com/tx/2KENHnmfS2XDatWRwVhvVGTs4iY3dDiRdQsJ8ZB7SqQuiLnv2YWtddVowkJsZaJfo2dCbEGZ57H59QhjxgX6LNNF?cluster=devnet))
and cast a ballot each. `stonkfun-sim.mjs --yes` ran StonkFun's round: the two drips' 25,000 ACME of tax
[harvested](https://explorer.solana.com/tx/3uohMfY9sd8LepnttuUErXP3AThrfNut8PdAJHKWmvhQ87zDFrpk83DdhYzMPWrWMbeCfy4tw4uQowUDGPMRKSxz?cluster=devnet)
and [withdrawn](https://explorer.solana.com/tx/2zg6pjbpzYomBuKNaLcPT1x7WkPEWNzULnhTYfJk8itqjcLdJe6jUz5dwXQsdEEJz7nVQSin2hFbU8g6YbiJt4ud?cluster=devnet)
to the simulator, "sold" at $0.0004 for 0.0667 SOL and [paid](https://explorer.solana.com/tx/27DLZDULb6aPsC25rAxFvKXubA4tDYifU9n7oZVstRZLkJ8ZF52TXpW4ZWeXDhjTFUkCt3xkP7utoLJAQp2ZsyeG?cluster=devnet)
to the two holders of $20 or more. **Not on devnet yet:** the P5 builds of both programs. One after the other, the
upgrades take about 4.8 SOL at the start (house's buffer 3.0 and 0.5 to grow its program account, then arena's buffer
4.2; each buffer's rent comes back); the deployer has 3.10, and the CLI's airdrop is refused from this machine. With
the SOL there: the two `solana program deploy` commands above (house first), then `setup.mjs --usdc own --acme --yes`
proposes the project token and sets the auctions' lowest rates (half the devnet price), and a run two days later
applies the token, after which the keeper's auctions start; ACME rooms on devnet need the new arena (the client passes
the memo program, which the old arena counts as a payout account).

It makes the race server's keys (one file per role in `~/.config/solana/acme/devnet/`, with 1, 0.2 and 0.5
SOL for the sponsor, keeper and faucet: raise `TERMS.topUp` and run it again to add more), initializes the house (50 / 30 / 20; a cup takes at most a fifth of the prize vault,
once every 6 days) and the arena (the gate and resolver keys; settle 6 h, challenge 24 h, results 3 days, claims 7
days), adds ruleset 1 (sha256 of the game's rules as canonical JSON, the departments' rating points), SOL and USDC in
both programs, the workshop's USDC prices and the desk's test stocks, puts the studio's pool in the prize vault, and
prints the server's settings. Each step looks first, so it can run again; it refuses mainnet. `tests/chain/setup.test.js`
runs it on a local validator with the programs deployed and nothing else. `node tools/chain/devnet-smoke.mjs` then
checks it end to end: two new wallets given test money, a USDC room opened, joined, raced, paid and closed on devnet. Before mainnet, move the upgrade authorities
and admins to a Squads multisig.

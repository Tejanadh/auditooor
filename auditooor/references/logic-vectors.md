# Logic Vectors — scenario · property · key-variable · confirm

The finding muscle, structured. GPTScan (ICSE'24) showed **~80% of web3 bugs are
logic bugs no fixed-pattern tool catches**, and the way to catch them without
drowning in false positives is a three-part decomposition per bug *type*:

- **scenario** — *where* it lives (which functions to even look at)
- **property** — the *exact* broken behaviour that makes it a bug
- **key variable** — the one value whose dataflow decides it

A hunter matches scenario+property; then a cheap deterministic pass confirms the
key variable actually flows the way the bug needs — and *refutes* it cheaply when
it does not, before a PoC is ever forged. That refutation is the whole point:
`exploit-patterns.md` tells you what pays; this tells you how to tell a real
instance from a look-alike **without** spending a PoC to find out.

`{scan} confirm <file> --function F --check cei|guard|unchecked-return` runs the
deterministic pass for the shapes regex can decide. It is a **filter, not a
prover** — CONFIRMED means "worth a PoC", REFUTED (exit 3) means "drop it",
INCONCLUSIVE means "regex can't tell, the human/PoC decides". The fork PoC stays
the only proof.

## How a hunter uses this

1. From the impact map, pick a function and the vector whose **scenario** matches.
2. State the **property** as a concrete claim about the **key variable**.
3. If the vector has a `confirm` check, run it. REFUTED → kill the lead, log it,
   move on. CONFIRMED / INCONCLUSIVE → keep, escalate to skeptic then PoC.
4. A candidate that cannot be phrased as "property P on key-variable V is
   violated" is not a finding yet — it is a vibe. Sharpen or drop it.

## The catalog

Ordered by what pays (`exploit-patterns.md` P-weights). Each row is a hypothesis
template, not a detector — the LLM supplies the judgement, `confirm` supplies the
cheap filter.

### L1 · Reentrancy / stale-read via CEI violation  `confirm: cei`
- **scenario:** any function that makes an external call (`.call`, `.transfer`, a
  token/NFT hook, a callback) *and* writes state.
- **property:** a state variable that a second entrant reads is written **after**
  the external call, so the re-entered call sees stale state.
- **key variable:** the balance/share/flag written after the call.
- **confirm:** `--check cei`. CONFIRMED = a write follows the call; REFUTED = the
  function is checks-effects-interactions and this shape is absent.
- **payout note:** the write-after-call shape is necessary, not sufficient — a
  `nonReentrant` guard or a same-actor-only path can still make it safe. Confirm
  the shape, then argue reachability.

### L2 · Unguarded value sink  `confirm: guard`
- **scenario:** a function whose body moves value (`transfer`, `_mint`, `call{value}`).
- **property:** no `require`/`revert` gates the sink, and the caller is unprivileged.
- **key variable:** the recipient/amount, and `msg.sender`.
- **confirm:** `--check guard` (statement-level) **plus** `{scan} entries` (is the
  function permissionless at all). Both must agree before this is real — `confirm`
  sees the body, `entries` sees the modifier.

### L3 · Unchecked low-level call  `confirm: unchecked-return`
- **scenario:** any `.call` / `.delegatecall`.
- **property:** the success boolean is dropped or never `require`d, so a failed
  transfer/settlement is treated as success.
- **key variable:** the returned success flag.
- **confirm:** `--check unchecked-return`.

### L4 · Directional rounding, amplified  `confirm: —` (fuzz)
- **scenario:** any `mulDiv` / integer division on a value/share/fee path.
- **property:** rounding favours the caller in a direction that repeats (mint
  rounds shares up, or redeem rounds assets up).
- **key variable:** the share↔asset conversion.
- **confirm:** no regex check — this is a *fuzz* target. Author the round-trip
  invariant (`invariant-library.md`) and let Foundry shrink it. Reasoning alone
  never confirms a rounding bug; the shrunk sequence does.

### L5 · Donation / raw-balance accounting  `confirm: —` (money-map + fuzz)
- **scenario:** any decision that reads `token.balanceOf(address(this))` directly
  instead of a tracked internal accounting variable.
- **property:** an attacker can `transfer` tokens in to move the raw balance and
  shift a price/share/limit the contract derives from it.
- **key variable:** the raw balance read.
- **confirm:** money-map lane + a conservation invariant. `detectors` ACC-01/02
  flags the raw read as a lead.

### L6 · Stale / cross-function state  `confirm: —` (fuzz)
- **scenario:** a value cached in one function and consumed in another across txs.
- **property:** the cache is not refreshed on a path that changes the underlying,
  so a later call acts on a stale number.
- **key variable:** the cached value and its refresh sites.
- **confirm:** stateful fuzz (P6/P8) — a two-transaction shrink, never eyeballs.

### L7 · Oracle / freshness contract  `confirm: —` (read + skeptic)
- **scenario:** any external price/rate read.
- **property:** staleness/round/deviation is not checked before the value drives a
  liquidation, mint, or redemption.
- **key variable:** the price and its timestamp/round id.
- **confirm:** read the freshness check; if absent, a fork PoC at a stale block.

## Honest ceiling

`confirm` decides only L1–L3 — the shapes with a local, syntactic signature. The
paying logic bugs L4–L7 live in relationships **between** values across calls and
transactions; those are confirmed by the **invariant campaign** (`invariant-library.md`,
`{scan} harness`), not by any single-function pass. That division is the point:
the cheap filter kills the cheap-to-check look-alikes, and the fuzzer spends its
budget on the bugs that actually need it. Neither is the LLM's replacement — they
are what stop the LLM's hypotheses from each costing a PoC to disprove.

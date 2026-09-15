# Invariant discovery fleet — the 89%

Canonical properties (solvency, conservation) are table stakes and already emitted by `{scan} harness --system`. The bugs that pay violate **protocol-specific** economics. This file is how `/auditooor FUZZ` authors those properties, implements them, and turns a break into a fork PoC.

Spawn **five discovery agents in parallel**, then one synthesizer. Do not spawn the 12 hacking-agent fleet here — that is a different mode (`hunting-fleet.md`).

## When to run

- User invoked `/auditooor FUZZ`.
- X-ray posture is `INVARIANT`, `HUNT`, or `SEAM` and Phase 0 said `EV: PROCEED`.
- Skip on `FORTRESS` / EV abort.

## Step 1 — pack + mechanical harness

```
{scan} pack <dir> --out <dir>/auditooor-recon
{scan} harness --system <src_dir> --out <test>
```

Start from `auditooor-recon/PROPERTIES.md` — do not invent deposit↔withdraw from scratch; it is already seeded. Fill `setUp()` wiring (the one manual step). Seed **other depositors'** funds (3 actors are already in the generated handler). Do not invent a deploy graph if the repo already has one — reuse it. The handler emits **clamped** `h_*` and **unclamped** `h_raw_*` wrappers; keep both — donation/inflation lives in the raw path.

## Step 2 — five discovery agents (one message, parallel)

Each agent reads: `PROPERTIES.md`, `xray.json` (`deltas.gaps`, `census.protocol_types`), `references/protocol-threats.md` (matching types only), in-scope source, the generated harness, and `invariant-library.md`. Each returns properties tagged `SHOULD-HOLD` (docs/spec/exact identity, cite it) or `EXPLORATORY` (inferred). No proof, no report. One-sided `deltas.gaps` are the first place to look — a function that only credits is the sibling of a drain.

| Role | Obsession |
|---|---|
| Conservation | Every aggregate (`total*`, `sum*`, `accumulated*`) equals the sum of its parts across **all** write sites. A function that writes one side only is the candidate. |
| Round-trip | Every forward+inverse pair (deposit/withdraw, mint/burn, …). `forward(x); inverse()` must return **≤ x** absent explicit fees. Rounding direction at *this* protocol's scale. |
| State machine | Enum/one-shot latches, authority monotonicity, supply/rate caps that must hold in aggregate not per-call. |
| Adversarial profit | Flash-loan sized attacker. Donation/inflation, first-depositor, share-price jump, unbacked reward from a sibling contract. If they cannot name a profit path, they emit nothing. |
| Protocol-type | Detect vault / lending / AMM / staking / escrow and apply **this protocol's** numbers (LTV after *this* rounding step, tick spacing, epoch bounds) — not a generic template. |

Prompt each with: target path, x-ray permissionless money map, "you author properties, you do not hunt signatures, you do not write the report."

## Step 3 — synthesizer (one agent, after the five)

Merge, dedup by (state variables, predicate shape), drop anything that is just a per-call `require` restated as an invariant (`xray.md` / `invariant-library.md`: guards ≠ invariants). Write:

- `PROPERTIES.md` in the target (or `/tmp/auditooor-properties.md` if the tree must stay clean): checkbox list with stable IDs `P-01…`, Guarantee tag, which files.
- Rank: `SHOULD-HOLD` + value-touching first.

## Step 4 — implement and run

Edit the generated harness: ghosts in the handler, assertions for every `P-NN` you actually implement. Leave unimplemented IDs unchecked. Then:

```
forge test --match-path <test> -vvv
```

A break is a **candidate**. Foundry's shrinker output is the PoC seed.

## Step 5 — gates, then fork PoC

Same gates as any lead (`impact-model.md`, `novelty-gate.md`, `proof-standard.md`):

- `SHOULD-HOLD` break + unprivileged profit on the impact map → forge `{scan} harness <file> --fork --address <live> --rpc-env <ENV> --out <poc>` and execute.
- `EXPLORATORY` break → LEAD for the hunter, not a finding, until intent is checked.
- Duplicate / known-class / intended fee → kill, record `outcome`.

No live RPC → stop at the shrunk Foundry sequence and say so. Do not submit a fresh-deploy unit test to Immunefi.

## Hard rules

1. Five discovery agents, one synthesizer. Not 1–2 generic "write invariants" agents.
2. Properties that cannot be falsified by a call sequence are not properties.
3. Coverage theatre is not a goal. A 40% harness that hits the money pair beats a 90% harness that never withdraws.
4. Do not require Echidna or Medusa. Foundry invariant + shrink is the default; other fuzzers are optional if already in the repo.

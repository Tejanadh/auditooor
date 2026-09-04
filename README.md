<h1 align="center">auditooor</h1>
<p align="center"><b>A reward-first smart contract audit methodology.</b></p>
<p align="center"><i>Optimises dollars-per-run, not findings-per-run.</i></p>

<p align="center">
  <img src="https://img.shields.io/badge/Focus-Target%20selection%20%2B%20invariants-627EEA" />
  <img src="https://img.shields.io/badge/Chains-EVM%20%C2%B7%20Solana%20%C2%B7%20ZK-9945FF" />
  <img src="https://img.shields.io/badge/Bias-Proof%20over%20severity-2ea44f" />
</p>

> This repo documents the **methodology** I hunt with — how I choose *where* to look, what I hunt *for*, and the gates a candidate crosses before it becomes a report. It is deliberately a doctrine, not a tool dump: the value is the judgment, and judgment is what transfers.

---

## The one thesis that reorders everything

**Live bugs are usually simple. They hide in complex paths.** Most real fixes are a single missing check — but the check is missing at the end of a branch every prior auditor stopped reading before.

So the first move is never "read the code top to bottom." It is **rank by money-proximity × path-complexity** and start at the value sink buried deepest. A deep path with no value at the end is not a target. A value sink at the end of a deep path is *the* target.

Complexity alone is a weak signal; complexity **combined with value-flow** is the strong one. Simple, clean codebases shouldn't have exploit paths — I don't grind them. I grind large DeFi, math-heavy systems, and many-integration protocols.

---

## The bug era this is calibrated for (2025–2026 loss data)

The distribution moved, and it decides where the dollars are:

- **~89% of protocol losses are protocol-*logic* bugs** — application-specific failures in accounting, collateral, state transitions, governance, and economic assumptions. These have **no per-file syntactic signature**; they live in the relationships *between* values across contracts. Pattern scanners cannot see them. **Invariants can.**
- **Templated classes collapsed to <1%.** Flash-loan oracle manipulation and composability reentrancy fell from ~19% of losses (2022) to under 1% (2025) — mature oracle designs and reentrancy guards did their job.
- **OWASP SC Top 10 (2026):** Access Control still #1 by lifetime loss; **Business Logic is now #2**; **Proxy/Upgradeability is a brand-new category** (storage collisions, uninitialized proxies, upgrade-path bugs).

**Consequence:** the paying frontier is **novel logic in complex protocols, found by invariants** — not signature-matching on fresh launches (that lane is commoditised and gone within hours of a program opening).

---

## How it hunts — invariant-first

Pattern detectors triage. **Protocol-specific invariants do the hunting.** The properties that pay are never the canonical ones (solvency, conservation) — those are table stakes. They are the ones authored *per protocol* from its intended economics:

- *"this vault's share price is monotonic except on a realized loss"*
- *"borrow ≤ collateral × LTV after **this** rounding step"*
- *"total external withdrawn ≤ total external deposited, per token, across any call sequence"*

Authoring those from the code's intended economics — and then fuzzing them across the whole system with Foundry / stateful campaigns — is the core loop. It is also **the part no tool can do for you**, which is exactly why it's the edge.

---

## The gates — how a lead becomes a report

Most "findings" die here, on purpose. A candidate crosses four gates or it isn't written up:

| Gate | Kills | The question |
|------|-------|--------------|
| **EV pre-check** | dead/unreachable value, bad-faith programs, out-of-scope | *is the money real, reachable **now**, and will this program actually pay?* |
| **Impact** | trusted-role-only, temp-DoS/griefing, dev-code not matching the live deployment | *does an **untrusted** actor lose or lock real funds?* |
| **Novelty** | public duplicates, already-ledgered issues, known-issue-list matches | *has this exact unit already been reported?* |
| **Proof** | anything unprovable | *does a deterministic PoC on the real code reproduce it?* |

Two rules that never bend:
1. **Unprovable ≠ finding.** No PoC against the real implementation (live RPC / mainnet fork), no report.
2. **Novelty is checked *before* the PoC is built** — proving a duplicate costs time and pays nothing.

---

## Target-selection lanes (ranked by EV, not by hype)

- **Sharpest — the un-audited periphery of an otherwise-audited protocol.** The core is a fortress; the *seams* (routers, wrappers, zaps, migration helpers, cross-integration glue) still hold reachable value and nobody swept them.
- **Best — novel logic in complex protocols** a shallow scanner can't reason about. This is where the 89% lives.
- **Good — dormant value + non-trivial logic:** legacy vaults / DAOs / treasuries where money still sits, nobody's watching, and bug classes discovered *after* the code shipped now apply.
- **Thin — dormant *simple* code** for a lone access-control miss. Worth one pass (AC is #1 by lifetime loss), not a campaign.

Maturity picks the tactic: **sloppy code → hunt the low-hanging unguarded entrypoints; clean/complex code → hunt the deepest, branchiest value path and the newest mechanism.** Recently-changed and "gas-optimized" code is weighted up — optimizations hide the edge cases the devs didn't anticipate.

---

## Archetypes — which hunter to be on this target

The posture changes with the target. The five I switch between:

- **Miner** — deep, single-target excavation of one complex protocol.
- **Differ** — hunt the *diff*: what changed between versions, what a fix moved instead of killing, what a deleted guard was secretly load-bearing for.
- **Speedrunner** — fresh program, race the clock, breadth over depth.
- **Watchman** — long-running programs, dormant value, the contract everyone forgot is still live.
- **One-Day Hunter** — reproduce a just-disclosed class against everything else that shares the pattern.

---

## The honest ceiling — read this before believing any output

I state the limits, or the whole thing is hype:

- **Automated tooling is a first-filter (~40% of what a full manual audit finds).** Every major exploit of an *audited* protocol since 2020 involved a bug the tools didn't flag. The jackpot bugs are, by definition, in the 60% the muscle misses.
- **Generic invariants ≠ the paying invariant.** Canonical properties are table stakes; the bug that pays violates a *protocol-specific* one that must be authored by hand.
- **Novelty has a floor.** A world-search kills *public* duplicates. It cannot see another hunter's submission from three hours ago, or a finding in a private paid audit. On crowded programs, **target selection and speed beat novelty-checking.**
- **Autonomous swarms end in $0.** Spraying agents across fortress targets finds nothing and is net-negative after cost. The method points *scarce human hours* at the one seam where a bug still sits — it does not replace the human.

The winnable game: route to the un-audited seam, auto-generate the canonical invariants fast, and spend judgment only on the protocol-specific economic modeling no tool can do.

---

## What's in this repo

This is the **doctrine layer** — the reusable judgment. The operational engines (per-vector provers for EVM / Solana / ZK, the invariant harnesses, the outcome ledger) are kept private; a methodology is worth sharing, an edge is worth keeping.

Applied write-ups that were produced with this method live in my [**audit portfolio**](https://github.com/Tejanadh/audit-portfolio).

---

<sub>Independent work. Grounded in public loss data (2025–2026) and the code-complexity / vulnerability literature. Not affiliated with any audit firm. Responsible disclosure first.</sub>

<p align="center">
  <a href="https://github.com/Tejanadh">GitHub</a> ·
  <a href="https://x.com/TEJANadh10">Twitter / X</a> ·
  <a href="mailto:tejanadh927@gmail.com">Email</a>
</p>

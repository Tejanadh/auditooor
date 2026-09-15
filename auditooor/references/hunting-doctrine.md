# Hunting Doctrine — where the money actually is

Distilled from field guidance by working hunters (zhero-style archetypes; a $1M+/yr hunter's AI workflow) and grounded in the code-complexity/vulnerability literature (arXiv 2411.17343). This is the *posture* the whole stack hunts with. The crit* engines carry the vectors; this file carries the judgment.

## The one thesis that reorders everything

**Live bugs are usually simple. They hide in complex paths.** Most real fixes are a single missing check. Therefore:

- **Rank by path depth and branchiness, not by how clever the code looks.** `auditooor-scan surface` now emits `risk_score` = money-proximity × path-complexity, and ranks on it. The winning target is a value sink buried where every prior auditor stopped reading.
- Complexity is a **complementary** signal, never standalone (the literature: raw complexity correlates weakly alone, strongly when combined with value-flow). A deep path with no value sink is not a target; a value sink at the end of a deep path is *the* target.
- Simple, clean codebases shouldn't have exploit paths — don't grind them. Grind blockchains, large DeFi, math-heavy systems, and many-integration protocols.

## The honest ceiling — read this before believing any output

This stack is a **force-multiplier on the hunter's judgment, not an autonomous bug-finder.** Name the limits or the brand breaks:

- **The muscle is a first-filter (~40%).** Trail of Bits' own data: AI tooling catches ~40% of what a full manual audit finds, used as a first pass so humans focus on business logic and novel attack modeling. **Every major exploit of an audited protocol since 2020 involved a bug the tools did not flag.** The jackpot bugs are, by definition, in the 60% the muscle misses. The deterministic scanners (`surface`, `detectors`, `seams`) triage; they do not find the paying bug for you.
- **Generic invariants ≠ the paying invariant.** The system harness proves *canonical* properties (solvency, conservation). The bugs that pay violate *protocol-specific* invariants — "this vault's share price is monotonic except on realized loss", "borrow ≤ collateral × LTV after *this* rounding step". Those cannot be pre-written; they must be authored per-protocol from the code's intended economics. **That authoring is the moat, and it is 100% the human's job.**
- **Novelty has a floor.** World-search kills *public* dups. It cannot see the other hunter's submission from three hours ago or a finding in a private paid audit. On hot programs, first-valid-submission means **speed + target selection beat novelty-checking** (79/79 rejected on one crowded Code4rena program). See `novelty-gate.md`.
- **Autonomous sweeps end in $0.** 20-agent speed-kill runs across fortress targets found nothing, net-negative after costs. Do not run this as a swarm. Run it to *point your scarce hours* at the one periphery contract where a bug still sits.

The winnable game: the tool routes you to the un-audited seam, auto-writes the generic invariants fast, and spends your judgment only on the protocol-specific economic modeling no tool can do.

## The bug era you are actually hunting in (2025–2026 loss data)

The distribution moved, and it decides where the dollars are:

- **~89% of protocol losses are protocol-LOGIC bugs** — application-specific failures in accounting, collateral, state transitions, governance, and economic assumptions. Each is a unique flaw in a particular app. **These have no per-file syntactic signature** — they live in the relationships *between* values across contracts. `surface` and `detectors` cannot find them; the **system-level invariant campaign and the LLM engines** do.
- **Templated classes collapsed to <1%.** Flash-loan oracle manipulation and composability reentrancy fell from ~19% of losses (2022) to under 1% (2025) — mature oracle designs, reentrancy guards, and standard access-control patterns did their job.
- **OWASP SC Top 10 (Feb 2026):** Access Control still #1 ($953M lifetime), **Business Logic jumped to #2**, and **Proxy/Upgradeability is a brand-new category (SC10)** — storage collisions, uninitialized proxies, upgrade-path bugs.

**Consequence for this stack:** the deterministic muscle (`surface`, `detectors`) is sharpest on the machine-auditable classes (SIG/AC) — real, but a small slice of the money. **The paying frontier is novel logic in new, complex protocols, found by invariants.** So:

- **Promote the invariant harness from a per-file discovery gadget to the PRIMARY hunting surface.** Write protocol-level invariants — value conservation, collateralization/solvency, monotonic share price, no-free-money — and fuzz them across the *whole system*. This is how top Code4rena wardens and the winning academic frameworks actually catch logic bugs. `surface`/`detectors` triage; invariants hunt the 89%.

## Maturity → tactic (auditooor-scan detect prints this)

- **Sloppy / low-maturity** (low avg complexity, missing NatSpec, best-practices ignored) → hunt **low-hanging fruit**: unguarded entrypoints, missing checks, obvious access-control gaps. Sloppiness in the small predicts bugs in the large.
- **Clean / mature / complex** → hunt the **deepest, branchiest value paths** and **novel mechanisms**. The bug is a simple check nobody read far enough to find.
- **Innovation & optimizations are bug biomes.** New approaches leave attack paths unexplored; optimization commits hide edge cases the devs didn't anticipate. Weight recently-changed and "gas-optimized" code up.

## Competition & timing (feeds the EV pre-check)

- Vulnerabilities cluster **right after launch or when a bounty program first opens**. If you hunt fresh code, do it *immediately* — automation and other hunters are racing you.
- As a program ages, shift to the **most complex paths and novel ideas** — the simple stuff is gone.
- Hunt where competitors won't: **neglected chains, old contracts, unpopular projects, and the complex/unknown paths of popular ones**. Popular + simple = already found.

## The uncrowded lane: dormant value (2026 market reality)

The AI-scanner swarm (sub-cent scans, 1–2h full audits, 72% exploit-rate on curated benchmarks) has commoditised **fast, shallow scanning of fresh launches**. Because bounty is first-reporter-wins, shallow bugs on new, popular programs are gone within hours. Do not race there — it is the most crowded strategy in the market.

**But aim it correctly** — dormant *simple* code is mostly the templated classes that decayed to <1% of losses. The uncrowded lane pays when it combines *thin competition* with *logic depth*, not when it's just old-and-simple. Rank the plays:

- **SHARPEST — the un-audited periphery of an otherwise-audited protocol.** The audited core is a fortress; the *seams* — routers, wrappers, zaps, migration helpers, cross-integration glue added around it, "audit coverage unclear" — still hold reachable value and nobody's agent swept them (confirmed 2026 trend: $17M+ calldata injection in periphery routers). Run `auditooor-scan seams <dir>` to rank CORE vs SEAM and hunt the SEAM. This is a *concrete computed input*, sharper than any age heuristic.
- **BEST — novel logic in complex protocols the swarm can't reason about.** New accounting, new collateral models, new economic mechanisms. This is where the 89% lives and where invariants earn their keep. Complexity + innovation is the biome, not age.
- **GOOD — old / unmaintained protocols where money still sits AND the logic is non-trivial.** Dormant vaults, legacy DAOs, treasuries — value locked, nobody watching, and bug *classes discovered after the code shipped* now apply. The edge is the *combination* of dormant value + a logic bug, not dormancy alone.
- **THIN — dormant simple code for a lone SIG/AC miss.** Still worth a `detectors` pass (AC is #1 by lifetime losses and a missed initializer is a real crit), but don't expect a rich vein here; the templated stuff is largely gone.
- **Neglected chains and old deployments** of a protocol that's since moved on — the old contract is still live, still holds funds, still in-scope on a long-running program.
- **Complex/unknown paths inside popular protocols** — the swarm skims the obvious entrypoints; the deep path is yours (this is the `risk_score` thesis).
- **Verify the money is real and reachable *now*** (`impact-model.md#ev-precheck`): dormant is good, *dead* is worthless — a bug in a contract holding $0 or unreachable pays nothing.

Root-cause reality check (what actually paid / broke in 2025–2026): access-control takeover (#1 loss class), rounding-direction (Balancer $120M), AMM-math overflow (Cetus $223M), reentrancy (GMX $40M), flash-loan+oracle manipulation, and cross-chain / EIP-712 signature replay. Hunt these classes against dormant value first.

## What NOT to hunt (deterministic payability kills — apply in the impact gate)

- **Deprecated / unused / in-development code** that doesn't match the live deployment → no live risk → no reward. Verify against the deployed bytecode.
- **Temporary DoS / griefing** and other low-severity impacts → most programs don't pay. Don't spend a PoC on them.
- **Trusted-role-only issues** → usually out of scope unless the program explicitly scopes admin actions in. If the trigger needs a malicious owner, it's a trust assumption, not a bug.
- **Third-party / out-of-scope components** → read the special clauses first.
- **Anything you cannot prove.** Unprovable ≠ finding. This is the proof gate, restated.

## Getting paid (feeds target selection, before any code)

You lose all leverage the moment you disclose. Before hunting, evaluate: project reputation, prior bounty stories, treasury size, and program rules. **Red flags: vague rules, low caps, prior disputes, slow/no response.** A real exploit on a bad-faith program pays nothing — that is an EV `ABORT`, not a target. After disclosure expect delays and pushback; stay professional, keep hunting.

## The AI-hunter workflow (how this stack is meant to be run)

- Treat the LLM engines as a **non-deterministic static analyzer**: cheap, high-throughput lead generation. ~70–80% of leads can be AI-surfaced. Spend tokens freely to produce leads.
- **The human/gate validates intent.** AI is strong at spotting *unusual* behavior, weak at recognising *intended* behavior — that gap is the entire false-positive surface. The impact gate, intent ledger, and proof standard exist to close it.
- **Discipline over shiny objects.** Don't chase trendy targets if the goal is dollars; don't chase the smallest-scope easy contest. Focus on inputs (leads proven), not outputs (bugs this week). Dry spells are normal even for the best — the outcome ledger, not a mood, is the feedback loop.

## How the doctrine binds to the machinery

| Doctrine | Enforced by |
|---|---|
| complex paths first | `surface` `risk_score` (complexity-weighted) |
| maturity → tactic | `detect` `tactic` field + `repo_maturity` |
| don't re-find known bugs | `fingerprint` novelty gate |
| unprovable ≠ finding | `references/proof-standard.md` |
| no live risk → abort | `references/impact-model.md#ev-precheck` |
| trusted-role / DoS / dev-code kills | `references/impact-model.md` code-flaw test |
| archetypes (Miner/Differ/Speedrunner/Watchman/One-Day) | routed crit* engine archetype layer |

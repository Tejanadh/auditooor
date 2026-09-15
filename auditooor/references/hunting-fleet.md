# Hunting Fleet — spawn the whole pashov-style agent fleet, never 1–2 generics

When auditooor hunts, it does **not** spawn one or two generic agents. It spawns the **full specialized fleet** — one agent per hacking role — in parallel, each with a single obsession, then collects, dedups, gates, and PoCs their candidates. This is the back-half of the pipeline: `xray` (seams/surface/entries/git) nominates the surface; the fleet hunts it.

**Hard rule:** a real hunt (`/auditooor`, `/auditooor DEEP`, a live program) spawns the fleet. Spawning a single generic "look for bugs" agent is a bug in the operator, not a shortcut. On Charm this was violated (only 2 generic agents) — never again.

**Mode split:** `/auditooor XRAY` spawns **zero** hunting agents (recon only). `/auditooor FUZZ` spawns the **five invariant-discovery agents** in `invariant-discovery.md`, not this 12. Do not mix the two fleets in one turn unless the user asked for a full pipeline.

## The fleet (12 roles, from `{engine}/references/hacking-agents/`)

`{engine}` = the routed engine dir (`~/.grok/commands/critfindsaudit` then `~/.claude/commands/critfindsaudit`, per `ecosystem-router.md`). Each file is a complete hunting persona — spawn one agent per role, loading that file as the agent's mandate.

| Agent | Obsession |
|---|---|
| `access-control-agent` | who can call what; missing/mis-scoped guards; deploy/init-state access |
| `asymmetry-agent` | paired-function & branch & writer/reader asymmetries (deposit↔withdraw, user↔admin) |
| `boundary-agent` | first/last/empty/max/zero, off-by-one, boundary-condition census |
| `economic-security-agent` | fee/reward/share math, donation/inflation, MEV, flash-loan amplification |
| `execution-trace-agent` | full call-path tracing, reentrancy, cross-contract state during external calls |
| `first-principles-agent` | question every "obviously true" assumption in the intended economics |
| `flow-gap-agent` | value/permission flows with a missing step (credit without debit, etc.) |
| `invariant-agent` | protocol-specific invariants (conservation, solvency, monotonicity) that must hold |
| `math-precision-agent` | rounding direction, precision loss, unit/decimal mismatch |
| `numerical-gap-agent` | overflow/underflow/unchecked, downcast, wraparound |
| `periphery-agent` | routers/wrappers/adapters/migration glue — the un-audited seam |
| `trust-gap-agent` | trusted-input assumptions: oracle, token, callback, external return values |

(Also read `{engine}/references/hacking-agents/shared-rules.md`, `senior-auditor-sop.md`, and `bounty-rules.md` — every agent inherits these.)

## Fleet selection

- **Default / DEEP:** spawn **all 12**. More coverage on a real target is the point of the fleet.
- **Token-constrained / QUICK:** spawn the subset the money-map + `surface`/`detectors` nominate — always include `economic-security`, `invariant`, `math-precision`, `access-control`, plus `periphery` whenever `seams` returned any SEAM.
- **Fortress warning:** if EV Phase 0 flagged a fortress (many audits + FV, no un-audited seam), spawning the full fleet still finds $0 — the swarm data is unambiguous. Do not spawn a fleet at a fortress to feel productive; re-target instead (`impact-model.md`). More agents never beats a better target.

## Spawn protocol (this is the part that has to beat a generic agent)

Pashov's auditor works because every specialist gets **the whole source in one bundle** and all 12 run **in the same turn**. Serial one-agent-at-a-time is how Charm died. Do this:

1. **Pack once** (before any spawn):

```
{scan} pack <dir> --out <recon> --agents {skill}/references/hunters
```

(`{engine}/references/hacking-agents` is the fallback if this skill was installed without `references/hunters/`.)

That writes `source.md`, `hunt.md`, `entries.md`, `PROPERTIES.md`, `xray.json`, and `fleet/*-bundle.md`. Do not ask agents to glob the repo.

2. **Spawn all selected roles in ONE message**, `background: true` (Grok `spawn_subagent` default). Description `[auditooor:<role>]`. Do **not** pass `capability_mode`. `background: false` serializes the fleet — forbidden on a real hunt.

   Prompt for each role (do not inline source into the prompt):

```
Read {recon}/fleet/<role>-bundle.md fully (source + SOP + specialty + bounty-rules).
Game: <bounty|contest>
EV: <Phase 0>
Posture: <xray posture>
Impact map: permissionless rows in {recon}/entries.md only.

Return FINDING/LEAD blocks per shared-rules.md. Every FINDING needs proof: AND impact_bound:.
No bundle → no hunt. Do not re-read in-scope files for the initial scan.
```

3. **Wait for every agent.** Completeness gate before merge: list every unique `(contract, function)` in any raw FINDING/LEAD. Each must appear in the merged output. Silent drop = operator bug. Dedup by `(contract, function, mechanism)` — never across functions.

4. **Four sequential merge gates** (fail any → reject or demote; later gates not evaluated). Same idea as a contest judging sheet, priced for bounties:

   | # | Gate | Fail |
   |---|---|---|
   | 1 | **Executes** — quoted guards on the path do not interrupt before harm | REJECT |
   | 2 | **Reachable** on live/normal usage, not a structurally impossible state | REJECT / DEMOTE |
   | 3 | **Unprivileged trigger** (or a named amplifier: race / retroactive / asymmetric formula / access gap) | DEMOTE on bounty; contest may keep admin findings |
   | 4 | **Material impact_bound** with units, loser, attacker_gain (`bounty-rules.md`) | LEAD |

   Then: **coverage wave** (`coverage-wave.md`) → **skeptics** (`skeptic-agent.md`) on survivors → novelty (`novelty-gate.md`) **before** any PoC. Survivors → fork PoC (`proof-standard.md`). No PoC, no finding.

   Load `references/protocol-threats.md` for the `census.protocol_types` rows only — do not dump a 100-item catalog into the prompt.

## Why the fleet, not one big agent

One agent reasoning about "everything" spreads thin and regresses to the obvious. Twelve agents each with a single obsession go deeper on their lane and *disagree productively* — the asymmetry agent sees what the math agent skips. That divergence is the coverage. But it is coverage, not magic: on a fortress, twelve deep agents still find nothing, because there is nothing. The fleet multiplies a good target; it cannot rescue a bad one.

# Hunting Fleet — headline three, supporting specialists, never 1–2 generics

When auditooor hunts, it does **not** spawn one or two generic agents. It spawns specialists in parallel from **`{skill}/references/hunters/`** (this skill; no engine required). `xray` nominates the surface; the fleet hunts it.

**Hard rule (unchanged):** a hunt never runs on *generic* agents. Charm spawned 2 generics — never again. Every spawned agent is a named specialist from `{hunters}` with one obsession, a bundle, and the shared rules.

**What did change (v0.8): count is not the rule — specialisation is.** The default run is **LITE: three specialists** (`lite-mode.md`), because the recorded runs in `../../benchmark/CALIBRATION.md` show the full fleet adding about one extra lead over a focused pass and none of them payable — while Charm burned a 12-agent fleet on a $10k picked-clean core. This file describes **DEEP**. Spawn it when `{scan} ev` prints `mode: DEEP` or the user types `DEEP`.

**Mode split:** `/auditooor XRAY` = zero hunters. LITE = 3 specialists. `/auditooor QUICK` = 7. DEEP = 15. `/auditooor FUZZ` = invariant-discovery agents, not this fleet.

## Headline (ours — spawn these first, always)

| Agent | Obsession |
|---|---|
| `money-map-agent` | isolated accounting-first books; no other agent's map (CritFinds Agent 8, in-skill) |
| `lifecycle-agent` | init → operate → pause → upgrade → sunset; guards that die on a transition |
| `spec-divergence-agent` | NatSpec/spec/comment claim vs what the code does |

**QUICK** = these three + `access-control`, `economic-security`, `invariant`, `periphery`.

## Supporting specialists (DEEP / Default)

Each file is a hunting persona. Licence notes in `NOTICE.md`. The product is the gates around them.

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

**LITE (default)** = headline 3. **QUICK** = headline 3 + AC/economic/invariant/periphery (7). **DEEP** = headline 3 + supporting 12 (15), plus optional `critfindsaudit` if installed.

**Money-map isolation (HARD):** do **not** inject the xray impact map or other agents' FINDINGs into the money-map prompt. Source bundle + `exploit-patterns.md` rounding/AC recipes only. Agreement with the 12 is only a signal if this agent started from the code.

Every agent inherits `{skill}/references/shared-rules.md`, `senior-auditor-sop.md`, and `{skill}/references/hunters/bounty-rules.md`.

## Fleet selection

- **LITE (default):** headline 3 only — or on `SEAM` posture, `periphery` + `money-map` + `access-control`. No coverage wave, one skeptic only on a Phase-4 candidate.
- **DEEP:** spawn **headline 3 + supporting 12**.
- **QUICK:** headline 3 + `access-control`, `economic-security`, `invariant`, `periphery`.
- **Fortress warning:** if EV Phase 0 flagged a fortress (many audits + FV, no un-audited seam), spawning the full fleet still finds $0 — the swarm data is unambiguous. Do not spawn a fleet at a fortress to feel productive; re-target instead (`impact-model.md`). More agents never beats a better target.

## Spawn protocol (this is the part that has to beat a generic agent)

Every specialist gets **the whole source in one bundle** and the fleet runs **in the same turn**. Serial one-agent-at-a-time is how Charm died. Do this:

1. **Pack once** (before any spawn):

```
{scan} pack <dir> --out <recon> --agents {skill}/references/hunters --deep
```

(Without `--deep` the pack is LITE-sized: 3 bundles, top-8 focus set. On DEEP you are buying every role and every file — that is the point of the flag.)

(`{engine}/references/hacking-agents` is the fallback if this skill was installed without `references/hunters/`.)

That writes `source.md`, `hunt.md`, `entries.md`, `PROPERTIES.md`, `xray.json`, and `fleet/*-bundle.md`. Do not ask agents to glob the repo. Bundles inline the focus set; files below the cut are listed as a read-on-demand manifest and read by path only when a lead points at one.

2. **Spawn all selected roles in ONE message, in the background** — Claude Code `Agent` with `run_in_background: true`; Grok `spawn_subagent` with `background: true` (no `capability_mode`); Cursor's subagent tool if exposed. Description `[auditooor:<role>]`. Foreground/serialized spawning is forbidden on a real hunt when the runtime can parallelize; with no subagent tool, run roles sequentially inline per `SKILL.md` *Runtime dispatch*.

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

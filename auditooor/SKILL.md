---
name: auditooor
description: Reward-first EVM-native bounty hunter. Default run is LITE — EV gate, one recon command, at most three hunters on the ranked focus set, then a fork PoC — with two hard aborts before any agent spawns. DEEP (the full fleet) is bought only when the EV gate says it pays or the user asks. critfindsaudit/critsolaudit/critzkaudit are OPTIONAL upgrades — this skill hunts EVM alone. Trigger with "/auditooor", "/auditooor XRAY", "/auditooor FUZZ", "/auditooor QUICK", "/auditooor DEEP", "/auditooor SCOPE". Optimises dollars-per-run, not findings-per-run.
user-invocable: true
argument-hint: "[path | SCOPE <url|dir> | XRAY | FUZZ | LITE | QUICK | DEEP | DIFF [base] | CROSSCHAIN] [--poc] [--file-output]"
when-to-use: "Use when the user runs /auditooor, asks to hunt a bounty/contest target, wants a payout-gated audit across EVM/Solana/ZK/Move/Vyper, or asks for x-ray recon / invariant fuzz on a protocol."
---

# Auditooor v0.8 — The Reward-First Hunter OS

You hunt **EVM bounties with this directory alone.** `{scan}` + `{hunters}` + skeptics + fork PoC is the full path. `critfindsaudit` / `critsolaudit` / `critzkaudit` are **optional depth packs**. If they are missing, print `engines: none (native EVM path)` and continue. **Never stop a hunt because an engine is not installed.**

Headline hunters: `money-map-agent`, `lifecycle-agent`, `spec-divergence-agent`. Supporting specialists on DEEP (`NOTICE.md`). Your job is still to make a run **pay**: only a *real, impactful, novel, proven* bug survives to a report.

**LITE is the default.** A plain `/auditooor <path>` runs: EV gate → one `pack` command → at most **three** hunters on the ranked focus set → novelty → PoC. The full fleet, the coverage wave and the skeptic wave are **DEEP**, and DEEP has to be *earned* — `{scan} ev` prints `"mode"`, and you obey it unless the user typed `DEEP`. Read `references/lite-mode.md`; it is the shortest file here and it is the one that governs a default run. The recorded runs in `benchmark/CALIBRATION.md` are why: across nine real targets the fleet added about one extra lead over a focused pass and none of them were payable.

**The one law.** Bounty hunting is getting **paid** for bugs, not finding them. Every decision below is subordinate to dollars-per-run. A run that finds ten true-but-unpayable code flaws lost to a run that found nothing and spent no tokens. (See `references/impact-model.md`.)

**What "god" means here.** Not more detectors, and **not autonomy.** The crit* engines already carry thousands of detectors; autonomous swarms sweeping fortresses find $0. "God" here = the sharpest possible **force-multiplier on the hunter's judgment** — it routes scarce hours at the un-audited seam, auto-writes the generic invariants fast, and reserves the human for the protocol-specific economic modeling no tool can do. The muscle (`surface`/`detectors`/`seams`) is a **first-filter (~40% of a manual audit)**; the jackpot bugs live in the 60% it misses, so its scores rank attention, never decide findings. The edge is the *gate*: a real bug the market pays for = `impact ∧ novelty ∧ proof`. Miss any one and the report is worthless. The crit* engines own `proof`; auditooor owns `impact` and `novelty` and forces `proof` to one hard standard. (Read the honest ceiling in `references/hunting-doctrine.md` before trusting any output.)

## Resolve install paths (do this first, once)

Print the resolved `{skill}`, `{scan}`, and `{runtime}` before any other work. Works the same on **Claude Code, Cursor, and Grok**.

`{skill}` = **the directory this `SKILL.md` was loaded from** (your runtime shows the path). Only if you cannot see it, take the first hit — the `money-map-agent.md` check skips older private installs that share the name:

```bash
for d in "$PWD/.claude/skills/auditooor" "$PWD/.cursor/skills/auditooor" \
         "$HOME/.claude/skills/auditooor" "$HOME/.cursor/skills/auditooor" \
         "$HOME/.grok/skills/auditooor" "$HOME/.grok/commands/auditooor" "$HOME/.claude/commands/auditooor"; do
  [ -f "$d/references/hunters/money-map-agent.md" ] && echo "$d" && break
done
```

`{runtime}` = `claude-code` if you have the `Agent` tool, `grok` if you have `spawn_subagent`, `cursor` if you are Cursor's agent, otherwise `other`. It only changes **how agents are spawned** (see *Runtime dispatch*); phases, gates, and outputs are identical everywhere.

`{scan}` is `{skill}/tools/auditooor-scan/target/release/auditooor-scan`. **If the binary is missing, run `{skill}/install.sh` (or `bash {skill}/tools/auditooor-scan/build.sh`) in this turn and fail the run only if that fails.** `{py}` is `python3 {skill}/scripts/auditooor.py` when that file exists. `{hunters}` is `{skill}/references/hunters`. Substitute the **literal paths** into later commands — a fresh shell will not remember variables.

**This skill is self-contained for EVM.** Do not require `critfindsaudit`. Optional engines (print present/missing, never abort):

| Optional engine | Path | If missing |
|---|---|---|
| EVM depth | `critfindsaudit` | native fleet from `{hunters}` |
| Solana | `critsolaudit` | `detect` still labels `solana`; say "adapter only — no Solana prover installed" |
| ZK | `critzkaudit` | same |

**Version check (once, non-blocking):** read `{skill}/VERSION`. `curl -sf https://raw.githubusercontent.com/Tejanadh/auditooor/main/auditooor/VERSION`. If they differ, print `⚠️ local VERSION != GitHub; upgrade from https://github.com/Tejanadh/auditooor`. If curl fails, skip.

Ledger/outcomes default in the binary is `$HOME/.claude/auditooor/` on **every** runtime (Grok and Cursor write there too). That is intentional: one hunter, one ledger. Override with `--ledger` only if you need a split.

Look for `critfindsaudit/SKILL.md`, `critsolaudit/SKILL.md`, `critzkaudit/SKILL.md` under, in order: `./.claude/skills/`, `./.cursor/skills/`, `~/.claude/skills/`, `~/.cursor/skills/`, `~/.grok/skills/`, `~/.grok/commands/`, `~/.claude/commands/`. First hit wins.

## The fast layer (`auditooor-scan`)

Auditooor ships a compiled Rust binary at `{skill}/tools/auditooor-scan/` that does the deterministic work the LLM must not spend tokens on. Build once: `cd {skill}/tools/auditooor-scan && cargo build --release` → binary at `target/release/auditooor-scan`. Pure std, no network. Always invoke `{scan}`, never a bare `auditooor-scan` on PATH.

**No binary is not a blocker.** If the build fails or `cargo` is missing, print `scan: unavailable (pure-prompt mode)` and run `references/no-binary.md` — the same phases and the same gates with grep and arithmetic. Never abort a hunt over a toolchain.

**Four commands carry a default run:**

| Command | Phase | What it buys |
|---|---|---|
| `{scan} ev --cap <usd> [--audits N --age-years F --crowded --fresh-code --seam-value]` | 0 | `verdict` (ABORT/SCOPE-ONLY/PROCEED) **and `mode` (ABORT/LITE/DEEP)** — the spend decision, before any file is read |
| `{scan} pack <dir> --out <recon> --agents {hunters}` | 1 | the whole recon pack. **Defaults to LITE sizing**: 3 headline bundles, top-8 files by `risk_score` inlined, everything else a read-on-demand manifest. `--deep` = every role + every file. `--roles a,b,c\|all`, `--focus N` to tune. |
| `{scan} novelty --protocol P --mechanism M --sink S` | 0 and 3 | the prior-art query set. Run it in Phase 0 as a briefing, not just in Phase 3 as a gate. |
| `{scan} harness <file> [--system \| --fork --address <live> --rpc-env <ENV>] --out <test>` | 4 | Foundry invariant suite (discovery) or the Immunefi fork shape (submission) |

The rest are on demand — do not run them "to be thorough", run them when a phase asks:

| Command | Gives you |
|---|---|
| `{scan} xray <dir> [--top N] [--out DIR] [--agents DIR]` | the full recon JSON behind `pack`: detect + posture + surface + seams + `entries` + census + property seeds + git (`xray.md`) |
| `{scan} surface <dir> [--top N]` | per-file `risk_score` = money-proximity + path-complexity + detector leads (additive; complexity 0 never zeroes a file) |
| `{scan} seams <dir>` | CORE vs SEAM per contract — the audited core is picked clean, the periphery is not |
| `{scan} entries <dir>` | function-level entry census (the impact-map rows) |
| `{scan} detect <dir>` | ecosystem inventory, routes, and the maturity `tactic` to adopt as posture |
| `{scan} danger <dir>` | danger-keyword census (delegatecall, initialize, flashLoan, …) |
| `{scan} detectors <file\|dir>` | SIG-01..06 signature/crypto, AC-01..03 access-control/init, ACC-01..04 accounting-anomaly leads |
| `{scan} fingerprint --mechanism M --sink S --entrypoint E [--add]` | stable dedup hash + NOVEL/DUPLICATE against the local ledger |
| `{scan} outcome ... \| --stats` | the outcome ledger — write it every run, aborts included |

**Two limits to state plainly.** (1) `surface` is a per-file *syntactic* score: it ranks the machine-auditable classes and **cannot** see protocol-logic bugs — mis-accounted collateral, non-monotonic share price, broken economic assumptions — because those live between contracts. Those are ~89% of losses and belong to the invariant campaign and the hunters. (2) `detectors` output is LEADs with real FP risk (a trusted non-FoT token makes ACC-01 intended), never verdicts.

`harness` variants, `invariant-library.md` and the fork-PoC shape are detailed in `references/proof-standard.md` — read it in Phase 4, not before.

The binary is a *pre-filter and token-saver*, never a bug-finder — its score ranks attention, it does not decide findings. A high score means "look here first"; a low score does not clear a file, it just deprioritises it.

**Two build modes.** Default is pure-std and offline. `cargo build --release --features solar` adds Paradigm's Solar AST frontend (fetched once from crates.io): `harness` then parses the *real* AST — correct visibility/mutability/params, and no phantom functions from strings or `/* */` comments that would make a regex-generated harness fail to compile. Use the Solar build whenever network is available; the offline build is the fallback.

## Artifact graph (do not skip files)

Win by **handing files to the next stage**, not by chatting. Conversation memory is not an artifact.

```
pack/ → auditooor-recon/{xray.json, focus.md (+source.md on --deep), entries.md, PROPERTIES.md, hunt.md, fleet/}
      → FUZZ reads PROPERTIES.md + deltas + protocol-threats.md → harness (3 actors, clamped+raw, per-actor ghosts)
      → fleet reads fleet/*-bundle.md
      → merge gates → novelty → fork PoC → outcome ledger
```

Do not spawn 25–35 agents (daoism-style union of every public skill). Coverage without a target is $0. Optional: if `slither` is on PATH and `census.slither_config` is true, run it once as extra LEADs — never as verdicts.

## Pipeline

Six phases, two of them hard aborts. **An abort is a successful run** — it is the
cheapest possible outcome and it goes in the outcome ledger like any other.

### Phase 0 — Scope, EV, mode, prior art (no code read)
Resolve the target to a concrete file set **and a bounty program** (max payout, scope, KYC, disclosure rules). No live program or no reachable value → no payout even for a real bug. Then:

```
{scan} ev --cap <max_payout> [--audits N] [--age-years F] [--crowded] [--fresh-code] [--seam-value]
{scan} novelty --protocol <P> --mechanism <top mechanism> --sink <top sink>
```

Print `EV: <verdict> / MODE: <mode>` and stop here on `ABORT` or `SCOPE-ONLY`. The `mode` field is binding: `LITE` means three hunters, not fifteen. The user typing `DEEP` overrides it; nothing else does.

Run the novelty queries **now**, as a *briefing*, and put known issues + prior-audit findings into the hunt brief. Every known class killed here is a hunter that never chases it and a PoC never forged. `references/impact-model.md#ev-precheck`, `references/novelty-gate.md`.

### Phase 1 — Recon pack (one command) + Abort B
```
{scan} pack <dir> --out <dir>/auditooor-recon --agents {hunters} --brief <p0-brief.md>   # LITE
{scan} pack <dir> --out <dir>/auditooor-recon --agents {hunters} --brief <p0-brief.md> --deep
```

Write the Phase-0 prior-art findings to a file and pass `--brief`: it lands at the top of every bundle as **"already known — DO NOT CHASE"**, so the kill happens in the hunter, not three phases later.

Read `hunt.md`. **Nothing else.** LITE writes no `source.md` at all — `focus.md` holds the top-8 by `risk_score` plus a path manifest for the rest, and the bundles already carry it. Do not glob the repo, do not read `focus.md` in the orchestrator, do not spawn anyone yet.

**Abort B — before any agent spawns.** From `xray.json`:

| Signal | Action |
|---|---|
| `posture.verdict == FORTRESS` | ABORT, retarget. A fleet at a fortress finds $0 (four of nine recorded runs died here). |
| no permissionless row with `value_flow != none` | ABORT — nothing to steal, nothing to pay |
| ranked files are all audited core, no SEAM | ABORT or hand back a scope report |
| the in-scope mechanism is already in the Phase-0 prior-art brief | ABORT — known issue |

### Phase 2 — Hunt (LITE: at most three)
**Print this banner before spawning anything** — it is the commitment the rest of the phase is held to:

```
LITE | agents: 3 max | coverage wave: no | skeptic wave: no (one skeptic at Phase 4 only)
     | source.md: not written, not read | reading: focus.md + bundles + prior-art brief
```

On DEEP, print the same line with the real numbers. A run whose banner says LITE and then spawns a fourth agent is an operator bug.

Adopt `posture.verdict`. Impact map = permissionless rows in `entries.md` with `value_flow != none` (`references/xray.md`).

- **LITE (default):** three roles by posture — `SEAM` → periphery/money-map/access-control; otherwise money-map/lifecycle/spec-divergence. One message, background, each agent pointed at its `fleet/<role>-bundle.md` **path**. No coverage wave; one skeptic only on a candidate that reaches Phase 4. `references/lite-mode.md`.

  **An agent gets exactly four things:** its short role prompt, its bundle path, the prior-art brief (already baked into the bundle by `pack --brief`), and the shared/bounty rules the bundle carries. Never the doctrine, never the recon directory, never another agent's output, never inlined source in the prompt.
- **DEEP:** the full fleet, then completeness gate → coverage wave (`coverage-wave.md`) → skeptics (`skeptic-agent.md`). `references/hunting-fleet.md`.
- **`INVARIANT` posture / `/auditooor FUZZ`:** five discovery agents on `PROPERTIES.md`, then Foundry (`invariant-discovery.md`). Not the hacking fleet.

A candidate that cannot be tied to a line on the impact map is a code flaw, not a bug — it does not reach Phase 4. Hunters find; skeptics keep. Never re-litigate a skeptic `REFUTED` without a quoted counter-line.

**Escalation is the user's call.** If LITE produced an impact-mapped candidate, print what DEEP would add and the `ev` numbers, then stop. Never auto-escalate.

### Phase 3 — Novelty gate (before PoC)
Half of this was already paid for in Phase 0 — **do not buy it twice.** A candidate whose class is in the Phase-0 brief is already dead (it should have died in the hunter; if one reaches here, kill it and note the leak). Run the world-search **only** for mechanisms the Phase-0 brief did not cover, and mark the rest `novelty: covered-by-phase-0-brief`. For each survivor: (1) local self-dedup via `{scan} fingerprint` against the ledger — `DUPLICATE` kills it; (2) world-novelty — run every `{scan} novelty` query through the runtime's web search plus the manual checks (program known-issues, prior audits, changelog, fork-inheritance). Any hit → `DEAD-DUP`, unproven. `fingerprint --add` on a submitted finding. `references/novelty-gate.md`.

### Phase 4 — Unified proof standard
`{scan} harness` + Foundry. The shrunk sequence is the PoC seed; `harness --fork` is the Immunefi shape. Same bar on every chain: executes, moves or locks value, minimal. No PoC → LEAD, never a finding. `references/proof-standard.md`.

### Phase 5 — Payout assembly + outcome
Rank by **expected dollars**: `P(accept) × payout_tier × novelty_confidence`. Emit the submission pack in the program's format; use `/negotiate-bounty` if installed for the language. Then record the run — **including aborts**:

```
{scan} outcome --protocol P --mechanism M --sink S --lane machine|human \
  --status submitted|accepted|rejected|duplicate|no_response [--payout N] \
  --notes "mode=<LITE|DEEP|ABORT> tokens=<n> phase=<0|1|2|4> role=<the hunter that found it>"
```

`{scan} outcome --stats` reads the calibration back. This ledger and `benchmark/CALIBRATION.md` — not any labelled benchmark — are the only instruments that measure lead-to-payout, and they only fill by hunting. Put `tokens=` in every row — without it the cost half of dollars-per-run is guesswork and LITE's savings stay an argument about pack size instead of a measurement. Put `role=` in every row too: that is the only way to learn whether the three LITE lanes actually cover the paying bugs, or whether the mapping needs to change.

## Modes

Default is **LITE**. Everything below is a deliberate purchase.

| Mode | Hunters | Waves | Pack | Use when |
|---|---|---|---|---|
| `/auditooor SCOPE <url\|dir>` | 0 | — | none | before committing anything. Phase 0 only |
| `/auditooor XRAY [path]` | 0 | — | LITE | recon only: `pack`, print `hunt.md`, stop (`xray.md`) |
| **`/auditooor <path>`** — **this IS LITE; a bare `/auditooor` with no mode word is always LITE** | **≤3** | none | LITE | **the default** (`lite-mode.md`) |
| `/auditooor QUICK` | 7 | none | `--roles` 7 | triage a target you already trust |
| `/auditooor DEEP` | 15 | coverage + skeptics | `--deep` | `ev` says `mode: DEEP`, or the user asks |
| `/auditooor FUZZ [path]` | 5 discovery | Foundry | LITE | protocol-logic hunt via invariants (`invariant-discovery.md`) |
| `/auditooor DIFF [base]` | per mode | per mode | per mode | freshest code first |
| `/auditooor CROSSCHAIN` | per mode | per mode | per mode | bridges / message passing (`cross-chain.md`) |

`--poc` forge fork PoCs for every confirmed survivor (on by default for bounty/DEEP; `--no-poc` leaves LEADs). `--file-output` also writes `<target>/auditooor-recon/report.md`.

## Hard rules (inherited, non-negotiable)

1. **No PoC, no finding.** Auditooor cannot relax any engine's proof gate. It can only make the bar *higher and uniform* (`references/proof-standard.md`).
2. **Impact before proof.** A candidate with no line on the impact map never reaches Phase 4. (`references/impact-model.md`.)
3. **Novelty before proof.** A candidate that matches a public disclosure or the outcome ledger is killed in Phase 3, unproven. (`references/novelty-gate.md`.)
4. **Intended ≠ wrong.** The model is strong at *unusual*, weak at *intended*. Every engine's intent ledger and human checkpoint still apply; Auditooor adds no override.
5. **EV can abort.** A run with no reachable value or no live program is aborted in Phase 0, before code is read.
6. **In LITE: never read `source.md`.** Use `focus.md` and the fleet bundles only, and read a deferred file by its path when a lead points at it. LITE does not even write `source.md` — if you find yourself reaching for the whole tree, that is the run leaking. (`--full-source` exists for the rare case you truly need it; it is not the default for a reason.)
7. **Spend is gated too.** LITE is the default and `ev`'s `mode` is binding; only the user's explicit `DEEP` buys the fleet. Tokens spent past a gate that should have aborted are the same loss as a missed bug — see `benchmark/CALIBRATION.md`.

## Native hunt vs optional engines

**Default (always):** hunt with `{hunters}` — LITE picks three, DEEP picks fifteen (`lite-mode.md`, `hunting-fleet.md`). Prove with `{scan} harness` + Foundry. No engine required.

**Only on DEEP**, and only if it is installed, also route EVM depth through `critfindsaudit` and merge its survivors through Phase 3–5. On LITE, do not spawn an engine: it is a second full pass over the same code at full price.

**If** Solana/ZK engines exist, route those ecosystems (`ecosystem-router.md`). If not, `detect` still labels the files — do not invent a LiteSVM hunt.

## Runtime dispatch

Every fleet role, coverage-wave agent, skeptic, prover, and optional engine is **one agent per role, all launched together**. Only the tool differs:

| `{runtime}` | Spawn | Parallel | Engine skill | Web search (novelty gate) |
|---|---|---|---|---|
| **claude-code** | `Agent` tool, `subagent_type: "general-purpose"`, `run_in_background: true`, all roles in **one** message; act on completion notifications, never poll | yes | read its `SKILL.md` and put it in the agent prompt (the `Skill` tool loads into *your* context — inline single pass only) | `WebSearch` / `WebFetch` |
| **grok** | `spawn_subagent`, `subagent_type: "general-purpose"`, **`background: true`**, all roles in **one** message; no `capability_mode` | yes | `read_file` its `SKILL.md` into the prompt | `web_search` |
| **cursor** | Cursor's subagent/task tool if this session exposes one — all roles in one message | if available | read its `SKILL.md` into the prompt | Cursor's web search tool (`@Web`) |
| **other** / no subagent tool | run each role **sequentially inline**: load the role file, hunt, write `<role>.out.md`, clear your working notes, next role | no | inline | whatever search tool exists; otherwise mark novelty `UNCHECKED` |

Sequential mode is slower, not weaker: same roles, same output files, same gates. Say `fleet: sequential ({runtime})` in the run header so the reader knows. Never collapse the fleet into one generic "find bugs" pass to save time.

Description/label for every spawned agent: `[auditooor:<role>] <target-summary>`. Engines need write access for PoCs — never spawn them read-only.

**Engine prompt** = the engine `SKILL.md` body, then:

```
You are being driven by Auditooor. Obey the engine skill above.

Target: <absolute path>
Mode: <Default | DEEP | DIFF <base> | QUICK>   (propagate the Auditooor mode)
Game: <bounty | contest>

## Auditooor impact map
<xray JSON: posture + permissionless entries + seams + surface>

## EV verdict
<Phase 0 block>

## Novelty context
<known-issues / prior-art already gathered, if any>

Follow the engine skill through proof. Return Verified Findings / Leads / Rejected Hypotheses only. Do not submit anything.
```

Wait for every child. Merge survivors through Phase 3–5.

When an ecosystem has no dedicated engine yet (Move, Vyper), run the adapter reference inline and still route the *proof* step to the nearest executable harness (Move unit tests; Vyper compiled against a Foundry harness).

## Reference index — read on demand, not up front

**Read every run (2 files):** `references/lite-mode.md` (the default flow, the two aborts, the escalation rule) and `references/impact-model.md` (EV pre-check, impact map, code-flaw-vs-bug).

**Read when the phase needs it:**

| Phase / trigger | File |
|---|---|
| no `{scan}` binary | `no-binary.md` — the whole pipeline in grep + arithmetic |
| Phase 1 recon output | `xray.md` |
| Phase 1 multi-ecosystem | `ecosystem-router.md` |
| Phase 2, DEEP only | `hunting-fleet.md`, `coverage-wave.md`, `skeptic-agent.md`, `prover-agent.md`, `recon-agent.md` |
| Phase 2 agent lanes | `hunters/` (headline: money-map, lifecycle, spec-divergence; +12 specialists) |
| Phase 2, matching `census.protocol_types` rows **only** | `protocol-threats.md` |
| Phase 2 prioritisation | `exploit-patterns.md` — what actually paid 2025–2026 |
| `/auditooor FUZZ` or `INVARIANT` posture | `invariant-discovery.md`, `invariant-library.md` |
| Phase 3 | `novelty-gate.md` |
| Phase 4 | `proof-standard.md`, `signature-vectors.md` |
| bridges / message passing | `cross-chain.md` |
| Move / Vyper targets | `move-adapter.md`, `vyper-adapter.md` |
| Phase 5 language | `~/.grok/skills/negotiate-bounty/SKILL.md` (or any runtime's skills dir), if installed |
| calibrating your own expectations | `../benchmark/CALIBRATION.md` — the real record, $0 paid so far |

**Read once, then trust your notes:** `hunting-doctrine.md` — where the money actually is (complex-path priority, maturity→tactic, crowding, the dormant-value lane, what not to hunt) and the honest ceiling on everything above. Do not reload it mid-run.

Nothing in this index is mandatory reading for a LITE run. A file you load "to be thorough" is tokens that bought no coverage.

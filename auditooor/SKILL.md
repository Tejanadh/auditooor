---
name: auditooor
description: Reward-first EVM-native bounty hunter. Packs recon, hunts with money-map/lifecycle/spec-divergence plus a supporting specialist fleet, skeptics claims, then demands a fork PoC. critfindsaudit/critsolaudit/critzkaudit are OPTIONAL upgrades — this skill hunts EVM alone. Trigger with "/auditooor", "/auditooor XRAY", "/auditooor FUZZ", "/auditooor QUICK", "/auditooor DEEP", "/auditooor SCOPE". Optimises dollars-per-run, not findings-per-run.
user-invocable: true
argument-hint: "[path | SCOPE <url|dir> | XRAY | FUZZ | DEEP | QUICK | DIFF [base] | CROSSCHAIN] [--poc] [--file-output]"
when-to-use: "Use when the user runs /auditooor, asks to hunt a bounty/contest target, wants a payout-gated audit across EVM/Solana/ZK/Move/Vyper, or asks for x-ray recon / invariant fuzz on a protocol."
---

# Auditooor v0.7 — The Reward-First Hunter OS

You hunt **EVM bounties with this directory alone.** `{scan}` + `{hunters}` + skeptics + fork PoC is the full path. `critfindsaudit` / `critsolaudit` / `critzkaudit` are **optional depth packs**. If they are missing, print `engines: none (native EVM path)` and continue. **Never stop a hunt because an engine is not installed.**

Headline hunters: `money-map-agent`, `lifecycle-agent`, `spec-divergence-agent`. Supporting specialists on DEEP (`NOTICE.md`). Your job is still to make a run **pay**: only a *real, impactful, novel, proven* bug survives to a report.

**The one law.** Bounty hunting is getting **paid** for bugs, not finding them. Every decision below is subordinate to dollars-per-run. A run that finds ten true-but-unpayable code flaws lost to a run that found nothing and spent no tokens. (See `references/impact-model.md`.)

**What "god" means here.** Not more detectors, and **not autonomy.** The crit* engines already carry thousands of detectors; autonomous swarms sweeping fortresses find $0. "God" here = the sharpest possible **force-multiplier on the hunter's judgment** — it routes scarce hours at the un-audited seam, auto-writes the generic invariants fast, and reserves the human for the protocol-specific economic modeling no tool can do. The muscle (`surface`/`detectors`/`seams`) is a **first-filter (~40% of a manual audit)**; the jackpot bugs live in the 60% it misses, so its scores rank attention, never decide findings. The edge is the *gate*: a real bug the market pays for = `impact ∧ novelty ∧ proof`. Miss any one and the report is worthless. The crit* engines own `proof`; auditooor owns `impact` and `novelty` and forces `proof` to one hard standard. (Read the honest ceiling in `references/hunting-doctrine.md` before trusting any output.)

## Resolve install paths (do this first, once)

Print the resolved `{skill}` and `{scan}` before any other work. Prefer Grok, then Claude, then the directory of this `SKILL.md`:

```bash
for d in "$HOME/.grok/commands/auditooor" "$HOME/.grok/skills/auditooor" "$HOME/.claude/commands/auditooor"; do
  [ -f "$d/SKILL.md" ] && echo "$d" && break
done
```

That directory is `{skill}`. `{scan}` is `{skill}/tools/auditooor-scan/target/release/auditooor-scan`. **If the binary is missing, run `{skill}/install.sh` (or `bash {skill}/tools/auditooor-scan/build.sh`) in this turn and fail the run only if that fails.** `{py}` is `python3 {skill}/scripts/auditooor.py` when that file exists. `{hunters}` is `{skill}/references/hunters`. Substitute the **literal paths** into later commands — a fresh shell will not remember variables.

**This skill is self-contained for EVM.** Do not require `critfindsaudit`. Optional engines (print present/missing, never abort):

| Optional engine | Path | If missing |
|---|---|---|
| EVM depth | `~/.grok/commands/critfindsaudit` | native fleet from `{hunters}` |
| Solana | `~/.grok/commands/critsolaudit` | `detect` still labels `solana`; say "adapter only — no Solana prover installed" |
| ZK | `~/.grok/commands/critzkaudit` | same |

**Version check (once, non-blocking):** read `{skill}/VERSION`. `curl -sf https://raw.githubusercontent.com/Tejanadh/auditooor/main/auditooor/VERSION`. If they differ, print `⚠️ local VERSION != GitHub; upgrade from https://github.com/Tejanadh/auditooor`. If curl fails, skip.

Ledger/outcomes default in the binary is `$HOME/.claude/auditooor/` (shared with Claude). That is intentional: one hunter, one ledger. Override with `--ledger` only if you need a split.

Optional engine lookup (first hit wins) — **upgrade, not a dependency**:

| Engine | Paths |
|---|---|
| EVM depth | `~/.grok/commands/critfindsaudit/SKILL.md`, `~/.claude/commands/critfindsaudit/SKILL.md` |
| Solana | `~/.grok/commands/critsolaudit/SKILL.md`, `~/.claude/commands/critsolaudit/SKILL.md` |
| ZK | `~/.grok/commands/critzkaudit/SKILL.md`, `~/.claude/commands/critzkaudit/SKILL.md` |

## The fast layer (`auditooor-scan`)

Auditooor ships a compiled Rust binary at `{skill}/tools/auditooor-scan/` that does the deterministic work the LLM must not spend tokens on. Build once: `cd {skill}/tools/auditooor-scan && cargo build --release` → binary at `target/release/auditooor-scan`. Pure std, no network. Subcommands feed the phases below. Always invoke `{scan}`, never a bare `auditooor-scan` on PATH.

- `auditooor-scan xray <dir> [--top N] [--out DIR] [--agents hacking-agents/]` → **one JSON recon pack**: detect + posture + surface + seams + function-level `entries` + **census** (toolchain, nSLOC, invariant/echidna/medusa/fork presence, protocol type) + **mechanical property seeds** + git. `--out` writes the on-disk pack (`xray.json`, `source.md`, `entries.md`, `PROPERTIES.md`, `hunt.md`, optional `fleet/*-bundle.md`) that FUZZ and the 12-agent fleet consume. See `references/xray.md`.
- `auditooor-scan pack <dir> --out DIR [--agents DIR]` → same as `xray --out` (alias).
- `auditooor-scan entries <dir>` → function-level entry census alone (same rows as `xray.entries`).
- `auditooor-scan danger <dir>` → CritFinds-v2 danger-keyword census (delegatecall, initialize, mulDown, flashLoan, …). Folded into `xray.json` as `danger.hot_files`. LEADs, not findings.
- `auditooor-scan detect <dir>` → ecosystem inventory + engine routes (JSON). Drives **Phase 1** when xray is not used.
- `auditooor-scan seams <dir>` → ranks every contract **CORE vs SEAM** (audited-core fortress vs un-audited periphery/glue: routers, wrappers, migration helpers). The audited core is picked clean; **value still sits in the seams** — point the hunt there. Feeds **Phase 0/2**. See `references/impact-model.md` (fortress detection) and `references/hunting-doctrine.md`.
- `auditooor-scan detect <dir>` also prints a **maturity `tactic`** (sloppy→low-hanging fruit; mature/complex→deepest paths) per `references/hunting-doctrine.md`. Adopt it as the run's posture.
- `auditooor-scan surface <dir> [--top N]` → every file ranked by an **additive** `risk_score` = money-proximity (weighted value-move sites, +3 when no sender/role guard nearby) + path-complexity (how deep/branchy each site is buried) + **detector leads** (SIG/AC hits, so flat critical bugs with no value-keyword signal still surface). Additive, **not** a product — complexity=0 never zeroes a file. Drives **Phase 2**: read files in `risk_score` order. **Hard limit to state plainly:** `surface` is a per-file *syntactic* score. It ranks the machine-auditable classes (value flow, SIG/AC). It **cannot** find protocol-logic bugs — mis-accounted collateral, non-monotonic share price, broken economic assumptions — because those live in relationships *between* values across contracts, not in any one file. Those are ~89% of 2025 losses and are hunted by the **system-level invariant campaign** (Phase 4 harness run as a primary surface) and the LLM engines, not by `surface`.
- `auditooor-scan fingerprint --mechanism M --sink S --entrypoint E --ledger P [--add]` → stable dedup hash + `NOVEL`/`DUPLICATE` against a persistent ledger. Drives **Phase 3**.
- `auditooor-scan detectors <file|dir>` → **signature (SIG-01..06)**, **access-control/init (AC-01..03)**, and **accounting-anomaly (ACC-01..04)** leads (comments/strings blanked first). SIG/AC recognise known CWE patterns; **ACC is the detective layer** — it flags structural accounting mismatches (fee-on-transfer nominal-amount credit, raw-balance consumer, inflow-less credit, sibling divergence) and points at the *function*, the way a human reading `depositToken` does. All are LEADS with FP risk (a trusted non-FoT token makes ACC-01 intended), never verdicts — 0 ACC FP on 10 audited OZ files. See `references/signature-vectors.md`. Run in **Phase 2** alongside `surface`.
- `auditooor-scan harness <file.sol> [--contract N] [--import PATH] [--out FILE]` → generates a runnable **Foundry invariant suite** for *discovery* (handler + ghost accounting + solvency/conservation invariants). Foundry shrinks any break to a minimal call sequence.
- `auditooor-scan harness --system <src_dir> --out <test>` → generates a **system-level invariant suite** spanning *every* contract in the directory: deploys them all, drives actors across all of them, and asserts conservation at the **system boundary**. This is the **primary surface for protocol-logic bugs (the 89%)** — the ones that live in cross-contract interactions and have no per-file signature. Fill the `setUp()` wiring, then run. Proven: caught a Pool+Staker interaction bug (each contract correct alone) and shrank it to `stake → unstake` extracting unbacked funds. See `references/invariant-library.md`.
- `auditooor-scan harness <file.sol> --fork --address <live> --rpc-env <ENV> [--block N] --out <test>` → generates an **Immunefi-compliant fork PoC** for *submission*: forks the live chain at a block, attaches to the deployed contract (no fresh deploy), and asserts attacker profit against real state. **Immunefi rejects fresh-deploy unit-test PoCs — this is the shape you submit.** Drives **Phase 4**. Verified: the invariant suite broke `invariant_solvency` and shrank to `deposit → emergencyDrain`; the fork PoC compiles against forge-std and attaches to the live address.

The binary is a *pre-filter and token-saver*, never a bug-finder — its score ranks attention, it does not decide findings. A high score means "look here first"; a low score does not clear a file, it just deprioritises it.

**Two build modes.** Default is pure-std and offline. `cargo build --release --features solar` adds Paradigm's Solar AST frontend (fetched once from crates.io): `harness` then parses the *real* AST — correct visibility/mutability/params, and no phantom functions from strings or `/* */` comments that would make a regex-generated harness fail to compile. Use the Solar build whenever network is available; the offline build is the fallback.

## Artifact graph (do not skip files)

Win by **handing files to the next stage**, not by chatting. Conversation memory is not an artifact.

```
pack/ → auditooor-recon/{xray.json, source.md, entries.md, PROPERTIES.md, hunt.md, fleet/}
      → FUZZ reads PROPERTIES.md + deltas + protocol-threats.md → harness (3 actors, clamped+raw, per-actor ghosts)
      → fleet reads fleet/*-bundle.md
      → merge gates → novelty → fork PoC → outcome ledger
```

Do not spawn 25–35 agents (daoism-style union of every public skill). Coverage without a target is $0. Optional: if `slither` is on PATH and `census.slither_config` is true, run it once as extra LEADs — never as verdicts.

## Pipeline

Run these phases in order. Any phase may `ABORT` the run — aborting early is the point, not a failure.

### Phase 0 — Scope & EV pre-check
Resolve the target to a concrete file set and a **bounty program** (max payout, scope, KYC, disclosure rules). No live program, no reachable value → there is no payout even for a real bug. Read `references/impact-model.md#ev-precheck`. Emit `EV: PROCEED` or `EV: ABORT <reason>`. Do this *before* reading code.

### Phase 1 — Recon pack (one command)
Write the pack, then read `hunt.md` — do not glob the repo first.

```
{scan} pack <dir> --out <dir>/auditooor-recon --agents {hunters}
```

JSON stdout + on-disk `source.md` / `entries.md` / `PROPERTIES.md` / `hunt.md` / `fleet/`. One target can route to several ecosystems (`ecosystem-router.md`). **Do not spawn anyone yet.**

### Phase 2 — Posture, then the matching fleet
Adopt `posture.verdict` from `xray.json`. Impact map = permissionless rows in `entries.md` with `value_flow != none`. Read `references/xray.md` and `references/impact-model.md#impact-map`.

Then dispatch — **all agents in ONE message, `background: true`**:

- `FORTRESS` → do **not** spawn; retarget or ABORT.
- `SEAM` → 12-agent fleet (`hunting-fleet.md`) restricted to SEAM contracts; each agent reads `fleet/<role>-bundle.md`.
- `HUNT` / `DEEP` → full 12 the same way. Serial `background: false` is forbidden.
- `INVARIANT` and `/auditooor FUZZ` → five discovery agents (`invariant-discovery.md`) starting from `PROPERTIES.md`, then Foundry.

A candidate that cannot be tied to a line on the impact map is a code flaw, not a bug — it does not proceed to a PoC.

After wave 1 returns: completeness gate → **coverage wave** (`coverage-wave.md`, unread permissionless money fns) → **skeptics** (`skeptic-agent.md`) → novelty → fork PoC. Hunters find; skeptics keep. Do not let the orchestrator re-litigate a skeptic `REFUTED` without a quoted counter-line.

### Phase 3 — Novelty gate (before PoC)
For every surviving candidate, run the **two-layer** novelty check *before* the engine forges its proof — forging a PoC for a known bug is pure token waste. (1) **Local self-dedup:** `auditooor-scan fingerprint` against the ledger; `DUPLICATE` → kill. (2) **World-novelty (the pillar that actually rejects submissions):** `auditooor-scan novelty --protocol P --mechanism M --sink S [--fork-family F]` generates a targeted prior-art query set — **run every query via WebSearch** and complete the manual checks (program known-issues, prior audits, changelog, fork-inheritance). A hit anywhere → `DEAD-DUP`. Only an all-clear is world-novel. On a proven, submitted finding, `fingerprint --add` to record it. Read `references/novelty-gate.md`.

### Phase 4 — Unified proof standard
**EVM default:** `{scan} harness` + Foundry. Shrink is the PoC seed; `harness --fork` is the Immunefi shape. Optional engines may add LiteSVM / forged-verifier. Same bar: executes, moves or locks value, minimal. `references/proof-standard.md`. No PoC → LEAD.

### Phase 5 — Payout assembly
Rank survivors by **expected dollars**, not severity label: `P(accept) × payout_tier × novelty_confidence`. Emit the submission pack in the program's format. **Draft and later triager replies use `/negotiate-bounty`** (`~/.grok/skills/negotiate-bounty/SKILL.md`) — precise impact, no oversell, no disclosure threats. Then **record every outcome**: `auditooor-scan outcome --protocol P --mechanism M --sink S --lane machine|human --status accepted|rejected|duplicate|no_response [--payout N]`. Read the calibration with `auditooor-scan outcome --stats`. This ledger — not any benchmark — is the **only** instrument that measures in-the-wild precision (dead leads per real bug) and calibrates `human_EV`'s P(model an un-modeled assumption). Every labeled benchmark measures recall; only this measures lead-to-payout, and it only fills by hunting.

## Modes

- **`/auditooor <path>`** — full pipeline on a local file set.
- **`/auditooor SCOPE <url|dir>`** — Phase 0 only: resolve program, scope, and EV verdict. Cheap. Run it before committing tokens.
- **`/auditooor XRAY [path]`** — recon only. `{scan} pack --out <path>/auditooor-recon`, print `hunt.md`, stop. No fleet, no PoC. Read `references/xray.md`.
- **`/auditooor FUZZ [path]`** — invariant campaign: pack + system harness + five discovery agents on `PROPERTIES.md` + Foundry (clamped + unclamped handlers, 3 actors) + fork-PoC for survivors. Read `references/invariant-discovery.md`. Do not spawn the 12 hacking agents in this mode.
- **`/auditooor DEEP`** — full 12 + spec-divergence + lifecycle + isolated money-map (15). Also pass `DEEP` to routed engines.
- **`/auditooor QUICK`** — 7 hunters (`hunting-fleet.md`). Rapid triage, still proof-gated.
- **`--poc`** — on by default for bounty/DEEP. Forge fork PoCs for every skeptic-CONFIRMED survivor. `--no-poc` leaves them as LEADs.
- **`--file-output`** — also write the report under `<target>/auditooor-recon/report.md`. Off by default.
- **`/auditooor CROSSCHAIN`** — force the cross-chain lens (`references/cross-chain.md`) even if only one ecosystem is present locally; use when the target bridges or passes messages.
- **`/auditooor DIFF [base_ref]`** — pass `DIFF` through to every routed engine; freshest code first.

## Hard rules (inherited, non-negotiable)

1. **No PoC, no finding.** Auditooor cannot relax any engine's proof gate. It can only make the bar *higher and uniform* (`references/proof-standard.md`).
2. **Impact before proof.** A candidate with no line on the impact map never reaches Phase 4. (`references/impact-model.md`.)
3. **Novelty before proof.** A candidate that matches a public disclosure or the outcome ledger is killed in Phase 3, unproven. (`references/novelty-gate.md`.)
4. **Intended ≠ wrong.** The model is strong at *unusual*, weak at *intended*. Every engine's intent ledger and human checkpoint still apply; Auditooor adds no override.
5. **EV can abort.** A run with no reachable value or no live program is aborted in Phase 0, before code is read.

## Native hunt vs optional engines

**Default (always):** spawn `{hunters}` per `hunting-fleet.md` (headline 3 + supporting 12 on DEEP). Forge proof with `{scan} harness` + Foundry.

**If** `critfindsaudit` exists **and** the user asked DEEP: also route EVM depth through that engine (230 vectors, money-map isolation already in-skill). Merge survivors through Phase 3–5.

**If** Solana/ZK engines exist, route those ecosystems. If not, `detect` still labels files; do not invent a LiteSVM hunt.

Route table: `{skill}/references/ecosystem-router.md`.

**Grok (this host):** there is no Skill tool. Dispatch with `spawn_subagent`:

1. `read_file` the engine `SKILL.md` in full (paths in the table above).
2. Call `spawn_subagent` with `subagent_type: "general-purpose"`, **`background: true`**, `description: "[auditooor:<role>] <target-summary>"`. All selected roles in **one** message. Do **not** pass `capability_mode`. `background: false` serializes the fleet — that is an operator bug.
3. Prompt = the engine skill body, then:

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

4. Wait for the child. Merge its survivors through Phase 3–5. World-novelty search on Grok uses the `web_search` tool (not Claude `WebSearch`).

**Claude:** if a Skill tool is available, invoke `critfindsaudit` / `critsolaudit` / `critzkaudit` with the same payload. Otherwise use the same `spawn_subagent` path.

When an ecosystem has no dedicated engine yet (Move, Vyper), run the adapter reference inline and still route the *proof* step to the nearest executable harness (Move unit tests; Vyper compiled against a Foundry harness).

## Reference index

- `references/impact-model.md` — the reward-first brain: EV pre-check, the impact map, and the code-flaw-vs-bug test. **Read first.**
- `references/ecosystem-router.md` — detection → engine dispatch, multi-ecosystem fan-out.
- `references/hunting-fleet.md` — **spawn the specialized hunting fleet, never 1–2 generics**: roster, parallel spawn, merge gates.
- `references/coverage-wave.md` — second wave on entry points no hunter actually opened.
- `references/skeptic-agent.md` / `prover-agent.md` / `recon-agent.md` — verify and prove; hunters are not verifiers.
- `references/hunters/` — **headline:** money-map, lifecycle, spec-divergence. **Supporting:** twelve specialists (`NOTICE.md`). Default `--agents` path.
- `references/exploit-patterns.md` — what actually paid 2025–2026 (rounding amplified, donation, init AC). Hunt these before any taxonomy. From CritFindsAudit.
- `references/xray.md` — bounty-first recon: `{scan} pack` → hunt brief → next command. Standalone `/auditooor XRAY`.
- `references/protocol-threats.md` — paying assumption-breaks by `census.protocol_types` (vault/lending/amm/staking/escrow). Load matching rows only.
- `references/invariant-discovery.md` — five-agent property fleet + synthesizer + Foundry run + fork-PoC. Standalone `/auditooor FUZZ`. Canonical properties live in `invariant-library.md`; this file authors the protocol-specific ones.
- `references/novelty-gate.md` — duplicate/public-disclosure kill before PoC.
- `references/proof-standard.md` — the one PoC bar across all chains.
- `references/cross-chain.md` — bridge / message-passing / multi-VM vectors no single engine owns.
- `references/hunting-doctrine.md` — where the money actually is: complex-path priority, maturity→tactic, competition/timing, the dormant-value uncrowded lane, what-not-to-hunt, getting-paid, and the AI-hunter workflow. The posture the whole stack hunts with.
- `references/signature-vectors.md` — the statically-detectable payout classes (SIG signature/crypto, AC access-control/init) that `auditooor-scan detectors` flags, plus how to turn each lead into a fork-PoC.
- `references/invariant-library.md` — **the primary surface for protocol-logic bugs (~89% of losses)**: the canonical system-level invariants (conservation, solvency, round-trip, share-price, collateralization, authority monotonicity) fuzzed across the whole protocol at flash-loan scale. `surface`/`detectors` triage the machine-auditable classes; invariants hunt the logic bugs no per-file score can see. Proven: a generated invariant caught a rounding-direction logic bug with all access guards intact.
- `references/move-adapter.md` — Move (Aptos/Sui) native vectors + proof harness.
- `references/vyper-adapter.md` — Vyper native vectors + compiler-version traps + proof harness.
- `~/.grok/skills/negotiate-bounty/SKILL.md` — Phase 5 report language and triager negotiation (separate skill; do not duplicate here).

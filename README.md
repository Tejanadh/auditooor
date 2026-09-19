<h1 align="center">auditooor</h1>
<p align="center"><b>A reward-first EVM bounty hunter.</b></p>
<p align="center"><i>Optimises dollars-per-run, not findings-per-run.</i></p>

<p align="center">
  <img src="https://img.shields.io/badge/v0.8.3-skill-627EEA" />
  <img src="https://img.shields.io/badge/EVM_native-optional_engines-9945FF" />
  <img src="https://img.shields.io/badge/proof-Foundry_fork_PoC-2ea44f" />
  <img src="https://img.shields.io/badge/runtimes-Claude_Code_·_Cursor_·_Grok-orange" />
</p>

Most AI audit tools are a lens: "you are a senior auditor, find bugs." auditooor
is a **gate**. It assumes the finding is the easy part and the hard part is
getting **paid** — so it kills the three things that actually cost you money
*before* they cost you anything: a target that can't pay, a bug that's already
known, and a hypothesis that only looks like a bug. What survives all three gets
a Foundry fork PoC. Nothing else is called a finding.

```
EV gate  ─►  recon + hard abort  ─►  ≤3 specialist hunters  ─►  confirm  ─►  novelty  ─►  fork PoC  ─►  ledger
 can it       is there anything      hunt the money, not        cheap       already      the only     learn from
 pay?         to steal here?         the whole repo             filter      known?       proof         every run
```

> **Honest status (this is the whole point):** 11 real targets · 1 real bug found
> and proven autonomously (a duplicate) · **$0 paid**. The tool is demonstrably
> good at *not* wasting money on fortresses and duplicates; it has not yet landed
> a payout. The full record — including what it does **not** establish — is in
> **[benchmark/CALIBRATION.md](benchmark/CALIBRATION.md)**. No fake numbers live here.

---

## 30 seconds

```bash
git clone https://github.com/Tejanadh/auditooor.git
bash auditooor/auditooor/install.sh          # installs into every runtime it finds, then builds the scanner
```

Then, in the agent chat:

```
/auditooor SCOPE https://immunefi.com/bounty/<program>   # should I even hunt this?  (2 commands, no code read)
/auditooor ./src                                         # the default hunt — LITE
/auditooor DEEP ./src                                    # buy the full 15-agent fleet (only when it pays)
```

Or just ask: *"run auditooor on src/Vault.sol"*.

**No Rust?** It still runs. The scanner is a token-saver, not a dependency — with
no binary the skill falls back to a [pure-prompt path](auditooor/references/no-binary.md)
with the same phases and the same gates. Foundry (`forge`) is needed only to
*prove* a bug.

---

## Why it's different from the other 65

There is a [good index of ~65 AI web3 audit tools](https://github.com/0xfirefistt/ai-web3-security).
auditooor takes the two ideas from it that fill a real gap and ignores the rest —
because more auditors is coverage, and *coverage without a target is $0*. What it
has that a plain audit skill does not:

| | What it does | Why it matters |
|---|---|---|
| **EV gate** | `scan ev` turns cap + audits + age + freshness into a binding `ABORT / LITE / DEEP`. | A $10k, 4×-audited, 5-year-old core is a **hard kill** before any code is read. Four of eleven recorded runs were this profile. |
| **Machine abort** | `scan gate <dir>` reads a **function-level** permission census and exits non-zero when every entry point is gated. | The abort is an *exit code*, not a paragraph an agent can forget. A gate you can skip isn't a gate. |
| **Known-issues register** | `scan known` builds a per-protocol register from the real audit reports, then kills any candidate that matches **before** a PoC. | Duplicates are the #1 bounty rejection. Adapted from [K.I.T](https://github.com/J4X-Security/K.I.T). |
| **Confirm loop** | `scan confirm` statically confirms/refutes a hunter's hypothesis (reentrancy CEI, unguarded sink, unchecked call). | The [GPTScan](https://github.com/GPTScan/GPTScan) lesson: a cheap deterministic filter stops each guess from costing a full PoC to disprove. |
| **Reward-first ledger** | Every run — including aborts — is recorded with tokens and outcome. | The only instrument that measures lead-to-payout. It only fills by hunting, and it says **$0** until that changes. |

The hunters themselves (money-map, lifecycle, spec-divergence, +12 specialists)
are adapted from [pashov/skills](https://github.com/pashov/skills) and the
CritFinds engine, MIT, credited in [NOTICE.md](auditooor/NOTICE.md). **The product
is the gates around them.**

---

## The pipeline

```
Phase 0   scan ev + scan known         →  ABORT here, or PROCEED with a mode      (no code read)
Phase 1   scan pack + scan gate        →  hard ABORT if nothing is reachable
Phase 2   ≤3 hunters on the focus set  →  each states a scenario·property·key-var
          scan confirm                 →  drop the look-alikes cheaply
Phase 3   scan known + novelty         →  kill duplicates before proving them
Phase 4   scan harness + Foundry       →  fork PoC — the only thing called a finding
Phase 5   report + scan outcome        →  record it, aborts included
```

**LITE is the default.** A plain `/auditooor ./src` runs ≤3 specialist hunters on
the top files by risk score — not the whole repo, not 15 agents. On the recorded
runs the full fleet added ~one non-payable lead per target over a focused pass, so
DEEP has to be *earned*: `scan ev` only returns `mode: DEEP` for a large,
uncrowded, fresh-code scope over ~3k lines. Cap size is not scope size.

Two hard aborts sit before any agent spawns — a fortress dies in Phase 0 for the
price of two commands, and an all-gated scope dies in Phase 1. **An abort is a
successful run.**

---

## Install options

| Runtime | Command | Installs to |
|---|---|---|
| Claude Code | `bash auditooor/auditooor/install.sh --claude` | `~/.claude/skills/auditooor` |
| Cursor | `bash auditooor/auditooor/install.sh --cursor` | `~/.cursor/skills/auditooor` |
| Grok | `bash auditooor/auditooor/install.sh --grok` | `~/.grok/skills/auditooor` |
| One repo only | `bash auditooor/auditooor/install.sh --project /path/to/repo` | `<repo>/.claude/skills` + `.cursor/skills` |

Default is a symlink, so `git pull` updates every install. `--copy` if your editor
doesn't follow symlinks; `--force` to replace a foreign install (backed up first).
**Update:** `cd auditooor && git pull`.

**Needs:** Foundry (`forge`) to prove a bug. Rust (`cargo`) makes the scanner
available and the run cheaper. Python 3 optional. On Claude Code and Grok the
fleet runs in parallel; on Cursor it parallelises when the session exposes
subagents, else runs the same roles sequentially.

---

## Every mode

| Command | Spend | What it does |
|---|---|---|
| `/auditooor SCOPE <url\|dir>` | ~2 commands | EV verdict + mode. No code read. |
| `/auditooor XRAY .` | 1 command | recon pack + hunt brief, no hunters |
| `/auditooor .` | **default** | LITE — ≤3 specialists on the focus set |
| `/auditooor QUICK .` | 7 hunters | triage a target you already trust |
| `/auditooor FUZZ .` | 5 + Foundry | invariant campaign for protocol-logic bugs |
| `/auditooor DEEP .` | 15 + waves | the full fleet, when EV says it pays |
| `/auditooor DIFF [ref]` | per mode | freshest code first |

---

## The scanner (`auditooor-scan`)

A pure-std Rust binary that does the deterministic work so the LLM doesn't burn
tokens on it. Standalone-usable:

```bash
scan ev --cap 500000 --audits 4 --age-years 5 --crowded      # → ABORT (the fortress profile)
scan gate ./contracts                                        # → exit 3 if every entry is gated
scan pack ./src --out recon --agents hunters --brief pa.md   # LITE recon pack: 3 bundles, top-8 files
scan confirm Vault.sol --function withdraw --check cei       # → CONFIRMED / REFUTED(exit 3) / INCONCLUSIVE
scan known check --protocol foo --mechanism M --sink S ...   # → KNOWN(exit 3) / REVIEW / NOVEL
scan harness Vault.sol --fork --address 0x.. --rpc-env RPC   # → Immunefi-shaped fork PoC
```

76 unit tests, plus a [census regression corpus](benchmark/census/run.sh)
that pins the permission census against 8 hand-verified directories in both
directions (OpenZeppelin, Uniswap, Aave, Solmate, Fluid) — because the abort is
only as trustworthy as the census under it.

---

## Chains

`scan detect` **labels** EVM / Solana / Vyper / Move / ZK files. **Hunting depth
is EVM.** Solana and ZK need the optional `critsolaudit` / `critzkaudit` engines;
Move and Vyper are adapter notes, not a full hunt.

---

<p align="center">
  <a href="benchmark/CALIBRATION.md">honest record</a> ·
  <a href="auditooor/demo/DEMO.md">recorded demo</a> ·
  <a href="https://github.com/Tejanadh">GitHub</a> ·
  <a href="https://x.com/TEJANadh10">X</a>
</p>
<p align="center"><sub>Reward-first. The ledger says $0 until it doesn't — and it won't lie when it does.</sub></p>

<h1 align="center">auditooor</h1>
<p align="center"><b>A reward-first EVM bounty hunter.</b></p>
<p align="center"><i>Optimises dollars-per-run, not findings-per-run.</i></p>

<p align="center">
  <img src="https://img.shields.io/badge/v0.8.2-skill-627EEA" />
  <img src="https://img.shields.io/badge/EVM_native-optional_engines-9945FF" />
  <img src="https://img.shields.io/badge/Proof-Foundry-2ea44f" />
</p>

**EVM hunts with this repo alone.** `critfindsaudit` / `critsolaudit` / `critzkaudit` are optional depth packs. Missing them is not a silent fail.

Works with **Claude Code** (CLI, VS Code, JetBrains), **Cursor**, and **Grok**.

## 30 seconds

```bash
git clone https://github.com/Tejanadh/auditooor.git
bash auditooor/auditooor/install.sh     # every runtime it finds, then builds {scan}
```

Then, in the agent chat:

```
/auditooor SCOPE https://immunefi.com/bounty/<program>   # should I even hunt this?  (2 commands, no code read)
/auditooor ./src                                         # the default hunt: LITE
/auditooor DEEP ./src                                    # buy the full 15-agent fleet
```

That is the whole interface. `/auditooor ./src` runs **LITE**: an EV gate that can
abort before a single file is read, one recon command, at most **three** specialist
hunters on the top-ranked files, then novelty, then a Foundry fork PoC. It is the
default because it costs roughly an eighth of the fleet and, on the runs recorded in
[benchmark/CALIBRATION.md](benchmark/CALIBRATION.md), the fleet's extra reach was
worth about one non-payable lead per target.

**No Rust?** It still runs. `{scan}` is a token-saver, not a dependency — with no
binary the skill falls back to [a pure-prompt path](auditooor/references/no-binary.md)
with the same phases and the same gates. Foundry is needed only to *prove* a bug.

**Honest status:** 9 real targets, 1 real bug found and proven autonomously (a
duplicate), **$0 paid**. The full record, including what it does not establish, is in
[benchmark/CALIBRATION.md](benchmark/CALIBRATION.md).

## Install options



| Runtime | Command | Installs to |
|---|---|---|
| Claude Code | `bash auditooor/auditooor/install.sh --claude` | `~/.claude/skills/auditooor` |
| Cursor | `bash auditooor/auditooor/install.sh --cursor` | `~/.cursor/skills/auditooor` |
| Grok | `bash auditooor/auditooor/install.sh --grok` | `~/.grok/skills/auditooor` |
| One repo only | `bash auditooor/auditooor/install.sh --project /path/to/repo` | `<repo>/.claude/skills` + `<repo>/.cursor/skills` |

The default is a symlink, so `git pull` updates every install. Use `--copy` if your editor doesn't follow symlinks. An existing install that isn't this checkout is never touched unless you pass `--force`, and even then it's backed up first.

Every mode:

| Command | Spend | What it does |
|---|---|---|
| `/auditooor SCOPE <url\|dir>` | ~2 commands | EV verdict + mode. No code read. |
| `/auditooor XRAY .` | 1 command | recon pack + hunt brief, no hunters |
| `/auditooor .` | **default** | LITE: ≤3 specialists on the focus set |
| `/auditooor QUICK .` | 7 hunters | triage a target you already trust |
| `/auditooor FUZZ .` | 5 + Foundry | invariant campaign for protocol-logic bugs |
| `/auditooor DEEP . --file-output` | 15 hunters + waves | the full fleet, when EV says it pays |

Or just ask: *"run auditooor on src/Vault.sol"*.

**Update:** `cd auditooor && git pull` (symlink installs) or re-run `install.sh --copy`.

Need: Foundry (`forge`) to prove a bug. Rust (`cargo`) makes `{scan}` available and the run cheaper — without it the skill uses the pure-prompt path. Python 3 optional (coverage parse). On Claude Code and Grok the fleet runs in parallel. On Cursor it runs in parallel when the session exposes subagents; otherwise it runs sequentially with the same roles and gates.

**Recorded demo (not a bounty):** [auditooor/demo/DEMO.md](auditooor/demo/DEMO.md) — `pack` labelled `drainTo` permissionless out; Foundry `[PASS]` thief takes 10 ETH.

---

## Headline hunters (ours)

`money-map` (isolated accounting) · `lifecycle` (init→upgrade→sunset) · `spec-divergence` (docs vs code).

Supporting specialists on DEEP (MIT, see `NOTICE.md`). The product is the gates: abort a fortress, novelty before PoC, skeptics, unread-function wave, Foundry / fork proof.

---

## Chains

`auditooor-scan detect` **labels** EVM / Solana / Vyper / Move / ZK files (see fixtures dry-run in DEMO.md). **Hunting depth is EVM.** Solana/ZK need the optional engines; Move/Vyper are adapter notes, not a full hunt.

---

## Outcome ledger

`$HOME/.claude/auditooor/outcomes.tsv` is empty until you record a real program. The OpenVault row is a **$0 fixture**. Do not call this a calibrated hunter yet.

---

## Artifact graph

```
install.sh → auditooor-scan pack → auditooor-recon/
  hunt.md entries.md PROPERTIES.md xray.json fleet/
        ↓
  headline 3 (+ supporting 12 on DEEP) → coverage wave → skeptics
        ↓
  novelty → Foundry / fork PoC → outcome
```

Gates: **EV → impact → novelty → proof**.

[GitHub](https://github.com/Tejanadh) · [X](https://x.com/TEJANadh10)

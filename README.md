<h1 align="center">auditooor</h1>
<p align="center"><b>A reward-first audit skill.</b></p>
<p align="center"><i>Optimises dollars-per-run, not findings-per-run.</i></p>

<p align="center">
  <img src="https://img.shields.io/badge/v0.5.0-skill-627EEA" />
  <img src="https://img.shields.io/badge/Chains-EVM%20%C2%B7%20Solana%20%C2%B7%20ZK%20%C2%B7%20Move%20%C2%B7%20Vyper-9945FF" />
  <img src="https://img.shields.io/badge/Bias-Proof%20over%20severity-2ea44f" />
</p>

Install the skill. Point it at a repo. It packs recon, hunts with a specialized fleet, measures unread entry points, skeptics the claims, then demands a **fork PoC** — or it aborts because the target will not pay.

```
npx skills add https://github.com/Tejanadh/auditooor --skill auditooor
```

```
git clone https://github.com/Tejanadh/auditooor.git
ln -s "$(pwd)/auditooor/auditooor" ~/.grok/skills/auditooor
ln -s "$(pwd)/auditooor/auditooor" ~/.claude/skills/auditooor   # optional
cd auditooor/auditooor/tools/auditooor-scan && cargo build --release
```

Then:

```
/auditooor SCOPE <program-url>
/auditooor XRAY .
/auditooor FUZZ .
/auditooor DEEP .
```

**Need:** Rust (for `auditooor-scan`), Foundry (for harness + fork PoC), Python 3 (coverage/parse, stdlib only).

---

## Why this is not Pashov/skills

[pashov/skills](https://github.com/pashov/skills) is a **contest auditor**: always run 12 agents, vendor-neutral report, Medusa/Echidna suite. It is excellent at that.

auditooor is a **bounty hunter OS**:

| | Pashov | auditooor |
|---|---|---|
| Abort a fortress | no | `EV: ABORT` / posture `FORTRESS` |
| Novelty before PoC | no | ledger + world search |
| Fork PoC (Immunefi shape) | unit/fuzz | `harness --fork` |
| Unread-function wave | no | coverage wave 2 |
| Independent skeptics | orchestrator judges | separate skeptic agents |
| Multi-chain | Solidity | EVM / Solana / ZK / Move / Vyper |
| Mechanical recon | LLM x-ray | compiled `auditooor-scan pack` |

The twelve hunter personas, SOP, and shared-rules are **Pashov’s MIT agents** (see `NOTICE.md`). We did not rewrite them. We put gates, a scanner, a second wave, and skeptics around them.

---

## Artifact graph

```
auditooor-scan pack → auditooor-recon/
  xray.json hunt.md entries.md PROPERTIES.md source.md fleet/*-bundle.md
        ↓
  fleet (14, parallel) → coverage wave → skeptics
        ↓
  novelty gate → Foundry invariant / fork PoC → outcome ledger
```

`conversation memory` is not an artifact. If it is not a file in `auditooor-recon/`, the next stage cannot see it.

---

## The thesis

Live bugs are usually simple. They hide in **complex paths with value at the end**. Rank by money-proximity × path-complexity. Skip clean fortresses. The 89% of 2025 losses were protocol-logic — invariants, not signature scanners.

Gates a lead must clear: **EV → impact → novelty → proof**. Miss one, do not write it up.

---

## Honest limits

- The Rust scanner ranks attention. It does not find the 89%.
- Twelve agents on a fortress still find nothing. Target selection is the product.
- Novelty cannot see a private submission from three hours ago.
- No published contest scoreboard yet. Do not cite this as “82% recall.”

Independent work. Not affiliated with Pashov Audit Group. Responsible disclosure first.

[GitHub](https://github.com/Tejanadh) · [X](https://x.com/TEJANadh10) · [email](mailto:tejanadh927@gmail.com)

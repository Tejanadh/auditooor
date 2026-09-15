<h1 align="center">auditooor</h1>
<p align="center"><b>A reward-first EVM bounty hunter.</b></p>
<p align="center"><i>Optimises dollars-per-run, not findings-per-run.</i></p>

<p align="center">
  <img src="https://img.shields.io/badge/v0.7.0-skill-627EEA" />
  <img src="https://img.shields.io/badge/EVM_native-optional_engines-9945FF" />
  <img src="https://img.shields.io/badge/Proof-Foundry-2ea44f" />
</p>

**EVM hunts with this repo alone.** `critfindsaudit` / `critsolaudit` / `critzkaudit` are optional depth packs. Missing them is not a silent fail.

```
git clone https://github.com/Tejanadh/auditooor.git
bash auditooor/auditooor/install.sh
ln -s "$(pwd)/auditooor/auditooor" ~/.grok/skills/auditooor
```

`install.sh` builds `{scan}` or **exits 1**. Then:

```
/auditooor XRAY .
/auditooor QUICK .
/auditooor DEEP --file-output
```

Need: Rust (`cargo`), Foundry (`forge`) for PoCs. Python 3 optional (coverage parse).

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

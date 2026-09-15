# Recorded demo — pack → LEAD → executed PoC

Target: `demo/OpenVault.sol` (this repo). Not a bounty. Proves the mechanical path works without `critfindsaudit`.

Machine: auditooor-scan 0.7.0 · forge 1.7.1 · 2026-09-15

## 1. Pack (no engines)

```
./install.sh
{scan} pack demo --out demo/auditooor-recon --agents references/hunters
```

`hunt.md` (verbatim):

```
**Posture:** `HUNT`
sloppy code + unguarded value — low-hanging AC/flow bugs first, then invariants
```

`entries.md` (verbatim):

| access | flow | contract.fn | loc |
|---|---|---|---|
| permissionless | in | `OpenVault.deposit` | OpenVault.sol:9 |
| permissionless | out | `OpenVault.withdraw` | OpenVault.sol:13 |
| permissionless | out | `OpenVault.drainTo` | OpenVault.sol:20 |

Impact map seed = `drainTo` permissionless `out`. 15 hunter bundles written. `engines: none` still hunts.

## 2. LEAD

`drainTo` has no `msg.sender` check, burns no shares, sends `address(this).balance`. Victim's 10 ETH deposit stays as `shares[victim]`. Books insolvent.

## 3. Proof (Foundry, this tree)

```
cd demo && forge test --match-test test_unprivileged_drainTo_steals_victim_deposit -vvv
```

```
[PASS] test_unprivileged_drainTo_steals_victim_deposit() (gas: 56000)
Suite result: ok. 1 passed; 0 failed; 0 skipped
```

Attacker (`0xB0B`) received 10 ETH. Vault balance 0. Victim shares still 10 ETH.

This is a **fresh-deploy unit PoC**. Immunefi still wants `harness --fork` against live state. The demo proves the gate: no passing test → no finding.

## 4. Detect dry-run (fixtures, not a Solana hunt)

```
{scan} detect tools/auditooor-scan/fixtures
```

```
ecosystems: evm=4, solana=1, vyper=1, move=1, rust=1
```

Labeling is real. A Solana *prover* is not. README does not claim otherwise.

## 5. Outcome row (payout $0 — fixture)

```
{scan} outcome --protocol OpenVault-demo --mechanism permissionless-drain --sink ETH \
  --lane machine --status accepted --payout 0 --notes "fixture demo; not a bounty"
```

The in-wild ledger used for calibration is `$HOME/.claude/auditooor/outcomes.tsv`. It stays empty until a real program is recorded. Do not cite `precision=1.0` from this fixture.

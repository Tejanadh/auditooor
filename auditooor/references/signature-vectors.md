# Signature & Access-Control Vectors — the statically-detectable money classes

Two payout-heavy bug classes that a deterministic scanner flags with near-zero miss rate — exactly where the probabilistic LLM swarm is weakest. `auditooor-scan detectors <file|dir>` emits these as LEADS (line + id + severity). A LEAD is a starting point, never a finding; the fork-PoC gate still decides. These live disproportionately in **old/dormant code** (the target biome), because the guards below became standard *after* that code shipped.

## Signature / crypto-scheme (SIG)

- **SIG-01 — Signature malleability.** Raw `ecrecover` with no high-s reject and no OpenZeppelin ECDSA. For any `(v,r,s)` a second valid signature exists via `s' = n - s`, `v' = 1 - v`. If replay tracking keys on the *raw signature* instead of the *digest/nonce*, the malleated form replays. Fix: OZ ECDSA ≥4.7.3, or reject `s > secp256k1n/2` and restrict `v ∈ {27,28}`.
- **SIG-02 — Missing zero-address check.** `ecrecover` fails silently to `address(0)`. Without `signer != address(0)`, a malformed signature authorizes actions as the zero address (e.g., a permit approving zero-address funds).
- **SIG-03 — Replay: no nonce.** Signature verification with no nonce mapping/increment. The same signature is accepted repeatedly ("one signature, multiple payments"). The nonce must be consumed atomically with the authorized action.
- **SIG-04 — EIP-712 domain separator without `block.chainid`.** Domain separator omits chainId, or caches it at construction with no recompute. After a fork/chain split, or on a sibling deployment, the same signed message verifies where it must not — cross-chain / cross-domain replay. (Real: the bug that hit 40+ wallet vendors; multiple Sherlock/Immunefi findings.)
- **SIG-05 — Signed approval without deadline/expiry.** No `deadline`/`expiry` bound: a leaked or intercepted signature is valid forever. EIP-2612 mandates a deadline check.
- **SIG-06 — Phantom permit ("billion-dollar no-op").** `permit()` invoked via a low-level `call`/`encodeWithSignature`. On a token that lacks permit but has a fallback, the call succeeds silently, the approval never happens, and funds are pulled anyway. Verify `code.length` and that the token really implements EIP-2612, or use SafeERC20's `permit` wrapper.

## Measured — STRATIFIED, per-bucket (`benchmark/bench_strat.py`)

Detectors are validated against **real** contracts (DeFiVulnLabs = SunWeb3Sec's own corpus; clean = audited OpenZeppelin), reported **per root-cause bucket with abstention** — never one aggregate number, because an aggregate would either grade the muscle on bugs it was never built for or quietly drop them from the denominator. Current (20 vuln / 10 clean):

| bucket | mode | result |
|---|---|---|
| signature | GRADED (SIG-*) | recall **4/4 = 100%** |
| access-control (init/proxy) | GRADED (AC-*) | recall **1/2 = 50%** (storage-collision uncovered) |
| access-control (tx.origin/visibility) | COVERAGE-GAP | 0/3 — **not claimed** by the detectors |
| reentrancy / oracle / economic / integer | ABSTAIN | harness/LLM's job — **out of static scope** |
| clean OZ (per-detector FP) | — | **0 FP across 10 audited files** |

The benchmark drove four real fixes it caught that crafted fixtures hid: SIG-03 firing on a pure recovery *library*, SIG-04 firing on a file that only *calls* the inherited domain getter, phantom-permit missed (added SIG-06), and **AC-02 firing on OZ's abstract `_authorizeUpgrade` UUPS base** (an intentionally-open, accepted-risk pattern — now requires an implemented body).

**HONEST SCOPE — read this.** This measures the **first-filter (static detectors) only**. It says **nothing** about pipeline recall: oracle/reentrancy/economic/cross-contract-logic bugs (the majority of DeFiHackLabs losses) need the invariant harness + **fork execution against archive state**, which this does not run. A good detector number is not a pipeline number — that conflation is the exact "machine runs vs machine is good" trap, one level up. The full DeFiHackLabs blind-recall run (line-level labels, per-bucket, on a machine with archive RPC) is the number that would make the pipeline *measured*, and it is not done.
- **Also hunt manually** (not yet auto-flagged): digest missing `address(this)`/`verifyingContract` (same sig valid across sibling contracts); typed data crafted to look like one action while authorizing another (blind-sign); signature covering fewer fields than the action uses.

## Accounting anomalies (ACC) — the detective layer

SIG/AC recognise known CWE *patterns*. ACC flags structural *accounting mismatches* — the shapes that become real economic bugs with no CWE name, and the ones the muscle used to miss (it highlighted files; ACC points at the *function*). These are **leads with real false-positive risk** — a trusted non-FoT token makes ACC-01 intended — not verdicts. They put the right function on the list; accounting review + PoC decide.

- **ACC-01 — Fee-on-transfer / nominal-amount credit.** A function pulls tokens via `transferFrom` then credits the *amount argument* to internal accounting, with **no `balanceOf(address(this))` before/after measurement**. A fee-on-transfer, deflationary, or rebasing token delivers less than `amount` → the contract over-credits → pool drains. (The classic EtherDelta `depositToken` bug.) The credit is recognised across eras — modern `+=`/`_mint` and the SafeMath-era `safeAdd`/`.add(` that 2016 code uses. Safe pattern: measure `received = balanceAfter - balanceBefore`, credit that.
- **ACC-02 — Raw balance consumer.** Share/price math reads `balanceOf(address(this))` directly. A direct token *donation* (transfer, not deposit) inflates that balance without minting shares → first-depositor / inflation-attack surface. Prefer an internal accounted total.
- **ACC-03 — Inflow-less credit.** A function credits the caller (`[msg.sender] +=` / `safeAdd` / `_mint`) but takes **no `msg.value` and no `transferFrom`** in the same body — a balance minted with no matching inflow. **A printer only credits; a ledger MOVE debits one side and credits the other** (transfer, batch transfer, internal settlement like EtherDelta `tradeBalances`) — those net to zero and are skipped (body debits via `-=`/`safeSub`/`.sub(`, or the function is a named token mechanic incl. `batchTransferFrom`). Confirm value arrives another way, else it prints claims against the pool.
- **ACC-04 — Sibling divergence.** A shortcut variant (`emergency*`/`force*`/`admin*`/`unsafe*`) that dropped the guard its base sibling has — "one code path forgot the check". Tightened to shortcut affixes so it stays quiet on legitimately-different siblings (audited `transfer`/`grantRole` do not trip it: 0 FP on 10 OZ files).

**Credit-op recognition is era-agnostic:** ACC-01/03 count `+=`, `_mint(`, and the SafeMath-era `safeAdd`/`.add(` — 2016 code (EtherDelta) never uses `+=`, and a `+=`-only check misses the real SHA.

**Honest scope:** ACC is a *junior detective*, not a senior one. It surfaces the anomaly shapes that most often became payable bugs, so the human reads the right three functions first — it does not confirm the bug. The needle still comes from accounting/diffs/weird entrypoints; ACC just moves the right function to the top of the stack.

## Access control / initialization (AC)

- **AC-01 — Unprotected initializer.** A function named **exactly `initialize(`** (not `initializePool` / `initializeStdChains` — those are Uni-v4-style permissionless pool creates, and a `pure`/`internal` library `initialize` is a struct packer, not an entrypoint), **external/public**, with no `initializer`/`reinitializer` modifier, **in an upgradeable/constructor-less contract** (a contract with a real `constructor` + immutables is not an uninitialized proxy). Anyone can call it before the deployer (front-run to seize ownership) or again after. Highest-value class of 2025–2026 ($1.6B H1 2025). The name/visibility/context gates exist because a raw `initialize` substring screams on every pool constructor.
- **AC-02 — `_authorizeUpgrade` without access guard.** The UUPS upgrade hook must revert for all but the upgrader. No `onlyOwner`/role/`require` → anyone points the proxy at malicious code.
- **AC-03 — Upgradeable impl without `_disableInitializers()` in constructor.** An implementation with `initialize()` but no `_disableInitializers()` can be initialized directly on the implementation address (CPIMP class — a malicious middleman implementation).
- **Also hunt manually:** ownership renounce without transferring upgrade rights; role admin set to a role that regular users can obtain; deploy-and-initialize as two transactions (assume a bot front-runs the gap).

## Turning a SIG/AC lead into an Immunefi payout

1. **Confirm live reachability** — the contract is deployed, holds/controls funds, and the flawed entrypoint is callable now (`impact-model.md#ev-precheck`). Dormant is fine; dead is worthless.
2. **Confirm intent** — is the missing guard actually absent, or enforced elsewhere (a wrapper, a modifier, an inherited base)? The scanner sees one file; read the inheritance chain. Peers-do-it-differently is a question, not a finding.
3. **Fork PoC** (`proof-standard.md`): `auditooor-scan harness <file> --fork --address <live> --rpc-env <ENV> --block <N> --out <test>`, then:
   - **Replay (SIG-01/03/04):** capture or forge the signature, submit it once (succeeds), submit the malleated/second copy or re-submit on the forked sibling chain, assert the second action also succeeds and extracts value.
   - **Init takeover (AC-01/03):** on the forked state, call `initialize()` / initialize the implementation as the attacker, assert you now hold owner/upgrade rights, then exercise that right to move value.
4. **Novelty gate** (`novelty-gate.md`) — old contracts may have this disclosed already; fingerprint before you invest in the PoC.

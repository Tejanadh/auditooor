# Invariant Library — hunting the 89% (protocol-logic bugs)

~89% of 2025 losses were protocol-**logic** bugs: mis-accounted value, broken economic assumptions, state-transition flaws. They have **no per-file syntactic signature** — `surface` and `detectors` cannot find them. They are caught by **system-level invariants fuzzed across the whole protocol**. This is how top Code4rena wardens work, and it is the **primary** hunting surface for logic bugs, not a per-file afterthought.

Proven in this stack: the generated `invariant_no_free_money` caught a rounding-direction bug (Balancer $120M class) with every access guard intact, shrinking it to `deposit → withdraw` extracting the pool's rounding bonus.

## How to run invariants as the primary surface

**Generate the system harness:** `auditooor-scan harness --system <src_dir> --out <test>`. It parses every contract in the directory, deploys them all, drives actors across **all** of them from one handler, and asserts conservation at the **system boundary** (`invariant_system_no_free_money`: total value out ≤ total value in; `invariant_system_solvency`). Then:

1. **Fill the WIRING** in `setUp()` — constructor args and connect-the-contracts (`staker.setPool(...)`, etc.). This is the one manual step; the tool can't infer deploy order.
2. **Seed realistic state** — other depositors' funds already in the pool (`vm.deal(address(pool), 100 ether)`), non-empty share supply. Many logic bugs only extract value when *someone else's* funds are present.
3. **Run it.** A break is a candidate; Foundry's shrinker hands you the minimal cross-contract exploit sequence — that is your PoC seed.

**Proven:** on a Pool+Staker system where each contract is correct *in isolation* — Pool only pays your own shares, Staker only pays a configured bonus — the generated `invariant_system_no_free_money` caught the interaction bug (Staker's bonus is funded from the shared Pool, unbacked) and shrank it to `stake → unstake` extracting 10 ETH the system never received. No per-file scan or single-contract invariant can see this; the system harness fuzzed across both contracts found it.

## The canonical invariants (assert these on every protocol)

**Conservation / solvency**
- **No free money:** absent yield, aggregate value out ≤ aggregate value in. Catches rounding-direction, double-credit, mint/burn asymmetry.
- **Solvency:** contract's asset balance ≥ sum of all claims (shares × price, or recorded liabilities). Catches over-issuance and pay-out-more-than-held.
- **Sum-of-parts:** `totalSupply == Σ balances`; `totalAssets == Σ deposits − Σ withdrawals ± yield`. Catches accounting drift across contracts.

**Share / price math (vaults, AMMs, lending)**
- **Round-trip loss:** `deposit(x)` then immediately `redeem(sharesReceived)` returns **≤ x**. Profit = a bug. Catches inflation attacks and wrong rounding.
- **Share price sanity:** price-per-share does not jump on a deposit/withdraw by a new actor (first-depositor inflation). A drop on a real loss is fine; a jump an attacker induces is not.
- **No dust griefing of price:** a 1-wei donation cannot move price-per-share enough to grief the next depositor.

**Lending / collateral**
- **Collateralization:** every borrow position stays `collateralValue ≥ debtValue × ratio` after any sequence. Catches liquidation-ordering and oracle-timing bugs.
- **Liquidation conserves value:** a liquidation cannot leave the protocol worse off than before it (bad-debt socialisation done right).

**Governance / access-state**
- **Authority monotonicity:** no unprivileged sequence grants an actor a role/allowance they weren't given. Catches privilege-escalation logic (distinct from the flat AC-01/02/03 that `detectors` finds).
- **Supply cap / rate-limit holds in aggregate:** per-call caps that don't compose across calls are a classic bypass.

## Amplification is part of the property, not separate

When you assert these, drive the handler with **flash-loan-scale** capital (bound inputs up to the largest borrowable amount, not the attacker's own balance). A 0.1% skew is invisible at small size and a Critical at $500M. 62.1% of price-manipulation attacks used flash loans; the invariant must be fuzzed at that scale or it under-tiers the bug.

## What an invariant break is (and isn't)

A broken invariant is a **candidate**, subject to the same gates as any lead:
- **Intent:** is the "loss" actually intended (a fee, a haircut, slippage)? Model those in the handler so they don't false-positive.
- **Novelty:** old rounding bugs are often disclosed — fingerprint + world-novelty check before investing.
- **Proof:** convert the shrunk sequence into a **fork PoC** (`proof-standard.md`) against live state before submitting.

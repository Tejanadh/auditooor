# Protocol-type threats — bounty-first, not a catalog

`census.protocol_types` picks the row. These are **assumption breaks that pay**, not OWASP lists. Load only the matching types into FUZZ / fleet prompts. Do not hunt every bullet.

## vault
- Share price jumps on a donation / first depositor / dead-share.
- `deposit` and `withdraw` use different rounding directions or different `totalAssets` snapshots.
- Async vault: claim/settle path skips a check the request path made.
- Idle assets in the contract not counted in `totalAssets` (or counted twice).

## lending
- Decision on price A, liquidation/payout on price B, no re-check.
- One close path re-checks, sibling (market vs keeper vs partial) does not.
- Health factor uses a different decimal/rounding than the seize math.
- Bad-debt socialisation leaves the protocol worse than before the liquidation.

## amm
- Amount-out uses a reserve snapshot taken after the attacker’s first hop.
- Fee-on-transfer / rebasing token as a pool asset, balances vs `reserve` diverge.
- Empty-pool / one-wei first LP owns the price.

## staking
- Reward funded from a shared pool the staker does not own (system-boundary conservation).
- Unstake path pays a bonus the stake path never locked.
- Epoch / cooldown skipped on one exit.

## escrow / lock
- Release without the matching lock id, or unlock amount not tied to locked amount.
- Timeout path and happy path disagree on beneficiary.

## Cross-cutting (always, if permissionless value exists)
- Sibling functions: one guarded, one not, same sink (`hunting-doctrine.md` differ).
- Upgrade / initializer still callable (`detectors` AC-01/03).
- Cross-contract: each contract is locally conservative, the system is not (`harness --system`).

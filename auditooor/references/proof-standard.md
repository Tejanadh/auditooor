# Proof Standard — one bar, every chain

Auditooor cannot weaken any engine's proof gate. It exists to make the bar **uniform and high** across EVM, Solana, ZK, Move, Vyper and cross-chain, so a "finding" means the same strong thing no matter where it came from. A reasoning-only LEAD is never a finding.

## The four properties every PoC must have

1. **It executes.** Real harness, real run, exit 0. Not pseudocode, not "this test would pass."
   - EVM/Vyper → Foundry test (`forge test`), forked state where the bug needs live state.
   - Solana → LiteSVM or Mollusk test that runs the program.
   - ZK → a *forged* witness/proof that the real verifier **accepts** (or a sound statement the verifier wrongly rejects). Under-constraint is only proven when a bad witness verifies.
   - Move → `move test` unit exercising the module against the vulnerable path.
2. **It moves or locks real value — or verifies a false statement.** The assertion at the end must show the impact-map value sink changed in the attacker's favor, or funds are permanently locked, or (ZK) a false statement verified. "State variable X has an unexpected value" is not impact unless that value *is* the loss.
3. **It's minimal.** Smallest sequence of transactions/instructions that triggers it, from a clean attacker account with a realistic starting balance. Strip every step that isn't load-bearing. A minimal PoC is what makes a triager believe it in 60 seconds — and belief is what gets paid.
4. **It ties to the impact map line.** The PoC's final assertion references the specific value sink and mover from Phase 2. If you can't point the PoC at a line on the impact map, the impact gate was wrong to pass it — go back.

## Quantified loss

The proof must **print the numbers**: attacker balance before/after, protocol value sink before/after, and — where repetition amplifies — the per-iteration delta and the realistic cap. Severity and payout tier are argued from these printed numbers, never asserted. This is also the sizing input for `E[$]`.

## Live-fork PoC is mandatory for EVM bounty submissions

Immunefi (and most programs) **reject unit-test PoCs against a fresh deploy**. A submittable EVM PoC MUST:

- **Fork the live chain** at a pinned block (`vm.createSelectFork(vm.envString("RPC"), block)`), never run against mainnet or a public testnet (doing so is an attack and an instant permanent ban).
- **Attach to the deployed contract** at its real address — never `new Target()`. The bug must reproduce against *actual on-chain state*, because that is what the program pays to protect.
- **Assert attacker profit / protocol loss** against live balances, with the numbers printed.

Generate the compliant skeleton with `auditooor-scan harness <file> --fork --address <live_addr> --rpc-env <ENV> --block <N> --out <test>`. It emits the fork setup, an interface to the live contract, and the profit assertion; you fill the exploit sequence. The fresh-deploy invariant suite (`harness` without `--fork`) is for *discovery* — it finds the break; the fork PoC is what you *submit*.

## What does NOT count as proof

- Inter-agent agreement, vector-family hits, or "N detectors flagged it."
- A test that passes only under state the program can never reach.
- A test whose trigger requires a trusted role to act maliciously, unless the program explicitly scopes admin actions into bounty range.
- A ZK "PoC" that shows the constraint is missing but not that a forged witness verifies.

## Cross-chain proofs

A cross-chain bug needs the *seam* proven, not just one leg: demonstrate the message/state that the source leg emits and the wrongful action the destination leg takes on it (replay, unauthenticated mint, mismatched decimals, reordered delivery). Two single-leg PoCs stapled together do not prove a cross-chain bug — the exploit is the interaction. See `references/cross-chain.md`.

## Gate result

`PROVEN` (all four properties + printed loss) → eligible for the report. Anything else stays a **LEAD** and never enters `## Verified Findings`. Auditooor's only allowed adjustment to any engine's proof step is to *raise* it to this standard, never lower it.

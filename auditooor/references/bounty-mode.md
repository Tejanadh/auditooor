# Bounty Mode (`--bounty`)

An audit is paid per engagement; a bounty is paid per **accepted** report. An invalid, duplicate, or oversold report costs reputation and pays nothing. `--bounty` keeps the whole pipeline and adds three gates.

## Gate 0 — Expected value, before reading code

Resolve the program: platform, max payout, assets in scope, severity table, exclusions (most exclude trusted-role, griefing, temporary DoS, third-party oracle failure, and anything in prior audits or known issues), PoC requirements (Immunefi requires a fork PoC against live state), KYC.

Abort (`EV: ABORT <reason>`) if: no live program for these contracts; the in-scope contracts hold no reachable value; the code under review is not what is deployed (diff the verified source); or the program's exclusions cover every impact class the code could plausibly have. Aborting here is a success — it costs nothing.

Pass hunters `Game: bounty` and the program's exclusions verbatim.

## Gate 1 — Impact

A bounty finding needs all of: an **unprivileged** attacker, a **live** deployment in its current configuration, and a **material** impact in the program's severity table. Fill for every candidate:

```
impact_bound: maximum value at risk while the condition holds, with units, from live state
recoverable:  yes/no — and by whom
loser:        the cohort that loses it
attacker_gain: what the attacker takes, or "none — griefing"
program_class: the exact severity row it maps to
```

A cheap permissionless trigger is reachability, not impact. If the condition clears itself once enough value arrives, `impact_bound` is that threshold.

## Gate 2 — Novelty, before proof

Proving a duplicate pays zero. For each survivor, before the prover runs:

1. Search the program's known-issues page and every prior audit report for the mechanism (not the wording).
2. Web-search `"<protocol>" <mechanism> <function>`, the fork parent's name + mechanism, and the platform's disclosed reports.
3. Check the repo's commit history and open/closed issues and PRs for a fix or discussion.

Any hit → `DEAD-DUP`, listed in Rejected with the link.

## Gate 3 — Proof against live state

Bounty PoCs fork the live chain at a pinned block and attach to the deployed addresses — never `new Target()`:

```solidity
function setUp() public {
    vm.createSelectFork(vm.envString("RPC_URL"), BLOCK);
    target = ITarget(DEPLOYED_ADDRESS);
}
```

Never send a transaction to a live network. `UNPROVEN` candidates are **dropped** in bounty mode, not reported.

## Output

Rank survivors by expected payout `P(accept) × payout tier`, not by severity label, and write each in the program's submission format.

# Impact Model — the reward-first brain

The single reason this layer exists: **a code flaw is not a bug, and a bug is not a payout.** This file is the filter that turns "the code is wrong here" into "an attacker takes $X here, and the program pays me $Y for showing it."

The crit* engines are excellent at finding *unusual code*. Left alone they will hand you rounding quirks, missing zero-checks, and gas foot-guns — all true, all worthless. This file makes them hunt money and kills everything else.

## EV is a target RANKER, not a go/no-go gate

Payouts are savagely fat-tailed (one $3M bounty was 38% of Immunefi's entire Q1 2026). So the highest-value decision each day is **which program**, not "is this one worth it". Run the EV logic below across the *whole live program list* and rank; hunt the top, not the first. A go/no-go gate on a single target leaves the fat tail on the table.

## Fortress detection — TWO lanes, not one (do not bury the whale)

Audit density crushes EV for the **machine-auditable lane** but barely touches the **human-moat lane**. Score them separately:

- **Machine-auditable EV** (what `surface`/`detectors`/the muscle can find): a target with many prior audits, formal verification (Certora/Halmos), long time-live, and heavy prior agent-sweeps is a **fortress** — everything machine-findable is gone, EV ≈ 0. The scanner-swarm proves it: 20 agents × 14 fortress targets (Lido 120+ audits, Fluid V2 + Certora, Ondo $3.7B) → **$0**. For this lane, **high audit density + core code = bottom-rank**, and point the muscle at the **un-audited periphery** instead (`auditooor-scan seams` → hunt the SEAM, not the fortress core).
- **Human-moat EV** (a novel, protocol-specific *logic/economic* bug you author invariants for): audit density should **barely move this**. A critical in Aave/Lido pays multiples of a critical in a rando protocol, and "120 audits + $3.7B TVL + a deep economic assumption nobody has modeled" is **the whale, not a fortress**. Never let the audit-density rule steer you away from a blue-chip core for the one bug class that is your actual edge.

Practical rule: `machine_EV = payout_tier × P(muscle finds it) × (1 / audit_density)`; `human_EV = payout_tier × P(you model an un-modeled economic assumption)` — audit density is almost absent from the second. Rank a target by `max(machine_EV_on_seams, human_EV_on_core)`, never by audit count alone.

## <a name="ev-precheck"></a>EV pre-check (Phase 0 — before any code is read)

Answer these. If any answer is "no / unknown and unknowable", emit `EV: ABORT`.

1. **Is there a live program with a bounty?** Immunefi / HackenProof / Cantina / Code4rena / Sherlock / a self-hosted policy. No program → no payout → abort (unless the user explicitly says otherwise).
2. **Is the code in scope?** Out-of-scope files are worth zero no matter what's in them. Intersect the file set with the program's scope *now*.
3. **Is value actually reachable?** A max-severity bug in a contract holding $0 or paused/undeployed pays nothing. Confirm TVL / reachable value at the target address.
4. **What is the realistic payout tier?** Program max is a ceiling, not an expectation. Estimate the tier by severity policy and cap (`min(policy_cap, %_of_funds_at_risk)`).
5. **Is a solo hunter competitive here?** Contest with 200 wardens and a mature codebase → your EV per hour is low. Fresh deploy / thin crowd → high. (This mirrors critfindsaudit Phase −2 crowding.)

`EV = P(you find a novel payable bug) × P(accepted) × payout_tier − token_cost`. If `EV ≤ 0`, abort and say why. Aborting cheaply is a win — it's dollars-per-run, and a skipped bad run raises the average.

**Run the EV gate as a HARD verdict — do not let a correct fortress ranking still spawn a fleet.** (Field failure this fixes: a $10k, 4-audit, 5-yr, patched target was *correctly ranked* a fortress, but the fleet ran anyway because nothing stopped it — the opposite of dollars-per-run.) Compute the verdict deterministically:

```
auditooor-scan ev --cap <max_bounty_usd> [--audits N] [--age-years F] \
                   [--crowded] [--fresh-code] [--seam-value] [--fleet-cost USD]
```

- **`ABORT`** → do not spawn the fleet. Low-cap multi-audited old cores hard-kill regardless of the arithmetic; negative dollars-per-run kills the rest.
- **`SCOPE-ONLY`** → marginal. Run cheap Phase-0 + `seams` recon only; escalate to a full fleet *only* if recon surfaces a concrete un-audited value seam (pass `--seam-value` and re-run).
- **`PROCEED`** → positive dollars-per-run; run the fleet. Note a high cap ($500k) can clear the bar even on an audited core — the gate respects cap, it does not blanket-abort "audited".

The only real lifts on a picked-clean core are `--fresh-code` (launch/DIFF window) and `--seam-value` (un-audited periphery holding reachable value). Absent both, an audited low-cap core is an `ABORT`, not a hunt.

6. **Ingest the program document itself — not just "funds in scope".** Pull and read, before any code: the program's **severity rubric** (how Critical/High map to dollars — this *is* your `payout_tier`), the **known-issues / previously-reported** section, **prior audit reports**, and the **additional guidelines/PoC rules**. This one step feeds three pillars at once: it is your real **payout-tier map** (impact), your first **world-novelty / dup filter** (novelty — a report is closed if it matches a known issue), and your **compliance check** (Immunefi closes non-compliant reports regardless of validity). Skipping it is how a real bug earns $0.

## <a name="impact-map"></a>The impact map (Phase 2 — steer the engines before they hunt)

Before any engine runs its vector taxonomy, build this. It is the target expressed as *value and control*, not as code.

For the target, enumerate:

- **Value sinks** — every place funds/shares/collateral/rewards accrue or rest. Address, token, approximate size.
- **Movers** — every entrypoint that can *decrease* a value sink or *increase* a claim on it. For each: who can call it (permissionless? role-gated? owner?), and what it checks.
- **The shortest permissionless path** from an attacker's starting balance to a value sink decreasing in the attacker's favor. This is the thing to hunt. Depth matters: **live bugs are usually simple and hide in complex paths** — rank paths by how many auditors would have skimmed them, not by cleverness.

Hand this map to the routed engine as the hunting priority. Order every engine's effort by *proximity to a value sink*, not by vector-family completeness.

## The code-flaw-vs-bug test (gate before Phase 4)

A candidate proceeds to PoC forging **only if it clears all four**:

1. **Value delta.** Name the value sink that changes and the sign of the change. "Rounding is down here" is not a value delta. "Attacker's share claim exceeds deposits by N wei per call, unbounded by repetition" is.
2. **Attacker reachability.** A concrete caller reaches the flawed line with attacker-controlled inputs and no honest precondition that never holds. If the trigger requires the owner to be malicious, it's a trust assumption, not a bug (unless the program scopes admin actions in — check).
3. **Materiality — and AMPLIFICATION is part of it, not an afterthought.** The value delta, integrated over feasible repetition *and the largest capital an attacker can point at it*, crosses the program's minimum payable severity threshold. Payout *tier* is frequently decided entirely by amplifiability: a 0.1% rounding error or a 1-wei skew is nothing until a **flash loan** pushes $500M through it in one transaction (62.1% of price-manipulation attacks used flash loans). Immunefi sizes funds-at-risk as **token count × price** — so always ask: *what is the largest position an attacker can borrow/stake/mint to multiply this delta, in a single atomic tx, with no capital of their own?* An impact map that scores the per-call delta without the amplifier systematically under-tiers the bugs that pay the most. (Never argue yourself out of the *mechanism* while hunting; argue relentlessly about *size* while sizing — but size *with* the flash-loan multiplier, not without it.)
4. **Not intended.** Cross-check the intent ledger. Fees, slippage, haircuts, and rounding *toward the protocol* are usually intended. Peers-do-it-differently is a question, never a finding.

Fail any → the candidate is a **code flaw**. Log it, do not forge a PoC, do not report it. Forging proofs for code flaws is the main way these runs go EV-negative.

## Sizing → dollars, not severity labels

Rank survivors for submission by expected dollars:

```
E[$] = P(accepted) × payout_tier × novelty_confidence
```

Severity label (Crit/High/Med) is an input to `payout_tier`, not the ranking key. A "High" with a clean PoC on a $50M program and no dup risk outranks a "Critical" that's likely a duplicate on a $2M program. Submit in `E[$]` order; stop when marginal `E[$]` drops below the cost of writing the next report.

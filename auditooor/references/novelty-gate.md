# Novelty Gate — kill duplicates before you pay to prove them

A proven bug that's already public, already patched, or already in your outcome ledger pays **zero** — worse than zero, because you spent PoC tokens on it. Run this gate in Phase 3, *before* any engine forges a proof.

## Run half of it in Phase 0, as a briefing

The gate is cheapest the earlier it fires, and the recorded runs prove it: on Cork
the stack found and fully PoC'd a real bug, then killed it as known-issue #213
(`../../benchmark/CALIBRATION.md`, run 5). Every token after "this is a
`redeemEarlyLv` DoS" was waste, and the kill was sitting in the program's own
known-issues list the whole time.

### The protocol known-issues register (`{scan} known`) — the layer that kills duplicates cheaply

`fingerprint` is self-dedup; the corpus is global classes; neither knows that
*this* protocol's last audit already reported *this* bug. The register closes
that. Adapted from J4X-Security/K.I.T (MIT):

1. **Build once per protocol** (Phase 0): read every audit report, contest, and
   known-issues list, and `known add` each finding — keyed by root cause, surface,
   mechanism, sink, impact. The LLM does the extraction; the binary stores it.
2. **Check every candidate** (Phase 3, before any PoC): `known check` scores the
   candidate against the register on two factors (identifier overlap AND
   root-cause vocabulary). `KNOWN` exits 3 — kill it. This is precisely the step
   that was missing when the Cork run forged a full PoC for known-issue #213.
3. **Feed it forward:** a submitted finding gets `known add`ed, so the register
   grows into an asset that makes every future run against that protocol cheaper.

`known brief` renders the register as the "DO NOT CHASE" block that `pack --brief`
bakes into every hunter bundle — so a known class dies in the hunter, not in Phase 3.

So in **Phase 0**, before any code is read:

1. `{scan} novelty --protocol P --mechanism <the protocol's top mechanism> --sink <its main value sink>` and run the queries.
2. Read the program's **known-issues / previously-reported** section in full.
3. Skim the **titles** of every prior audit's findings and the changelog for this protocol — titles only, not bodies. That is enough to name the classes.

Paste the resulting list of dead classes into the hunt brief as
**"already known — do not chase"**, and hand it to every hunter. A hunter that
never opens a known class costs nothing; a PoC for one costs the whole run.
Phase 3 then only has to clear what is genuinely new.

## ⚠️ Honest limit: `fingerprint` is SELF-dedup only — novelty is the weakest pillar

The `auditooor-scan fingerprint` ledger is a **local** file. It only knows what *you* logged. It **cannot** tell you whether a bug is:
- already public (a disclosed report, a blog, a known exploit),
- in a prior audit report of this protocol,
- in the program's own **known-issues** list,
- or already submitted by another hunter you can't see.

"Duplicate of an existing report" is the **#1 reason live-program submissions get rejected**. So of the three pillars (impact ∧ novelty ∧ proof), **novelty is the hardest and least-covered**, not the cheapest. The fingerprint step is necessary but nowhere near sufficient. **World-novelty requires global prior-art lookup** — which is a manual step today and a top build target:
1. Read every prior audit of this protocol (the audit-report inversion) and its changelog.
2. Search disclosed reports (Immunefi/Code4rena/Sherlock published findings, GitHub issues/PRs).
3. Read the program's known-issues / previously-reported section — Immunefi programs can close a report on non-compliance with these.
4. Only then fingerprint against the local ledger for self-dedup.

Treat a local `NOVEL` as "not a *self*-duplicate", never as "novel to the world".

## Corpus pre-filter (baked in — read `corpus_verdict` FIRST)

`auditooor-scan novelty` now emits a `corpus_verdict` from a curated library of **publicly-documented bug classes** (ALM/TWAP-tick MEV, Arrakis spot-mint NAV, first-depositor inflation, fee-rounding, fee-on-transfer, AC-03 uninit-proxy, ecrecover malleability, temp-DoS/grief). This closes the field-reported hole where the local hash returned `NOVEL_LOCALLY` on textbook ALM MEV. It is a **local pre-filter, precision-tuned** — a hit is authoritative, a miss proves nothing.

Act on `corpus_verdict` before spending any web-search or PoC budget:
- **`KNOWN-CLASS / KILL (intended-or-oos)`** → dead. Intended behavior / accepted MEV / trusted-role / trusted-token. Do **not** dress as novel, do **not** forge a PoC. (Also the LEAD-inflation kill: this is where intended-MEV/dust/grief LEADs die.)
- **`KNOWN-CLASS / DEMOTE (low-tier)`** → dust/temp-DoS/grief class. Cap at Low/Medium; only pursue if it compounds to material loss or flips solvency.
- **`KNOWN-CLASS / GATE (payable only if novel instance)`** → known class (e.g. first-depositor) but a *fresh, unmitigated, reachable* instance still pays. Proceed **only** if you can show the standard mitigation is absent AND reachable, then run the full world-search.
- **`CORPUS-MISS`** → no known-class match. **NOT** proof of novelty — this is the genuinely-novel-or-genuinely-unknown bucket, exactly where the paying logic bugs live. Run the full world-search below and fingerprint locally.

The corpus never *replaces* the world-search on a miss; it removes the textbook noise so the search budget goes to real candidates.

**And even world-search has a floor.** It kills *public* dups only. It cannot see the concurrent submission another hunter filed three hours ago, or a finding sitting in a private paid audit — and bounties are first-valid-submission. On hot/crowded programs that concurrent-dup risk is the dominant rejection reason (one program rejected 79/79 submissions). The mitigation is not more searching — it is **speed and target selection**: hunt the un-crowded seam (`seams`, `impact-model.md` fortress detection), not the fresh headline program every agent is already sweeping.

## Executable world-novelty procedure (run this in Phase 3)

The tool generates the lookup plan; the LLM executes it with WebSearch. Do not skip it — this is the pillar most likely to cost you the payout.

```
auditooor-scan novelty --protocol <P> --mechanism <M> --sink <S> [--fork-family <F>] [--ledger <path>]
```

It returns the local self-dedup verdict **plus** a targeted query set (disclosed-reports, protocol-audits, github-issues-prs, mechanism-prior-art, hack-history, fork-inheritance) and a manual-check list. Then:

1. **Run every generated query via WebSearch.** If the mechanism on this protocol (or its fork family) is already written up, disclosed, or patched → `DEAD-DUP`, drop it.
2. **Complete the manual checks:** the program's own known-issues/previously-reported section (a match = auto-close), prior audit reports, the changelog for a post-deployment fix, and fork-inheritance.
3. **Only if all clear** is the candidate world-novel — proceed to the PoC.

Proven end-to-end: a candidate the local ledger rated `NOVEL_LOCALLY` ("first-depositor share inflation" on an ERC4626 vault) was revealed as textbook-known by the `mechanism-prior-art` query (OpenZeppelin/MixBytes/Solodit writeups, real incidents) — a guaranteed duplicate the local hash could never have caught.

## Inputs

For each surviving candidate (post impact gate, pre-PoC):
- mechanism class (rounding-direction / share-inflation / oracle-staleness / access-control-on-deploy / reentrancy / under-constraint / cross-chain-replay / …)
- the exact value sink and entrypoint
- the fork/family the protocol belongs to (Uniswap-v2 fork, Compound fork, Anchor SPL vault, tornado-style mixer, …)

## Checks (any hit → kill or downgrade)

1. **Outcome ledger.** Has Auditooor (or the underlying engine) already submitted this mechanism on this protocol or a sibling fork? If accepted-elsewhere-and-patched → dead. If rejected-as-known → dead.
2. **Public disclosure.** Search prior audits of *this* protocol (the audit-report inversion: prior findings are the map of what's already claimed), Immunefi/Code4rena/Sherlock published reports, and the protocol's own changelog/patches. A candidate matching a disclosed-and-fixed issue is dead; matching a disclosed-but-unfixed issue may still pay under some programs — verify the program's dedup policy.
3. **Fork inheritance.** If the protocol is a known fork and the bug exists in the upstream reference unchanged, it's almost certainly already claimed upstream or explicitly accepted; treat as likely-dup unless the fork *introduced* the flaw.
4. **Contest overlap risk.** In a live contest with a large crowd, a "found in the first hour by reading the obvious entrypoint" bug has high duplicate probability. Assign `novelty_confidence ∈ [0,1]` and carry it into the `E[$]` ranking — do not just pass/fail; a low-novelty high-payout bug can still be worth submitting.

## Output

Emit per candidate: `NOVEL (conf=0.x)` / `LIKELY-DUP (reason)` / `DEAD-DUP (reference)`.
- `DEAD-DUP` → drop; do not forge a PoC.
- `LIKELY-DUP` → forge a PoC only if payout tier is high enough that even a shared bounty clears the report-writing cost.
- `NOVEL` → proceed to Phase 4 at full priority.

## Rule

Never let excitement about a *mechanism* override a duplicate signal. The impact gate protects against unpayable-because-trivial; this gate protects against unpayable-because-known. Both are dollars-per-run, and this one is the cheaper save because it happens before the expensive PoC step.

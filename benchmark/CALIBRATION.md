# Calibration — what auditooor has actually done

Honest data beats big claims. This file is the in-the-wild record: every target
the stack has been pointed at, what it produced, and whether it would have paid.
It is retrospective and self-reported (reconstructed from the operator's run
notes), it is small, and it is not independently verified. Read it as the
**floor** on what is known, not as a benchmark score.

Nothing here has been paid. That is the headline number and it should stay at the
top of this file until it changes.

## The record

| # | Target | Date | Game | Pipeline | Outcome | Would it have paid? |
|---|---|---|---|---|---|---|
| 1 | HyperEVM — CoreDepositWallet + Felix | 2026-08 | bounty | fleet | no payable crit; no payable path on the payable entrypoints | no |
| 2 | Decentraland — Credits + Marketplace | 2026-08 | bounty | fleet | one cashout weakness proven, low payability; not submitted | no |
| 3 | DeXe — gov-core | 2026-08 | bounty | fleet + invariants | 4×-audited + SphereX; invariants held; no crit | no |
| 4 | Charm — Alpha Vaults v2 ($10k cap) | 2026-08 | bounty | **12-agent fleet** | clean; every obvious money path already mitigated | no — and the fleet should never have spawned |
| 5 | Cork | 2026-08 | bounty | fleet | **found + PoC'd a real bug** (`redeemEarlyLv` DoS) autonomously | no — killed as a known issue (DEAD-DUP #213) |
| 6 | Reserve Index DTF — Folio 6.0.0 | 2026-08 | bounty | 8-agent fleet + 6.0.0 diff | clean; one JIT self-fee thread, self-defeating/MEV | no |
| 7 | Snowman | 2026-09 | First Flight (EXP) | 12-agent fleet **and** a solo pass | 9 findings; only L-3 came from the fleet | n/a (EXP-only) |
| 8 | MyCut | 2026-09 | First Flight (EXP) | solo, 2 rounds | 3H / 3M / 8L, 13 PoCs | n/a (EXP-only) |
| 9 | Alpenglow | 2026-09 | contest → standing program | scope + DIFF hunt | competition was closed (P=0); pivoted; every lead died on per-rank dedup / 20% byzantine bound | no |

**Totals:** 9 runs · 1 real bug found and proven autonomously · 0 novel payable
bugs · **$0 paid** · 2 EXP contest runs with graded findings.

## What the record actually says

**The methodology works; target selection is the whole game.** Run 5 is the proof
the pipeline can find and prove a real bug with no human pointing at it. It was
killed by novelty, not by correctness — the bug was already public. Every dollar
lost in this table was lost at target selection, not at hunting.

**The fleet is not where the value came from.** Run 7 ran a 12-agent fleet and a
solo pass on the same target: the fleet added exactly one finding (L-3) over the
solo pass. Run 6's 8-agent fleet came back clean. Run 4 spent a full 12-agent
fleet on a $10k, 4×-audited, 5-year-old core and found nothing, which is the
expected result and the reason the EV gate now hard-kills that profile. This is
the direct evidence behind **LITE as the default** and behind `LITE_REACH = 0.55`
in `ev.rs` — if anything that constant is generous to DEEP.

**Fortresses are the dominant failure.** Runs 1, 3, 4, 6 were all mature,
multiply-audited code. Four of nine runs were spent on targets the EV gate would
now abort in Phase 0 for the price of two commands.

**Novelty has to come earlier.** Run 5 forged a full PoC before the duplicate was
discovered. Everything after the moment the bug class was identifiable was waste.
Hence the Phase-0 prior-art brief in `lite-mode.md`.

## What this record does *not* establish

- Nothing about **precision** in the sense a benchmark measures it — there is no
  labelled ground truth here, so there is no true-positive rate to quote.
- Nothing about **dollars**. $0 paid over 9 runs is a real number, but with one
  duplicate as the only real find it cannot separate "the tool is weak" from
  "the targets were bad". Both are consistent with it.
- **Token cost per run was not recorded.** That is the single biggest gap in this
  file and the reason LITE's savings are argued from pack size, not from run
  cost.

## Recording the next run (do this every time)

Every run, including aborts, gets a row. An abort is a data point: it is the
cheapest possible outcome and it belongs in the denominator.

```
auditooor-scan outcome --protocol <name> --mechanism <mechanism> --sink <sink> \
  --lane machine|human --status submitted|accepted|rejected|duplicate|no_response \
  [--payout N] --notes "mode=<LITE|DEEP|ABORT> tokens=<n> phase=<0|1|2|4> role=<finder>"
```

Read it back with `auditooor-scan outcome --stats`. Then add the row here with
the four columns that matter: **pipeline, outcome, would-it-have-paid, tokens**.
Until `tokens=` is in every row, the cost side of dollars-per-run is guesswork —
the LITE-is-cheaper claim is currently argued from pack size, which is a proxy, not
a measurement. `role=` answers the other open question: whether the three LITE
lanes actually cover the paying bugs.

**Two open questions this file cannot answer yet**, stated so nobody mistakes the
design for evidence:

1. **LITE quality vs cost.** Nine runs say the fleet added ~1 non-payable lead per
   target. That is an argument for LITE, not proof that LITE finds what DEEP finds.
   Only runs where a *payable* bug existed can settle it, and this table has none.
2. **Novelty against the invisible.** The Phase-0 brief kills public prior art.
   Concurrent submissions and private findings remain unknowable — the Cork row is
   a known-issue kill, which is the easy case.

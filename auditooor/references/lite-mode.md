# LITE — the default run

LITE is what `/auditooor <path>` does when nobody asks for anything else. It is
not a degraded DEEP. It is the same five gates (EV → impact → novelty → proof →
outcome) bought with roughly an eighth of the tokens, because on almost every
target the extra reach of a 15-agent fleet is worth less than the burn it adds.
`{scan} ev` prints `"mode"` — obey it. DEEP is a purchase, not a default.

## The whole flow

```
Phase 0   EV + prior-art brief   (no code read)          → ABORT here or continue
Phase 1   {scan} pack (LITE)     (one command)           → ABORT here or continue
Phase 2   <=3 hunters on the focus set                   → candidates
Phase 3   novelty (already half-done in Phase 0)         → survivors
Phase 4   Foundry / fork PoC on survivors only           → findings
Phase 5   report + outcome ledger
```

No coverage wave. No skeptic wave unless a candidate reaches Phase 4 — then one
skeptic on that one candidate, because the skeptic is cheap next to a PoC and
expensive next to nothing.

## Two hard aborts (this is where the savings are)

**Abort A — after Phase 0, before any file is read.**
`{scan} ev --cap <max> [--audits N --age-years F --crowded --fresh-code --seam-value]`

- `mode: ABORT` → stop and say why. Do not "just take a quick look". The quick
  look is how Charm cost a fleet at a $10k picked-clean core.
- `verdict: SCOPE-ONLY` → print the scope report and stop. Escalate only if the
  user names a concrete un-audited seam.
- No live program / no reachable value / out-of-scope path → stop.

**Abort B — after Phase 1, before any agent spawns.** From `xray.json`:

| Signal | Action |
|---|---|
| `posture.verdict == FORTRESS` | ABORT. Retarget. A fleet at a fortress finds $0. |
| 0 permissionless rows with `value_flow != none` in `entries.md` | ABORT — nothing to steal, nothing to pay. |
| every ranked file is an audited-core contract, no SEAM | ABORT or hand back a scope report. |
| in-scope code already covered by the prior-art brief | ABORT — it is a known issue. |

An abort is a **successful** run. It cost two commands and it saved the fleet.

## Phase 0 also buys the prior-art brief

Before any hunting, spend one cheap search pass on the program's **known
issues, prior audit reports, and changelog**, and paste the result into the hunt
brief. This is the cheapest token in the pipeline: every known-class bug killed
here is a hunter that never chases it and a PoC never forged. Novelty in Phase 3
then only has to clear what is genuinely new. See `novelty-gate.md`.

## Phase 1 — one command

```
{scan} pack <dir> --out <dir>/auditooor-recon --agents {hunters} --brief <p0-brief.md>
```

The default pack is LITE-sized: **3 headline bundles** (money-map, lifecycle,
spec-divergence) and only the **top-8 files by `risk_score`** inlined as
`focus.md`, with every other file listed as a path manifest at its end. A file an
agent needs but did not get, it reads by path. Nobody pays for a file nobody opens.

`--brief` puts the Phase-0 prior-art list at the top of every bundle as
**"already known — DO NOT CHASE"**, so a known class dies in the hunter instead of
in Phase 3 after a PoC.

Override when you have a reason: `--focus N`, `--roles a,b,c|all`, `--deep`,
`--full-source`.

### The source.md rule (hard)

**LITE does not write `source.md`, and you do not read it.** The whole tree
concatenated on disk is exactly the file an orchestrator wanders into, and one
such read costs more than the focus set saved. In LITE:

| Allowed to read | Never |
|---|---|
| `hunt.md` (orchestrator) | `source.md` — it does not exist in LITE, and `--full-source` is not the default for a reason |
| `focus.md` + bundles (agents) | the repo by glob |
| a deferred file **by path**, when a lead points at it | the whole deferred manifest, file by file |

If you catch yourself wanting the whole tree, the honest move is to say so and
propose DEEP — not to read it silently inside a LITE run.

## What an agent gets — exactly four things

1. Its short role prompt (target, game, posture, impact-map pointer).
2. Its bundle **path** — never inlined source in the prompt.
3. The prior-art brief (already inside the bundle via `--brief`).
4. The shared + bounty rules the bundle carries.

Not the doctrine. Not the recon directory. Not another agent's findings (the
money-map agent's isolation is a hard rule). In LITE the bundle also omits the
senior-auditor SOP — the role file plus the shared rules already carry the method.

## Phase 2 — at most three hunters

**Print the commitment banner first:**

```
LITE | agents: 3 max | coverage wave: no | skeptic wave: no (one skeptic at Phase 4 only)
     | source.md: not written, not read | reading: focus.md + bundles + prior-art brief
```

Then spawn in one message, in the background, each pointed at its bundle:

| Posture | Roles |
|---|---|
| `SEAM` | `periphery`, `money-map`, `access-control` |
| `HUNT` / mixed | `money-map`, `lifecycle`, `spec-divergence` |
| `INVARIANT` | skip hunters — go to `/auditooor FUZZ` |

This mapping is a **hypothesis under measurement**, not a settled answer: it is
three roles chosen because they cover accounting, state-transition and intent,
which is where the recorded findings clustered. Log the role that produced each
candidate in the outcome notes (`role=`), and when the ledger says a lane never
pays on a posture, change the mapping rather than adding a fourth agent.

Prompt stays as in `hunting-fleet.md`, minus the coverage-wave language. Agents
get bundle paths, never inlined source in the prompt.

## Phase 3 — do not pay for novelty twice

The Phase-0 brief already cleared the known classes. In Phase 3, run the
world-search **only** for mechanisms the brief did not cover; mark the others
`novelty: covered-by-phase-0-brief` and move on. A candidate whose class *is* in
the brief should never have reached here — kill it, and note the leak, because it
means a hunter ignored the "do not chase" block.

## Escalating to DEEP

LITE escalates **only** by an explicit decision, and the decision is the user's.
Print this and stop:

```
LITE found <n> impact-mapped candidate(s) on <files>.
ev mode: <LITE|DEEP>   ev_usd: <x>   ev_lite_usd: <y>
DEEP would add: <the specific lanes LITE did not cover>
Run `/auditooor DEEP <path>` to buy it.
```

Never auto-escalate. Auto-escalation is how a cheap default becomes an expensive
one — and the calibration runs in `../../benchmark/CALIBRATION.md` show the fleet
adding roughly one new lead per target over a focused pass, none of them payable.

## What LITE gives up, honestly

- Lanes nobody is watching: boundary/numerical/asymmetry hunters do not run, so a
  pure off-by-one with no money keyword near it can slip.
- Files below the focus cut get read only if a lead points at them.
- No cross-agent disagreement, which is where some of the fleet's value lives.

That is the trade. Buy DEEP when the cap is large, the code is fresh, or LITE
already put a candidate on the impact map — not to feel thorough.

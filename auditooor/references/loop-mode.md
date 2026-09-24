# LOOP — the v4 pass, kept inside the bounty gates

`/auditooor LOOP [N]` is the thing solidity-auditor v4 added that a one-shot fleet does not have. v4 (commit f6c7f0d) runs the same 12 specialties again, and every pass after the first is told what the earlier passes found, so it hunts new ground and reuses the same `bug_class` label for the same bug. A measured 3-pass scan of ~2.2k lines took about 45 minutes. That is a purchase. LITE stays the default.

`N` defaults to **3**. Below 1 or above 5, use 3. The user typing a number wins (`/auditooor LOOP 2`).

EV and Abort B still run first. A fortress does not get three passes.

## What a pass is

One pass is the **12 solidity-auditor specialties**, all spawned together, each pointed at its bundle path:

`math-precision`, `access-control`, `economic-security`, `execution-trace`, `invariant`, `periphery`, `first-principles`, `asymmetry`, `boundary`, `numerical-gap`, `trust-gap`, `flow-gap`.

Pass 1 also spawns the three auditooor headlines (`money-map`, `lifecycle`, `spec-divergence`) in the same message. Passes 2..N do not. Those three already ran once; the loop's job is the second look, not a second product.

Pack for every pass is full source, SOP included, deploy scripts included (`script/`, `*.s.sol` are in scope):

```
{scan} pack <dir> --out <dir>/auditooor-recon --agents {hunters} --deep \
  --roles math-precision,access-control,economic-security,execution-trace,invariant,periphery,first-principles,asymmetry,boundary,numerical-gap,trust-gap,flow-gap,money-map,lifecycle,spec-divergence \
  --brief <p0-brief.md>
```

On pass 2 and later, drop the three headlines from `--roles` and add `--known <recon>/known-findings.md`.

## Hunter prompt (every pass)

Four things, plus the known-findings paragraph only when that file was appended:

```
You are an attacker. Specialty, source, and output rules are in the bundle. Read it fully.

Read first: <bundle path> — do not re-read in-scope files for the initial scan.
You are read-only inside the repository. Do not create or edit a file there.
description: and fix: are Simplified Technical English (the bundle's Report language section).
bug_class, group_key, and quoted code stay verbatim.

Output format: the bundle's shared rules and bounty rules. Bounty rules win on impact sizing.
```

When `known-findings.md` is in the bundle, add:

```
The bundle ends with "Known findings — ground already walked". Spend your reading on
functions and mechanisms that list does not name. If you reach a listed bug again,
report it in full with that exact bug_class label. Invent a label only for a new class.
The "DO NOT CHASE" brief at the top is different: those are public prior findings. Do not report them.
```

Do not respawn a dead agent. The next pass covers that specialty. If pass 1 returns nothing, stop. If a later pass returns nothing, stop the loop and judge what you have.

## Between passes

Wait until every agent from this pass has finished. Dedup with `group_key` (`judging.md`). Then write `<recon>/known-findings.md` from this pass's FINDINGs and LEADs, grouped by `Contract.function`. One bullet per item: `` `bug_class` — FINDING|LEAD — one-line title ``.

The file starts with this prose, then the groups. Do not paraphrase it:

```
# Known findings — ground already walked

These are findings and leads from earlier passes of THIS scan, grouped by contract and function.

Spend your reading on functions and mechanisms this list does not name.
If you reach a listed bug again, report it in full and reuse the backticked bug_class label.
A new label for the same class is a second record. The scanner will not merge them.

The "DO NOT CHASE" section at the top of the bundle is public prior art. It is not this list.
Do not write those up again.
```

Rebuild the bundles with `--known` before spawning the next pass. One recon directory. Overwrite the bundles. The durable record is `known-findings.md` plus each pass's raw blocks, saved as `<recon>/pass-K.md` before the overwrite.

## Across scans

Append every deduped item to `$HOME/.claude/auditooor/hunt-memory/<repo-basename>.tsv`.

Header, once: `#auditooor-hunt-memory v1`

Columns, tab-separated, six fields: `key`, `kind` (`FINDING` or `LEAD`), `scans` (integer), `confidence` (`0` if you have no number), `title`, `bug_class`.

`key` is `contract|function|bug_class`, lowercased, every run of non-alphanumeric characters collapsed to one hyphen.

On a later LOOP or DEEP of the same repo, before pass 1:

1. Build a name list from the packed source: `contract`, `library`, and `function` names, normalised the same way as a key segment.
2. Drop a memory row whose contract or function is absent. `constructor`, `receive`, and `fallback` stay when the contract stays. Print each dropped row. Do not drop it silently.
3. If any row remains, write `known-findings.md` from them and pass `--known` on pass 1 as well.

A bad header or a row that does not have six fields stops the scan. Do not start fresh over a broken ledger.

## After the last pass

One judging pass, one skeptic wave, then the normal Phase 3–5 gates. The loop does not skip them.

- Same `group_key` across passes is one candidate. Note how many passes saw it. Convergence raises priority. It is not a proof and it is not a payout.
- Public prior art and `scan known check` still kill a candidate before any PoC. v4 re-reports a bug the client already knows, because the engagement pays for the report. A bounty pays for a new bug. Re-reporting a listed public issue is a wasted PoC.
- Hunters stayed read-only. The prover writes the fork PoC under `<recon>/poc/`, not into `src/` or `test/`.

## What LOOP does not copy

v4's `assemble.sh` prints an engagement report. A bounty submission is still Phase 5 plus `/negotiate-bounty` when that skill is installed. Do not paste the Pashov report template into an Immunefi form.

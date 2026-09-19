# Blind judge — instructions

You score two anonymous audit reports, **Report A** and **Report B**, against the contest's judged High/Medium findings (ground truth). You do not know which tool produced which report and must not try to infer it. Never favour length, formatting, or confidence language.

## Inputs
- `gt.json` — ground-truth findings: `finding_id`, `severity`, `title`, `description`.
- `report-A.md`, `report-B.md` — each tool's final report. Everything the report presents counts as *reported*: findings of any severity or status **and** leads. Items in a "Rejected"/"refuted" appendix do **not** count as reported.
- The source code under `src_root` — read it when you need to decide whether two descriptions are the same root cause, or whether a reported finding is factually wrong.

## Step 1 — Match ground truth (one decision per GT item, per report)

A report item **matches** a GT finding only if it identifies the **same root cause** — the same code-level defect in the same place — such that fixing what the report item describes would fix the GT issue. Same function but different bug = no match. Same bug, different wording = match. A generic item ("reentrancy risk in the contract") that does not pinpoint the defect = no match.

Record the match tier:
- `finding` — matched by a non-lead item (any severity/status that the report presents as a finding)
- `lead` — matched only by a lead
- `none`

## Step 2 — Classify every report item that did not match GT (both reports)

For each unmatched **finding** (not leads), read the code and classify:
- `valid-unlisted` — a real issue the contest did not list (contests are not exhaustive; low severity, or genuinely missed)
- `invalid` — the claimed exploit does not work (quote the line that stops it) or the issue is by-design / out of scope
- `factually-wrong` — the item asserts something checkably false about the code (a function, check, or behaviour that does not exist). Also count this for *matched* items whose details are materially false.

Be economical: a one-line reason each.

## Output — write `scores.json`

```json
{
  "gt": [{"finding_id": "...", "severity": "high|medium",
          "A": {"tier": "finding|lead|none", "item": "<report item id/title>", "reason": "..."},
          "B": {"tier": "...", "item": "...", "reason": "..."}}],
  "unmatched": {"A": [{"item": "...", "class": "valid-unlisted|invalid|factually-wrong", "reason": "..."}],
                "B": [...]},
  "factually_wrong_matched": {"A": [...], "B": [...]}
}
```

Then reply with a compact table: for A and B — High matched as finding / as lead, Medium matched as finding / as lead, reported findings, invalid, factually-wrong.

# X-Ray — bounty-first recon pack

`/auditooor XRAY` is the cheap pass: decide *whether* and *where* to hunt before a fleet or a PoC. It is not a contest-prep report and it does not emit findings.

Run once:

```
{scan} pack <dir> --out <dir>/auditooor-recon [--agents {engine}/references/hacking-agents]
```

That writes `xray.json` (detect, posture, surface, seams, entries, **census**, **properties**, git), `source.md` (every in-scope file concatenated — this is what agents read), `entries.md`, `PROPERTIES.md`, `hunt.md`. Do **not** re-run `detect` / `surface` / `seams`. Print `hunt.md`. Overlay protocol-specific invariant wording onto `PROPERTIES.md`. Stop.

## Posture (the only verdict that matters)

`posture.verdict` is attention, not a bug:

| Verdict | What you do |
|---|---|
| `HUNT` | Unguarded value on sloppy/mixed code. Sweep permissionless `entries` with `value_flow` in/out/both. Then author invariants. |
| `SEAM` | Core looks swept; glue still moves value. Restrict the fleet to `seams[].classification == "SEAM"` contracts. |
| `INVARIANT` | Value is reachable but the paying bugs are protocol-logic. Skip signature grinding. Go to `/auditooor FUZZ` / Phase 4 harness. |
| `FORTRESS` | No permissionless value, no seams, high complexity. `EV: ABORT` or retarget. Do not spawn the 12-agent fleet to feel busy. |

If Phase 0 already aborted, x-ray is optional colour — do not override an EV abort with a juicy surface score.

## Hunt brief (write this, nothing else)

Keep it under ~80 lines. No architecture SVG. No vendor-neutral filler.

1. **Posture + why** — copy `posture.verdict` / `posture.why`.
2. **Permissionless money map** — every `entries[]` row with `access=permissionless` and `value_flow != none`. That list *is* the impact map seed (`impact-model.md#impact-map`).
3. **Seams** — SEAM contracts, one line each. CORE is a non-target unless a permissionless value entry lives there.
4. **Git hotspots** — if `git.available`, overlay `hotspots` and `fix_commits` on the money map. A fix-shaped commit on a permissionless sink is a differ-archetype lead (`hunting-doctrine.md`). `squashed_import: true` → skip git, it is noise.
5. **Invariant seeds** — already in `PROPERTIES.md` (pairs, aggregates, conservation, guard-lifts). Add 3–7 *protocol-specific* lines; do not rediscover that deposit↔withdraw exists.
6. **Test gaps** — `census.foundry_invariant` / `echidna` / `medusa` / `fork_tests` = 0 is a hunt signal (the 89% is untested), not a finding.
7. **Next command** — one of: `ABORT`, `/auditooor FUZZ <dir>`, `/auditooor DEEP <dir>` (fleet), `/auditooor DIFF`.

## What this is not

- Not a threat-model novel. Attack surfaces that cannot be tied to a permissionless `entries` row are out of scope for a bounty run.
- Not a finding. Detector hits inside the JSON are LEADs; they still need impact ∧ novelty ∧ proof.
- Not a substitute for Phase 0. No live program → recon is practice, not a hunt.

# No-binary fallback — run auditooor with nothing but a shell

`{scan}` is a token-saver, not a dependency. If `cargo` is missing, the build
fails, or you are somewhere you cannot compile, **do not abort the hunt** — say
`scan: unavailable (pure-prompt mode)` and run the commands below. The gates are
unchanged; only the recon is coarser.

Try the build once first: `bash {skill}/install.sh` or
`bash {skill}/tools/auditooor-scan/build.sh`. One failure is enough — fall through.

## Phase 0 — EV, by hand

`{scan} ev` is arithmetic. Do it yourself:

```
p = 0.12
  × 0.7^audits                (each prior audit picks the core cleaner)
  × 1/(1 + 0.15 × age_years)  (older = more swept)
  × 0.40 if crowded/hot
  × 1.80 if fresh/just-shipped code
  × 1.60 if an un-audited value seam holds funds
clamp p to [0.005, 0.60]

EV_deep = cap × p        − fleet_cost   (fleet_cost ≈ $200-equivalent)
EV_lite = cap × p × 0.55 − lite_cost    (lite_cost ≈ $25-equivalent)
```

- `audits ≥ 3` and `age ≥ 3y` and no fresh code and no seam and `cap < $50k` → **ABORT** (the fortress profile).
- `EV_deep ≤ 0` → **ABORT**.
- `cap × (p − 0.55p) ≤ 3 × (fleet_cost − lite_cost)` → **LITE**. Otherwise DEEP is earned.

## Phase 1 — recon, with grep

```bash
# in-scope files, biggest first (the crude risk proxy)
find <dir> -name '*.sol' -not -path '*/test*' -not -path '*/lib/*' \
  -not -path '*/node_modules/*' | xargs wc -l | sort -rn | head -20

# money movers: every value-moving site
grep -rnE '\.transfer\(|\.transferFrom\(|\.call\{value|safeTransfer|_mint\(|_burn\(|withdraw|redeem' \
  <dir> --include='*.sol' | head -60

# the impact map: external/public functions with no obvious guard
grep -rnE 'function .*(external|public)' <dir> --include='*.sol' \
  | grep -vE 'only[A-Za-z]*|auth|onlyRole|internal|view|pure' | head -60

# the seam: routers/wrappers/adapters/migration glue — the un-audited periphery
find <dir> -name '*.sol' | grep -iE 'router|wrapper|adapter|periphery|migrat|helper|zap'

# danger census
grep -rnE 'delegatecall|initialize\(|selfdestruct|assembly|\.balanceOf\(address\(this\)\)' \
  <dir> --include='*.sol' | head -40
```

Focus set = the intersection of "money movers" and "unguarded external" files,
plus everything the seam query returned. That is your top-8. Read those; list the
rest as read-on-demand.

**Abort B still applies:** no unguarded external function touches value → stop.

## Phase 2 — hunters

Spawn the same LITE roles with the same prompts from `hunting-fleet.md`, but
point each at explicit file paths (there are no bundles):

```
Read <focus file 1..8> fully. Your lane: <specialty file from references/hunters/>.
Other files exist — list at the end; read one only if a lead points at it.
```

## Phases 3–5 — unchanged, minus the local ledger

- Novelty: the world-novelty search (`novelty-gate.md`) is the pillar and needs
  no binary. The local fingerprint ledger is skipped; say so in the report.
- Proof: Foundry only. `{scan} harness` is a convenience — write the invariant
  suite by hand from `invariant-library.md`, and the fork PoC from
  `proof-standard.md`. **The PoC bar does not move.**
- Outcome: append a line to `~/.claude/auditooor/outcomes.tsv` yourself —
  `date<TAB>protocol<TAB>mechanism<TAB>sink<TAB>lane<TAB>status<TAB>payout`.

Everything auditooor sells — abort a fortress, hunt the seam, novelty before PoC,
no PoC no finding — survives without the binary. The binary only makes it cheaper.

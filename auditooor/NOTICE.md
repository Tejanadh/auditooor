# Third-party notice

The twelve hunter personas under `references/hunters/*-agent.md`, plus
`senior-auditor-sop.md` and `shared-rules.md`, originate from Pashov Audit
Group's MIT-licensed repository https://github.com/pashov/skills
(commit c577eb7). Their licence is preserved at
`references/hunters/LICENSE-pashov-skills`.

auditooor adds: reward-first EV/novelty/proof gates, the Rust `auditooor-scan`
pre-filter (xray/pack/harness/fork PoC), coverage wave 2, independent skeptics,
Immunefi-shaped fork proofs, spec-divergence and lifecycle hunters, and the
outcome ledger.

`references/exploit-patterns.md` and `references/hunters/money-map-agent.md`
come from the CritFindsAudit engine (https://github.com/critfinds/critfinds-Audit-Skill,
MIT). Danger-keyword census is the v2 Turn 2 recon, compiled into `auditooor-scan danger`.

The protocol known-issues register (`auditooor-scan known`, `references/novelty-gate.md`)
adapts the approach of J4X-Security/K.I.T (https://github.com/J4X-Security/K.I.T, MIT):
build a structured register of a protocol's already-known findings from its real
audit reports, then check every candidate against it before forging a PoC. The
register schema, two-factor matcher, and CLI are an independent implementation.

The logic-vector decomposition (`references/logic-vectors.md`) and the static
confirmation pass (`auditooor-scan confirm`) adapt the method of GPTScan (ICSE'24,
https://github.com/GPTScan/GPTScan): break a logic-bug type into scenario +
property + key variable, let the model propose, then use static confirmation on
the key variable to cut false positives before proof. The checks and CLI are an
independent implementation; no GPTScan code is used.

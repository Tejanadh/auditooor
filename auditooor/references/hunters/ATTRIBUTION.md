# Attribution

The specialist agent files in this directory and `senior-auditor-sop.md` originate from the
**Pashov Audit Group `skills` repository** — https://github.com/pashov/skills — used under the MIT
licence, a copy of which is preserved here as `LICENSE-pashov-skills`.

Files taken essentially as-is:
`senior-auditor-sop.md`, `shared-rules.md`, and the twelve `*-agent.md` specialists
(access-control, asymmetry, boundary, economic-security, execution-trace, first-principles,
flow-gap, invariant, math-precision, numerical-gap, periphery, trust-gap).

## Why these were adopted rather than rewritten

The decomposition is better than what this engine had. CritFindsAudit previously split its scanning
agents by **attack-vector list range** (agent 3 hunts V101-V150), which is an arbitrary slice of a
taxonomy. These split by **reasoning role** — a boundary agent hunts boundaries everywhere, an
asymmetry agent hunts asymmetries everywhere. Bugs cluster by shape, not by list position.

Two further ideas worth naming, both of which this engine lacked:

1. **Enforced mental-tool markers.** Agents must emit literal `[Feynman: ...]`, `[Socratic: ...]` and
   `[Inversion: ...]` tags when the trigger fires, and the orchestrator greps for them afterwards.
   That makes reasoning depth *verifiable* rather than merely instructed.
2. **FINDING vs LEAD with a mandatory `proof:` field**, plus a `group_key` for mechanical dedup.

## What is added on top — `bounty-rules.md`

The upstream skill is built for **audit engagements**, where you are paid per engagement and severity
is negotiated with a client. This engine hunts **bounties**, where an invalid or oversold report costs
you and there is no negotiation. `bounty-rules.md` in this directory carries the delta and is appended
to every agent bundle after `shared-rules.md`.

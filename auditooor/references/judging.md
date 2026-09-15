# Judging — dedup, severity, status

The orchestrator applies this after the hunt waves finish and again after verification. Deterministic where possible, recorded where it is a judgement.

## 1. Dedup (from Pashov Audit Group's solidity-auditor, kept because it works)

1. Run `auditooor.py parse {bundle_dir}` to get every block as JSON. Never dedup from memory of the transcripts.
2. Group by `group_key` (Contract | function | bug-class). Exact match first, then merge synonymous bug_class within the same (Contract, function). Keep the best-evidenced item; annotate `[hunters: N]`.
3. **Function isolation (hard).** Never merge across different functions. Different function = different item.
4. **Mechanism preservation (hard).** A merged group whose members describe different mechanisms (different code-level cause, attack path, or fix) must list every mechanism — or split. Scan the body of every member, not just its tag.
5. **Fix preservation (hard).** If members propose materially different fixes (different check, different parameter, different direction), keep them as Option A / Option B, verbatim.
6. **Cross-function root cause.** When the *same missing check* appears in several functions, keep one item per function but add `Same root cause as #N` so a reader sees the pattern.
7. **Completeness (hard).** Before verification print `Completeness: N unique (Contract, function) in raw, N covered after dedup.` The numbers must match.
8. **Chains.** If A's output satisfies B's precondition and the combined impact exceeds either, add `Chain: #A + #B`.

## 2. What goes to the skeptics

- Every FINDING.
- Every LEAD flagged by 2+ hunters, or whose `unverified:` step is a single concrete check the skeptic can settle by reading.
- Other LEADs go straight to the Leads section unverified.

Batch candidates by contract, at most 8 per skeptic, so each skeptic holds one contract's context.

## 3. Severity rubric (the skeptic assigns; ties → the orchestrator, recorded)

Impact × likelihood, contest-style:

| | Likelihood high (unprivileged, no special state) | Likelihood medium (achievable state / timing) | Likelihood low (unusual state, trusted-role amplifier) |
|---|---|---|---|
| **Impact high** — theft/loss of principal, permanent freeze, insolvency | **High** | **High** | Medium |
| **Impact medium** — loss of yield/fees, temporary freeze, material leak, core function broken | Medium | Medium | Low |
| **Impact low** — dust, recoverable inconvenience | Low | Low | Low |

Do not inflate: a permissionless trigger is likelihood, not impact. Do not deflate by category: griefing that locks user funds for a meaningful period is medium impact.

## 4. Status — what a reader can trust

Every reported item carries exactly one status, from strongest to weakest:

| Status | Meaning |
|---|---|
| `PROVEN` | Executable test passes and asserts the harm (prover). |
| `CONFIRMED` | Independent skeptic traced it end-to-end; no blocking line exists. |
| `UNCERTAIN` | Skeptic could not settle one named step. |
| `LEAD` | Not verified; a trail for manual review. |

`REFUTED` and `DISPROVEN` items never appear in Findings. They are listed, one line each with the blocking quote, in the report's **Rejected** appendix — so a reader can overrule a wrong rejection in seconds.

**Convergence is not proof.** `[hunters: 5]` raises priority for verification; it never changes status by itself.

## 5. Confidence (for ordering inside a severity)

Start at 100. Partial path −20. Bounded non-compounding impact −15. Specific but achievable state −10. UNCERTAIN −15. PROVEN floors at 95.

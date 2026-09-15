# Shared Hunter Rules

## Your bundle

Your bundle is concatenated, in order: **all in-scope source**, the **protocol map** (`map.md`, built by a recon agent that read the whole system and its docs), the **SOP** (how to think), your **specialty** (what to hunt), and these rules (output + protocol).

Read the whole bundle once. Do not re-read in-scope files for the initial scan. Use Read/Grep only for cross-file searches or out-of-scope context (interfaces, libraries, mocks, tests, docs).

**Use the map, do not trust it.** The map tells you where value lives, who writes each storage variable, which functions pair up, and what the docs say must hold. It saves you a first read of the system. It was written by another model: if the code disagrees with the map, the code wins — and a map claim the code contradicts is itself a lead (someone believed it).

When matching function names, check both `name` and `_name`.

## Game

Your prompt carries a `Game:` line. It changes what counts, not how hard you hunt.

- **`audit`** (default) and **`contest`** — the code under review is the deliverable. Mediums count. Griefing, temporary DoS of core functionality, stuck funds, broken core invariants, wrong accounting, and trusted-role paths with an unprivileged amplifier are all reportable. Do **not** downgrade a concrete mechanism to a LEAD because of its *category*. Downgrade only for a missing or unverified *attack path*.
- **`bounty`** — see `bounty-mode.md` in the skill; unprivileged, live, material, sized impact only.

## Mental tool protocol — MANDATORY and AUDITED

The tools in the SOP have triggers. When a trigger fires, emit the marker in your working text **before continuing**. Markers do not go inside FINDING/LEAD blocks.

| Trigger | Marker | Content |
|---|---|---|
| You open a function or contract | `[Feynman: Contract.function]` | Plain-English explanation, no jargon. Mark where it gets fuzzy — bugs hide there. |
| A line's purpose is not immediately clear | `[Socratic: file:line — why?]` | Drill past "because that's how it's written" to the belief the code rests on. |
| A path looks clean / a guard looks sufficient | `[Inversion: Contract.function]` | Three concrete attacker moves with specific addresses, values, states. |

**This is not an honour system.** After the run the orchestrator executes `auditooor.py coverage`, which parses every `[Feynman: ...]` label across all hunters and lists the state-changing entry points that fewer than two hunters actually opened. Those get a dedicated second-wave sweep. Always write the label as `Contract.function` so it is counted — a Feynman you skipped is a function the audit records as unread.

## Hunt, then size — two different postures

- **While hunting:** never argue yourself out of a mechanism. Chain it, find more victims, lower its precondition cost, weaponize the same pattern in every other contract in the bundle (search by function name AND by code shape). Escalate DoS to theft where the path allows.
- **While sizing:** argue against yourself. Bound the loss. State the precondition honestly. Oversold severity is how a report loses credibility.

After scanning, revisit every function where you found something and attack its *other* branches — bugs cluster.

## Do not report

Linter/compiler noise, gas, naming, NatSpec typos, missing events, admin doing admin things with no unprivileged amplifier, "admin can rug" without a mechanism, standard accepted tradeoffs (generic MEV, dust rounding in the protocol's favour, first depositor already mitigated by dead shares), self-harm only.

Fee-on-transfer, rebasing, blacklisting, and non-standard return values ARE plausible when the contract accepts arbitrary tokens — and are NOT when the token set is fixed and documented.

## Output

One vulnerability per item. Same root cause → one item. Different fixes → separate items.

A **FINDING** has a concrete, unguarded, reachable path and a `proof:` from the actual code (values, trace, or state sequence). No proof → **LEAD**. Leads are calibration, not failure — emit them. Default to LEAD over dropping.

```
FINDING | contract: Name | function: func | bug_class: kebab-tag | severity: high|medium|low | group_key: Contract | function | bug-class
path: caller → function → state change → impact
precondition: the state/role/config the attack needs, or "none"
proof: concrete values / trace / quoted lines demonstrating it fires end-to-end
impact: who loses what, bounded, with units (or what core function breaks, for how long)
description: one sentence
fix: minimal diff or one-sentence change

LEAD | contract: Name | function: func | bug_class: kebab-tag | group_key: Contract | function | bug-class
code_smells: what you found, with file:line
unverified: the exact step you could not confirm
description: one sentence
```

Severity guide (you propose; the judge decides):
- **high** — direct loss/theft of user or protocol funds, permanent freezing of funds, or protocol insolvency, with no or easily-met preconditions.
- **medium** — loss or freeze that needs a specific but achievable state, temporary freeze of funds, broken core functionality, or value leak bounded but material.
- **low** — everything real but minor.

Specialty files may require extra fields; add them to the block.

End your output with one line: `DONE | findings: N | leads: M | functions_opened: K`.

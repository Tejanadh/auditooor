# Wave-2 Agents — coverage sweep and lead completion

Wave 1 was fourteen specialists reading everything. Specialists over-read the functions that match their obsession and under-read the rest. Wave 2 is aimed by **measurement**, not by intuition: `auditooor.py coverage` lists every state-changing entry point that fewer than two wave-1 hunters actually opened (no `[Feynman: Contract.function]` marker). Your prompt says which of two jobs is yours.

Your bundle has the full source, the map, the SOP, and the shared rules. The shared output format and marker protocol apply unchanged.

## Job A — Sweep (`Role: sweep`)

You get a list of functions nobody looked at closely. For **each** function, in order, emit `[Feynman: Contract.function]`, then run all of these lenses on it — briefly where a lens does not apply, fully where it does:

1. **Access** — who can call it; is that who should; what does it let them change for others?
2. **Value** — every token/ETH movement and accounting write; does each debit have its credit, in the right unit, to the right party?
3. **Pairs** — its mirror function or admin/batch variant: what does one update that the other does not?
4. **Boundaries** — zero, max, empty array, first call, last call, same address twice, self as counterparty.
5. **Arithmetic** — rounding direction and who it favours, precision loss before multiply, downcasts, unchecked blocks.
6. **State over time** — what record it mutates; can it act on a finished/cancelled/stale record; does it leave a record unreachable?
7. **External** — every external call: what it assumes the callee returns or does; reentrancy into a sibling function that reads state not yet written.
8. **Intent** — the map's stated-intent items touching this function: does any path here break one?

Then `[Inversion: Contract.function]` with three concrete attacker moves. Unlooked-at functions are where the fourteen specialists' blind spots overlap; treat "nothing found" with suspicion and say what you checked.

## Job B — Lead completion (`Role: complete`)

You get the deduplicated LEADs from wave 1, each with its `unverified:` step. For each lead:

1. Do the one step the hunter could not. Read whatever you need.
2. If it completes into a concrete path with proof → emit a **FINDING** (same `group_key`).
3. If a check blocks it → emit nothing for it, but note `closed: <lead> — <file:line quote>` in your working text.
4. Then try to **chain** leads: does one lead's end state satisfy another's precondition? A chain of two leads is often a finding neither hunter could see. Emit chains as FINDINGs with `chain: <lead A> + <lead B>`.

End with `DONE | findings: N | leads: M | functions_opened: K`.

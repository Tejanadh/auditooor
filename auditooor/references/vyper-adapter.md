# Vyper Adapter — native vectors + compiler-version traps + proof harness

Vyper compiles to EVM bytecode, so the EVM value-flow taxonomy (`critfindsaudit`) still applies — route economic/rounding/oracle/reentrancy hunting there. This adapter covers what is **specific to Vyper**: the compiler itself has shipped exploitable codegen bugs, and Vyper's idioms differ from Solidity in ways that create their own traps. Curve's 2023 loss came from a *Vyper compiler* reentrancy-lock bug, not from the contract logic — that's the headline lesson.

## Compiler-version traps (check FIRST — this is the Vyper edge)
- **VY1 — Malfunctioning `@nonreentrant` lock (v0.2.15, 0.2.16, 0.3.0).** These versions generated reentrancy locks that did not actually protect — the storage slot handling was broken. A contract that *looks* guarded is not. **Grep the pragma version first.** If the contract relies on `@nonreentrant` on a vulnerable compiler, the whole reentrancy surface is live even though the code reads as protected. This is the Curve class and still pays where old contracts persist.
- **VY2 — Other known-bad codegen per version.** Vyper's release notes list security fixes (default-value handling, `raw_call` return checks, `create_from_blueprint`, `slice`/`concat` bounds, sqrt/pow rounding). Map the contract's pragma to the compiler's known-issue list for that version; a bug fixed in a later release is live in a contract pinned to an earlier one.
- **VY3 — Immutable/constant miscompilation.** Older versions mishandled `immutable`/`constant` initialization edge cases.

## Vyper-idiom vectors (VY4–VY14)
- **VY4 — Default visibility & decorator gaps.** Missing `@external`/`@internal` intent, or a state-mutating function without the access decorator the author assumed. No modifiers in Vyper — access checks are inline `assert`; an omitted `assert msg.sender == ...` is the bug.
- **VY5 — `raw_call` unchecked.** `raw_call(..., revert_on_failure=False)` return value ignored → failed transfer treated as success. Also `raw_call` with attacker-influenced target/calldata = arbitrary call.
- **VY6 — `send`/`raw_call` value & reentrancy.** External call before state update; combined with VY1 this is critical.
- **VY7 — `create_from_blueprint` / `create_minimal_proxy_to` trust.** Deploying from an attacker-influenceable blueprint address or unverified init.
- **VY8 — Integer semantics.** Vyper reverts on overflow (good), but `unsafe_add`/`unsafe_mul`/`unsafe_sub`/`unsafe_div` explicitly opt out — hunt every `unsafe_*` for wraparound in value math. `//` floor division rounding direction (P1 class).
- **VY9 — Fixed-size list / bytes bounds.** `DynArray[T, N]` and `Bytes[N]` are bounded; logic that assumes more elements, or that silently truncates at N, mis-accounts.
- **VY10 — `for` loop bounded-range assumption.** Vyper requires a compile-time loop bound; logic that needs to iterate over an unbounded set is either truncated (missed elements) or the design forces a cap an attacker can stuff (DoS).
- **VY11 — Decimal type quirks.** Vyper's `decimal` (fixed-point) precision/range differs from integer math; conversions `convert()` between `decimal`/`uint256` lose precision or overflow the decimal range.
- **VY12 — Reentrancy key collision.** Different `@nonreentrant("key")` keys where the author meant the same lock, or the same key where they meant different — locking/unlocking the wrong surface.
- **VY13 — `msg.sender` in delegate context.** `create_minimal_proxy`/library patterns where `self`/`msg.sender` assumptions break.
- **VY14 — Interface mismatch.** `interface`-declared external calls whose real target returns different data (missing return, wrong decimals) — Vyper's stricter ABI decoding can revert or mis-decode.

## Proof harness
Vyper deploys as EVM bytecode, so **prove with Foundry** (the `critfindsaudit` proof standard):
1. Compile the `.vy` with the **exact pinned compiler version** (`vyper==<pragma>`), because VY1–VY3 only reproduce on the vulnerable version — `pip install vyper==0.2.15` (or via `vvm`), `vyper contract.vy -f bytecode`.
2. Deploy the compiled bytecode from a Foundry test via `vm.deployCode`/raw create, or use `foundry`'s vyper support / a small deploy helper.
3. Drive the attack from the Foundry test, assert the impact-map value sink moved wrongly, print before/after. For VY1, the PoC must show reentrancy succeeding *despite* the `@nonreentrant` decorator being present — that's what proves the compiler bug rather than a logic bug.

Clear `references/proof-standard.md` like any other engine.

## Impact framing
VY1 (broken nonreentrant on a vulnerable pin) with a live pool holding funds is top-tier and historically real money. Route the rest of the economic hunt (rounding, oracle, share math) to `critfindsaudit` — this adapter's unique value is the compiler-version trap and the `unsafe_*`/decorator-gap idioms.

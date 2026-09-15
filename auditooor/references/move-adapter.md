# Move Adapter — Aptos & Sui native vectors + proof harness

Move (Aptos, Sui) is under-hunted because it requires reading an ecosystem most EVM auditors skip — which is exactly why it pays. Move's resource model kills whole EVM bug classes (no reentrancy in the EVM sense, linear resources prevent double-spend of a value) but introduces its own. Hunt these, not the EVM taxonomy.

## What Move gives you for free (don't waste time here)
Linear resources can't be copied or implicitly dropped; no null; no reentrancy via `call` (there is no dynamic dispatch to arbitrary code the way EVM has). Integer overflow aborts by default. So the classic EVM reentrancy/overflow/uninitialized hunts are mostly dead — look elsewhere.

## Vector list (MV1–MV16)

**Ability & resource semantics**
- **MV1 — Wrong ability declaration.** A struct given `copy`/`drop` that represents value/authority → it can be duplicated or silently discarded. A `key` resource with `store` that lets it be moved to unintended accounts.
- **MV2 — `public(friend)` / visibility gap.** A function that mutates value declared `public` when it should be `entry`/`friend`-only → unauthorized callers reach it.
- **MV3 — Missing capability check.** Move idiom is capability objects (a `MintCapability`, `AdminCap`). A function that mutates state without requiring the cap by value/reference → anyone calls it. (Sui: an owned `AdminCap` object; Aptos: a stored capability.)

**Account & signer**
- **MV4 — Signer not required / spoofable.** An entry function taking an `address` argument where it should take `&signer` → caller acts on behalf of an account they don't control.
- **MV5 — `signer::address_of` vs argument mismatch.** Logic authorizes using a passed-in address instead of `signer::address_of(account)`.

**Sui object model (Sui-specific)**
- **MV6 — Shared vs owned object confusion.** A value that should be an owned object made a shared object → any tx can mutate it; or an owned object where ownership transfer isn't gated.
- **MV7 — Object wrapping/unwrapping ID reuse.** Unwrapping and re-wrapping lets an object ID be reused to bypass a one-time check.
- **MV8 — Missing `TxContext` sender check.** Action authorized without checking `tx_context::sender`.

**Aptos framework**
- **MV9 — Resource account / SignerCapability leak.** A stored `SignerCapability` retrievable by an unprivileged path → attacker impersonates the resource account.
- **MV10 — `borrow_global_mut` on attacker-chosen address.** Mutation keyed by a caller-supplied address without ownership check.

**Value & accounting**
- **MV11 — Coin/FungibleAsset rounding toward user.** Share/coin math rounding in the depositor's favor, amplified by repetition (same class as EVM P1, still applies).
- **MV12 — `Coin::merge`/`split` accounting gap.** Split/merge that doesn't conserve total, or zero-value coin used to bypass a check.
- **MV13 — First-depositor / empty-pool inflation.** Vault share price manipulable by the first depositor + donation, same as EVM ERC4626 inflation.

**Generics & type**
- **MV14 — Unconstrained type parameter.** A generic `<T>` value function callable with an attacker-supplied phantom type that bypasses an asset-type check → wrong-asset deposit credited as the valuable one.
- **MV15 — `entry` function arg injection.** Vector/struct args deserialized without bounds → out-of-range index or oversized loop (DoS / gas).
- **MV16 — Aborts as control flow / DoS.** A path an attacker can force to always abort, locking a shared object or an escrow permanently (fund-lock, payable).

## Proof harness

Move has first-class unit tests — use them; no external harness needed.
- Aptos: `#[test]` / `#[test(account = @0x...)]` functions, run `aptos move test` (or `move test`). Construct the attacker signer, call the vulnerable entry, assert the value sink (`coin::balance`, resource field) moved wrongly.
- Sui: `#[test]` with `test_scenario` — take the shared/owned object, run the attacker transaction, assert the object state / balance. `sui move test`.
- The proof must print before/after balances of the impact-map value sink and clear `references/proof-standard.md`. Under-authorization bugs (MV3/MV4) are proven by the test succeeding *from an account that should not be able to do it*.

## Impact framing
Top tier: MV3/MV4/MV9 (unauthorized value movement — unbounded theft), MV13 (share inflation), MV16 (permanent fund-lock). Size against the module's held `Coin`/object value.

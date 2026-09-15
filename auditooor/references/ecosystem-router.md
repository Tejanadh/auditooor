# Ecosystem Router — detection → engine dispatch

Auditooor never re-implements a hunt. It detects the ecosystem(s) present and drives the right engine. On Grok that is `spawn_subagent` after reading the engine `SKILL.md`. On Claude it is the Skill tool if present, otherwise the same spawn path. Multi-ecosystem targets fan out to several engines plus the cross-chain lens.

## Detection

Run cheap signals over the scoped file set (respect each engine's own exclude patterns):

| Signal | Ecosystem | Engine |
|---|---|---|
| `*.sol`, `pragma solidity`, `foundry.toml`/`hardhat` | EVM/Solidity | `critfindsaudit` |
| `*.vy`, `# pragma version`, `@external`/`@payable` decorators | Vyper (EVM VM, non-Solidity) | `references/vyper-adapter.md` → proof via Foundry |
| `*.rs` + `declare_id!`/`#[program]`/Anchor | Solana/Anchor | `critsolaudit` |
| `*.rs` + `plonky2`/`halo2`/`arkworks`, `*.circom` | ZK circuit | `critzkaudit` |
| `*.move`, `module ...::`, `Move.toml` | Move (Aptos/Sui) | `references/move-adapter.md` → proof via Move unit tests |
| bridge/messaging: `LayerZero`, `Wormhole`, `CCIP`, `Hyperlane`, `IBC`, mint-on-remote, `lzReceive`/`_execute` | cross-chain | `references/cross-chain.md` (in addition to per-leg engines) |

Ambiguous or mixed repo → route to **every** ecosystem detected. Do not force a single engine; a protocol's value often leaks at the seam between two.

## Dispatch — spawn the FLEET, not one agent

**The dispatch spawns the full specialized hunting fleet** (`references/hunting-fleet.md`) — one agent per hacking role from `{engine}/references/hacking-agents/` (access-control, asymmetry, boundary, economic-security, execution-trace, first-principles, flow-gap, invariant, math-precision, numerical-gap, periphery, trust-gap), in parallel. Spawning a single generic "find bugs" agent is an operator bug — never do it for a real hunt. Collect → dedup by (function, mechanism) → novelty-gate → PoC.

Pass the target path and the propagated mode (`DEEP`/`DIFF` if the Auditooor mode set them).

Engine files (first existing path):

| Ecosystem | Engine dir |
|---|---|
| EVM | `~/.grok/commands/critfindsaudit` then `~/.claude/commands/critfindsaudit` |
| Solana | `~/.grok/commands/critsolaudit` then `~/.claude/commands/critsolaudit` |
| ZK | `~/.grok/commands/critzkaudit` then `~/.claude/commands/critzkaudit` |

- EVM → read that `SKILL.md`, spawn the **full fleet** (12 agents from `critfindsaudit/references/hacking-agents/`) with path + `DEEP`/`DIFF [base_ref]` as set. Default/DEEP = all 12; QUICK = the money-map subset (always economic-security + invariant + math-precision + access-control, plus periphery if `seams` found a SEAM).
- Solana → same fleet pattern for `critsolaudit`.
- ZK → same for `critzkaudit`.
- Dual EVM+Solana quick pass → `critaudit` exists but prefer the specialised engines; use `critaudit` only when the user asks for the older unified flow.
- Move → no engine yet: load `references/move-adapter.md` and run its vector list inline, then forge proof with Move unit tests.
- Vyper → no engine yet: load `references/vyper-adapter.md`, run its vectors inline, forge proof by compiling the `.vy` and driving it from a Foundry test (vyper deploys as EVM bytecode).

**Grok:** one `spawn_subagent` (`general-purpose`, foreground) **per fleet role**, all launched together. Prefix each description `[auditooor:<role>]`. Do not pass `capability_mode`. Each agent's prompt = the role file's text + target paths + the Auditooor impact map, EV verdict, and novelty context + the gate rules.

**Claude:** one `Agent` call (`subagent_type: general-purpose`) **per fleet role**, all in one message so they run concurrently. Same prompt composition. (The Skill tool loads an engine into *your* context — use it only for a single-threaded inline pass; the fleet is spawned, not loaded.)

See `references/hunting-fleet.md` for the full roster, selection policy, spawn protocol, and the candidate-gating steps.

## What Auditooor injects into each engine

Every dispatched engine receives, from Auditooor, **before** it starts its taxonomy:

1. The **impact map** (`references/impact-model.md#impact-map`) as the hunting priority order.
2. The **EV verdict** and payout tier, so the engine sizes candidates against a real threshold, not an abstract severity rubric.
3. The **novelty context** (known public bugs for this protocol/fork family) so it does not spend depth re-deriving a disclosed issue.

## Fan-out coordination

When multiple engines run:
- De-duplicate candidates across engines by value-sink + mechanism before Phase 3, so a shared bug isn't proven twice.
- The cross-chain lens runs **after** the per-leg engines, because it needs each leg's trust boundaries mapped first.
- Merge all survivors into one `E[$]`-ranked submission list in Phase 5; never emit per-engine reports separately unless the programs differ.

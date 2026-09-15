# Cross-Chain Lens — the seam no single engine owns

Bridges and cross-chain messaging are where the biggest payouts live (most nine-figure hacks are bridge hacks) because the bug is in the *interaction* between two trust domains, and each domain's auditor only saw their half. Run this lens after the per-leg engines have mapped each side's trust boundaries.

## The core question

For every cross-chain message: **who is authorized to cause the destination action, and does the destination actually verify that the source really said it?** Most cross-chain bugs are a destination action (mint, release, unlock, execute) that trusts a message it never authenticated to the true source.

## Vector list (CC1–CC14)

- **CC1 — Unauthenticated `receive`/`_execute`.** The destination handler (`lzReceive`, `_execute`, `handle`, `wormholeReceive`) doesn't verify the message came from the trusted source contract/chain-id, or accepts any relayer. → attacker forges a delivery and mints/unlocks.
- **CC2 — Missing source-chain / source-address binding.** Handler checks the message format but not `(srcChainId, srcAddress)`. A message from an attacker-deployed contract on a cheap chain is accepted as if from the canonical peer.
- **CC3 — Replay across delivery.** No nonce / no consumed-message set, or the nonce isn't bound to `(srcChain, dstChain, payload)`. Same signed VAA/message replayed → double mint/release.
- **CC4 — Replay across chains / forks.** A message valid for chain A accepted on chain B (missing dst-chain-id in the signed payload), or replay across a fork after a chain split.
- **CC5 — Decimal / unit mismatch at the seam.** Source token has 18 decimals, destination wrapper assumes 6 (or vice-versa); amount is scaled wrong → mint inflation. Also token-address collision where the same symbol maps to different assets.
- **CC6 — Guardian/validator set desync.** Destination trusts a guardian set index that the source rotated; stale set still accepted → forged VAA with retired keys.
- **CC7 — Message ordering assumption.** Handler assumes messages arrive in emission order; out-of-order or dropped delivery corrupts accounting (e.g., a "burn then mint" pair where mint lands first).
- **CC8 — Partial/failed delivery fund lock.** A revert on the destination leaves source funds burned/locked with no refund path → permanent loss (payable as fund-lock even without attacker profit).
- **CC9 — Fee/gas griefing on relay.** Attacker sets destination gas so low the execute always reverts but the source considers the message sent → stuck funds.
- **CC10 — Trusted-remote configuration gap.** `setTrustedRemote` unset, set to zero, or updatable by a role in bounty scope → attacker becomes the trusted peer.
- **CC11 — Wrapped-asset / canonical-asset confusion.** Unlock on the wrong pool because the message doesn't bind which of several pools/assets it refers to.
- **CC12 — Rate-limit / cap bypass across messages.** Per-message cap enforced but not per-epoch aggregate; many small messages drain past the intended cap.
- **CC13 — Reorg / finality assumption.** Destination acts on a source event before source finality; a reorg unwinds the source burn but not the destination mint.
- **CC14 — Multi-VM type confusion.** EVM↔Solana / EVM↔Move payloads decoded with mismatched serialization (borsh vs abi), letting a crafted payload deserialize to a privileged action.

## Proof requirement

Prove the **seam**, not the leg (see `references/proof-standard.md#cross-chain-proofs`). The PoC must show: source emits message M (or attacker fabricates a deliverable M), destination accepts M, destination value sink moves wrongly. Where you can't run both chains, simulate the destination handler with the exact bytes a source leg would/ could produce and show the handler authorizes the action from an unauthorized origin.

## Impact framing

Cross-chain fund-lock (CC8/CC9) is payable even with no attacker profit — permanent loss of user funds is usually a Critical. Unauthenticated mint/unlock (CC1–CC6) is the top tier: unbounded theft. Size against the destination pool's reachable value.

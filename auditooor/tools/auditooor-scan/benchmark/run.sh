#!/usr/bin/env bash
# Reproducible detector benchmark against REAL contracts (not vendored — fetched).
# Vuln set: DeFiVulnLabs (real exploit patterns). Clean set: OpenZeppelin (audited).
set -euo pipefail
cd "$(dirname "$0")"
D=$(mktemp -d); V="$D/vuln"; C="$D/clean"; mkdir -p "$V" "$C"
echo "==> fetching vuln set (DeFiVulnLabs)"
git clone --depth 1 https://github.com/SunWeb3Sec/DeFiVulnLabs.git "$D/dvl" >/dev/null 2>&1
for f in SignatureReplay SignatureReplayNBA ecrecover phantom-permit; do
  cp "$D/dvl/src/test/$f.sol" "$V/" 2>/dev/null || true
done
echo "==> fetching clean set (OpenZeppelin, audited)"
oz() { gh api "repos/OpenZeppelin/openzeppelin-contracts/contents/$1" --jq '.content' | base64 -d > "$C/$2"; }
oz contracts/utils/cryptography/ECDSA.sol ECDSA.sol
oz contracts/token/ERC20/extensions/ERC20Permit.sol ERC20Permit.sol
oz contracts/proxy/utils/Initializable.sol Initializable.sol
oz contracts/utils/Nonces.sol Nonces.sol
oz contracts/utils/cryptography/EIP712.sol EIP712.sol
echo "==> building"
( cd .. && cargo build --release --offline >/dev/null 2>&1 )
echo "==> running"
python3 bench.py "../target/release/auditooor-scan" "$V" "$C"

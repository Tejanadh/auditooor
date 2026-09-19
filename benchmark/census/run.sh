#!/usr/bin/env bash
# Census regression corpus — the gate is only as good as the recon under it.
#
# Fluid hunt 2026-09-19: the entry census called 74 fully-guarded functions
# "permissionless", Abort B could not fire, and 502k tokens bought a negative the
# gate should have given for ~5k. This script is the standing check that the
# census still tells the truth on codebases whose answer we know by hand.
#
#   bash benchmark/census/run.sh [corpus_dir]     (default: /tmp/auditooor-census)
#
# Exit 0 = every expectation held. Exit 1 = a row moved; go read the diff before
# trusting any abort.
set -uo pipefail
CORPUS="${1:-/tmp/auditooor-census}"
SCAN="$(cd "$(dirname "$0")/../.." && pwd)/auditooor/tools/auditooor-scan/target/release/auditooor-scan"
[ -x "$SCAN" ] || { echo "build auditooor-scan first"; exit 2; }
mkdir -p "$CORPUS"

clone() { [ -d "$CORPUS/$2" ] || git clone -q --depth 1 "https://github.com/$1" "$CORPUS/$2"; }
clone OpenZeppelin/openzeppelin-contracts oz
clone transmissions11/solmate solmate
clone Uniswap/v2-core v2core
clone aave-dao/aave-v3-origin aave
clone Instadapp/fluid-contracts-public fluid

fail=0
# name | path | min% permissionless | max% permissionless
check() {
  local name="$1" path="$2" lo="$3" hi="$4"
  if [ ! -d "$path" ]; then printf "  SKIP %-30s (missing)\n" "$name"; return; fi
  local out pct n p
  out=$("$SCAN" entries "$path" 2>/dev/null | python3 -c "
import sys,json
from collections import Counter
e=json.load(sys.stdin); c=Counter(x['access'] for x in e)
n=len(e); p=c['permissionless']
print(n, p, (100*p)//max(n,1))")
  read -r n p pct <<<"$out"
  if [ "$pct" -ge "$lo" ] && [ "$pct" -le "$hi" ]; then
    printf "  ok   %-30s %4s entries, %3s%% permissionless (expect %s-%s%%)\n" "$name" "$n" "$pct" "$lo" "$hi"
  else
    printf "  FAIL %-30s %4s entries, %3s%% permissionless (expect %s-%s%%)\n" "$name" "$n" "$pct" "$lo" "$hi"
    fail=1
  fi
}

echo "census regression corpus"
# Ground truth established by reading the source, 2026-09-19:

# Every entry is onlyRebalancer / TEAM_MULTISIG / pause-auth. Must be 0%.
# This is the row that, when it was wrong, cost a 502k-token run.
check "fluid contracts/config"   "$CORPUS/fluid/contracts/config"                 0 0
# operate + operateOnBehalfOf are THE permissionless money entrypoints. Must be 100%.
# Guards the opposite failure: a false gate here would abort a live target.
check "fluid liquidity/userModule" "$CORPUS/fluid/contracts/liquidity/userModule" 100 100
# setFeeTo / setFeeToSetter / initialize are inline-guarded; the rest is open.
check "uniswap v2-core"          "$CORPUS/v2core/contracts"                      70 80
# ERC20 + extensions: only crosschainMint/Burn (onlyTokenBridge) are gated.
check "OZ token/ERC20"           "$CORPUS/oz/contracts/token/ERC20"              85 95
# renounceRole / acceptOwnership / AccessManager schedule-execute-cancel are
# genuinely open; everything else is onlyRole/onlyOwner.
check "OZ access/"               "$CORPUS/oz/contracts/access"                   15 30
check "OZ governance/"           "$CORPUS/oz/contracts/governance"               45 60
check "solmate tokens/"          "$CORPUS/solmate/src/tokens"                    60 80
check "aave v3 protocol"         "$CORPUS/aave/src/contracts/protocol"           45 60

echo
echo "gate check (Abort B must fire on the fully-gated directory)"
"$SCAN" gate "$CORPUS/fluid/contracts/config" >/dev/null 2>&1
if [ $? -eq 3 ]; then echo "  ok   gate exits 3 on fluid/contracts/config"; else echo "  FAIL gate did not abort"; fail=1; fi
"$SCAN" gate "$CORPUS/fluid/contracts/liquidity/userModule" >/dev/null 2>&1
if [ $? -eq 3 ]; then echo "  FAIL gate aborted a target with real permissionless entries"; fail=1; else echo "  ok   gate proceeds on fluid/liquidity/userModule"; fi

exit $fail

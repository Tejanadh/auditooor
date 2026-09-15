#!/usr/bin/env bash
# One-command: build auditooor-scan or fail loudly.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
SCAN="$ROOT/tools/auditooor-scan"
BIN="$SCAN/target/release/auditooor-scan"

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo not on PATH. Install Rust: https://rustup.rs" >&2
  exit 1
fi

bash "$SCAN/build.sh"

if [ ! -x "$BIN" ]; then
  echo "FAIL: expected $BIN after build" >&2
  exit 1
fi

echo "OK {scan}=$BIN"
"$BIN" --version
echo "Optional: Foundry (forge) for PoCs — https://getfoundry.sh"
command -v forge >/dev/null && forge --version | head -1 || echo "forge: not installed (PoCs will wait)"
echo "Engines (optional, not required):"
for e in critfindsaudit critsolaudit critzkaudit; do
  if [ -f "$HOME/.grok/commands/$e/SKILL.md" ] || [ -f "$HOME/.claude/commands/$e/SKILL.md" ]; then
    echo "  $e: present"
  else
    echo "  $e: missing (native EVM path still works)"
  fi
done

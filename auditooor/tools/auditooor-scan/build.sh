#!/usr/bin/env bash
# Build the auditooor-scan release binary and run the full test suite.
# Pure std / no network deps — works offline.
set -euo pipefail
cd "$(dirname "$0")"
echo "==> cargo test"
cargo test --offline
echo "==> cargo build --release"
cargo build --release --offline
BIN="$(pwd)/target/release/auditooor-scan"
echo "==> built: $BIN"
"$BIN" --version
echo "==> smoke test on fixtures"
"$BIN" detect fixtures

# Optional: AST-accurate build (needs network to fetch solar-parse once).
if [ "${1:-}" = "--solar" ]; then
  echo "==> building with Solar AST feature (fetches solar-parse)"
  cargo test --features solar
  cargo build --release --features solar
  echo "==> Solar build ready: harness now uses real AST (no phantom functions)"
fi

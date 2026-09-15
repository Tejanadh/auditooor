#!/usr/bin/env bash
# Install auditooor for Claude Code, Cursor, and/or Grok, then build auditooor-scan (fail loud).
#
#   bash auditooor/install.sh                 # every runtime found on this machine
#   bash auditooor/install.sh --claude        # only Claude Code  (~/.claude/skills/auditooor)
#   bash auditooor/install.sh --cursor        # only Cursor       (~/.cursor/skills/auditooor)
#   bash auditooor/install.sh --grok          # only Grok         (~/.grok/skills/auditooor)
#   bash auditooor/install.sh --project DIR   # per-repo: DIR/.claude/skills + DIR/.cursor/skills
#
#   --copy      copy instead of symlink (symlink = `git pull` updates the installed skill)
#   --force     replace an existing install that is not this checkout (old one is backed up)
#   --no-build  skip building auditooor-scan
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
SCAN="$ROOT/tools/auditooor-scan"
BIN="$SCAN/target/release/auditooor-scan"

want=() project="" mode=link force=0 build=1
while [ $# -gt 0 ]; do
  case "$1" in
    --claude) want+=(claude) ;;
    --cursor) want+=(cursor) ;;
    --grok)   want+=(grok) ;;
    --all)    want+=(claude cursor grok) ;;
    --project) project="$(cd "${2:?--project needs a directory}" && pwd)"; shift ;;
    --copy)   mode=copy ;;
    --force)  force=1 ;;
    --no-build) build=0 ;;
    -h|--help) sed -n 2,13p "$0"; exit 0 ;;
    *) echo "unknown flag: $1 (see --help)" >&2; exit 2 ;;
  esac
  shift
done

dests=()
if [ -n "$project" ]; then
  dests+=("$project/.claude/skills/auditooor" "$project/.cursor/skills/auditooor")
fi
if [ ${#want[@]} -eq 0 ] && [ -z "$project" ]; then
  # Auto: every runtime whose home directory exists. Nothing found -> Claude Code.
  [ -d "$HOME/.claude" ] && want+=(claude)
  [ -d "$HOME/.cursor" ] && want+=(cursor)
  [ -d "$HOME/.grok" ]   && want+=(grok)
  [ ${#want[@]} -eq 0 ] && want+=(claude)
fi
for w in ${want[@]+"${want[@]}"}; do
  case "$w" in
    claude) dests+=("$HOME/.claude/skills/auditooor") ;;
    cursor) dests+=("$HOME/.cursor/skills/auditooor") ;;
    grok)   dests+=("$HOME/.grok/skills/auditooor") ;;
  esac
done

install_one() {
  local dest="$1"
  mkdir -p "$(dirname "$dest")"
  if [ -L "$dest" ] && [ "$(readlink -f "$dest")" = "$(readlink -f "$ROOT")" ]; then
    if [ "$mode" != copy ]; then echo "  = $dest (already linked to this checkout)"; return; fi
    rm "$dest"
  fi
  if [ -e "$dest" ] || [ -L "$dest" ]; then
    if [ "$force" -ne 1 ]; then
      echo "  ! $dest exists and is not this checkout — left untouched (re-run with --force to replace; it is backed up)"
      return
    fi
    local bak="$dest.bak-$(date +%Y%m%d%H%M%S)"
    mv "$dest" "$bak"
    echo "  ~ backed up existing install to $bak"
  fi
  if [ "$mode" = copy ]; then
    cp -R "$ROOT" "$dest"
    rm -rf "$dest/tools/auditooor-scan/target"
    echo "  + $dest (copy — rerun install.sh after git pull)"
  else
    ln -s "$ROOT" "$dest"
    echo "  + $dest -> $ROOT"
  fi
}

echo "Installing auditooor skill:"
for d in "${dests[@]}"; do install_one "$d"; done

if [ "$build" -eq 1 ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    echo "FAIL: cargo not on PATH. Install Rust: https://rustup.rs  (skill files are installed; the scan binary is not)" >&2
    exit 1
  fi
  bash "$SCAN/build.sh"
  if [ ! -x "$BIN" ]; then
    echo "FAIL: expected $BIN after build" >&2
    exit 1
  fi
  echo "OK {scan}=$BIN"
  "$BIN" --version
  if [ "$mode" = copy ]; then
    for d in "${dests[@]}"; do
      [ -d "$d" ] && [ ! -L "$d" ] && mkdir -p "$d/tools/auditooor-scan/target/release" \
        && cp "$BIN" "$d/tools/auditooor-scan/target/release/" 2>/dev/null || true
    done
  fi
fi

echo "Optional: Foundry (forge) for PoCs — https://getfoundry.sh"
command -v forge >/dev/null && forge --version | head -1 || echo "forge: not installed (PoCs will wait)"
echo "Engines (optional, not required):"
for e in critfindsaudit critsolaudit critzkaudit; do
  found=""
  for base in "$HOME/.claude/skills" "$HOME/.cursor/skills" "$HOME/.grok/skills" "$HOME/.grok/commands" "$HOME/.claude/commands"; do
    [ -f "$base/$e/SKILL.md" ] && found="$base/$e" && break
  done
  echo "  $e: ${found:-missing (native EVM path still works)}"
done
echo
echo "Run it:  Claude Code / Cursor -> type /auditooor in the agent chat   ·   Grok -> /auditooor"

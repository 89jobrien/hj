#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj handup"
(
  cd "$repo"
  print_cmd "$HJ_BIN handup --max-depth 3"
  "$HJ_BIN" handup --max-depth 3
  print_cmd "$HANDUP_BIN --max-depth 3"
  "$HANDUP_BIN" --max-depth 3
  print_cmd "sed -n '1,120p' \"$HOME/.ctx/handoffs/$(basename "$repo")/HANDUP.json\""
  sed -n '1,120p' "$HOME/.ctx/handoffs/$(basename "$repo")/HANDUP.json"
)

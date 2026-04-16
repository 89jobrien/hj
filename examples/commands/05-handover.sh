#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj handover"
(
  cd "$repo"
  print_cmd "$HJ_BIN handover"
  "$HJ_BIN" handover
  print_cmd "$HANDOVER_BIN"
  "$HANDOVER_BIN"
  print_cmd "sed -n '1,120p' .ctx/HANDOVER.md"
  sed -n '1,120p' .ctx/HANDOVER.md
)

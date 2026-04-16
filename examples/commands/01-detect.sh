#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj detect"
(
  cd "$repo"
  print_cmd "$HJ_BIN detect"
  "$HJ_BIN" detect
  print_cmd "$HANDOFF_DETECT_BIN --project"
  "$HANDOFF_DETECT_BIN" --project
)

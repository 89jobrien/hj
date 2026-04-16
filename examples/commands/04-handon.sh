#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj handon"
(
  cd "$repo"
  print_cmd "$HJ_BIN handon"
  "$HJ_BIN" handon
  print_cmd "$HANDON_BIN"
  "$HANDON_BIN"
)

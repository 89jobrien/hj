#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"

print_banner "hj refresh"
(
  cd "$repo"
  print_cmd "$HJ_BIN refresh"
  "$HJ_BIN" refresh
  print_cmd "ls .ctx"
  ls .ctx
)

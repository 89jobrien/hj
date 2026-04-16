#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
require_command_or_skip doob "hj audit needs doob on PATH"
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj audit"
(
  cd "$repo"
  print_cmd "$HJ_BIN audit"
  "$HJ_BIN" audit || true
)

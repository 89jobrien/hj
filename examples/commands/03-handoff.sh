#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"

print_banner "hj handoff"
(
  cd "$repo"
  print_cmd "$HJ_BIN handoff --allow-create --build clean --tests passing --notes 'Demo repo for examples.' --log-summary 'Create demo handoff state'"
  "$HJ_BIN" handoff \
    --allow-create \
    --build clean \
    --tests passing \
    --notes "Demo repo for examples." \
    --log-summary "Create demo handoff state"

  print_cmd "$HANDOFF_BIN --build clean --tests passing --notes 'Alias refresh' --log-summary 'Refresh via handoff alias'"
  "$HANDOFF_BIN" \
    --build clean \
    --tests passing \
    --notes "Alias refresh" \
    --log-summary "Refresh via handoff alias"
)

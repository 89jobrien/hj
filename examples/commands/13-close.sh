#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"

print_banner "hj close"
(
  cd "$repo"
  print_cmd "$HJ_BIN close --allow-create --build clean --tests passing --notes 'Close compatibility path.' --log-summary 'Close via legacy command'"
  "$HJ_BIN" close \
    --allow-create \
    --build clean \
    --tests passing \
    --notes "Close compatibility path." \
    --log-summary "Close via legacy command"
)

#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
seed_handoff "$repo"

print_banner "hj handoff-db"
(
  cd "$repo"
  cat > .ctx/HANDOFF.demo-app.demo-app.yaml <<'EOF'
project: demo-app
id: demo-ap
updated: 2026-04-16
items:
  - id: demo-app-1
    name: demo-db-item
    priority: P1
    status: open
    title: Show handoff db output
log: []
EOF
  print_cmd "$HJ_BIN handoff-db upsert --project demo-app --handoff .ctx/HANDOFF.demo-app.demo-app.yaml"
  "$HJ_BIN" handoff-db upsert --project demo-app --handoff .ctx/HANDOFF.demo-app.demo-app.yaml
  print_cmd "$HJ_BIN handoff-db query --project demo-app"
  "$HJ_BIN" handoff-db query --project demo-app
  print_cmd "$HANDOFF_DB_BIN query --project demo-app"
  "$HANDOFF_DB_BIN" query --project demo-app
)

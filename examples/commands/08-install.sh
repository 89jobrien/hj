#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
install_root="$(dirname "$repo")/install-root"

print_banner "hj install"
print_cmd "$HJ_BIN install --root $install_root"
printf 'This example may require Cargo registry/network access even for local path installs.\n'
printf 'Run it manually when registry resolution is available.\n'

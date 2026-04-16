#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/common.sh"

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"
install_root="$(dirname "$repo")/update-root"

print_banner "hj update"
print_cmd "$HJ_BIN update --root $install_root"
printf 'This example requires registry/network access to install the published `hj-cli` package.\n'
printf 'Run it manually when network access is available.\n'

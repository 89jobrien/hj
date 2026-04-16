#!/usr/bin/env bash
set -euo pipefail

EXAMPLES_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$EXAMPLES_ROOT/.." && pwd)"
HJ_BIN="$REPO_ROOT/target/debug/hj"
HANDOFF_BIN="$REPO_ROOT/target/debug/handoff"
HANDON_BIN="$REPO_ROOT/target/debug/handon"
HANDOVER_BIN="$REPO_ROOT/target/debug/handover"
HANDOFF_DETECT_BIN="$REPO_ROOT/target/debug/handoff-detect"
HANDOFF_DB_BIN="$REPO_ROOT/target/debug/handoff-db"
HANDUP_BIN="$REPO_ROOT/target/debug/handup"

ensure_hj_built() {
  (
    cd "$REPO_ROOT"
    env RUSTC_WRAPPER= cargo build -p hj-cli --bins >/dev/null
  )
}

create_demo_repo() {
  DEMO_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/hj-examples.XXXXXX")"
  export DEMO_ROOT
  export HOME="$DEMO_ROOT/home"
  mkdir -p "$HOME"

  DEMO_REPO="$DEMO_ROOT/demo-app"
  export DEMO_REPO
  mkdir -p "$DEMO_REPO/src"
  cat >"$DEMO_REPO/Cargo.toml" <<'EOF'
[package]
name = "demo-app"
version = "0.1.0"
edition = "2024"
EOF
  cat >"$DEMO_REPO/src/lib.rs" <<'EOF'
pub fn status() -> &'static str {
    // TODO: wire a real status source
    "ok"
}
EOF

  git init -q "$DEMO_REPO"
  git -C "$DEMO_REPO" config user.name "hj examples"
  git -C "$DEMO_REPO" config user.email "examples@local"
  git -C "$DEMO_REPO" add .
  git -C "$DEMO_REPO" commit -qm "init demo repo"
}

seed_handoff() {
  local repo="$1"
  (
    cd "$repo"
    "$HJ_BIN" refresh >/dev/null
    "$HJ_BIN" handoff \
      --allow-create \
      --build clean \
      --tests passing \
      --notes "Demo repo for examples." \
      --log-summary "Create demo handoff state" >/dev/null
  )
}

print_banner() {
  printf '\n# %s\n\n' "$1"
}

print_cmd() {
  printf '$ %s\n' "$*"
}

require_command_or_skip() {
  local program="$1"
  local reason="$2"
  if ! command -v "$program" >/dev/null 2>&1; then
    printf 'Skipping: %s\n' "$reason"
    exit 0
  fi
}

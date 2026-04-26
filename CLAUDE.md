# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Build & Test

All commands run from the workspace root. The `env RUSTC_WRAPPER=` prefix is
required to bypass sccache/wrapper issues:

```bash
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace --locked
cargo check --workspace --locked
```

Run a single test:

```bash
env RUSTC_WRAPPER= cargo test --workspace --locked -- test_name
```

Install binaries from checkout to `~/.local/bin`:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hj-cli --bins --force --root ~/.local
```

## Architecture

Rust workspace (edition 2024) with six crates under `crates/`:

| Crate       | Role                                                           |
| ----------- | -------------------------------------------------------------- |
| `hj-cli`    | Clap-based CLI, command dispatch, install/update logic         |
| `hj-core`   | Shared data models (handoff items, handup reports, priorities) |
| `hj-git`    | Repo discovery, `.ctx` scaffolding, handoff scanning           |
| `hj-render` | Markdown rendering for `HANDOFF.md` and `HANDOVER.md`          |
| `hj-sqlite` | SQLite persistence (handoff rows, handup checkpoints)          |
| `hj-doob`   | Reconciliation adapter between handoff items and `doob` todos  |

A conformance test suite lives at `tests/conformance/`.

**Data flow:** CLI parses args -> `hj-git` discovers repo and reads YAML ->
`hj-core` models are populated -> `hj-render` emits markdown, `hj-sqlite`
persists, `hj-doob` reconciles with external todo state.

**Key paths:**

- Handoff YAML: `.ctx/HANDOFF.<project>.<repo>.yaml`
- Session state: `.ctx/HANDOFF.<project>.<repo>.state.yaml` (gitignored)
- SQLite stores: `~/.local/share/atelier/handoff.db`,
  `~/.ctx/handoffs/handup.db`

## cargo binstall

All public binary crates must include `[package.metadata.binstall]` in their
`Cargo.toml` so users can install via `cargo binstall hj-cli`. The metadata
points at GitHub Releases tarballs:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ target }.tar.gz"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "tgz"
```

This requires a release workflow that builds and uploads per-target archives
to GitHub Releases. Without it, binstall falls back to `cargo-quickinstall`
or source compilation.

## Conventions

- Rust 2024 edition: `set_var`/`remove_var` require `unsafe {}`, match
  ergonomics differ from earlier editions.
- Keep modules narrowly scoped by crate responsibility. New cross-cutting
  logic belongs in the owning crate, not `hj-cli`.
- CLI parsing stays in `crates/hj-cli/src/cli.rs`.
- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`,
  `release:`.
- Do not commit real `.ctx` state files or local SQLite databases.

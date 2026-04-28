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
env RUSTC_WRAPPER= cargo install --path crates/hjx --bins --force --root ~/.local
```

## Architecture

Rust workspace (edition 2024) with two crates under `crates/`:

| Crate   | Role                                                              |
| ------- | ----------------------------------------------------------------- |
| `hjlib` | Library: core models, detect, git, render, doob, sqlite modules   |
| `hjx`   | Binary: Clap-based CLI, command dispatch, install/update logic    |

A conformance test suite lives at `tests/conformance/`.

**Data flow:** CLI parses args -> `hjlib::git` discovers repo and reads
YAML -> `hjlib` models are populated -> `hjlib::render` emits markdown,
`hjlib::sqlite` persists, `hjlib::doob` reconciles with external todo
state.

**Key paths:**

- Handoff YAML: `.ctx/HANDOFF.<project>.<repo>.yaml`
- Session state: `.ctx/HANDOFF.<project>.<repo>.state.yaml` (gitignored)
- SQLite stores: `~/.local/share/atelier/handoff.db`,
  `~/.ctx/handoffs/handup.db`

## cargo binstall

All public binary crates must include `[package.metadata.binstall]` in their
`Cargo.toml` so users can install via `cargo binstall hjx`. The metadata
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
  logic belongs in the owning module, not `hjx`.
- CLI parsing stays in `crates/hjx/src/cli.rs`.
- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`,
  `release:`.
- Do not commit real `.ctx` state files or local SQLite databases.

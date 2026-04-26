# Repository Guidelines

## Project Structure & Module Organization

`hj` is a Rust workspace centered on handoff workflows. Crates live under `crates/`:

- `hj-cli`: CLI entrypoints, Clap args, command dispatch, install/update logic
- `hj-core`: shared handoff and handup data models plus helper logic
- `hj-git`: repo discovery, `.ctx` scaffolding, scans, and `.gitignore` management
- `hj-render`: markdown rendering for `HANDOFF.md` and `HANDOVER.md`
- `hj-sqlite`: SQLite persistence for handoff rows and handup checkpoints
- `hj-doob`: `doob` reconciliation adapter

Runnable demos live in `examples/`, with command-focused scripts in `examples/commands/` and the visual demo in `examples/visual/`.

## Build, Test, and Development Commands

Run all commands from the repo root.

- `cargo check --workspace --locked`: fast compile check across all crates
- `cargo fmt --all --check`: enforce formatting
- `env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings`: lint at CI strictness
- `env RUSTC_WRAPPER= cargo test --workspace --locked`: run the full test suite
- `hj refresh`: initialize `.ctx` scaffolding in a repo under test
- `./examples/visual-demo.sh`: regenerate the standalone visual walkthrough

## Coding Style & Naming Conventions

Use Rust 2024 conventions and keep modules narrowly scoped by responsibility. Follow existing crate boundaries instead of adding cross-cutting helpers to `hj-cli`. Prefer descriptive snake_case for functions and modules, CamelCase for types, and keep CLI parsing in `crates/hj-cli/src/cli.rs`. Format with `cargo fmt`; do not hand-format around it.

## Testing Guidelines

Add unit tests near the code they verify, typically in `#[cfg(test)]` modules. Cover workflow rules with focused tests in the owning crate rather than only through CLI smoke tests. For integration-style behavior, prefer disposable temp repos and isolated `HOME` setups, following the patterns in `examples/` and existing tests. Run `cargo test --workspace --locked` before opening a PR.

## Commit & Pull Request Guidelines

Recent history uses conventional commits such as `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, and `release:`. Keep subject lines short and imperative, for example `fix: rebind nested handoff state paths`. PRs should describe user-visible behavior changes, list key commands used for verification, and link the relevant GitHub issues. Include screenshots only for UI changes such as `examples/visual/index.html`.

## Security & Repo Hygiene

Do not commit generated local `.ctx` state from real repos. Treat `~/.local/share/atelier/handoff.db`, `~/.ctx/handoffs/`, and `doob` data as external state. Keep example artifacts intentional and avoid committing editor junk or OS metadata files.

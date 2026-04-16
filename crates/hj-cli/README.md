# hj-cli

`hj-cli` is the installable command-line package for the `hj` workspace.

It provides:

- `hj`
- `handoff`
- `handon`
- `handover`
- `handoff-detect`
- `handoff-db`
- `handup`

## Install From a Checkout

From the repository root:

```bash
hj install
```

Manual equivalent:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hj-cli --bins --force --root ~/.local
```

This refreshes all seven binaries in `~/.local/bin`.

## Update To the Latest Release

```bash
hj update
```

`hj update-all` is a synonym for the same release update path.

## Top-Level Commands

```text
hj detect
hj handoff
hj handon
hj handover
hj handoff-db
hj handup
hj install
hj update
hj update-all
hj refresh
hj reconcile
hj audit
hj close
```

## Architecture

- `src/main.rs` and `src/bin/*.rs` are composition roots only. They delegate into the shared library crate.
- `src/lib.rs` owns command dispatch and keeps CLI parsing separate from command execution.
- `src/cli.rs` contains Clap types only.
- `src/install.rs` contains install and update concerns only.
- `src/handoff.rs` contains handoff, handon, handover, detect, reconcile, and SQLite-backed triage workflows.
- `src/handup.rs` contains tree survey, report synthesis, checkpointing, and summary rendering for handup.

## Notes

- `hj install` uses the current checkout and should be run from an `hj` repository.
- `hj update` installs the latest published `hj-cli` package.
- `handoff`, `handon`, `handover`, `handoff-detect`, `handoff-db`, and `handup` are thin aliases that route into `hj`.

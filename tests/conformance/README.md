# hj conformance tests

`tests/conformance` verifies contracts across the public module boundaries exposed by `hjlib`.
It covers the handoff domain model, SQLite persistence, `doob` mapping, Git/filesystem discovery,
and Markdown rendering independently of the `hjx` command handlers.

## Workspace role

This directory is a Rust workspace package, not Cargo's conventional loose `tests/` directory.
The root `Cargo.toml` includes `tests/conformance` in `workspace.members`, and this directory has
its own package manifest:

```toml
[package]
name = "conformance"
publish = false
```

The package contains only test modules and development dependencies. `src/lib.rs` enables each
module under `#[cfg(test)]`, so Cargo builds the package as a library test target. It depends on
`hjlib` through a workspace-relative path and uses temporary SQLite databases and repositories for
isolation.

## Contract map

Test names begin with the section they cover, such as `s1_1_` or `s5_4_`.

| Module | Contract area | Representative behavior |
| --- | --- | --- |
| `src/core.rs` | Domain model | Reconciliation, title variants, priority, serialization |
| `src/sqlite.rs` | SQLite adapter | Upsert/query, pruning, status changes, checkpoints |
| `src/doob.rs` | `doob` adapter | Priority mapping, title normalization, snapshot type |
| `src/git.rs` | Git/filesystem adapter | Handoff discovery, path naming, manifest lookup, refresh |
| `src/render.rs` | Markdown adapter | Output structure, sorting, filtering, and log formatting |
| `src/cli.rs` | CLI boundary note | Explains why private Clap tests live inside `hjx` |

The section names retain the pre-collapse boundary vocabulary from `.ctx/conformance.md`
(`hj-core`, `hj-sqlite`, and similar). In the current workspace those boundaries are modules within
`hjlib`, while `hjx` remains the composition root.

## What the suite guarantees

### Domain contracts

- audit reconciliation produces no create operations
- sync reconciliation creates only missing active items
- active, closed-upstream, and orphaned todo titles are classified separately
- handoff activity includes only `open` and `blocked` items
- title variants are non-empty and deduplicated
- priority inference follows the documented P0/P1/P2 keywords
- state values serialize and deserialize without losing covered fields

### Persistence contracts

- SQLite upserts round-trip and order rows by priority and ID
- repeated upserts update rather than duplicate a `(project, id)` row
- removed items are pruned only from the selected project
- completion stamps `done` and a date; generic status updates preserve completion data
- handup checkpoints append rather than replacing earlier checkpoints

Every persistence test uses a `tempfile` database path through `HandoffDb::with_path` or
`HandupDb::with_path`.

### Adapter contracts

- `doob` priorities map from P0/P1/P2 to 5/4/3 and default to 1
- handoff Markdown sections become survey items with inferred priorities and sequential IDs
- managed handoff and state paths follow the `.ctx/HANDOFF.<project>.<repo>.*` convention
- refresh is idempotent and does not duplicate its managed `.gitignore` block
- rendered output includes stable headers, state, sorted active items, and at most five log entries

Git discovery tests create temporary repositories with local-only test identity configuration. They
do not modify the developer's repository or global Git configuration.

## Running the tests

Run the package from the workspace root:

```bash
env RUSTC_WRAPPER= cargo test -p conformance --locked
```

Run one contract section or one exact test:

```bash
env RUSTC_WRAPPER= cargo test -p conformance --locked s1_1
env RUSTC_WRAPPER= cargo test -p conformance --locked s5_4_log_capped_at_five_entries
```

Run it as part of the complete workspace suite:

```bash
env RUSTC_WRAPPER= cargo test --workspace --locked
```

The `s3_3_snapshot_returns_todo_snapshot_type` test is ignored by default because it invokes a live
`doob` executable. Run it explicitly only when `doob` is on `PATH` and its configured state is safe
to query:

```bash
env RUSTC_WRAPPER= cargo test -p conformance --locked \
  s3_3_snapshot_returns_todo_snapshot_type -- --ignored
```

## Adding or changing contracts

1. Put the test in the module that owns the boundary behavior.
2. Prefix the function with its contract section, for example `s2_3_`.
3. Use `tempfile` paths for filesystem and SQLite state.
4. Avoid network access and live tools; mark unavoidable external-process tests ignored.
5. Keep CLI-private parsing tests in `crates/hjx/src/lib.rs` unless the API becomes public.
6. Run the package test, then the complete workspace test and lint gates.

The repository's development gate is:

```bash
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace --locked
cargo check --workspace --locked
```

# hj

`hj` is a Rust workspace for handoff state, reconciliation, rendering, and handup surveys.

## Workspace Crates

| Crate | Purpose | README |
|---|---|---|
| `hj-cli` | Top-level CLI and installed binaries | [`crates/hj-cli/README.md`](./crates/hj-cli/README.md) |
| `hj-core` | Shared handoff and handup data types plus priority/title helpers | [`crates/hj-core/README.md`](./crates/hj-core/README.md) |
| `hj-doob` | `doob` reconciliation helpers and priority mapping | [`crates/hj-doob/README.md`](./crates/hj-doob/README.md) |
| `hj-git` | Repo discovery, `.ctx` scaffolding, handoff scanning, and gitignore management | [`crates/hj-git/README.md`](./crates/hj-git/README.md) |
| `hj-render` | Markdown rendering for structured handoff state | [`crates/hj-render/README.md`](./crates/hj-render/README.md) |
| `hj-sqlite` | SQLite persistence for handoff rows and handup checkpoints | [`crates/hj-sqlite/README.md`](./crates/hj-sqlite/README.md) |

## Workspace Features

| Area | What it covers |
|---|---|
| Structured handoff state | YAML-backed `HANDOFF.*.yaml` plus per-project state files under `.ctx/` |
| Repo discovery | Detect repo roots, infer project names, and migrate legacy root handoffs |
| Handoff rendering | Render active items and recent log entries to `HANDOFF.md` |
| Handup survey | Scan nested repos and TODO markers, emit `HANDUP.json`, and checkpoint the run |
| SQLite sync | Persist handoff rows in `~/.local/share/atelier/handoff.db` and handup checkpoints in `~/.ctx/handoffs/handup.db` |
| Doob reconciliation | Audit or sync handoff items against `doob` todos |
| Installed binary management | Install the current checkout or update to the latest published `hj-cli` release |

## Workspace Commands

### Installed Binaries

| Binary | Role |
|---|---|
| `hj` | Main CLI |
| `handoff` | Shortcut for `hj handoff` |
| `handon` | Shortcut for `hj handon` |
| `handover` | Shortcut for `hj handover` |
| `handoff-detect` | Shortcut for `hj detect` |
| `handoff-db` | Shortcut for `hj handoff-db` |
| `handup` | Shortcut for `hj handup` |

### `hj` Subcommands

| Command | Purpose |
|---|---|
| `hj detect` | Resolve the active handoff path or repo metadata |
| `hj handoff` | Write handoff YAML, state, `HANDOFF.md`, `HANDOVER.md`, SQLite sync, and reconcile output |
| `hj handon` | Read the current repo handoff and print grouped P0/P1/P2 triage |
| `hj handover` | Regenerate `.ctx/HANDOVER.md` from the current handoff and state |
| `hj handoff-db` | Inspect or update the handoff SQLite store |
| `hj handup` | Survey repos and TODO markers into a handup report |
| `hj install` | Install binaries from the current checkout into `~/.local/bin` |
| `hj update` | Update installed binaries to the latest published `hj-cli` release |
| `hj update-all` | Synonym for `hj update` |
| `hj refresh` | Initialize or refresh `.ctx` scaffolding and ignore rules |
| `hj reconcile` | Sync open handoff items into `doob` |
| `hj audit` | Audit handoff coverage against `doob` without mutating state |
| `hj close` | Write handoff YAML, state, markdown render, SQLite sync, and reconcile output |

## Install

Install from the current checkout:

```bash
hj install
```

Manual equivalent from the repo root:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hj-cli --bins --force --root ~/.local
```

`hj install` is intended to be run from an `hj` checkout. It installs `hj`, `handoff`, `handon`, `handover`, `handup`, `handoff-db`, and `handoff-detect` into `~/.local/bin` by default.

## Update

Update the installed binaries to the latest published `hj-cli` release:

```bash
hj update
```

`hj update-all` is a synonym for `hj update`.

## Commands

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

## Common Workflows

Detect the active handoff path for the current repo:

```bash
hj detect
handoff-detect --project
```

Survey a tree and write `HANDUP.json` plus a checkpoint:

```bash
hj handup
handup --max-depth 5
```

Inspect or update the SQLite handoff store:

```bash
handoff-db query --project hj
handoff-db upsert --project hj --handoff .ctx/HANDOFF.hj.hj.yaml
```

Refresh `.ctx` scaffolding:

```bash
hj refresh
```

Close out a handoff and render markdown:

```bash
hj handoff --log-summary "Finished the current work slice"
hj close --log-summary "Finished the current work slice"
```

Print repo-local triage:

```bash
hj handon
handon --project hj
```

Regenerate the compact handover summary:

```bash
hj handover
handover
```

## Development

Run the standard Rust checks from the workspace root:

```bash
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace
```

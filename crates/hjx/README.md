# hjx

`hjx` is the executable package for the `hj` handoff workflow. It parses commands with Clap,
delegates handoff and persistence operations to `hjlib`, and installs the main `hj` command plus
compatibility entry points.

## Workspace role

`hjx` is the workspace's CLI composition root:

- `crates/hjlib` owns the data model, rendering, repository discovery, SQLite, and `doob` logic.
- `crates/hjx` owns argument parsing, command dispatch, alias rewriting, and installation/update
  behavior.
- `tests/conformance` checks the library boundary contracts; CLI-private parsing tests remain in
  `src/lib.rs` because the Clap types are crate-private.

The package name is `hjx`; the primary installed binary is `hj`.

## Contents

- [Install](#install)
- [Installed binaries](#installed-binaries)
- [Quick start](#quick-start)
- [Commands](#commands)
- [Library entry points](#library-entry-points)
- [Source-grounded examples](#source-grounded-examples)
- [Development and testing](#development-and-testing)

## Install

Install a published release and all package binaries:

```bash
cargo binstall hjx
```

Install every binary from this checkout:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hjx --bins --force --root "$HOME/.local"
```

The CLI can perform the checkout install itself:

```bash
hj install
hj install --root /custom/prefix
```

`hj update` and `hj update-all` both install the latest published `hjx` package with
`cargo install --locked --force`. Their default installation root is `$HOME/.local`.

## Installed binaries

| Binary | Dispatch target |
| --- | --- |
| `hj` | Main subcommand-based CLI |
| `handoff-detect` | `hj detect` |
| `handoff` | `hj handoff` |
| `handon` | `hj handon` |
| `handover` | `hj handover` |
| `handoff-db` | `hj handoff-db` |
| `handup` | `hj handup` |

Each compatibility binary calls the same `hjx::main_entry` function. The executable name is
rewritten to the corresponding `hj` subcommand before Clap parses the arguments.

## Quick start

Run these commands inside a Git repository:

```bash
# Create managed .ctx scaffolding and .gitignore entries.
hj refresh

# Create or update a handoff at session close.
hj handoff \
  --allow-create \
  --build clean \
  --tests passing \
  --log-summary "Create initial handoff state"

# Print prioritized open and blocked work at the next session start.
hj handon
```

`hj handoff` writes the YAML handoff, JSON state, `.ctx/HANDOFF.md`, and
`.ctx/HANDOVER.md`; syncs SQLite; appends session logs; and reconciles open items through `doob`.
The `doob` executable must be on `PATH` for reconciliation and closeout commands.

## Commands

| Command | Purpose |
| --- | --- |
| `detect` | Resolve the repository root, project, handoff name, or handoff path |
| `refresh` | Create `.ctx`, package state files, and managed `.gitignore` entries |
| `handon` | Print state, review-on-wake edits, and P0/P1/P2 triage |
| `handoff` | Close a session, persist state, render summaries, and reconcile todos |
| `close` | Compatibility alias for `handoff` |
| `handover` | Regenerate `.ctx/HANDOVER.md` from a handoff and its state |
| `handoff-db` | Initialize, synchronize, query, or update the handoff SQLite database |
| `handup` | Survey nested handoffs and TODO markers and write a cross-repo report |
| `reconcile` | Create missing `doob` todos for active handoff items |
| `audit` | Report reconciliation gaps without creating todos |
| `install` | Install all binaries from the current checkout |
| `update` | Install the latest published `hjx` package |
| `update-all` | Dispatch to the same published-package update path as `update` |

### Target selection

`handon`, `handover`, `reconcile`, and `audit` accept:

```text
--handoff <PATH>    Use an explicit handoff YAML file
--project <NAME>    Override the inferred project name
```

An explicit handoff path also relocates its associated state and rendered output to that file's
directory.

### `detect`

```text
--name       Print the resolved handoff filename
--root       Print the Git repository root
--project    Print the resolved project slug
--init       Refresh .ctx before resolving the target
```

Without a selector, `detect` prints the handoff path. If the file is absent, it still prints the
managed path and exits with status `2`. A legacy root-level handoff may be migrated into `.ctx`.

### `handoff` and `close`

```text
--handoff <PATH>        Use an explicit handoff YAML file
--project <NAME>        Override the inferred project name
--force-refresh         Rebuild managed .ctx scaffolding
--allow-create          Create a missing handoff model
--build <STATUS>        Record build state
--tests <STATUS>        Record test state
--notes <TEXT>          Record session notes
--log-summary <TEXT>    Add a session log entry
--commit <SHA>          Attach a commit; may be repeated
```

When `--log-summary` is present without `--commit`, the current short Git HEAD is attached when it
can be resolved. Log entries are persisted to YAML, SQLite, and `$HOME/.ctx/handoff-log.jsonl`.

### `refresh`

`hj refresh --force` recreates the managed setup even when `.ctx/.initialized` already exists.
Refresh scans Cargo, Python, and Go manifests to create per-package JSON state files.

### `handoff-db`

The database command uses `$HOME/.ctx/handoff.db` by default:

```bash
hj handoff-db init
hj handoff-db upsert --project demo --handoff .ctx/HANDOFF.demo.repo.yaml
hj handoff-db query --project demo
hj handoff-db complete --project demo --id demo-1
hj handoff-db status --project demo --id demo-1 --status blocked
```

`upsert` synchronizes every YAML item and prunes rows removed from that project. `complete` sets
`done` and stamps the current date; `status` applies the supplied status string.

### `handup`

```bash
hj handup --max-depth 3
```

The default depth is `5`. The command scans nested handoffs and source TODO markers, writes
`$HOME/.ctx/handoffs/<current-directory>/HANDUP.json`, and appends a checkpoint to
`$HOME/.ctx/handoffs/handup.db`.

### `reconcile` and `audit`

```bash
hj reconcile
hj audit
```

Both compare active handoff items with `doob` pending, in-progress, completed, and cancelled
todos. `reconcile` creates missing todos. `audit` makes no changes and exits with status `1` when
items are not captured or are already closed upstream.

## Library entry points

The package also exposes two small public Rust functions:

| API | Purpose |
| --- | --- |
| `hjx::run()` | Parse the current process arguments and return command errors |
| `hjx::main_entry()` | Run the CLI, print a formatted error, and exit with status `1` on failure |

Business-facing types and reusable operations are provided by `hjlib`, not `hjx`.

## Source-grounded examples

The scripts under [`../../examples`](../../examples/README.md) build local binaries, create a
temporary Git repository, and replace `HOME` with an isolated directory. They exercise every
command without writing to the user's real handoff databases.

```bash
./examples/commands/01-detect.sh
./examples/commands/03-handoff.sh
./examples/commands/06-handoff-db.sh
./examples/commands/07-handup.sh
./examples/visual-demo.sh
```

The `reconcile` and `audit` demos skip themselves when `doob` is unavailable. Update demos require
registry and network access.

## Development and testing

Run commands from the workspace root:

```bash
cargo check --workspace --locked
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace --locked
```

Build or test only the CLI package:

```bash
env RUSTC_WRAPPER= cargo build -p hjx --bins --locked
env RUSTC_WRAPPER= cargo test -p hjx --locked
cargo run -p hjx --bin hj -- --help
```

Argument and alias tests live in `src/lib.rs`. Command implementation tests live beside the
handoff modules, while cross-boundary library behavior is covered by the `conformance` package.

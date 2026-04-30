# hj

`hj` tracks work-in-progress across sessions using structured YAML handoff files. Each repo
gets a `.ctx/HANDOFF.<project>.<repo>.yaml` that holds open items, priorities, and a session
log. The CLI reads that file to triage at session start, appends log entries at session end,
syncs state to SQLite, and renders markdown summaries for humans and agents.

## Quick Start

```bash
# Install via pre-built binary
cargo binstall hjx

# In any git repo: scaffold .ctx/ and .gitignore entries
hj refresh

# Start a session — prints P0/P1/P2 triage from the handoff file
hj handon

# End a session — appends a log entry, writes HANDOFF.md, syncs SQLite
hj handoff --log-summary "What you did"
```

## Handoff File Format

Handoff files live at `.ctx/HANDOFF.<project>.<repo>.yaml`:

```yaml
project: myproject
id: myproject
updated: 2026-04-30
items:
  - id: mp-1
    priority: P1
    status: open
    title: Wire the render pass
    description: |
      Description of what needs doing and why.
    files:
      - src/render.rs
  - id: mp-2
    priority: P2
    status: done
    title: Add conformance tests
    completed: 2026-04-28
log:
  - date: 20260428:131508
    summary: Shipped render pass skeleton, all tests pass.
    commits:
      - sha: abc1234
        branch: main
```

Items have `status: open | blocked | done`. The `log` section grows with each session.
Commits accept either bare SHAs (`abc1234`) or `{sha, branch}` maps.

## Install

Install the latest release via [`cargo binstall`](https://github.com/cargo-bins/cargo-binstall)
(downloads pre-built binaries from GitHub Releases):

```bash
cargo binstall hjx
```

Supported targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`. Unsupported targets fall back to
source compilation.

Install from the current checkout:

```bash
hj install
```

Manual equivalent:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hjx --bins --force --root ~/.local
```

Update to the latest published release:

```bash
hj update
```

## Installed Binaries

| Binary           | Equivalent            |
| ---------------- | --------------------- |
| `hj`             | Main CLI              |
| `handoff`        | `hj handoff`          |
| `handon`         | `hj handon`           |
| `handover`       | `hj handover`         |
| `handoff-detect` | `hj detect`           |
| `handoff-db`     | `hj handoff-db`       |
| `handup`         | `hj handup`           |

## Commands

| Command         | What it does                                                                               |
| --------------- | ------------------------------------------------------------------------------------------ |
| `hj detect`     | Resolve the active handoff path, repo root, or project name                                |
| `hj handon`     | Print grouped P0/P1/P2 triage from the current handoff                                     |
| `hj handoff`    | Append a log entry, write `HANDOFF.md` + `HANDOVER.md`, sync SQLite, reconcile with doob  |
| `hj close`      | Alias for `hj handoff`                                                                     |
| `hj handover`   | Regenerate `.ctx/HANDOVER.md` from the current handoff and session state                   |
| `hj handoff-db` | Inspect or update the handoff SQLite store                                                 |
| `hj handup`     | Scan nested repos and TODO markers, emit a handup report                                   |
| `hj refresh`    | Scaffold `.ctx/` and add gitignore entries for state files                                 |
| `hj reconcile`  | Create missing doob todos for open handoff items                                           |
| `hj audit`      | Report handoff items not covered by doob, without mutating state                           |
| `hj install`    | Install binaries from the current checkout into `~/.local/bin`                             |
| `hj update`     | Update installed binaries to the latest published `hjx` release                           |

### Key flags

`hj handoff` / `hj close`:

```
--log-summary <TEXT>   Session summary appended to the log
--commit <SHA>         Commit SHA(s) to attach (repeatable)
--build <STATUS>       Build state to record (e.g. clean, failing)
--tests <STATUS>       Test state to record
--notes <TEXT>         Freeform session notes
--project <NAME>       Override project name
--handoff <PATH>       Explicit handoff file path
--allow-create         Create the handoff file if it does not exist
--force-refresh        Force a full refresh of .ctx scaffolding
```

`hj detect`:

```
--name     Print the inferred project name
--root     Print the repo root
--project  Print the resolved project slug
--init     Initialize .ctx if missing
```

## Common Workflows

**Start a session:**

```bash
hj handon
```

**End a session:**

```bash
hj handoff --log-summary "Implemented X, fixed Y" --commit abc1234
```

**Survey all repos under the current directory:**

```bash
hj handup
handup --max-depth 3
```

**Inspect the SQLite store:**

```bash
handoff-db query --project myproject
handoff-db upsert --project myproject --handoff .ctx/HANDOFF.myproject.myproject.yaml
```

**Sync open items into doob:**

```bash
hj reconcile
hj audit   # dry-run: report only, no mutations
```

**Regenerate the handover summary:**

```bash
hj handover
```

## Workspace Crates

| Crate   | Role                                                             |
| ------- | ---------------------------------------------------------------- |
| `hjlib` | Library: models, detect, git, render, doob, sqlite              |
| `hjx`   | Binary: CLI entrypoints, Clap arg parsing, command dispatch      |

## Development

```bash
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace --locked
cargo check --workspace --locked
```

Run a single test by name:

```bash
env RUSTC_WRAPPER= cargo test --workspace --locked -- test_name
```

Runnable command demos live under [`examples/`](./examples/README.md). They use disposable
temp repos and an isolated `HOME`, so they do not touch your real handoff DB or `~/.local`.

```bash
./examples/commands/01-detect.sh
./examples/commands/03-handoff.sh
./examples/commands/07-handup.sh
./examples/visual-demo.sh
```

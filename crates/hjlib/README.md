# hjlib

`hjlib` is the reusable library behind the `hj` handoff CLI. It owns the handoff data model,
repository discovery, Markdown rendering, SQLite persistence, `doob` integration, and workspace
survey helpers. The crate performs the domain and adapter work that `hjx` exposes as commands.

## Workspace role

The `hj` workspace has three packages:

- `hjlib` contains models, pure reconciliation logic, and filesystem/process adapters.
- `hjx` is the CLI composition root and depends on `hjlib`.
- `conformance` verifies contracts at the `hjlib` module boundaries.

`hjlib` is configured as a publishable package separate from the CLI, with documentation at
<https://docs.rs/hjlib>.

## Add the crate

From crates.io:

```toml
[dependencies]
hjlib = "0.1.3"
```

From this workspace:

```toml
[dependencies]
hjlib = { path = "../hjlib" }
```

## Core data model

The crate serializes handoff YAML into the following primary types:

| Type | Purpose |
| --- | --- |
| `Handoff` | Project metadata, work items, and session log entries |
| `HandoffItem` | One open, blocked, done, or otherwise classified work item |
| `LogEntry` | A dated session summary with zero or more commits |
| `CommitRef` | A bare SHA or a `{ sha, branch }` commit reference |
| `HandoffState` | Ephemeral branch, build, test, notes, and recent-session state |
| `ExtraEntry` | Structured extra metadata, including human-edit review records |
| `HandupReport` | Cross-repository survey output produced by `handup` |

Unknown handoff and log fields are retained through flattened maps. `CommitRef` accepts both bare
SHA values and object values, including YAML scalars that would otherwise be interpreted as
numbers.

### Parse and inspect a handoff

This example uses the same model and active-item filtering exercised by the crate and conformance
tests:

```rust
use hjlib::Handoff;

let yaml = r#"
project: demo
items:
  - id: demo-1
    priority: P1
    status: open
    title: Wire the render pass
log: []
"#;

let handoff: Handoff = serde_yaml::from_str(yaml)?;
let active = handoff.active_items().collect::<Vec<_>>();

assert_eq!(active.len(), 1);
assert_eq!(active[0].todo_title(), "Wire the render pass");
# Ok::<(), serde_yaml::Error>(())
```

`Handoff::validate()` reports missing IDs, misplaced log records, missing summaries, and duplicate
IDs. `Handoff::repair()` moves log-shaped records out of `items`, preserves valid items, and sorts
the repaired log newest first.

## Modules and key APIs

### Root domain API

| API | Purpose |
| --- | --- |
| `Handoff::active_items` | Iterate over items whose status is `open` or `blocked` |
| `Handoff::ensure_project` | Fill an absent project name |
| `Handoff::ensure_id_prefix` | Fill an absent, sanitized seven-character ID prefix |
| `Handoff::validate` / `repair` | Diagnose and repair malformed handoff structure |
| `HandoffItem::todo_title` | Prefer a titleized `name` slug and mark blocked work |
| `HandoffItem::title_variants` | Produce matching variants used during reconciliation |
| `infer_priority` | Infer `P0`, `P1`, or `P2` from title and description signals |
| `build_reconcile_plan` | Compare handoff items with an external todo snapshot |
| `sanitize_name` / `default_id_prefix` | Normalize project names and item prefixes |

`build_reconcile_plan` is backend-independent. `ReconcileMode::Audit` reports missing items without
creating work; `ReconcileMode::Sync` returns `ReconcileCreate` values for a caller to persist.

### `detect`

Repository discovery and managed-path construction:

| API | Purpose |
| --- | --- |
| `discover` | Resolve the current Git repository into a `RepoContext` |
| `RepoContext::project_name` | Derive a project from Cargo, Python, Go, or directory metadata |
| `RepoContext::paths` | Build handoff, state, rendered, and handover paths |
| `RepoContext::refresh` | Create `.ctx`, state files, and the managed `.gitignore` block |
| `scan_package_names` | Discover package names while excluding generated/vendor trees |
| `branch_name` / `current_short_head` | Read current Git metadata |

For project `demo` in repository `repo`, `RepoContext::paths` resolves the primary files as:

```text
.ctx/HANDOFF.demo.repo.yaml
.ctx/HANDOFF.demo.repo.state.json
.ctx/HANDOFF.md
.ctx/HANDOVER.md
```

### `render`

- `render_markdown` emits the main handoff document with state, active items, and five log entries.
- `render_handover_markdown` emits the compact handover document used between sessions.

Both renderers sort active items by priority, status, and ID, and exclude non-active items.

```rust
use hjlib::render::render_markdown;
use hjlib::{Handoff, HandoffState};

let markdown = render_markdown(&Handoff::default(), Some(&HandoffState::default()));
assert!(markdown.contains("## Items"));
assert!(markdown.contains("## Log"));
```

### `sqlite`

`HandoffDb` stores project items and log entries. `upsert` updates matching rows and removes rows no
longer present in the target project's handoff. `complete` and `set_status` update individual
items. `HandupDb` appends cross-repository survey checkpoints.

Use `with_path` for isolated tools and tests:

```rust
use hjlib::Handoff;
use hjlib::sqlite::HandoffDb;

let db = HandoffDb::with_path(std::env::temp_dir().join("hj-example.db"));
db.upsert("demo", &Handoff::default(), "2026-09-19")?;
assert!(db.query("demo")?.is_empty());
# Ok::<(), anyhow::Error>(())
```

The default stores are `$HOME/.ctx/handoff.db` and `$HOME/.ctx/handoffs/handup.db`.

### `git`

- `discover_handoffs` scans for YAML and Markdown handoffs and returns `SurveyHandoff` values.
- `discover_todo_markers` scans Rust, shell, Python, and TOML files for `TODO:`, `FIXME:`, `HACK:`,
  and `XXX:` markers.

Markdown survey input recognizes bullets under `Known Gaps`, `Next Up`, `Parked`, and
`Remaining Work` headings.

### `doob`

`DoobClient` shells out to the `doob` executable to list and create todos. `snapshot` groups
pending/in-progress titles as active and completed/cancelled titles as closed. The pure helpers
`map_priority` and `unique_titles` are available without invoking `doob`.

## Development and testing

Run commands from the workspace root:

```bash
cargo check --workspace --locked
cargo fmt --all --check
env RUSTC_WRAPPER= cargo clippy --workspace --locked -- -D warnings
env RUSTC_WRAPPER= cargo test --workspace --locked
```

Run only this crate's tests:

```bash
env RUSTC_WRAPPER= cargo test -p hjlib --locked
```

Run the external boundary tests that exercise `hjlib`:

```bash
env RUSTC_WRAPPER= cargo test -p conformance --locked
```

Unit tests live beside their modules. Cross-module contracts live in
[`../../tests/conformance`](../../tests/conformance/README.md); the live-`doob` test there is
ignored by default.

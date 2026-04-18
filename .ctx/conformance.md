# Conformance Spec — hj workspace

This document defines the port/adapter boundary contracts for the hj workspace.
Each section maps to a crate boundary and lists the specific assertions that
conformance tests must verify. Tests live in `tests/conformance/` at the workspace
root and are tagged `#[cfg(test)]` with the filter string `conformance`.

All conformance tests must be pure (no network, no live git, no live doob process)
unless marked `[integration]`.

---

## 1. hj-core — Pure Domain

**Crate:** `hj-core`
**Rule:** Zero I/O. No `std::process`, no file access, no external crate calls.
**Tests must verify:**

### 1.1 `build_reconcile_plan` purity and correctness

- Given `Handoff` with items and a `TodoSnapshot`, returns `ReconcilePlan` with no
  side effects.
- `Audit` mode: never populates `creates`; populates `not_captured` for uncovered items.
- `Sync` mode: populates `creates` for uncovered items; increments `created_count`.
- Items already in `active_titles` increment `captured_count` and are not re-created.
- Items in `closed_titles` go to `closed_upstream`, not `not_captured` or `creates`.
- Titles in `active_titles` that match no handoff item go to `orphaned`.
- Matching uses all `title_variants()` — raw title, blocked title, todo_title,
  blocked todo_title.

### 1.2 `HandoffItem::todo_title()` and `title_variants()`

- When `name` is set (non-empty, non-"null"): `todo_title()` returns `titleize_slug(name)`.
- When `name` is absent: `todo_title()` returns `title`.
- When `status` is `"blocked"`: appends ` [BLOCKED]` suffix.
- `title_variants()` contains no duplicates.
- `title_variants()` contains no empty strings.

### 1.3 `infer_priority`

- Titles containing `"broken"`, `"fails"`, `"security"`, `"blocked"`, `"urgent"`,
  `"panic"`, `"segfault"`, `"can't deploy"` → `"P0"`.
- Titles containing `"fix"`, `"implement"`, `"refactor"`, `"wire"`, `"small change"`,
  `"known fix"` → `"P1"`.
- All other titles → `"P2"`.
- Description field contributes to classification when title alone does not trigger.

### 1.4 `Handoff::active_items()`

- Returns only items with `status == "open"` or `status == "blocked"`.
- Items with `status == "done"`, `"closed"`, `None`, or any other value are excluded.

### 1.5 `HandoffState` serialization

- `touched_files` is omitted from YAML output when the vec is empty
  (`skip_serializing_if`).
- Round-trip: serialize then deserialize `HandoffState` produces identical struct.

### 1.6 `titleize_slug`

- `"wire-render-pass"` → `"Wire Render Pass"`.
- Empty segments (consecutive dashes) are skipped.
- Single-word slug capitalizes first letter only.

### 1.7 `sanitize_name` and `default_id_prefix`

- Spaces and `/` are replaced with `-`, result is lowercase.
- `default_id_prefix` returns at most 7 characters.

---

## 2. hj-sqlite → hj-core Adapter

**Crate:** `hj-sqlite`
**Port:** `HandoffDb` wraps SQLite, accepts and returns `hj-core` types.
**Tests must verify:**

### 2.1 `HandoffDb::upsert()` → `query()` round-trip

- Items written via `upsert()` are retrievable via `query()`.
- `query()` returns rows ordered by `priority ASC, id ASC`.
- `upsert()` on the same `(project, id)` pair updates `status`, `completed`,
  `updated` without creating a duplicate row.

### 2.2 Pruning contract

- After `upsert()` with a reduced item list, rows for removed item IDs are deleted.
- Pruning is scoped to the target `project` — other projects' rows survive.
- `upsert()` with an empty `Handoff` deletes all rows for that project only.

### 2.3 `HandoffDb::complete()` and `set_status()`

- `complete()` sets `status = "done"` and stamps the provided date into `completed`.
- `set_status()` changes `status` without touching `completed`.
- Both return `true` when a row was updated, `false` when no row matched.

### 2.4 `HandupDb::checkpoint()`

- Each call inserts a new row; does not replace existing rows.
- `checkpoint()` returns the db path on success.
- The `checkpoints` table exists after first call.

---

## 3. hj-doob → hj-core Adapter

**Crate:** `hj-doob`
**Port:** `DoobClient` wraps the `doob` CLI process. Pure functions are testable
without spawning a process.
**Tests must verify:**

### 3.1 `map_priority` — shell parity

- `P0` → `5`, `P1` → `4`, `P2` → `3`, anything else → `1`.
- This mapping must match the atelier shell script contract exactly.

### 3.2 `unique_titles`

- Deduplicates exact-match strings (case-sensitive).
- Drops empty strings.
- Returns a sorted, stable result (BTreeSet order).

### 3.3 `DoobClient::snapshot()` type contract [integration]

- `snapshot()` returns a `TodoSnapshot` (hj-core type).
- `active_titles` contains titles from `pending` and `in_progress` statuses.
- `closed_titles` contains titles from `completed` and `cancelled` statuses.
- Requires `doob` on PATH — skip if absent.

---

## 4. hj-git → hj-core Adapter

**Crate:** `hj-git`
**Port:** `hj-git` reads filesystem and git state, produces `hj-core` types.
**Tests must verify:**

### 4.1 `is_handoff_file` filter

- Files matching `HANDOFF.*.yaml` are accepted.
- Files matching `HANDOFF.*.state.json` are rejected.
- Files with names not starting `HANDOFF.` are rejected.
- `HANDOFF.md` is accepted (markdown variant).

### 4.2 `parse_markdown_handoff`

- Bullets under `## Known Gaps`, `## Next Up`, `## Parked`, `## Remaining Work`
  are extracted as items.
- Bullets under other sections are ignored.
- Priority is inferred via `hj_core::infer_priority`.
- IDs are assigned as `md-1`, `md-2`, … in order.

### 4.3 `RepoContext::paths()` naming contract

- `handoff_path` is `<repo_root>/.ctx/HANDOFF.<project>.<base_name>.yaml`.
- `state_path` is `<repo_root>/.ctx/HANDOFF.<project>.<base_name>.state.json`.
- `explicit_project` overrides the derived project name.
- Project name is sanitized (lowercase, spaces/slashes → `-`).

### 4.4 `manifest_name` resolution order

- Reads `Cargo.toml` `[package].name` first.
- Falls back to `pyproject.toml` `[project].name` or `[tool.poetry].name`.
- Falls back to `go.mod` module basename.
- Returns `None` when no manifest is present.

### 4.5 `write_gitignore_block` idempotency

- Running twice produces identical output (no duplicate block).
- Replaces the existing `# handoff-begin` … `# handoff-end` block in place.
- Lines outside the managed block are preserved.

---

## 5. hj-render → hj-core Adapter

**Crate:** `hj-render`
**Port:** `hj-render` consumes `hj-core` types and produces markdown strings.
**Tests must verify:**

### 5.1 `render_markdown` structure

- Output starts with `# Handoff — {project} ({updated})\n`.
- Contains `**Branch:** … | **Build:** … | **Tests:** …` line.
- Contains `## Items` section with markdown table.
- Contains `## Log` section.
- When `state` is `None`, branch/build/tests render as `"unknown"`.

### 5.2 Item sort order

- Items sorted P0 before P1 before P2.
- Within same priority, `open` before `blocked`.
- Within same priority and status, sorted by `id` lexicographically.
- Items with `status` other than `open`/`blocked` do not appear.

### 5.3 `render_handover_markdown` structure

- Output starts with `## State\n`.
- Contains `Branch: … | Build: … | Tests: …` (no bold markers).
- Notes appear between status line and items table when non-empty.
- Log capped at 5 entries.

### 5.4 Commit formatting in log

- Entries with commits render as `- {date}: {summary} [{sha1, sha2}]`.
- Entries without commits render as `- {date}: {summary}` (no brackets).

---

## 6. hj-cli — Composition Root

**Crate:** `hj-cli`
**Rule:** No business logic. All computation delegated to library crates.
**Tests must verify:**

### 6.1 Alias rewriting

- `handoff-detect [args…]` rewrites to `hj detect [args…]`.
- `handoff [args…]` rewrites to `hj handoff [args…]`.
- `handon [args…]` rewrites to `hj handon [args…]`.
- `handover [args…]` rewrites to `hj handover [args…]`.
- `handoff-db [args…]` rewrites to `hj handoff-db [args…]`.
- Non-alias invocations (`hj [args…]`) pass through unchanged.

### 6.2 CLI parsing

- `hj install` defaults `--root` to `~/.local`.
- `hj update` and `hj update-all` default `--root` to `~/.local`.
- `hj handon --project <name>` populates `TargetArgs::project`.
- `hj detect`, `hj handoff`, `hj handon`, `hj handover` all parse without error.

---

## Test Location and Naming

```
tests/
  conformance/
    core.rs          # §1 — hj-core pure domain
    sqlite.rs        # §2 — hj-sqlite adapter
    doob.rs          # §3 — hj-doob adapter
    git.rs           # §4 — hj-git adapter
    render.rs        # §5 — hj-render adapter
    cli.rs           # §6 — hj-cli composition root
```

Each test function name must encode the section: e.g.
`fn s1_1_reconcile_audit_no_creates()`, `fn s2_3_complete_sets_done_status()`.

Integration tests requiring external processes are gated:
```rust
#[test]
#[cfg_attr(not(feature = "integration"), ignore)]
fn s3_3_snapshot_type_contract() { … }
```

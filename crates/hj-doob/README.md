# hj-doob

`hj-doob` contains the `doob` integration helpers used by `hj`.

## Provides

- `DoobClient` for listing and creating todos
- `TodoStatus` for status-specific queries
- `ReconcileReport` for summarize/audit output
- `map_priority`, `unique_titles`, and `ensure_doob_on_path`

## Responsibilities

- Query `doob todo list` output as JSON
- Add handoff items to `doob` with mapped priorities and tags
- Normalize and deduplicate todo titles during reconciliation
- Fail early when `doob` is not available on `PATH`

## Used By

- `hj-cli reconcile`
- `hj-cli audit`
- `hj-cli close`

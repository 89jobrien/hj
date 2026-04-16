# hj-sqlite

`hj-sqlite` contains the SQLite-backed stores used by `hj`.

## Provides

- `HandoffDb` and `HandoffRow`
- `HandupDb` and `HandupCheckpoint`
- `UpsertReport`

## Responsibilities

- Persist per-project handoff items in `~/.local/share/atelier/handoff.db`
- Prune rows removed from the current handoff during upsert
- Track handup survey checkpoints in `~/.ctx/handoffs/handup.db`
- Support status transitions such as `complete` and `set_status`

## Used By

- `hj-cli handoff-db`
- `hj-cli handup`
- `hj-cli close`

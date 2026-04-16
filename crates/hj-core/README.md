# hj-core

`hj-core` contains the shared data model and helper logic for the `hj` workspace.

## Provides

- `Handoff`, `HandoffItem`, `HandoffState`, and `LogEntry`
- `HandupReport`, `HandupProject`, and `HandupRecommendation`
- Priority inference and title helpers such as `infer_priority`, `sanitize_name`, and `default_id_prefix`

## Responsibilities

- Represent structured handoff YAML and handup JSON shapes
- Normalize project and item naming
- Infer missing priorities from title and description text
- Provide helper methods for active/open item filtering and doob title variants

## Used By

- `hj-cli` for command parsing and state transitions
- `hj-git` for parsing and discovery
- `hj-render` for markdown output
- `hj-sqlite` for persistence

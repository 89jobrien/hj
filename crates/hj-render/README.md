# hj-render

`hj-render` turns structured handoff data into markdown.

## Provides

- `render_markdown`

## Responsibilities

- Render project header state such as branch, build, and test status
- Sort active items by priority and status for readable output
- Render recent log entries into `HANDOFF.md`

## Used By

- `hj-cli close`

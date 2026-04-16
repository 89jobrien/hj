# hj-git

`hj-git` contains repository discovery and `.ctx` management utilities for `hj`.

## Provides

- `RepoContext` and `HandoffPaths`
- `discover`, `branch_name`, `current_short_head`, and `today`
- `discover_handoffs` and `discover_todo_markers`
- `.ctx` refresh and managed `.gitignore` rewriting

## Responsibilities

- Resolve repo roots and project names from the current working tree
- Build canonical `.ctx/HANDOFF.*` and `.state.yaml` paths
- Build canonical `.ctx/HANDOVER.md` output paths
- Initialize missing state files during `hj refresh`
- Scan nested repos for handoff files and TODO markers
- Maintain the managed handoff block inside `.gitignore`

## Used By

- `hj-cli detect`
- `hj-cli refresh`
- `hj-cli handup`
- `hj-cli close`
- `hj-cli handover`

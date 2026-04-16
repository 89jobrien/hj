# Examples

This directory shows the `hj` command surface in action against disposable demo repos.

All scripts:

- build the local `hj` binaries from this checkout if needed
- create a temporary demo repo under `/tmp`
- use an isolated `HOME` so SQLite state and install roots do not touch your real environment

## Commands

| Script | Shows |
|---|---|
| `commands/01-detect.sh` | `hj detect` and `handoff-detect` |
| `commands/02-refresh.sh` | `hj refresh` |
| `commands/03-handoff.sh` | `hj handoff` and `handoff` |
| `commands/04-handon.sh` | `hj handon` and `handon` |
| `commands/05-handover.sh` | `hj handover` and `handover` |
| `commands/06-handoff-db.sh` | `hj handoff-db` and `handoff-db` |
| `commands/07-handup.sh` | `hj handup` and `handup` |
| `commands/08-install.sh` | `hj install` into a temp root |
| `commands/09-update.sh` | `hj update` with a temp root |
| `commands/10-update-all.sh` | `hj update-all` with a temp root |
| `commands/11-reconcile.sh` | `hj reconcile` when `doob` is available |
| `commands/12-audit.sh` | `hj audit` when `doob` is available |
| `commands/13-close.sh` | legacy `hj close` compatibility path |

## Usage

Run any script from the repo root:

```bash
./examples/commands/01-detect.sh
./examples/commands/03-handoff.sh
./examples/commands/07-handup.sh
```

To run every example in sequence:

```bash
for script in ./examples/commands/*.sh; do
  echo "==> $script"
  "$script"
done
```

Generate the standalone visual walkthrough:

```bash
./examples/visual-demo.sh
```

## Notes

- `update` and `update-all` install from the published `hj-cli` package, so they require registry/network access.
- `reconcile` and `audit` require `doob` on `PATH`.
- The demo repos are temporary and can be deleted after each run.
- `visual-demo.sh` writes a browserable artifact to `examples/visual/index.html`.

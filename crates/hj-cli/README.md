# hj-cli

`hj-cli` is the installable command-line package for the `hj` workspace.

It provides:

- `hj`
- `handoff-detect`
- `handoff-db`
- `handup`

## Install From a Checkout

From the repository root:

```bash
hj install
```

Manual equivalent:

```bash
env RUSTC_WRAPPER= cargo install --path crates/hj-cli --bins --force --root ~/.local
```

This refreshes all four binaries in `~/.local/bin`.

## Update To the Latest Release

```bash
hj update
```

`hj update-all` is a synonym for the same release update path.

## Top-Level Commands

```text
hj detect
hj handoff-db
hj handup
hj install
hj update
hj update-all
hj refresh
hj reconcile
hj audit
hj close
```

## Notes

- `hj install` uses the current checkout and should be run from an `hj` repository.
- `hj update` installs the latest published `hj-cli` package.
- `handoff-detect`, `handoff-db`, and `handup` are thin aliases that route into `hj`.

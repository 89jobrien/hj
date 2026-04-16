# Visual Demo

Generate a standalone HTML walkthrough of the main local `hj` flow:

```bash
./examples/visual-demo.sh
```

That script:

- builds the local binaries if needed
- creates a disposable repo under `/tmp`
- runs the core commands against it
- writes a browserable artifact to `examples/visual/index.html`

The page embeds real command output and rendered files from the demo run. It avoids `install`, `update`, `reconcile`, and `audit` because those depend on the network or an external task backend.

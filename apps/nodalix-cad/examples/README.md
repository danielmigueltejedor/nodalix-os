# Nodalix CAD Examples

## Native project

Open:

```bash
cargo run
```

Then use `Open` and select:

```text
examples/simple.nodcad
```

## STEP metadata test

Daniel's sample STEP file can be inspected without opening the UI:

```bash
cargo run -- --inspect-step /mnt/data/5621-Separador.stp
```

The current STEP importer reads metadata and entity type counts only. It does not tessellate or render B-Rep geometry yet.

If the external sample path is not available in your environment, test the bundled parser example:

```bash
cargo run -- --inspect-step examples/minimal.step
```

## DXF export test

Open `examples/simple.nodcad`, then use `Export DXF`.

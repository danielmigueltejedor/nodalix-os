# Lix CAD

Native CAD and technical drawing application by Nodalia, part of the Lix Suite.

LixCAD is the standalone product name; “Lix CAD” is used in UI headings where spacing improves readability. It is designed to run both inside Nodalix OS and as an independent Linux engineering app.

## Current status

This is still early software, but it now has:

- real serializable `.nodcad` / `.lixcad` document model
- layers
- 2D entities: point, line, polyline/rectangle, circle, text placeholder, dimension placeholder
- first real CAD annotation/modeling entities: dimension, hatch boundary, table, block reference, construction guideline
- interactive line/rectangle/circle creation on the canvas
- real polyline drawing with connected vertices, live preview, Enter to finish, and Esc to cancel
- hover object snaps for endpoints, midpoints, centers, quadrants, intersections, perpendicular projections, and ortho guides
- contextual right-side tool parameters for drawing, dimensions, text, modify, blocks, hatch, tables, parametric constraints, and guidelines
- STEP/STP metadata/reference import
- integrated DWG import setup flow with converter detection
- DWG converter detection from `/home/dani/lixcad/settings.toml`, `~/.config/lixcad/settings.toml`, `~/.config/nodalix-cad/settings.toml`, PATH, and known ODA install paths
- automatic DWG -> DXF -> Lix CAD import when ODA File Converter or LibreDWG `dwg2dxf` is available
- DWG export bridge that writes a DXF intermediate for external conversion
- ASCII and binary STL metadata import with triangle count and bounding box
- basic DXF import for LINE, LWPOLYLINE/POLYLINE, CIRCLE, TEXT and MTEXT
- basic DXF export for line/polyline/circle/text/point
- SVG export for 2D entities
- reverse engineering workflow panel
- scale factor calculator: `factor = real_distance / measured_distance`
- technical drawing metadata for A4/A3-style future sheets/views

## Supported formats

| Format | Import | Export | Status |
| --- | --- | --- | --- |
| `.nodcad` / `.lixcad` | yes | yes | native JSON project format |
| `.stp` / `.step` | metadata/reference | no | B-Rep/tessellation planned |
| `.iges` / `.igs` | metadata/reference path | no | treated like STEP reference for now |
| `.dwg` | via converter setup | DXF intermediate | uses external DWG converter internally; ODA File Converter and LibreDWG `dwg2dxf` can import automatically |
| `.stl` | basic ASCII/binary mesh metadata | no | triangle count, bbox, scale metadata |
| `.obj` | reference placeholder | no | parsing planned |
| `.ply` | reference placeholder | no | parsing planned |
| `.dxf` | basic 2D import | yes | imports LINE/LWPOLYLINE/POLYLINE/CIRCLE/TEXT/MTEXT; exports line/polyline/circle/text/point plus early dimensions/tables/hatch/block references |
| `.svg` | no | yes | simple 2D entity export |
| `.pdf` | no | placeholder | drawing PDF export planned |

## Run

```bash
cd /home/dani/Projects/nodalix-os/apps/nodalix-cad
cargo run
```

## Build

```bash
cargo fmt
cargo build
cargo build --release
```

## Test STEP metadata import

```bash
cargo run -- --inspect-step /mnt/data/5621-Separador.stp
```

In the UI, use `Import sample STEP` or `Import` and choose the STEP file. The app will show metadata and entity counts. It will not render STEP geometry yet.

## DWG import

DWG is proprietary, so Lix CAD does not pretend to parse it natively. The app now treats DWG import as an integrated conversion workflow:

1. Detect local converters.
2. If ODA File Converter or LibreDWG `dwg2dxf` is available, convert DWG into a cache DXF under `~/.cache/lixcad/imports/`.
3. Import the generated DXF through the normal DXF importer.
4. Store the original DWG as an imported reference and show the conversion log path.

If no converter is available, opening a DWG shows the **DWG Import Setup** dialog instead of a dead-end error. The dialog reports:

- ODA File Converter status.
- LibreDWG `dwg2dxf` / `dwgread` status.
- FreeCAD status for future bridge work.
- Optional config files: `/home/dani/lixcad/settings.toml`, `~/.config/lixcad/settings.toml`, `~/.config/nodalix-cad/settings.toml`.

Example config:

```toml
[dwg_import]
backend = "oda"
converter_path = "/usr/bin/oda-file-converter"
```

Debug converter detection:

```bash
cargo run -- --dwg-status
cat /tmp/nodalix-cad.log
```

## Reverse engineering workflow target

The app is being shaped around this workflow:

1. Import STL mesh / STEP model.
2. Clean or crop mesh.
3. Measure a known distance.
4. Calculate scale factor.
5. Scale model metadata.
6. Align with project axes.
7. Create section/sketch from mesh.
8. Fit curves.
9. Reconstruct profile.
10. Extrude.
11. Create normalized drawing views.
12. Dimension drawing.
13. Export DXF/PDF.

Only the import, sketch primitives, scale metadata, native save/open, and DXF/SVG export foundation are implemented now.

## Limitations

- STEP import is metadata/reference only.
- DWG import depends on external converter quality. Current automatic command wiring is implemented for ODA File Converter and LibreDWG `dwg2dxf`; FreeCAD is detected/configured but still needs backend-specific command wiring.
- DWG export currently writes a DXF intermediate and reports the conversion requirement. Automatic DWG conversion will be enabled after the converter command is pinned and tested.
- STL rendering is a bounding-box/reference preview, not full 3D rendering.
- Snaps are early but functional; full constraint solving is still planned.
- No real B-Rep kernel yet.
- PDF export is not implemented.
- Dimensions are stored as placeholder entities.

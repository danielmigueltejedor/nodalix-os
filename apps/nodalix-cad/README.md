# Nodalix CAD

Native CAD and technical drawing application for Nodalix OS.

Nodalix CAD is moving from a visual prototype toward a real engineering workflow app for `Expresión Gráfica`, reverse engineering, and technical documentation on Linux.

## Current status

This is still early software, but it now has:

- real serializable `.nodcad` document model
- layers
- 2D entities: point, line, polyline/rectangle, circle, text placeholder, dimension placeholder
- first real CAD annotation/modeling entities: dimension, hatch boundary, table, block reference, construction guideline
- interactive line/rectangle/circle creation on the canvas
- contextual right-side tool parameters for drawing, dimensions, text, modify, blocks, hatch, tables, parametric constraints, and guidelines
- STEP/STP metadata/reference import
- DWG reference import with version signature detection
- DWG export bridge that writes a DXF intermediate for external conversion
- ASCII and binary STL metadata import with triangle count and bounding box
- basic DXF LINE import
- basic DXF export for line/polyline/circle/text/point
- SVG export for 2D entities
- reverse engineering workflow panel
- scale factor calculator: `factor = real_distance / measured_distance`
- technical drawing metadata for A4/A3-style future sheets/views

## Supported formats

| Format | Import | Export | Status |
| --- | --- | --- | --- |
| `.nodcad` | yes | yes | native JSON project format |
| `.stp` / `.step` | metadata/reference | no | B-Rep/tessellation planned |
| `.iges` / `.igs` | metadata/reference path | no | treated like STEP reference for now |
| `.dwg` | reference/signature | DXF intermediate | native geometry parsing planned; optional DWG/DXF bridge recommended |
| `.stl` | basic ASCII/binary mesh metadata | no | triangle count, bbox, scale metadata |
| `.obj` | reference placeholder | no | parsing planned |
| `.ply` | reference placeholder | no | parsing planned |
| `.dxf` | basic LINE import | yes | LINE/LWPOLYLINE/CIRCLE/TEXT/POINT plus early dimensions/tables/hatch/block references |
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
- DWG import is reference/signature only. Full DWG geometry needs an optional converter such as ODA File Converter or LibreDWG, then DXF import.
- DWG export currently writes a DXF intermediate and reports the conversion requirement. Automatic DWG conversion will be enabled after the converter command is pinned and tested.
- STL rendering is a bounding-box/reference preview, not full 3D rendering.
- No constraints/snaps beyond grid visual.
- No real B-Rep kernel yet.
- PDF export is not implemented.
- Dimensions are stored as placeholder entities.

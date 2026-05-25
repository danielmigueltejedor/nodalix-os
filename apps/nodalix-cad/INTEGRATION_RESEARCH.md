# Lix CAD Integration Research

Lix CAD should grow honestly and incrementally. Full STEP/B-Rep support requires a real CAD kernel or conversion bridge.

## OpenCascade / OCCT

Best long-term path for STEP, IGES, B-Rep, tessellation, booleans, and technical projections. Rust bindings are not as mature as C++/Python ecosystems, so integration may require either bindings research or a small bridge process.

## FreeCAD CLI / Python bridge

Practical optional bridge for Linux. FreeCAD can import STEP/IGES, tessellate shapes, and export meshes/DXF/SVG. It is heavy, so Lix CAD should detect it optionally and degrade gracefully.

## pythonOCC

Powerful OCCT Python binding. Useful for experiments and conversion pipelines, but introduces Python dependency and packaging complexity.

## Rust OCCT bindings

Research area. If viable, this would keep Nodalix CAD native, but API coverage and build complexity must be tested.

## STEP to mesh conversion

Intermediate path: convert STEP to triangle mesh for visualization while preserving the STEP file as the authoritative reference. This is not a replacement for B-Rep editing.

## Assimp

Useful for mesh formats such as OBJ/PLY/STL. It does not solve real CAD B-Rep. Optional CLI/library integration can help broad mesh import.

## DWG

DWG should be supported through a staged approach because it is a proprietary binary format. Lix CAD should use optional conversion backends internally so the user can open a DWG from the normal import flow without being left at a blocking error:

- ODA File Converter for DWG to DXF conversion when installed by the user.
- LibreDWG tools such as `dwgread` / `dwg2dxf` if available and compatible with the file version.
- FreeCAD CLI/Python bridge as a heavier fallback for future tests.
- A future native DWG parser only if licensing, stability, and coverage are acceptable.

Lix CAD should never claim full DWG compatibility until it can reliably load geometry, layers, blocks, units, dimensions, text styles, hatches, tables, and block references.

The desired practical pipeline is:

1. DWG input.
2. Optional converter creates DXF while preserving layers, line types, blocks, text, dimensions, hatches, and units as far as the converter supports.
3. Lix CAD imports DXF into its native document model.
4. Lix CAD edits the document.
5. Lix CAD exports DXF.
6. Optional converter writes DWG again.

Conversion must be explicit and transparent. If a property cannot be preserved, the summary dialog should say so.

Current implementation status:

- Detects ODA File Converter, LibreDWG and FreeCAD-style commands.
- Uses ODA File Converter or LibreDWG `dwg2dxf` automatically when available.
- Writes conversion output and logs under `~/.cache/lixcad/imports/`.
- Shows an integrated DWG Import Setup dialog when no automatic backend is ready.
- Stores the original DWG as an imported reference.
- Does not claim native DWG parsing.

Config path:

```toml
# /home/dani/lixcad/settings.toml
[dwg_import]
backend = "oda"
converter_path = "/usr/bin/oda-file-converter"
```

## DXF

DXF export can be implemented directly for basic 2D entities. Import currently supports LINE, LWPOLYLINE/POLYLINE, CIRCLE, TEXT and MTEXT. DXF remains the practical interchange target while DWG support matures.

## PDF/SVG export

SVG is straightforward for 2D drawings and can be converted to PDF later. PDF export can use Cairo printing or an SVG-to-PDF pipeline once drawing sheets mature.

## Safety

External tools must be optional. Missing dependencies must produce clear messages, not crashes. Commands must not overwrite user files without confirmation.

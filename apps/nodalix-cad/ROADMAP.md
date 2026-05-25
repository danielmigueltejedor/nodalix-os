# Nodalix CAD Roadmap

## Stage 0 — Prototype

- UI
- canvas
- grid
- placeholder tools

## Stage 1 — Real 2D sketching

- line
- real multi-segment polyline
- rectangle
- circle
- layers
- save/open `.nodcad` / `.lixcad`
- DXF export
- DXF import for LINE, LWPOLYLINE/POLYLINE, CIRCLE, TEXT and MTEXT
- keyboard finish/cancel flow for active drawing commands

## Stage 2 — Mesh import and reverse engineering

- STL import
- mesh metadata
- scale factor calculator
- align metadata
- section from mesh
- fit curves
- sketch reconstruction

## Stage 3 — STEP/STP and real CAD geometry

- STEP metadata/reference import
- DWG import setup wizard and converter detection
- LibreDWG DWG-to-DXF automatic import when `dwg2dxf` is available
- ODA File Converter and FreeCAD backend wiring
- OpenCascade/OCCT integration research
- optional FreeCAD/OCCT bridge
- B-Rep tessellation
- model tree

## Stage 4 — Technical drawings

- normalized views
- dimensions
- sections
- A3/A4 sheets
- SVG/PDF/DXF export

## Stage 5 — Production CAD features

- constraints
- snaps
- blocks
- assemblies
- parametric modeling
- advanced export/import

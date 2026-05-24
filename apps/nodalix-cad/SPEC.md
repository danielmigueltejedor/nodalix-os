# Specification

Nodalix CAD is a native CAD and technical drawing application for Nodalix OS.

## Scope

The app targets engineering graphics, reverse engineering, and technical documentation workflows on Linux.

## Current implemented foundation

- Native Rust + GTK4/libadwaita UI.
- CAD canvas with grid, axes, origin marker, cursor coordinates, and simple 2D rendering.
- 2D document entities: point, line, polyline/rectangle, circle, text placeholder, dimension placeholder.
- Layers: Default, Construction, Dimensions, Mesh Reference.
- Native `.nodcad` JSON format.
- STEP/STP metadata/reference import.
- ASCII/binary STL mesh metadata import.
- Basic DXF LINE import.
- DXF and SVG 2D export.
- Reverse engineering workflow panel and scale factor metadata.

## Honest limitations

- STEP B-Rep geometry is not rendered yet.
- STL is imported as mesh metadata/reference, not full 3D viewport geometry.
- No constraint solver.
- No real object snaps yet.
- PDF export is a placeholder.

## Safety

Unsupported formats must fail with friendly errors. External references must not overwrite source files. Export paths are user selected.


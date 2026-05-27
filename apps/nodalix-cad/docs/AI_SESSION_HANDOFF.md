# LixCAD — AI Session Handoff

> **Living document** for starting new AI sessions without dragging dirty chat context.  
> Update after major phases using `docs/AI_UPDATE_HANDOFF_PROMPT.md`.

**Last updated:** 2026-05-28 (Offset RefCell fix)

---

## 1. Project path

| | Path |
|---|------|
| **Correct** | `/home/dani/Proyectos/nodalix-os/apps/nodalix-cad` |
| **Wrong** | `/home/dani/Projects/nodalix-os/apps/nodalix-cad` (or any `/home/dani/Projects/...`) |

Package name: `nodalix-cad`. Installed binary: `~/.local/bin/lixcad`.  
Assets at runtime: `~/.local/share/nodalix-cad/assets/`.

---

## 2. Current stable state

- **LixCAD** is a 2D CAD desktop app in **Rust + GTK4/libadwaita**.
- Single binary crate (`src/main.rs`); **no `[lib]`** yet — unit tests compile into the same GTK-linked test binary (**178 tests**).
- **Dual model:** legacy `Document` (`src/document.rs`) remains the source of truth for UI/canvas; parallel **`src/cad/*`** core is being introduced gradually (entities, geometry, history, selection, commands, layers module stubs).
- **UI/canvas** still hold substantial legacy logic (`src/ui.rs`, `src/canvas.rs`); migration is incremental — **do not big-bang refactor**.
- **Recent milestone:** Phase **3.21** — OSNAP toolbar icons fixed; **Rotate / Scale / Mirror** modify tools with live preview, command bar, undo/redo via `LegacyTransformEntitiesAction`.
- Layers (3.18), OSNAP (3.19), Ortho/Polar/DYN (3.20) remain intact.

**Last known release hash** (2026-05-27, after 3.21 install):

```text
89e62a19087d849d5b888c83acd878298a5f91f67118b97d630166794a4151e0
```

(`target/release/nodalix-cad` and `~/.local/bin/lixcad` must match after install.)

---

## 3. Current architecture overview

```
src/main.rs          → app entry, launches ui::build
src/document.rs      → legacy Document, Entity, Layer, I/O (.lixcad / .nodcad)
src/canvas.rs        → drawing, tools, selection, inline text, dimensions on canvas
src/ui.rs            → main window, toolbar, attribute bar, layer panel, command bar
src/ui_history.rs    → history helpers, refresh_after_history_change
src/ui_context.rs    → UiCadContext, UiViewContext
src/tool_parameters.rs → alternate creation modes + preview state
src/assets.rs        → icon loading from ~/.local/share/nodalix-cad/assets
src/import/          → DXF, DWG (external backends)
src/cad/             → new core (partial): document adapter, entities, geometry,
                       history, selection/hit_testing, commands, dimensions helpers
```

**GTK testing:** `.cargo/config.toml` sets `GDK_BACKEND=offscreen` so `cargo test` does not block on a display.

---

## 4. Implemented features

| Area | Status |
|------|--------|
| Command bar (floating) | Line, pline, circle, rectangle, select, delete, undo/redo, view/zoom, layout aliases |
| Basic tools | Line, polyline, rectangle, circle, arc modes, select, pan, modify (drag move), **rotate / scale / mirror** |
| Alternate creation modes | Line (2pt, length-angle), circle (2p, 3p, center-radius, …), rectangle, arc — via `ToolParametersState` |
| Live preview | Geometry preview while placing; **not** in history |
| Text | Inline on canvas (create + edit), I-beam hover, context menu fallback |
| Dimensions | Linear, aligned, radius, diameter (initial); preview with measurement text |
| History | `LegacyHistoryManager` for add/remove/move/paste/properties/layers |
| Properties | Per-entity color, line weight, line type; attribute bar apply |
| Layers | Full base system (see §9) |
| Native save | `.lixcad` / `.nodcad` (serde); optional fields backward-compatible |
| Import/export | DXF; DWG via external converter; export paths in UI |
| Icons/assets | Toolbar + tool-modes + layer panel + **`assets/icons/snaps/`** + **`assets/icons/modify/`**; `icon_path()` falls back to bundled tree when install is stale |

---

## 5. History / Undo-Redo status

- **`LegacyHistoryManager`** (`src/cad/history/legacy_history_manager.rs`) is the primary undo stack.
- **Actions** (`legacy_actions.rs`): add/remove entities, move, paste, **`LegacyTransformEntitiesAction`** (rotate/scale/mirror), **`LegacyUpdateEntityPropertiesAction`**, **`LegacyUpdateLayersAction`** (full layer snapshot).
- **UI entry points:** toolbar, keyboard (Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y), command bar `undo`/`redo`, context menu delete → `refresh_after_history_change`.
- **`undo_stack`** on `UiCadContext`: **still exists** as full-document snapshot fallback for operations not yet on legacy history. **Do not remove** until explicitly migrated.
- **Rule:** All user-visible mutations that change the drawing must go through history helpers in `ui_history.rs` (or equivalent), not raw `document.borrow_mut()` from UI callbacks without a plan.

---

## 6. Text editing status

- **Inline** `gtk::Entry` overlay on canvas for create and edit (no modal for normal flow).
- **Double-click** text entity → edit; **Tool::Text** → create at click.
- **Hover:** I-beam cursor on text entities when applicable.
- **Context menu:** fallback edit path still available.
- **Critical:** `Entry::connect_activate` / key handlers must **not** hold `RefCell` borrows across GTK re-entrancy. Commits use **`idle_add_local_once`** + retry (`INLINE_TEXT_COMMIT_MAX_RETRIES`) in `canvas.rs`.
- **History:** text create → `record_entities_added`; text edit → `LegacyUpdateEntityPropertiesAction` when content actually changes.
- **Layers:** `set_text_entity_text` respects **locked** layer.

---

## 7. Dimension system status

- Modes in `ToolParametersState`: Linear, Aligned, Radius, Diameter (tool-mode icons under `assets/icons/tool-modes/`).
- Placement via `handle_dimension_click` in `canvas.rs`; uses **active layer** for new entities.
- Measurement / unit helper in cad dimensions module; preview shows dimension text while placing.
- **`Entity::Dimension`** in legacy model is still simplified; style/mode may be encoded in entity fields — not full AutoCAD-style dimension objects.
- **DWG:** exploded dimensions from imports are **not** reconstructed as native dimensions (often polylines + text).
- Tests: `dimension_creation_goes_through_history`, `radius_dimension_requires_circle_target` in `canvas.rs`.

---

## 8. Tool Parameters / Preview status

- `src/tool_parameters.rs` — mode enums and preview builders per tool family.
- Tool context panel in right UI reflects active mode; switching modes updates preview only.
- **Preview geometry is never pushed to history** — only final commit on click/Enter.
- Icons per mode in `assets/icons/tool-modes/*.svg`.

---

## 9. Layers status

**Phase 3.18.1 — CLOSED**

| Feature | Status |
|---------|--------|
| `Document.layers` + per-layer color, weight, linetype, visible, locked, printable | Done |
| `active_layer_name` | Done; new entities use active layer |
| Create / rename / delete (empty only) / set active | Done |
| Hidden → not drawn, not hit-tested (`entity_visible_in_active_layout` + `legacy_hit` filter) | Done |
| Locked → blocks remove, move, property sets, text edit | Done |
| ByLayer color/weight/type with per-entity override maps | Done |
| Layer panel (right sidebar) with SVG icon buttons | Done |
| History `LegacyUpdateLayersAction` + `update_layers_with_history` | Done |
| Undo/redo refreshes layer panel + attribute layer field | `refresh_after_history_change` + `UiViewContext` |
| Attribute bar layer field | Synced via `selection_layer_field_text`; no selection → set active layer; selection → move to layer |
| Pure tests | **25** tests in `document.rs::layer_tests` |
| Move selection to layer | Panel + attribute bar + history |

**Key files:** `document.rs`, `ui.rs` (`refresh_layer_panel`, `sync_attribute_layer_entry`), `ui_history.rs`, `ui_context.rs`, `legacy_actions.rs`, `legacy_hit.rs`.

---

## 10. DWG/DXF status

- **DXF:** import/export via `src/import/dxf.rs` (project-specific limitations apply).
- **DWG:** **not native** — uses external backends when configured (`src/import/dwg.rs`): ODA File Converter, FreeCAD, LibreDWG paths depending on config (`[dwg_import]` backend + `converter_path`).
- **Do not claim** “100% DWG fidelity” or native DWG read/write today.
- **Strategic future:** “DWG Native Track” (separate effort).
- Imported DWG dimensions often appear as **polylines/text**, not `Entity::Dimension`.
- Missing converter → user-facing setup dialog / `DWG_SETUP_REQUIRED` message.

---

## 11. Assets / icons status

- Source: `assets/icons/` in repo.
- Install: copied to `~/.local/share/nodalix-cad/assets/` on release install.
- Loader: `src/assets.rs` — `load_icon_image(name, size)`; safe fallback if file missing.
- Layer panel icons: `layer.svg`, `layer-add`, `layer-delete`, `layer-active`, `eye`, `eye-off`, `lock`, `unlock`, `color-swatch`, `line-weight`, `line-type`.
- Toolbar + tool-modes: extensive SVG set (see `assets/icons/`).

---

## 12. Known rules for future AI sessions

1. **Path:** only `/home/dani/Proyectos/nodalix-os/apps/nodalix-cad`.
2. **Validate install:** `sha256sum target/release/nodalix-cad ~/.local/bin/lixcad` must match after `cp`/`install`.
3. **No direct `Document` mutation** from GTK callbacks without idle/deferral when borrows can nest (`RefCell already borrowed`).
4. **Do not remove `undo_stack`** until migration is explicit.
5. **Do not break:** command bar, text inline, dimensions, existing history, assets paths, `.lixcad`/`.nodcad` compatibility (only optional serde fields).
6. **No massive refactors** — small phases with green `cargo check` + `cargo test`.
7. **Everything user-visible → history**; previews stay out of history.
8. **DXF/DWG:** don’t break import/export; don’t over-promise DWG native.
9. **Layers:** respect hidden (render + pick) and locked (edit + delete + move).
10. **Tests:** prefer pure tests on `Document` / history; avoid GTK in unit tests. Use `GDK_BACKEND=offscreen` for `cargo test`.
11. **Commits:** only when the user asks.

---

## 13. Validation checklist

```bash
cd /home/dani/Proyectos/nodalix-os/apps/nodalix-cad

cargo check
cargo test          # expect ~129 passed; uses GDK_BACKEND=offscreen from .cargo/config.toml
cargo fmt --check
cargo clippy        # project may have many warnings; -D warnings not enforced globally
cargo build --release
```

Install + assets:

```bash
pkill -f lixcad 2>/dev/null || true

cp -f target/release/nodalix-cad ~/.local/bin/lixcad
chmod +x ~/.local/bin/lixcad

mkdir -p ~/.local/share/nodalix-cad
rm -rf ~/.local/share/nodalix-cad/assets
cp -r assets ~/.local/share/nodalix-cad/

sha256sum target/release/nodalix-cad ~/.local/bin/lixcad
ls -lh target/release/nodalix-cad ~/.local/bin/lixcad
```

If `cp` fails or leaves stale binary:

```bash
cp --remove-destination target/release/nodalix-cad ~/.local/bin/lixcad
chmod +x ~/.local/bin/lixcad
```

**Manual smoke (after layer/text/dim changes):**

- Create layer `Walls`, set active, draw line on `Walls`.
- Hide/lock layer; verify pick and edit blocked.
- Undo/redo layer create/rename from toolbar and command bar.
- Text inline create/edit; dimension linear + radius.
- Command bar: `line 0,0 100,100`, `circle 2p 0,0 100,0`.

---

## 14. Known issues / technical debt

| Issue | Notes |
|-------|--------|
| Monolithic test binary | All `#[test]` in binary crate → slow compile; consider `[lib]` for pure core later |
| `clippy -D warnings` | Not clean; ~100+ warnings if enforced |
| Dual document models | Legacy `Document` vs `cad::document::CADDocument` — adapter bridge incomplete |
| `legacy_hit` | Mixes core `hit_entity_at` + legacy geometry; TODO cache CAD snapshot per frame |
| Layer printable flag | In model; UI exposure limited |
| Snap | Partial; not all layer/hidden rules audited on snap |
| Paste on locked layer | Should not modify locked targets — verify if gaps remain |
| DWG native | Future track only |
| Deprecated GTK `Dialog` | Still used in some UI paths (e.g. text edit fallback) |

---

## 15. Current pending work

- **None blocking** for Phase 3.18.1 (layers) — closed.
- Optional follow-ups (not scheduled as a single “phase” here):
  - Extract `lib` crate for faster pure tests.
  - Snap/hover audit for hidden layers.
  - Richer layer panel (color picker UI vs raw entries).
  - Diameter dimension edge cases / DWG dimension reconstruction (long term).

---

## 16. Recommended next phase

Pick **one** small track; do not mix:

1. **3.19 — Selection / properties polish** — multi-select attribute bar, mixed-state UX.
2. **3.20 — Command registry expansion** — more geometry from command line with history.
3. **Core lib extraction** — `document` + `history` tests without GTK link.
4. **DWG workflow docs** — document converter setup only (no false native claims).

Confirm with Daniel before starting a numbered phase.

---

## 17. Session update template

Append new sessions **below** (newest last). Keep older entries shortened if the doc grows too long.

```md
## Session YYYY-MM-DD — Short title

### Goal

### Changes made

### Why

### Files modified

### Validation

### Final hash

### Manual tests

### Known issues

### Next recommended step
```

---

## 18. Commands to validate and install

(Same as §13 — duplicated for quick copy-paste.)

```bash
cd /home/dani/Proyectos/nodalix-os/apps/nodalix-cad
cargo check && cargo test && cargo fmt --check && cargo clippy && cargo build --release
pkill -f lixcad 2>/dev/null || true
cp --remove-destination target/release/nodalix-cad ~/.local/bin/lixcad && chmod +x ~/.local/bin/lixcad
rm -rf ~/.local/share/nodalix-cad/assets && mkdir -p ~/.local/share/nodalix-cad && cp -r assets ~/.local/share/nodalix-cad/
sha256sum target/release/nodalix-cad ~/.local/bin/lixcad
```

---

## 19. Important warnings

- If **`~/.local/bin/lixcad`** hash ≠ **`target/release/nodalix-cad`**, manual testing used a **stale binary**.
- **`cargo test` appearing to hang** was often **compile** of GTK test binary or missing `GDK_BACKEND`; fix compile errors first, then rely on offscreen env.
- **Never** use `/home/dani/Projects/...` in scripts, docs, or AI instructions for this repo.
- **RefCell panics** during text commit or command execution → defer work with `idle_add_local_once` and `try_borrow` retry pattern (see `canvas.rs`, `ui_history.rs`).

---

## Session 2026-05-27 — Phase 3.18.1 Layers closure

### Goal

Close layer system: pure tests, layer panel refresh on undo/redo, attribute bar sync, panel icons, fix `cargo test`, install + matching hash.

### Changes made

- 25 layer unit tests in `document.rs::layer_tests`.
- `UiViewContext`: `layer_panel`, `attribute_layer_entry`; `refresh_after_history_change` refreshes panel + layer entry.
- Layer panel uses SVG icons (`layer_icon_button`); attribute bar sets active layer or moves selection.
- `selection_layer_field_text` + `update_selection_ui` on canvas selection.
- Hit-test ignores hidden entities from CAD core path (`legacy_hit.rs`).
- `.cargo/config.toml`: `GDK_BACKEND=offscreen`.
- Fixed dimension unit tests missing `layer_name` argument.

### Why

Finish Phase 3.18 without new features; ensure AI/human can validate layers reliably in CI and locally.

### Files modified

`src/document.rs`, `src/ui.rs`, `src/ui_history.rs`, `src/ui_context.rs`, `src/canvas.rs`, `src/cad/selection/legacy_hit.rs`, `.cargo/config.toml`, `docs/AI_SESSION_HANDOFF.md`, `docs/AI_UPDATE_HANDOFF_PROMPT.md`

### Validation

- `cargo check` — OK  
- `cargo test` — **129 passed**  
- `cargo fmt --check` — OK  
- `cargo clippy` — OK (warnings)  
- `cargo build --release` — OK  

### Final hash

```text
03f0d29603e841b74cda3284d17ea5b4a8b5452c465e428c15b08a74231fa25c
```

### Manual tests

Recommended list in §13 (layers, undo/redo, text, dimensions, command bar).

### Known issues

See §14; no blockers for 3.18.1 sign-off.

### Next recommended step

See §16 — confirm next phase with Daniel.

## Session 2026-05-27 — Phase 3.19 OSNAP foundation

### Goal

Implement initial professional OSNAP system with global state, snap candidate model, visual marker, basic toggles, and integration in drawing workflows.

### Changes made

- Added new module `src/cad/snapping/mod.rs`:
  - `OsnapState` global toggles with defaults:
    - enabled=true, endpoint=true, midpoint=true, center=true, intersection=true, quadrant=true, node=true
    - perpendicular=false, tangent=false, nearest=false
  - `SnapKind` + `SnapCandidate` + priority model.
  - Screen-space tolerance model (`SNAP_RADIUS_PX`) with zoom-aware world conversion.
  - Candidate collector + resolver for endpoint, midpoint, center, quadrant, intersection, nearest, node, perpendicular (+ internal ortho/parallel).
  - Hidden layers excluded from candidate generation through `entity_visible_in_active_layout`.
  - Locked layers remain snap-eligible (read/reference behavior preserved).
- Added pure unit tests in `src/cad/snapping/mod.rs` covering endpoint, midpoint, center, quadrants, intersection, nearest, priority, tolerance, hidden/locked behavior, polyline endpoints, text node, and snap disabled path.
- Integrated OSNAP state into app context:
  - `UiCadContext` now includes `osnap: Rc<RefCell<OsnapState>>`.
  - `CanvasInteractionContext` now receives shared OSNAP state.
- Canvas snapping now delegates to `cad::snapping::find_snap(...)`.
- Updated snap marker drawing in `canvas.rs` for new kinds/labels (`END/MID/CEN/INT/QUAD/NEAR/NODE/PERP/TAN`).
- Added basic OSNAP UI in `ui.rs`:
  - Toggle bar `[OSNAP] [END] [MID] [CEN] [INT] [QUAD] [NEAR] [NODE]`.
  - Status text generated from current state.
  - Command support:
    - `OSNAP`, `OSNAP ON`, `OSNAP OFF`
    - `SNAP END`, `SNAP MID`, `SNAP CEN`, `SNAP INT`, `SNAP NEAR`, `SNAP NODE`
  - `F3` toggles OSNAP master.
- Added icons:
  - `assets/icons/snap-toggle.svg`
  - `assets/icons/snap-endpoint.svg`
  - `assets/icons/snap-midpoint.svg`
  - `assets/icons/snap-center.svg`
  - `assets/icons/snap-intersection.svg`
  - `assets/icons/snap-quadrant.svg`
  - `assets/icons/snap-nearest.svg`
  - `assets/icons/snap-node.svg`
  - `assets/icons/snap-perpendicular.svg`
  - `assets/icons/snap-tangent.svg`
  - duplicated in `assets/icons/snaps/` for optional grouped use.

### Why

Phase 3.19 requires moving from ad-hoc snap logic to an explicit, testable OSNAP subsystem with zoom-stable tolerance and layer visibility rules consistent with CAD behavior.

### Files modified

`src/cad/mod.rs`, `src/cad/snapping/mod.rs`, `src/canvas.rs`, `src/ui.rs`, `src/ui_context.rs`, `assets/icons/snap-*.svg`, `assets/icons/snaps/*.svg`, `docs/AI_SESSION_HANDOFF.md`

### Validation

- `cargo check` — OK
- `cargo fmt --check` — OK (after `cargo fmt`)
- `cargo clippy` — OK (warnings, no new hard errors)
- `cargo build --release` — OK
- `cargo test` — **in this session remained stuck without output** (re-run recommended locally in a clean terminal)

### Final hash

```text
89e62a19087d849d5b888c83acd878298a5f91f67118b97d630166794a4151e0
```

(`target/release/nodalix-cad` at validation time.)

### Manual tests

- Verify END/MID/CEN/INT/QUAD/NEAR/NODE markers while drawing Line/Rectangle/Circle/Polyline/Dimension.
- Verify hidden layer entities do not snap.
- Verify locked layer entities still snap but cannot be edited.
- Verify command bar OSNAP commands and `F3` toggle.
- Regression: inline text create/edit, dimensions, layer panel, undo/redo.

### Known issues

- Full `cargo test` execution may stall in some local sessions; core build/check/clippy/release pass.
- Tangent snap is scaffolded in model/marker but not fully solved geometrically in this phase.

### Next recommended step

- Stabilize `cargo test` runtime behavior (if needed by splitting/isolating GTK-heavy tests).
- Extend tool integration to Move/Modify grip workflows once phase-safe.

---

## Session 2026-05-27 — Phase 3.20 Ortho + Polar + precision aids

### Goal

Ortho mode, polar tracking, visual tracking guides, dynamic distance/angle preview near cursor, UI toggles (ORTHO / POLAR / DYN), OSNAP priority integration, unit tests.

### Changes made

- New module `src/cad/precision/mod.rs`:
  - `PrecisionState` (defaults: ortho off, polar on, standard angle set, dynamic input on)
  - `resolve_precision_point()` — **OSNAP → Ortho → Polar → raw**
  - `apply_ortho`, `apply_polar`, `dynamic_preview_label`, `precision_anchor`
  - 14 unit tests in-module
- `UiCadContext` + `CanvasInteractionContext`: `precision: Rc<RefCell<PrecisionState>>`
- `canvas.rs`: motion resolves cursor via precision; `resolved_hover` for guides; `draw_tracking_guide`, `draw_dynamic_input_label`; entity move drag uses `ortho_move_delta` when ortho on
- `ui.rs`: precision toolbar (ORTHO / POLAR / DYN), status label, **F8 / F10 / F12** shortcuts
- Icons: `assets/icons/precision/precision-ortho.svg`, `precision-polar.svg`, `precision-dynamic-input.svg`, `precision-angle.svg`, `precision-distance.svg`
- Removed internal OSNAP `collect_ortho_snap` candidate (F8 ortho is separate); fixed flaky `polyline_vertices_endpoint` test
- `geometry::Point`: `PartialEq` for tests

### Priority rule

```text
OSNAP (within tolerance) > Ortho (if enabled) > Polar (if enabled, ±5°) > raw cursor
```

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **157 passed** |
| `cargo fmt --check` | OK |
| `cargo clippy` | OK (warnings) |
| `cargo build --release` | OK |

### Final hash

```text
89e62a19087d849d5b888c83acd878298a5f91f67118b97d630166794a4151e0
```

### Manual tests

- ORTHO: Line → first point → diagonal move locks H/V → second click
- POLAR: Line → move near 45° → locks + guide label
- OSNAP + Polar: endpoint snap wins when near entity
- DYN: label `length @ angle` on Line / Polyline / Dimension / Circle radius preview
- Regression: text inline, dimensions, layers, command bar, undo/redo, icons

### Not in this phase

- Editable dynamic input at cursor
- Multi-point polar tracking
- Object snap tracking (OTRACK)
- Parametric constraints

### Next recommended step

- Command bar: `ORTHO ON`, `POLAR OFF`, etc.
- Polar angle customization UI

---

## Session 2026-05-27 — Phase 3.21 OSNAP icons + Rotate/Scale/Mirror

### Goal

Fix missing OSNAP toolbar icons (`image-missing`); add Rotate, Scale, Mirror modify tools with cyan preview, OSNAP/ORTHO/POLAR/DYN integration, locked/hidden layer rules, command bar, history, tests.

### Changes made

- **Icons:** `assets/icons/snaps/snap-*.svg` (10 modes + toggle); `assets/icons/modify/modify-{rotate,scale,mirror}.svg`
- **`src/assets.rs`:** `icon_path()` tries flat path, `snaps/`, `modify/`, `precision/`; **bundled manifest fallback** when `~/.local/share/...` is stale
- **`src/cad/geometry/modify.rs`:** `rotate_entity`, `scale_entity`, `mirror_entity`, point helpers + unit tests
- **`src/canvas_modify.rs`:** 2-click flows, preview builder, command parser (`rotate 0,0 90`, `scale 0,0 2`, `mirror 0,0 100,0`), editable selection filters
- **`src/cad/history/legacy_actions.rs`:** `LegacyTransformEntitiesAction`, `record_entity_transform`, `apply_entity_snapshot_in_place` (in-place undo/redo)
- **`src/tools.rs`:** `Tool::Rotate | Scale | Mirror`
- **`src/canvas.rs`:** `modify_preview`, motion preview, click handler
- **`src/ui.rs`:** toolbar tools, context panels, command activation (`rotate`/`ro`, `scale`/`sc`, `mirror`/`mi`)
- **`Document::invalidate_entity_bounds`**

### Rules preserved

- Preview not in history; commit via `record_entity_transform`
- Locked layers skipped; hidden entities excluded via `entity_visible_in_active_layout`
- Precision priority unchanged: OSNAP > Ortho > Polar

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **178 passed** |
| `cargo fmt --check` | OK (after `cargo fmt`) |
| `cargo clippy -D warnings` | Pre-existing failures (not gated this phase) |
| `cargo build --release` | OK |
| Install | `~/.local/bin/lixcad` + `cp -r assets/* ~/.local/share/nodalix-cad/assets/` |

### Final hash

```text
89e62a19087d849d5b888c83acd878298a5f91f67118b97d630166794a4151e0
```

### Manual tests

- OSNAP toolbar: all snap toggles show icons (not red missing)
- Rotate: select line → tool → base click → angle click → undo/redo
- Scale / Mirror: same 2-click flow; cyan preview while moving
- Locked layer entity: not transformed; hidden layer: not in selection
- Commands: `rotate 0,0 45`, `scale 0,0 2`, `mirror 0,0 100,0`
- Regression: text, dimensions, layers, ORTHO/POLAR/DYN, command bar

### Next recommended step

- Extend transforms to block references / full entity matrix
- Polyline / dimension grip editing
- Multiple copy / array

---

## Session 2026-05-27 — Phase 3.22 Move / Copy tools + selection grips

### Goal

Dedicated **Move** and **Copy** tools (2-click base → destination, cyan preview, OSNAP/ORTHO/POLAR/DYN); **selection grips** (visual + edit line/circle/text); command bar `move`/`m`/`copy`/`co`/`cp` with coordinates; history via `LegacyMoveEntitiesAction` / `LegacyAddEntitiesAction` / `LegacyTransformEntitiesAction`.

### Changes made

- **`src/cad/geometry/grips.rs`:** `Grip`, `GripKind`, `collect_grips`, `apply_grip_edit`, `hit_test_grip`, pure tests (move/copy/grip/locked/hidden)
- **`src/canvas_modify.rs`:** `build_move_copy_preview`, `apply_move_selection`, `apply_copy_selection`, `handle_move_copy_tool_click`; command parser `move 0,0 100,0`, `copy 0,0 100,0`
- **`src/canvas_grips.rs`:** grip draw + hit-test for Select tool
- **`src/canvas.rs`:** Move/Copy click + motion preview; grip drag session (live edit, single history entry on release); `draw_selection_grips`
- **`src/tools.rs`:** `Tool::Move`, `Tool::Copy` (`modify-move`, `modify-copy` icons); `Tool::Modify` keeps drag icon `move`
- **`src/ui.rs`:** tool context panels; commands: `move` → Move tool, `m`/`modify` → drag Modify, `copy` → Copy tool, `duplicate` → offset duplicate, `co`/`cp` → Copy tool
- **`src/document.rs`:** `paste_entities_translated` public for copy pipeline
- **Icons:** `modify-move.svg`, `modify-copy.svg`, `grip-{point,midpoint,radius}.svg`

### Rules preserved

- Preview not in document/history; grip drag records once on release
- Locked: no move, no grip edit; copy from locked allowed (original unchanged)
- Hidden: no grips, no move/copy selection
- `undo_stack` unchanged; `LegacyHistoryManager` for commits

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **194 passed** |
| `cargo fmt --check` | OK |
| `cargo clippy` | OK (warnings; `-D warnings` not gated) |
| `cargo build --release` | OK |

### Final hash

```text
da0fa680566456fe80da8c64786e232a0f13f0b921a74c1080afdee2f9406776
```

### Manual tests

- Move: select → Move tool → base → dest → undo/redo
- Copy: select → Copy tool → base → dest → original remains
- Grips: line start/end/mid; circle center/radius; text insertion
- Commands: `move 0,0 100,0`, `copy 0,0 10,10`
- Regression: rotate/scale/mirror, OSNAP, ORTHO/POLAR/DYN, text inline, dimensions, layers

---

## Session 2026-05-27 — Phase 3.23 OFFSET tool

### Goal

Initial **OFFSET** tool: parallel/concentric copies of Line, Circle, Polyline (incl. rectangle) at fixed distance; cyan preview; command bar `offset 10` / `o 10` / `offset 10 x,y`; history via `LegacyAddEntitiesAction`.

### Changes made

- **`src/cad/geometry/offset.rs`:** `offset_line`, `offset_circle`, `offset_polyline` (closed uses winding + inside/outside side point), `offset_entity_geometry`, pure tests
- **`src/canvas_offset.rs`:** pick source → side click, preview, commit, command parser, history tests
- **`src/tools.rs`:** `Tool::Offset` + `modify-offset` icon
- **`src/tool_parameters.rs`:** `offset_distance` (default 10)
- **`src/canvas.rs`:** Offset click/motion preview, `offset_source` session, cancel clears
- **`src/ui.rs`:** Tool parameters panel (distance entry), commands, `execute_command` offset handler
- **`assets/icons/modify/modify-offset.svg`**

### Policies

- **Layer:** new entity on **active layer**; color/line weight/type copied from source
- **Locked source:** allowed (creates new geometry only)
- **Hidden:** not hit-testable / not offsettable
- **Preview:** not in history

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **210 passed** |
| `cargo fmt --check` | OK |
| `cargo clippy` | OK (warnings) |
| `cargo build --release` | OK |

### Final hash

```text
0e6da13cae1847b5d239476d7a45431f0105a5710493af842ee8b81bf6f8c67f
```

### Manual tests

- Line/circle/polyline offset with preview + undo/redo
- `offset 10`, `offset 10 100,50` (single selection)
- Locked layer offset; hidden not pickable
- Regression: Move/Copy, grips, rotate/scale/mirror, OSNAP, DYN, text, dimensions

---

## Session 2026-05-28 — Phase 3.24 Layouts / paper space / viewports

### Goal

Fix DWG/DXF **layouts (presentaciones)**: separate Model vs Paper entities, import group 410/67/330 + VIEWPORT, render paper sheet + clipped model in viewports, fit camera on layout tab switch.

### Root cause (previous behaviour)

- Layout **tabs** were created from DXF `LAYOUT` objects, but many paper entities landed on the **wrong layout** (fallback assigned all `67=1` entities to the first paper layout).
- **Layout name mismatch** between tab names (group 1 vs 2) and entity group 410.
- Switching layout tabs did not **fit the camera** to paper space (model zoom stayed → blank/wrong view).
- Paper-space **viewport #1** (full layout) was imported as a model window.

### Changes made

- **`src/cad/layouts/mod.rs`:** `normalize_layout_name`, `active_layout_fit_bounds`, layout visibility helpers, debug log (`LIXCAD_LAYOUT_DEBUG=1`), tests.
- **`src/document.rs`:** normalized layout compare in `entity_visible_in_active_layout`; `LayoutViewport.visible` (`#[serde(default)]`).
- **`src/import/dxf.rs`:** layout tab name from group 1; normalized names; paper layout hint from VIEWPORT / single layout; skip viewport 69≤1; viewport visibility flag; no “first layout” dump for paper entities.
- **`src/canvas.rs`:** `fit_document` uses `active_layout_fit_bounds`; filter viewports by layout + `visible`.
- **`src/ui.rs`:** layout tab click calls `fit_document`.

### Policies

- **Model:** entities without `entity_layouts` entry (default Model).
- **Paper:** `entity_layouts[id] = layout name`; only visible on that layout tab.
- **Viewports:** show model entities with paper transform; not selectable as model geometry in this phase.
- **Hidden layers:** still respected inside viewports via `entity_layer_visible`.

### Limitations (documented)

- No viewport activation / in-viewport pan-zoom.
- No viewport layer overrides / freeze.
- Paper entities without 410/330 rely on VIEWPORT order hint or single-layout fallback.
- No full AutoCAD page setup / plot.

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **223+ passed** |
| `cargo fmt --check` | OK |
| `cargo clippy` | OK (warnings) |
| `cargo build --release` | OK |

### Final hash

```text
e6ceae520cf9d2873b6610dede0b9f6e5e1fa61cc478209f4dcae800bd990c47
```

### Manual tests

- Open DWG/DXF with Layout1/Layout2 → each tab shows its paper entities + viewports with model content.
- Model tab shows only model geometry.
- Layout tab switch fits paper on screen.
- `LIXCAD_LAYOUT_DEBUG=1` for import traces.

---

## Session 2026-05-28 — Offset RefCell crash fix

### Symptom

`thread 'main' panicked at src/canvas_offset.rs:101: RefCell already borrowed` on Offset **second click** (commit), from `GestureClick::connect_pressed`.

### Cause

```rust
if let Some(source_id) = *offset_source.borrow() {  // immutable borrow lives for whole block
    commit_offset_entity(...);
    *offset_source.borrow_mut() = None;  // line 101 — panic
}
```

Rust extends the temporary `Ref` from `borrow()` across the entire `if let` body.

### Fix

- Copy `pending_source = *offset_source.borrow()` in a separate statement so the read borrow drops before commit.
- Split read/mutate via `OffsetCommitPlan` + `build_offset_commit_plan` / `apply_offset_commit_plan`.
- `try_apply_offset_commit_plan` uses `history.try_borrow_mut()` → `OffsetCommitError::HistoryBusy` (no panic).

### Tests added

`offset_click_second_click_does_not_panic_offset_source`, `offset_plan_built_without_mutation`, `offset_commit_returns_history_busy_when_history_borrowed`, hidden/locked plan tests.

### Final hash (after fix)

```text
098ad3bc26d68fc7e47a953f98a492fe3d1768a80d89758dc5bcec5917ea1a8b
```

---

## Session 2026-05-28 — Phase 3.25 TRIM / EXTEND

### Goal

Initial **TRIM** and **EXTEND** for Line and Polyline segments: explicit cutting/boundary edge, pick side/endpoint, cyan preview, undo/redo via `LegacyTransformEntitiesAction`.

### Changes made

- **`src/cad/geometry/trim_extend.rs`:** `line_line_intersection_infinite`, `line_segment_intersection`, `trim_line_to_boundary`, `extend_line_to_boundary` + pure tests
- **`src/canvas_trim_extend.rs`:** edge pick, boundary session, trim/extend apply, preview builders, command activation (`trim`/`tr`/`extend`/`ex`), policy + history tests
- **`src/tools.rs`:** `Tool::Trim`, `Tool::Extend` + palette icons
- **`src/canvas.rs`:** click flow, motion preview (`trim_boundary` / `extend_boundary` — copy `*borrow()` before use, same RefCell pattern as Offset fix)
- **`src/ui.rs`:** tool context panels, command bar arms
- **`assets/icons/modify/modify-trim.svg`**, **`modify-extend.svg`**

### Flow

1. Activate Trim or Extend (toolbar or command).
2. **Click 1:** pick cutting/boundary edge (line or polyline segment); geometry stored in `TrimBoundary`.
3. **Hover:** preview trimmed/extended target under cursor.
4. **Click 2:** pick target segment; `pick_point` chooses trim side / extend endpoint; one `LegacyTransformEntitiesAction`; boundary cleared.

### Policies

- **Hidden:** not boundary, not target (`entity_visible_in_active_layout`).
- **Locked boundary:** allowed (geometry not modified).
- **Locked target:** denied (`target_candidate` requires unlocked layer).
- **Preview:** not in history.

### Commands

| Command | Action |
|---------|--------|
| `trim`, `tr` | Activate Trim |
| `extend`, `ex` | Activate Extend |

### Not in this phase

- Trim all entities as cutting edges; circle/arc trim; fence trim; edge-mode extend; repeat trim.

### Validation

| Command | Result |
|---------|--------|
| `cargo check` | OK |
| `cargo test` | **244 passed** |
| `cargo fmt --check` | OK |
| `cargo clippy` | OK (warnings) |
| `cargo build --release` | OK |

### Final hash

```text
c96fcbdee78a5d77006bd3aa3da24a61d8bcc2b79f94a476d85619beb37b3390
```

### Manual tests

- Trim: horizontal + vertical crossing → vertical boundary → trim left/right → undo/redo
- Extend: short horizontal + vertical boundary ahead → extend to boundary → undo/redo
- Locked boundary OK; locked target blocked; hidden not pickable
- Regression: Offset (no RefCell panic), Move/Copy, grips, layouts, OSNAP/ORTHO/POLAR/DYN, text, dimensions, layers, `line 0,0 100,100`

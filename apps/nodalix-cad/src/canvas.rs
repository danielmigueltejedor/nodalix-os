use crate::{
    cad::dimensions::{
        dimension_offset_from_point, dimension_segments, format_distance_with_unit,
        parse_dimension_style,
    },
    cad::geometry::{CircleCreationMode, LineCreationMode, RectangleCreationMode},
    cad::history::{record_entities_added, record_entity_move_if_nonzero, LegacyHistoryManager},
    document::{Document, Entity, LayoutKind, LayoutViewport},
    geometry::{Point, Point3},
    tool_parameters::{
        finalize_arc_entity, finalize_circle_entity, finalize_rectangle_entity,
        preview_arc_three_points, preview_circle_center_diameter, preview_circle_center_radius,
        preview_circle_three_points, preview_circle_two_point_diameter, preview_line_two_points,
        preview_rectangle_center_size, preview_rectangle_corner_size,
        preview_rectangle_two_corners, ToolParametersState, ToolPreview,
    },
    tools::Tool,
};
use gtk::{gdk, prelude::*, DrawingArea};
use std::{cell::RefCell, rc::Rc};

const MIN_ZOOM: f64 = 0.000_05;
const MAX_ZOOM: f64 = 20_000.0;
const ZOOM_STEP: f64 = 1.25;
const SNAP_SCREEN_TOLERANCE: f64 = 13.0;
const FULL_SNAP_ENTITY_LIMIT: usize = 2_500;
const LARGE_DOCUMENT_MOTION_REDRAW_ENTITY_LIMIT: usize = 5_000;
const FIT_VIEW_MARGIN: f64 = 0.84;
const MIN_SCREEN_VERTEX_DISTANCE: f64 = 0.7;
const MIN_TEXT_SCREEN_HEIGHT: f64 = 4.0;

/// Shared document/tool/selection handles wired into canvas event handlers.
#[derive(Clone)]
pub struct CanvasInteractionContext {
    pub document: Rc<RefCell<Document>>,
    pub active_tool: Rc<RefCell<Tool>>,
    pub selected_entity: Rc<RefCell<Vec<u64>>>,
    pub history: Rc<RefCell<LegacyHistoryManager>>,
    pub tool_parameters: Rc<RefCell<ToolParametersState>>,
    pub inline_text_entry: gtk::Entry,
}

#[derive(Clone, Debug)]
struct EntityMoveDragSession {
    entity_ids: Vec<u64>,
    accumulated_dx: f64,
    accumulated_dy: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub rotation: f64,
}

#[derive(Clone, Copy, Debug)]
struct SnapTarget {
    point: Point,
    kind: SnapKind,
}

#[derive(Clone, Copy, Debug)]
enum SnapKind {
    Endpoint,
    Midpoint,
    Center,
    Quadrant,
    CircleEdge,
    Intersection,
    Perpendicular,
    Ortho,
    Parallel,
}

impl SnapKind {
    fn label(self) -> &'static str {
        match self {
            SnapKind::Endpoint => "endpoint",
            SnapKind::Midpoint => "midpoint",
            SnapKind::Center => "center",
            SnapKind::Quadrant => "quadrant",
            SnapKind::CircleEdge => "circle",
            SnapKind::Intersection => "intersection",
            SnapKind::Perpendicular => "perpendicular",
            SnapKind::Ortho => "ortho",
            SnapKind::Parallel => "parallel",
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            rotation: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct CadCanvas {
    area: DrawingArea,
    cursor: Rc<RefCell<Point>>,
    camera: Rc<RefCell<Camera>>,
    pending_start: Rc<RefCell<Option<Point>>>,
    pending_points: Rc<RefCell<Vec<Point>>>,
    polyline_vertices: Rc<RefCell<Vec<Point>>>,
    hover_point: Rc<RefCell<Option<Point>>>,
    hovered_entity: Rc<RefCell<Option<u64>>>,
    text_entry: gtk::Entry,
    inline_text_mode: Rc<RefCell<Option<InlineTextMode>>>,
}

#[derive(Clone, Debug)]
enum InlineTextMode {
    Creating {
        world_position: Point,
    },
    Editing {
        entity_id: u64,
        original_text: String,
        world_position: Point,
    },
}

const INLINE_TEXT_COMMIT_MAX_RETRIES: u8 = 64;

#[derive(Clone, Copy, Debug)]
struct SelectionBox {
    start_x: f64,
    start_y: f64,
    current_x: f64,
    current_y: f64,
}

impl CadCanvas {
    pub fn new(interaction: CanvasInteractionContext, selection_label: gtk::Label) -> Self {
        let CanvasInteractionContext {
            document,
            active_tool,
            selected_entity,
            history,
            tool_parameters,
            inline_text_entry,
        } = interaction;
        let area = DrawingArea::new();
        area.add_css_class("cad-canvas");
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_focusable(true);
        area.set_content_width(900);
        area.set_content_height(620);

        let cursor = Rc::new(RefCell::new(Point::default()));
        let camera = Rc::new(RefCell::new(Camera::default()));
        let pointer = Rc::new(RefCell::new(None::<(f64, f64)>));
        let snap_target = Rc::new(RefCell::new(None::<SnapTarget>));
        let pending_start = Rc::new(RefCell::new(None));
        let pending_points = Rc::new(RefCell::new(Vec::<Point>::new()));
        let polyline_vertices = Rc::new(RefCell::new(Vec::<Point>::new()));
        let hover_point = Rc::new(RefCell::new(None::<Point>));
        let hovered_entity = Rc::new(RefCell::new(None::<u64>));
        let inline_text_mode = Rc::new(RefCell::new(None::<InlineTextMode>));
        let selection_box = Rc::new(RefCell::new(None::<SelectionBox>));
        let draw_document = document.clone();
        let draw_pending = pending_start.clone();
        let draw_pending_points = pending_points.clone();
        let draw_polyline = polyline_vertices.clone();
        let draw_hover = hover_point.clone();
        let draw_hovered_entity = hovered_entity.clone();
        let draw_camera = camera.clone();
        let draw_selected = selected_entity.clone();
        let draw_tool = active_tool.clone();
        let draw_tool_parameters = tool_parameters.clone();
        let draw_snap = snap_target.clone();
        let draw_selection_box = selection_box.clone();
        area.set_draw_func(move |_, cr, width, height| {
            let Some(document) = draw_document.try_borrow().ok() else {
                // Command/edit path holds `borrow_mut`; skip this frame instead of panicking.
                return;
            };
            let selected = draw_selected.borrow();
            draw_scene(
                cr,
                width as f64,
                height as f64,
                &document,
                *draw_pending.borrow(),
                &draw_pending_points.borrow(),
                &draw_polyline.borrow(),
                *draw_hover.borrow(),
                *draw_camera.borrow(),
                &selected,
                *draw_tool.borrow(),
                *draw_tool_parameters.borrow(),
                *draw_hovered_entity.borrow(),
                *draw_snap.borrow(),
                *draw_selection_box.borrow(),
            );
        });

        let click = gtk::GestureClick::new();
        click.set_button(1);
        let click_document = document.clone();
        let click_tool = active_tool.clone();
        let click_pending = pending_start.clone();
        let click_pending_points = pending_points.clone();
        let click_polyline = polyline_vertices.clone();
        let queue_area = area.clone();
        let click_camera = camera.clone();
        let click_selected = selected_entity.clone();
        let click_selection_label = selection_label.clone();
        let click_snap = snap_target.clone();
        let click_history = history.clone();
        let click_tool_parameters = tool_parameters.clone();
        let click_text_entry = inline_text_entry.clone();
        let click_inline_mode = inline_text_mode.clone();
        click.connect_pressed(move |_, n_press, x, y| {
            queue_area.grab_focus();
            let point = screen_to_world(
                x,
                y,
                queue_area.width() as f64,
                queue_area.height() as f64,
                *click_camera.borrow(),
            );
            let point = click_pending
                .borrow()
                .and_then(|_| *click_snap.borrow())
                .map(|snap| snap.point)
                .or_else(|| click_snap.borrow().map(|snap| snap.point))
                .unwrap_or(point);
            let tool = *click_tool.borrow();
            if matches!(tool, Tool::Select | Tool::Modify) {
                let hit = hit_test(
                    &click_document.borrow(),
                    point,
                    10.0 / click_camera.borrow().zoom,
                );
                *click_selected.borrow_mut() = hit.into_iter().collect();
                update_selection_label(
                    &click_selection_label,
                    &click_document.borrow(),
                    &click_selected.borrow(),
                );
                *click_pending.borrow_mut() = None;
                if tool == Tool::Select && n_press >= 2 {
                    if let Some(id) = hit {
                        if let Some((existing, origin)) =
                            text_entity_at(&click_document.borrow(), id)
                        {
                            begin_inline_text_edit(
                                &click_text_entry,
                                &click_inline_mode,
                                id,
                                existing,
                                origin,
                                &queue_area,
                                *click_camera.borrow(),
                            );
                        }
                    }
                }
            } else {
                if tool == Tool::Text {
                    begin_inline_text_create(
                        &click_text_entry,
                        &click_inline_mode,
                        point,
                        &queue_area,
                        *click_camera.borrow(),
                    );
                } else {
                    handle_click(
                        &mut click_document.borrow_mut(),
                        &click_history,
                        tool,
                        *click_tool_parameters.borrow(),
                        point,
                        &click_pending,
                        &click_pending_points,
                        &click_polyline,
                    );
                }
            }
            queue_area.queue_draw();
        });
        area.add_controller(click);

        let motion = gtk::EventControllerMotion::new();
        {
            let pointer = pointer.clone();
            let document = document.clone();
            let camera = camera.clone();
            let pending_start = pending_start.clone();
            let polyline_vertices = polyline_vertices.clone();
            let snap_target = snap_target.clone();
            let hover_point = hover_point.clone();
            let hovered_entity = hovered_entity.clone();
            let active_tool = active_tool.clone();
            let area = area.clone();
            let text_entry = inline_text_entry.clone();
            motion.connect_motion(move |_, x, y| {
                *pointer.borrow_mut() = Some((x, y));
                let camera = *camera.borrow();
                let point =
                    screen_to_world(x, y, area.width() as f64, area.height() as f64, camera);
                let entity_count = document.borrow().entities.len();
                let tool = *active_tool.borrow();
                let pending_point = pending_start
                    .borrow()
                    .as_ref()
                    .copied()
                    .or_else(|| polyline_vertices.borrow().last().copied());
                let snap = if tool == Tool::Select {
                    None
                } else {
                    find_snap(
                        &document.borrow(),
                        point,
                        pending_point,
                        SNAP_SCREEN_TOLERANCE / camera.zoom,
                    )
                };
                let next_hovered_entity = if matches!(tool, Tool::Select | Tool::Modify) {
                    hit_test(&document.borrow(), point, 10.0 / camera.zoom)
                } else {
                    None
                };
                let hovered_entity_changed = *hovered_entity.borrow() != next_hovered_entity;
                *hovered_entity.borrow_mut() = next_hovered_entity;
                let text_hover = next_hovered_entity
                    .map(|id| is_text_entity(&document.borrow(), id))
                    .unwrap_or(false);
                if tool == Tool::Select && text_hover {
                    area.set_cursor_from_name(Some("text"));
                } else if !gtk::prelude::WidgetExt::is_visible(&text_entry) {
                    area.set_cursor_from_name(None);
                }
                let effective = snap.map(|snap| snap.point).unwrap_or(point);
                *hover_point.borrow_mut() = Some(effective);
                *snap_target.borrow_mut() = snap;
                let needs_preview_redraw =
                    pending_point.is_some() || !matches!(tool, Tool::Select | Tool::Modify);
                if needs_preview_redraw
                    || snap.is_some()
                    || hovered_entity_changed
                    || entity_count <= LARGE_DOCUMENT_MOTION_REDRAW_ENTITY_LIMIT
                {
                    area.queue_draw();
                }
            });
        }
        {
            let pointer = pointer.clone();
            let snap_target = snap_target.clone();
            let hover_point = hover_point.clone();
            let hovered_entity = hovered_entity.clone();
            let area = area.clone();
            motion.connect_leave(move |_| {
                *pointer.borrow_mut() = None;
                *snap_target.borrow_mut() = None;
                *hover_point.borrow_mut() = None;
                if hovered_entity.borrow().is_some() {
                    *hovered_entity.borrow_mut() = None;
                    area.queue_draw();
                }
            });
        }
        area.add_controller(motion);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let scroll_area = area.clone();
        let scroll_camera = camera.clone();
        let scroll_pointer = pointer.clone();
        scroll.connect_scroll(move |_, _dx, dy| {
            let factor = if dy < 0.0 {
                ZOOM_STEP
            } else if dy > 0.0 {
                1.0 / ZOOM_STEP
            } else {
                1.0
            };
            if factor != 1.0 {
                let width = scroll_area.width() as f64;
                let height = scroll_area.height() as f64;
                let (anchor_x, anchor_y) = scroll_pointer
                    .borrow()
                    .unwrap_or((width / 2.0, height / 2.0));
                zoom_camera_at(
                    &mut scroll_camera.borrow_mut(),
                    factor,
                    anchor_x,
                    anchor_y,
                    width,
                    height,
                );
                scroll_area.queue_draw();
            }
            gtk::glib::Propagation::Stop
        });
        area.add_controller(scroll);

        {
            let document = document.clone();
            let history = history.clone();
            let selected = selected_entity.clone();
            let selection_label = selection_label.clone();
            let area = area.clone();
            let camera = camera.clone();
            let mode = inline_text_mode.clone();
            let entry = inline_text_entry.clone();
            entry.connect_activate(move |entry| {
                commit_inline_text(
                    entry,
                    &mode,
                    &document,
                    &history,
                    &selected,
                    &selection_label,
                    &area,
                );
            });
        }
        {
            let document = document.clone();
            let history = history.clone();
            let selected = selected_entity.clone();
            let selection_label = selection_label.clone();
            let area = area.clone();
            let camera = camera.clone();
            let mode = inline_text_mode.clone();
            let entry = inline_text_entry.clone();
            let key = gtk::EventControllerKey::new();
            key.connect_key_pressed(move |_, keyval, _, _| {
                if keyval == gdk::Key::Escape {
                    cancel_inline_text(&entry, &mode, &area);
                    return gtk::glib::Propagation::Stop;
                }
                if keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter {
                    commit_inline_text(
                        &entry,
                        &mode,
                        &document,
                        &history,
                        &selected,
                        &selection_label,
                        &area,
                    );
                    return gtk::glib::Propagation::Stop;
                }
                gtk::glib::Propagation::Proceed
            });
            inline_text_entry.add_controller(key);
        }

        let move_drag = gtk::GestureDrag::new();
        move_drag.set_button(1);
        let drag_last_world = Rc::new(RefCell::new(None::<Point>));
        let drag_pan_start = Rc::new(RefCell::new(None::<Camera>));
        let entity_move_session = Rc::new(RefCell::new(None::<EntityMoveDragSession>));
        {
            let document = document.clone();
            let active_tool = active_tool.clone();
            let selected_entity = selected_entity.clone();
            let selection_label = selection_label.clone();
            let camera = camera.clone();
            let drag_last_world = drag_last_world.clone();
            let drag_pan_start = drag_pan_start.clone();
            let entity_move_session = entity_move_session.clone();
            let selection_box = selection_box.clone();
            let area = area.clone();
            move_drag.connect_drag_begin(move |gesture, x, y| {
                *entity_move_session.borrow_mut() = None;
                *selection_box.borrow_mut() = None;
                if gesture
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    *drag_pan_start.borrow_mut() = Some(*camera.borrow());
                    *drag_last_world.borrow_mut() = None;
                    return;
                }
                *drag_pan_start.borrow_mut() = None;
                if !matches!(*active_tool.borrow(), Tool::Select | Tool::Modify) {
                    *drag_last_world.borrow_mut() = None;
                    return;
                }
                let camera_state = *camera.borrow();
                let point = screen_to_world(
                    x,
                    y,
                    area.width() as f64,
                    area.height() as f64,
                    camera_state,
                );
                let hit = hit_test(&document.borrow(), point, 10.0 / camera_state.zoom);
                if let Some(id) = hit {
                    if !selected_entity.borrow().contains(&id) {
                        *selected_entity.borrow_mut() = vec![id];
                        let selected_ids = selected_entity.borrow().clone();
                        update_selection_label(&selection_label, &document.borrow(), &selected_ids);
                    }
                    *drag_last_world.borrow_mut() = Some(point);
                    *entity_move_session.borrow_mut() = Some(EntityMoveDragSession {
                        entity_ids: selected_entity.borrow().clone(),
                        accumulated_dx: 0.0,
                        accumulated_dy: 0.0,
                    });
                    area.queue_draw();
                } else if *active_tool.borrow() == Tool::Select {
                    selected_entity.borrow_mut().clear();
                    let selected_ids = selected_entity.borrow().clone();
                    update_selection_label(&selection_label, &document.borrow(), &selected_ids);
                    *selection_box.borrow_mut() = Some(SelectionBox {
                        start_x: x,
                        start_y: y,
                        current_x: x,
                        current_y: y,
                    });
                    *drag_last_world.borrow_mut() = None;
                    area.queue_draw();
                }
            });
        }
        {
            let document = document.clone();
            let selected_entity = selected_entity.clone();
            let selection_label = selection_label.clone();
            let camera = camera.clone();
            let drag_last_world = drag_last_world.clone();
            let drag_pan_start = drag_pan_start.clone();
            let entity_move_session = entity_move_session.clone();
            let selection_box = selection_box.clone();
            let area = area.clone();
            move_drag.connect_drag_update(move |gesture, dx, dy| {
                let Some((start_x, start_y)) = gesture.start_point() else {
                    return;
                };
                if let Some(start) = *drag_pan_start.borrow() {
                    let delta = screen_delta_to_world(dx, dy, start);
                    let mut camera = camera.borrow_mut();
                    camera.pan_x = start.pan_x - delta.x;
                    camera.pan_y = start.pan_y - delta.y;
                    area.queue_draw();
                    return;
                }
                let current_selection_box = *selection_box.borrow();
                if let Some(mut box_rect) = current_selection_box {
                    box_rect.current_x = start_x + dx;
                    box_rect.current_y = start_y + dy;
                    *selection_box.borrow_mut() = Some(box_rect);
                    let selected_ids = entities_in_screen_box(
                        &document.borrow(),
                        box_rect,
                        area.width() as f64,
                        area.height() as f64,
                        *camera.borrow(),
                    );
                    *selected_entity.borrow_mut() = selected_ids.clone();
                    update_selection_label(&selection_label, &document.borrow(), &selected_ids);
                    area.queue_draw();
                    return;
                }
                let selected_ids = selected_entity.borrow().clone();
                if selected_ids.is_empty() {
                    return;
                }
                let point = screen_to_world(
                    start_x + dx,
                    start_y + dy,
                    area.width() as f64,
                    area.height() as f64,
                    *camera.borrow(),
                );
                let Some(previous) = *drag_last_world.borrow() else {
                    *drag_last_world.borrow_mut() = Some(point);
                    return;
                };
                let move_dx = point.x - previous.x;
                let move_dy = point.y - previous.y;
                for id in &selected_ids {
                    document
                        .borrow_mut()
                        .translate_entity(*id, move_dx, move_dy);
                }
                if let Some(session) = entity_move_session.borrow_mut().as_mut() {
                    session.accumulated_dx += move_dx;
                    session.accumulated_dy += move_dy;
                }
                *drag_last_world.borrow_mut() = Some(point);
                area.queue_draw();
            });
        }
        {
            let drag_last_world = drag_last_world.clone();
            let drag_pan_start = drag_pan_start.clone();
            let entity_move_session = entity_move_session.clone();
            let selection_box = selection_box.clone();
            let history = history.clone();
            let document = document.clone();
            move_drag.connect_drag_end(move |_, _, _| {
                if let Some(session) = entity_move_session.borrow_mut().take() {
                    record_entity_move_if_nonzero(
                        &mut history.borrow_mut(),
                        session.entity_ids,
                        session.accumulated_dx,
                        session.accumulated_dy,
                    );
                    if session.accumulated_dx.abs() > 1e-9 || session.accumulated_dy.abs() > 1e-9 {
                        document.borrow_mut().modified = true;
                    }
                }
                *drag_last_world.borrow_mut() = None;
                *drag_pan_start.borrow_mut() = None;
                *selection_box.borrow_mut() = None;
            });
        }
        area.add_controller(move_drag);

        let pan_start = Rc::new(RefCell::new(Camera::default()));
        let drag = gtk::GestureDrag::new();
        drag.set_button(2);
        {
            let camera = camera.clone();
            let pan_start = pan_start.clone();
            drag.connect_drag_begin(move |_, _, _| {
                *pan_start.borrow_mut() = *camera.borrow();
            });
        }
        {
            let camera = camera.clone();
            let pan_start = pan_start.clone();
            let area = area.clone();
            drag.connect_drag_update(move |_, dx, dy| {
                let start = *pan_start.borrow();
                let mut camera = camera.borrow_mut();
                let delta = screen_delta_to_world(dx, dy, start);
                camera.pan_x = start.pan_x - delta.x;
                camera.pan_y = start.pan_y - delta.y;
                area.queue_draw();
            });
        }
        area.add_controller(drag);

        Self {
            area,
            cursor,
            camera,
            pending_start,
            pending_points,
            polyline_vertices,
            hover_point,
            hovered_entity,
            text_entry: inline_text_entry,
            inline_text_mode,
        }
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    pub fn cursor(&self) -> Rc<RefCell<Point>> {
        self.cursor.clone()
    }

    pub fn camera(&self) -> Rc<RefCell<Camera>> {
        self.camera.clone()
    }

    pub fn zoom_in(&self) {
        self.zoom_at_center(ZOOM_STEP);
    }

    pub fn zoom_out(&self) {
        self.zoom_at_center(1.0 / ZOOM_STEP);
    }

    pub fn reset_view(&self) {
        *self.camera.borrow_mut() = Camera::default();
        self.area.queue_draw();
    }

    pub fn set_view_rotation(&self, radians: f64) {
        self.camera.borrow_mut().rotation = radians;
        self.area.queue_draw();
    }

    pub fn rotate_view_by(&self, radians: f64) {
        self.camera.borrow_mut().rotation += radians;
        self.area.queue_draw();
    }

    pub fn entity_at_screen(&self, document: &Document, x: f64, y: f64) -> Option<u64> {
        let camera = *self.camera.borrow();
        let point = screen_to_world(
            x,
            y,
            self.area.width() as f64,
            self.area.height() as f64,
            camera,
        );
        hit_test(document, point, 10.0 / camera.zoom)
    }

    pub fn fit_document(&self, document: &Document) {
        let Some((min, max)) = document_bounds(document) else {
            return;
        };
        let width = self.area.width().max(1) as f64;
        let height = self.area.height().max(1) as f64;
        let span_x = (max.x - min.x).abs().max(1.0);
        let span_y = (max.y - min.y).abs().max(1.0);
        let zoom = ((width * FIT_VIEW_MARGIN / span_x).min(height * FIT_VIEW_MARGIN / span_y))
            .clamp(MIN_ZOOM, MAX_ZOOM);
        let rotation = self.camera.borrow().rotation;
        *self.camera.borrow_mut() = Camera {
            zoom,
            pan_x: (min.x + max.x) / 2.0,
            pan_y: (min.y + max.y) / 2.0,
            rotation,
        };
        self.area.queue_draw();
    }

    pub fn finish_polyline(
        &self,
        document: &Rc<RefCell<Document>>,
        history: &Rc<RefCell<LegacyHistoryManager>>,
    ) {
        let mut vertices = self.polyline_vertices.borrow_mut();
        if vertices.len() >= 2 {
            let id = document.borrow().next_id();
            let entity = Entity::Polyline {
                id,
                layer: "Default".to_string(),
                points: vertices.clone(),
                closed: false,
            };
            document.borrow_mut().add_entity(entity);
            record_entities_added(&mut history.borrow_mut(), &document.borrow(), &[id]);
        }
        vertices.clear();
        *self.pending_start.borrow_mut() = None;
        self.area.queue_draw();
    }

    pub fn cancel_interaction(&self) {
        self.polyline_vertices.borrow_mut().clear();
        *self.pending_start.borrow_mut() = None;
        self.pending_points.borrow_mut().clear();
        *self.hover_point.borrow_mut() = None;
        *self.hovered_entity.borrow_mut() = None;
        self.text_entry.set_visible(false);
        self.inline_text_mode.borrow_mut().take();
        self.area.queue_draw();
    }

    pub fn inline_text_editor(&self) -> &gtk::Entry {
        &self.text_entry
    }

    fn zoom_at_center(&self, factor: f64) {
        let width = self.area.width() as f64;
        let height = self.area.height() as f64;
        zoom_camera_at(
            &mut self.camera.borrow_mut(),
            factor,
            width / 2.0,
            height / 2.0,
            width,
            height,
        );
        self.area.queue_draw();
    }
}

pub fn screen_to_world(x: f64, y: f64, width: f64, height: f64, camera: Camera) -> Point {
    let view_x = (x - width / 2.0) / camera.zoom;
    let view_y = (height / 2.0 - y) / camera.zoom;
    let cos = camera.rotation.cos();
    let sin = camera.rotation.sin();
    Point {
        x: camera.pan_x + view_x * cos - view_y * sin,
        y: camera.pan_y + view_x * sin + view_y * cos,
    }
}

fn world_to_screen(point: Point, width: f64, height: f64, camera: Camera) -> Point {
    let dx = point.x - camera.pan_x;
    let dy = point.y - camera.pan_y;
    let cos = camera.rotation.cos();
    let sin = camera.rotation.sin();
    let view_x = dx * cos + dy * sin;
    let view_y = -dx * sin + dy * cos;
    Point {
        x: width / 2.0 + view_x * camera.zoom,
        y: height / 2.0 - view_y * camera.zoom,
    }
}

fn screen_delta_to_world(dx: f64, dy: f64, camera: Camera) -> Point {
    let view_x = dx / camera.zoom;
    let view_y = -dy / camera.zoom;
    let cos = camera.rotation.cos();
    let sin = camera.rotation.sin();
    Point {
        x: view_x * cos - view_y * sin,
        y: view_x * sin + view_y * cos,
    }
}

fn visible_world_bounds(width: f64, height: f64, camera: Camera) -> (Point, Point) {
    let points = [
        screen_to_world(0.0, 0.0, width, height, camera),
        screen_to_world(width, 0.0, width, height, camera),
        screen_to_world(width, height, width, height, camera),
        screen_to_world(0.0, height, width, height, camera),
    ];
    points_bounds(points)
}

fn bounds_intersect(a: (Point, Point), b: (Point, Point)) -> bool {
    let (a_min, a_max) = normalized_bounds(a);
    let (b_min, b_max) = normalized_bounds(b);
    a_min.x <= b_max.x && a_max.x >= b_min.x && a_min.y <= b_max.y && a_max.y >= b_min.y
}

fn normalized_bounds(bounds: (Point, Point)) -> (Point, Point) {
    (
        Point {
            x: bounds.0.x.min(bounds.1.x),
            y: bounds.0.y.min(bounds.1.y),
        },
        Point {
            x: bounds.0.x.max(bounds.1.x),
            y: bounds.0.y.max(bounds.1.y),
        },
    )
}

fn document_bounds(document: &Document) -> Option<(Point, Point)> {
    document_bounds_2d(document)
        .into_iter()
        .chain(document_bounds_3d(document))
        .reduce(merge_bounds)
}

fn document_bounds_2d(document: &Document) -> Option<(Point, Point)> {
    let entity_bounds = document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
        .filter_map(|entity| document.cached_entity_bounds(entity))
        .reduce(merge_bounds);
    let paper_bounds = if document.active_layout_kind() == LayoutKind::Paper {
        document.active_layout_ref().map(|layout| {
            (
                Point { x: 0.0, y: 0.0 },
                Point {
                    x: layout.paper.width,
                    y: layout.paper.height,
                },
            )
        })
    } else {
        None
    };
    entity_bounds
        .into_iter()
        .chain(paper_bounds)
        .reduce(merge_bounds)
}

fn document_bounds_3d(document: &Document) -> Option<(Point, Point)> {
    document
        .mesh_references
        .iter()
        .filter_map(|mesh| {
            let bbox = mesh.bounding_box?;
            mesh_box_points(bbox.min, bbox.max)
                .into_iter()
                .map(|point| {
                    project_mesh_point(point, mesh.transform.scale, mesh.transform.translation)
                })
                .reduce_bounds()
        })
        .reduce(merge_bounds)
}

fn entity_bounds(entity: &Entity) -> Option<(Point, Point)> {
    match entity {
        Entity::Point { point, .. }
        | Entity::Text { origin: point, .. }
        | Entity::BlockReference {
            insertion: point, ..
        } => Some((*point, *point)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(points_bounds([*start, *end])),
        Entity::Polyline { points, .. } => points.iter().copied().reduce_bounds(),
        Entity::Spline { control_points, .. } => control_points.iter().copied().reduce_bounds(),
        Entity::Hatch { boundary, .. } => boundary.iter().copied().reduce_bounds(),
        Entity::Circle { center, radius, .. } => {
            let radius = radius.abs();
            Some((
                Point {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                Point {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            ))
        }
        Entity::Table {
            origin,
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } => Some(points_bounds([
            *origin,
            Point {
                x: origin.x + *columns as f64 * *cell_width,
                y: origin.y + *rows as f64 * *cell_height,
            },
        ])),
    }
}

fn entities_in_screen_box(
    document: &Document,
    selection_box: SelectionBox,
    width: f64,
    height: f64,
    camera: Camera,
) -> Vec<u64> {
    let min_x = selection_box.start_x.min(selection_box.current_x);
    let max_x = selection_box.start_x.max(selection_box.current_x);
    let min_y = selection_box.start_y.min(selection_box.current_y);
    let max_y = selection_box.start_y.max(selection_box.current_y);
    if (max_x - min_x) < 3.0 || (max_y - min_y) < 3.0 {
        return Vec::new();
    }
    document
        .entities
        .iter()
        .filter_map(|entity| {
            let (min, max) = document.cached_entity_bounds(entity)?;
            let corners = [
                Point { x: min.x, y: min.y },
                Point { x: max.x, y: min.y },
                Point { x: max.x, y: max.y },
                Point { x: min.x, y: max.y },
            ];
            corners
                .into_iter()
                .map(|point| world_to_screen(point, width, height, camera))
                .any(|point| {
                    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
                })
                .then_some(entity.id())
        })
        .collect()
}

trait BoundsIterator {
    fn reduce_bounds(self) -> Option<(Point, Point)>;
}

impl<I> BoundsIterator for I
where
    I: Iterator<Item = Point>,
{
    fn reduce_bounds(self) -> Option<(Point, Point)> {
        self.map(|point| (point, point)).reduce(merge_bounds)
    }
}

fn points_bounds<const N: usize>(points: [Point; N]) -> (Point, Point) {
    points
        .into_iter()
        .reduce_bounds()
        .unwrap_or((Point::default(), Point::default()))
}

fn merge_bounds(first: (Point, Point), second: (Point, Point)) -> (Point, Point) {
    (
        Point {
            x: first.0.x.min(second.0.x),
            y: first.0.y.min(second.0.y),
        },
        Point {
            x: first.1.x.max(second.1.x),
            y: first.1.y.max(second.1.y),
        },
    )
}

fn zoom_camera_at(
    camera: &mut Camera,
    factor: f64,
    anchor_x: f64,
    anchor_y: f64,
    width: f64,
    height: f64,
) {
    let before = screen_to_world(anchor_x, anchor_y, width, height, *camera);
    camera.zoom = (camera.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    camera.pan_x = before.x - (anchor_x - width / 2.0) / camera.zoom;
    camera.pan_y = before.y - (height / 2.0 - anchor_y) / camera.zoom;
}

fn handle_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    tool: Tool,
    tool_parameters: ToolParametersState,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    pending_points: &Rc<RefCell<Vec<Point>>>,
    polyline_vertices: &Rc<RefCell<Vec<Point>>>,
) {
    match tool {
        Tool::Line => {
            if tool_parameters.line_mode == LineCreationMode::TwoPoints {
                two_point_entity(document, history, point, pending, |id, start, end| {
                    Entity::Line {
                        id,
                        layer: "Default".to_string(),
                        start,
                        end,
                    }
                })
            } else {
                *pending.borrow_mut() = None;
            }
        }
        Tool::Rectangle => match tool_parameters.rectangle_mode {
            RectangleCreationMode::TwoCorners => {
                two_point_entity(document, history, point, pending, |id, start, end| {
                    Entity::Polyline {
                        id,
                        layer: "Default".to_string(),
                        points: vec![
                            start,
                            Point {
                                x: end.x,
                                y: start.y,
                            },
                            end,
                            Point {
                                x: start.x,
                                y: end.y,
                            },
                        ],
                        closed: true,
                    }
                })
            }
            RectangleCreationMode::CornerDimensions | RectangleCreationMode::CenterDimensions => {
                *pending.borrow_mut() = None;
                pending_points.borrow_mut().clear();
                let id = document.next_id();
                if let Some(entity) = finalize_rectangle_entity(
                    tool_parameters.rectangle_mode,
                    id,
                    point,
                    tool_parameters.rectangle_width,
                    tool_parameters.rectangle_height,
                ) {
                    document.add_entity(entity);
                    record_entities_added(&mut history.borrow_mut(), document, &[id]);
                }
            }
        },
        Tool::Circle => handle_circle_click(
            document,
            history,
            point,
            pending,
            pending_points,
            tool_parameters.circle_mode,
        ),
        Tool::Arc => handle_arc_click(
            document,
            history,
            point,
            pending,
            pending_points,
            tool_parameters.arc_mode,
        ),
        Tool::Dimension | Tool::Measure => {
            handle_dimension_click(document, history, point, pending, pending_points);
        }
        Tool::Hatch => two_point_entity(document, history, point, pending, |id, start, end| {
            Entity::Hatch {
                id,
                layer: "Default".to_string(),
                boundary: vec![
                    start,
                    Point {
                        x: end.x,
                        y: start.y,
                    },
                    end,
                    Point {
                        x: start.x,
                        y: end.y,
                    },
                ],
                pattern: "ANSI31".to_string(),
                scale: 1.0,
                angle: 45.0,
            }
        }),
        Tool::Guideline | Tool::Parametric => {
            two_point_entity(document, history, point, pending, |id, start, end| {
                Entity::Guideline {
                    id,
                    layer: "Construction".to_string(),
                    start,
                    end,
                    construction: true,
                }
            })
        }
        Tool::Polyline => {
            *pending.borrow_mut() = None;
            let mut vertices = polyline_vertices.borrow_mut();
            if vertices
                .last()
                .map(|last| last.distance_to(point) > 1e-9)
                .unwrap_or(true)
            {
                vertices.push(point);
            }
        }
        Tool::Text => {
            *pending.borrow_mut() = None;
            pending_points.borrow_mut().clear();
        }
        Tool::Table => single_click_entity(document, history, point, |id| Entity::Table {
            id,
            layer: "Default".to_string(),
            origin: point,
            rows: 3,
            columns: 4,
            cell_width: 18.0,
            cell_height: 7.0,
        }),
        Tool::Block => single_click_entity(document, history, point, |id| Entity::BlockReference {
            id,
            layer: "Default".to_string(),
            name: "Block".to_string(),
            insertion: point,
            scale: 1.0,
            rotation: 0.0,
        }),
        _ => {
            *pending.borrow_mut() = None;
            pending_points.borrow_mut().clear();
        }
    }
}

fn handle_circle_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    pending_points: &Rc<RefCell<Vec<Point>>>,
    mode: CircleCreationMode,
) {
    if mode == CircleCreationMode::ThreePoint {
        *pending.borrow_mut() = None;
        let mut points = pending_points.borrow_mut();
        points.push(point);
        if points.len() < 3 {
            return;
        }
        let id = document.next_id();
        if let Some(entity) = finalize_circle_entity(mode, id, &points[..3]) {
            document.add_entity(entity);
            record_entities_added(&mut history.borrow_mut(), document, &[id]);
        } else {
            eprintln!("CIRCLE 3P: points are colinear");
        }
        points.clear();
        return;
    }

    pending_points.borrow_mut().clear();
    let mut pending_ref = pending.borrow_mut();
    if let Some(start) = *pending_ref {
        let id = document.next_id();
        if let Some(entity) = finalize_circle_entity(mode, id, &[start, point]) {
            document.add_entity(entity);
            record_entities_added(&mut history.borrow_mut(), document, &[id]);
        }
        *pending_ref = None;
    } else {
        *pending_ref = Some(point);
    }
}

fn handle_arc_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    pending_points: &Rc<RefCell<Vec<Point>>>,
    mode: crate::tool_parameters::ArcUiMode,
) {
    *pending.borrow_mut() = None;
    let mut points = pending_points.borrow_mut();
    points.push(point);
    if points.len() < 3 {
        return;
    }
    let id = document.next_id();
    if let Some(entity) = finalize_arc_entity(mode, id, &points[..3]) {
        document.add_entity(entity);
        record_entities_added(&mut history.borrow_mut(), document, &[id]);
    } else {
        eprintln!("ARC 3P: points are colinear or mode unsupported");
    }
    points.clear();
}

fn handle_dimension_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    pending_points: &Rc<RefCell<Vec<Point>>>,
) {
    *pending.borrow_mut() = None;
    let mut points = pending_points.borrow_mut();
    points.push(point);
    if points.len() < 3 {
        return;
    }
    let start = points[0];
    let end = points[1];
    let offset_point = points[2];
    let offset = dimension_offset_from_point(start, end, offset_point);
    let label = format_distance_with_unit(start.distance_to(end), document.units, 2);
    let entity = Entity::Dimension {
        id: document.next_id(),
        layer: "Dimensions".to_string(),
        start,
        end,
        label,
        style: format!("Linear@{:.6},{:.6}", offset.x, offset.y),
        precision: 2,
    };
    let id = entity.id();
    document.add_entity(entity);
    record_entities_added(&mut history.borrow_mut(), document, &[id]);
    points.clear();
}

fn two_point_entity<F>(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    build: F,
) where
    F: FnOnce(u64, Point, Point) -> Entity,
{
    let mut pending_ref = pending.borrow_mut();
    if let Some(start) = *pending_ref {
        let entity = build(document.next_id(), start, point);
        let id = entity.id();
        document.add_entity(entity);
        record_entities_added(&mut history.borrow_mut(), document, &[id]);
        *pending_ref = None;
    } else {
        *pending_ref = Some(point);
    }
}

fn single_click_entity<F>(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    _point: Point,
    build: F,
) where
    F: FnOnce(u64) -> Entity,
{
    let entity = build(document.next_id());
    let id = entity.id();
    document.add_entity(entity);
    record_entities_added(&mut history.borrow_mut(), document, &[id]);
}

fn begin_inline_text_create(
    entry: &gtk::Entry,
    mode: &Rc<RefCell<Option<InlineTextMode>>>,
    world_position: Point,
    area: &DrawingArea,
    camera: Camera,
) {
    *mode.borrow_mut() = Some(InlineTextMode::Creating { world_position });
    entry.set_text("");
    position_inline_text_entry(entry, area, camera, world_position);
    entry.set_visible(true);
    entry.grab_focus();
}

fn begin_inline_text_edit(
    entry: &gtk::Entry,
    mode: &Rc<RefCell<Option<InlineTextMode>>>,
    entity_id: u64,
    original_text: String,
    world_position: Point,
    area: &DrawingArea,
    camera: Camera,
) {
    *mode.borrow_mut() = Some(InlineTextMode::Editing {
        entity_id,
        original_text: original_text.clone(),
        world_position,
    });
    entry.set_text(&original_text);
    position_inline_text_entry(entry, area, camera, world_position);
    entry.set_visible(true);
    entry.grab_focus();
    entry.select_region(0, -1);
}

fn normalize_text_for_creation(value: &str) -> Option<String> {
    let text = value.trim();
    (!text.is_empty()).then_some(text.to_string())
}

fn should_apply_inline_text_edit(original_text: &str, new_text: &str) -> bool {
    original_text.trim() != new_text.trim() && !new_text.trim().is_empty()
}

fn build_text_entity(id: u64, point: Point, text: String) -> Entity {
    Entity::Text {
        id,
        layer: "Default".to_string(),
        origin: point,
        text,
        height: 2.5,
        rotation: 0.0,
    }
}

fn position_inline_text_entry(
    entry: &gtk::Entry,
    area: &DrawingArea,
    camera: Camera,
    world: Point,
) {
    let screen = world_to_screen(world, area.width() as f64, area.height() as f64, camera);
    entry.set_margin_start(screen.x.max(0.0) as i32);
    entry.set_margin_top(screen.y.max(0.0) as i32);
    entry.set_width_chars(24);
}

fn schedule_inline_text_commit(
    active_mode: Option<InlineTextMode>,
    raw_text: String,
    document: &Rc<RefCell<Document>>,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &Rc<RefCell<Vec<u64>>>,
    selection_label: &gtk::Label,
    area: &DrawingArea,
    attempt: u8,
) {
    let document = document.clone();
    let history = history.clone();
    let selected = selected.clone();
    let selection_label = selection_label.clone();
    let area = area.clone();
    gtk::glib::idle_add_local_once(move || {
        let Some(mode) = active_mode.clone() else {
            area.queue_draw();
            return;
        };
        match finish_inline_text_commit(
            mode,
            &raw_text,
            &document,
            &history,
            &selected,
            &selection_label,
            &area,
        ) {
            InlineCommitResult::Done => {}
            InlineCommitResult::Busy if attempt < INLINE_TEXT_COMMIT_MAX_RETRIES => {
                schedule_inline_text_commit(
                    active_mode,
                    raw_text,
                    &document,
                    &history,
                    &selected,
                    &selection_label,
                    &area,
                    attempt + 1,
                );
            }
            InlineCommitResult::Busy => {
                eprintln!("INLINE TEXT: document busy after retries; commit skipped");
            }
        }
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InlineCommitResult {
    Done,
    Busy,
}

fn finish_inline_text_commit(
    active_mode: InlineTextMode,
    raw_text: &str,
    document: &Rc<RefCell<Document>>,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &Rc<RefCell<Vec<u64>>>,
    selection_label: &gtk::Label,
    area: &DrawingArea,
) -> InlineCommitResult {
    let normalized = normalize_text_for_creation(raw_text);
    if normalized.is_none() {
        area.queue_draw();
        return InlineCommitResult::Done;
    }
    let text = normalized.unwrap_or_default();

    match active_mode {
        InlineTextMode::Creating { world_position } => {
            let Ok(mut doc) = document.try_borrow_mut() else {
                return InlineCommitResult::Busy;
            };
            let id = doc.next_id();
            doc.add_entity(build_text_entity(id, world_position, text));
            let Ok(mut history_ref) = history.try_borrow_mut() else {
                return InlineCommitResult::Busy;
            };
            record_entities_added(&mut history_ref, &doc, &[id]);
            let Ok(mut selected_ref) = selected.try_borrow_mut() else {
                return InlineCommitResult::Busy;
            };
            *selected_ref = vec![id];
            update_selection_label(selection_label, &doc, &selected_ref);
            area.queue_draw();
            InlineCommitResult::Done
        }
        InlineTextMode::Editing {
            entity_id,
            original_text,
            ..
        } => {
            // If there is no real change, we intentionally skip history.
            if !should_apply_inline_text_edit(&original_text, &text) {
                area.queue_draw();
                return InlineCommitResult::Done;
            }

            let before = {
                let Ok(doc) = document.try_borrow() else {
                    return InlineCommitResult::Busy;
                };
                crate::cad::history::capture_entity_property_state(&doc, entity_id)
            };
            let Some(before) = before else {
                area.queue_draw();
                return InlineCommitResult::Done;
            };

            {
                let Ok(mut doc) = document.try_borrow_mut() else {
                    return InlineCommitResult::Busy;
                };
                doc.set_text_entity_text(entity_id, &text);
            }

            let after = {
                let Ok(doc) = document.try_borrow() else {
                    return InlineCommitResult::Busy;
                };
                crate::cad::history::capture_entity_property_state(&doc, entity_id)
            };
            let Some(after) = after else {
                area.queue_draw();
                return InlineCommitResult::Done;
            };

            let Ok(mut history_ref) = history.try_borrow_mut() else {
                return InlineCommitResult::Busy;
            };
            crate::cad::history::record_entity_property_changes(
                &mut history_ref,
                vec![crate::cad::history::EntityPropertyChange {
                    entity_id,
                    before,
                    after,
                }],
            );
            let Ok(mut selected_ref) = selected.try_borrow_mut() else {
                return InlineCommitResult::Busy;
            };
            *selected_ref = vec![entity_id];
            let Ok(doc) = document.try_borrow() else {
                return InlineCommitResult::Busy;
            };
            update_selection_label(selection_label, &doc, &selected_ref);
            area.queue_draw();
            InlineCommitResult::Done
        }
    }
}

fn commit_inline_text(
    entry: &gtk::Entry,
    mode: &Rc<RefCell<Option<InlineTextMode>>>,
    document: &Rc<RefCell<Document>>,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &Rc<RefCell<Vec<u64>>>,
    selection_label: &gtk::Label,
    area: &DrawingArea,
) {
    let raw_text = entry.text().to_string();
    let active_mode = mode.borrow().clone();
    entry.set_visible(false);
    mode.borrow_mut().take();
    area.grab_focus();
    schedule_inline_text_commit(
        active_mode,
        raw_text,
        document,
        history,
        selected,
        selection_label,
        area,
        0,
    );
}

fn cancel_inline_text(
    entry: &gtk::Entry,
    mode: &Rc<RefCell<Option<InlineTextMode>>>,
    area: &DrawingArea,
) {
    entry.set_visible(false);
    mode.borrow_mut().take();
    area.grab_focus();
}

fn text_entity_at(document: &Document, id: u64) -> Option<(String, Point)> {
    document.entities.iter().find_map(|entity| match entity {
        Entity::Text {
            id: text_id,
            origin,
            text,
            ..
        } if *text_id == id => Some((text.clone(), *origin)),
        _ => None,
    })
}

fn is_text_entity(document: &Document, id: u64) -> bool {
    text_entity_at(document, id).is_some()
}

fn draw_scene(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    pending: Option<Point>,
    pending_points: &[Point],
    polyline_vertices: &[Point],
    hover_point: Option<Point>,
    camera: Camera,
    selected_entities: &[u64],
    active_tool: Tool,
    tool_parameters: ToolParametersState,
    hovered_entity: Option<u64>,
    snap: Option<SnapTarget>,
    selection_box: Option<SelectionBox>,
) {
    if document.active_layout_kind() == LayoutKind::Paper {
        draw_paper_background(cr, width, height, document, camera);
        draw_layout_viewports(cr, width, height, document, camera);
    } else {
        draw_grid(cr, width, height, camera);
    }
    let visible_bounds = visible_world_bounds(width, height, camera);
    draw_import_placeholders(cr, width, height, document, camera);
    for entity in document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
        .filter(|entity| {
            selected_entities.contains(&entity.id())
                || document
                    .cached_entity_bounds(entity)
                    .map(|bounds| bounds_intersect(bounds, visible_bounds))
                    .unwrap_or(true)
        })
    {
        draw_entity(
            cr,
            width,
            height,
            camera,
            entity,
            selected_entities.contains(&entity.id()),
            hovered_entity == Some(entity.id()),
            document.entity_color(entity.id()),
            document.entity_line_type(entity.id()),
            document.entity_line_weight(entity.id()),
        );
    }
    draw_tool_preview(
        cr,
        width,
        height,
        camera,
        active_tool,
        tool_parameters,
        pending,
        pending_points,
        hover_point,
    );
    draw_polyline_preview(cr, width, height, camera, polyline_vertices, hover_point);
    if let Some(snap) = snap {
        draw_snap_marker(cr, width, height, camera, snap);
    }
    if let Some(selection_box) = selection_box {
        draw_selection_box(cr, selection_box);
    }
}

fn draw_tool_preview(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    active_tool: Tool,
    tool_parameters: ToolParametersState,
    pending: Option<Point>,
    pending_points: &[Point],
    hover_point: Option<Point>,
) {
    if let Some(point) = pending {
        draw_pending_point(cr, width, height, camera, point);
    }
    for point in pending_points {
        draw_pending_point(cr, width, height, camera, *point);
    }
    let Some(hover) = hover_point else {
        return;
    };
    if matches!(active_tool, Tool::Dimension | Tool::Measure) {
        match pending_points {
            [start] => {
                draw_preview_shape(
                    cr,
                    width,
                    height,
                    camera,
                    ToolPreview::Line {
                        start: *start,
                        end: hover,
                    },
                );
                return;
            }
            [start, end] => {
                draw_dimension_preview(cr, width, height, camera, *start, *end, hover);
                return;
            }
            _ => {}
        }
    }
    let preview = match active_tool {
        Tool::Line if tool_parameters.line_mode == LineCreationMode::TwoPoints => pending
            .map(|start| preview_line_two_points(start, hover))
            .unwrap_or(ToolPreview::None),
        Tool::Rectangle if tool_parameters.rectangle_mode == RectangleCreationMode::TwoCorners => {
            pending
                .map(|start| preview_rectangle_two_corners(start, hover))
                .unwrap_or(ToolPreview::None)
        }
        Tool::Rectangle
            if tool_parameters.rectangle_mode == RectangleCreationMode::CornerDimensions =>
        {
            preview_rectangle_corner_size(
                hover,
                tool_parameters.rectangle_width,
                tool_parameters.rectangle_height,
            )
            .unwrap_or(ToolPreview::None)
        }
        Tool::Rectangle
            if tool_parameters.rectangle_mode == RectangleCreationMode::CenterDimensions =>
        {
            preview_rectangle_center_size(
                hover,
                tool_parameters.rectangle_width,
                tool_parameters.rectangle_height,
            )
            .unwrap_or(ToolPreview::None)
        }
        Tool::Circle => match tool_parameters.circle_mode {
            CircleCreationMode::CenterRadius => pending
                .and_then(|center| preview_circle_center_radius(center, hover))
                .unwrap_or(ToolPreview::None),
            CircleCreationMode::CenterDiameter => pending
                .and_then(|center| preview_circle_center_diameter(center, hover))
                .unwrap_or(ToolPreview::None),
            CircleCreationMode::TwoPointDiameter => pending
                .and_then(|p1| preview_circle_two_point_diameter(p1, hover))
                .unwrap_or(ToolPreview::None),
            CircleCreationMode::ThreePoint => match pending_points {
                [p1, p2] => {
                    preview_circle_three_points(*p1, *p2, hover).unwrap_or(ToolPreview::None)
                }
                [p1] => preview_line_two_points(*p1, hover),
                _ => ToolPreview::None,
            },
        },
        Tool::Arc => match pending_points {
            [p1, p2] => preview_arc_three_points(*p1, *p2, hover).unwrap_or(ToolPreview::None),
            [p1] => preview_line_two_points(*p1, hover),
            _ => ToolPreview::None,
        },
        _ => pending
            .map(|start| ToolPreview::Line { start, end: hover })
            .unwrap_or(ToolPreview::None),
    };
    draw_preview_shape(cr, width, height, camera, preview);
}

fn draw_dimension_preview(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    start: Point,
    end: Point,
    offset_point: Point,
) {
    let offset = dimension_offset_from_point(start, end, offset_point);
    let (ext_start, ext_end, dim_start, dim_end) = dimension_segments(start, end, offset);
    cr.set_line_width(1.2);
    cr.set_source_rgba(0.65, 0.95, 1.0, 0.72);
    cr.set_dash(&[7.0, 5.0], 0.0);
    draw_preview_segment(cr, width, height, camera, ext_start, dim_start);
    draw_preview_segment(cr, width, height, camera, ext_end, dim_end);
    draw_preview_segment(cr, width, height, camera, dim_start, dim_end);
    cr.set_dash(&[], 0.0);
}

fn draw_preview_shape(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    preview: ToolPreview,
) {
    cr.set_line_width(1.4);
    cr.set_source_rgba(0.65, 0.95, 1.0, 0.72);
    cr.set_dash(&[7.0, 5.0], 0.0);
    match preview {
        ToolPreview::None => {}
        ToolPreview::Line { start, end } => {
            let a = world_to_screen(start, width, height, camera);
            let b = world_to_screen(end, width, height, camera);
            cr.move_to(a.x, a.y);
            cr.line_to(b.x, b.y);
            let _ = cr.stroke();
        }
        ToolPreview::Circle { center, radius } => {
            let c = world_to_screen(center, width, height, camera);
            cr.arc(c.x, c.y, radius * camera.zoom, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
            cr.set_dash(&[], 0.0);
            draw_preview_segment(
                cr,
                width,
                height,
                camera,
                center,
                Point {
                    x: center.x + radius,
                    y: center.y,
                },
            );
        }
        ToolPreview::Rectangle { a, b } => {
            let top_left = world_to_screen(
                Point {
                    x: a.x.min(b.x),
                    y: a.y.max(b.y),
                },
                width,
                height,
                camera,
            );
            let bottom_right = world_to_screen(
                Point {
                    x: a.x.max(b.x),
                    y: a.y.min(b.y),
                },
                width,
                height,
                camera,
            );
            cr.rectangle(
                top_left.x.min(bottom_right.x),
                top_left.y.min(bottom_right.y),
                (bottom_right.x - top_left.x).abs(),
                (bottom_right.y - top_left.y).abs(),
            );
            let _ = cr.stroke();
        }
        ToolPreview::Polyline { points, closed } => {
            if append_points_path(cr, width, height, camera, &points, closed) {
                let _ = cr.stroke();
            }
        }
    }
    cr.set_dash(&[], 0.0);
}

fn draw_layout_viewports(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    paper_camera: Camera,
) {
    for viewport in document
        .layout_viewports
        .iter()
        .filter(|viewport| viewport.layout == document.active_layout)
    {
        draw_viewport_frame(cr, width, height, paper_camera, viewport);
        cr.save().ok();
        clip_to_viewport(cr, width, height, paper_camera, viewport);
        for entity in document.entities.iter().filter(|entity| {
            document.is_model_entity(entity.id()) && viewport_layer_visible(viewport, entity)
        }) {
            if !document
                .cached_entity_bounds(entity)
                .map(|bounds| bounds_intersect(bounds, viewport_model_bounds(viewport)))
                .unwrap_or(true)
            {
                continue;
            }
            let transformed = transform_entity_for_viewport(entity, viewport);
            if !entity_bounds(&transformed)
                .map(|bounds| bounds_intersect(bounds, viewport_bounds(viewport)))
                .unwrap_or(true)
            {
                continue;
            }
            draw_entity(
                cr,
                width,
                height,
                paper_camera,
                &transformed,
                false,
                false,
                document.entity_color(entity.id()),
                document.entity_line_type(entity.id()),
                document.entity_line_weight(entity.id()),
            );
        }
        cr.restore().ok();
    }
}

fn viewport_bounds(viewport: &LayoutViewport) -> (Point, Point) {
    (
        Point {
            x: viewport.center.x - viewport.width * 0.5,
            y: viewport.center.y - viewport.height * 0.5,
        },
        Point {
            x: viewport.center.x + viewport.width * 0.5,
            y: viewport.center.y + viewport.height * 0.5,
        },
    )
}

fn viewport_model_bounds(viewport: &LayoutViewport) -> (Point, Point) {
    let aspect = viewport.width / viewport.height.max(1.0);
    let model_width = viewport.view_height * aspect;
    (
        Point {
            x: viewport.view_center.x - model_width * 0.5,
            y: viewport.view_center.y - viewport.view_height * 0.5,
        },
        Point {
            x: viewport.view_center.x + model_width * 0.5,
            y: viewport.view_center.y + viewport.view_height * 0.5,
        },
    )
}

fn viewport_layer_visible(viewport: &LayoutViewport, entity: &Entity) -> bool {
    let layer = entity.layer();
    if !viewport.visible_layers.is_empty()
        && !viewport
            .visible_layers
            .iter()
            .any(|visible| visible == layer)
    {
        return false;
    }
    !viewport.hidden_layers.iter().any(|hidden| hidden == layer)
}

fn draw_viewport_frame(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    viewport: &LayoutViewport,
) {
    if !viewport.border_visible {
        return;
    }
    let min = world_to_screen(
        Point {
            x: viewport.center.x - viewport.width * 0.5,
            y: viewport.center.y - viewport.height * 0.5,
        },
        width,
        height,
        camera,
    );
    let max = world_to_screen(
        Point {
            x: viewport.center.x + viewport.width * 0.5,
            y: viewport.center.y + viewport.height * 0.5,
        },
        width,
        height,
        camera,
    );
    cr.set_line_width(1.0);
    cr.set_source_rgba(0.2, 0.7, 0.8, 0.42);
    cr.rectangle(
        min.x.min(max.x),
        min.y.min(max.y),
        (max.x - min.x).abs(),
        (max.y - min.y).abs(),
    );
    let _ = cr.stroke();
}

fn clip_to_viewport(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    viewport: &LayoutViewport,
) {
    let min = world_to_screen(
        Point {
            x: viewport.center.x - viewport.width * 0.5,
            y: viewport.center.y - viewport.height * 0.5,
        },
        width,
        height,
        camera,
    );
    let max = world_to_screen(
        Point {
            x: viewport.center.x + viewport.width * 0.5,
            y: viewport.center.y + viewport.height * 0.5,
        },
        width,
        height,
        camera,
    );
    cr.rectangle(
        min.x.min(max.x),
        min.y.min(max.y),
        (max.x - min.x).abs(),
        (max.y - min.y).abs(),
    );
    cr.clip();
}

fn viewport_point(point: Point, viewport: &LayoutViewport) -> Point {
    let scale = viewport.height / viewport.view_height.max(1.0);
    let dx = point.x - viewport.view_center.x;
    let dy = point.y - viewport.view_center.y;
    let cos = viewport.twist.cos();
    let sin = viewport.twist.sin();
    Point {
        x: viewport.center.x + (dx * cos - dy * sin) * scale,
        y: viewport.center.y + (dx * sin + dy * cos) * scale,
    }
}

fn transform_entity_for_viewport(entity: &Entity, viewport: &LayoutViewport) -> Entity {
    let mut copy = entity.clone();
    match &mut copy {
        Entity::Point { point, .. } | Entity::Text { origin: point, .. } => {
            *point = viewport_point(*point, viewport);
        }
        Entity::BlockReference {
            insertion: point, ..
        } => {
            *point = viewport_point(*point, viewport);
        }
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            *start = viewport_point(*start, viewport);
            *end = viewport_point(*end, viewport);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        } => {
            for point in points {
                *point = viewport_point(*point, viewport);
            }
        }
        Entity::Hatch { boundary, .. } => {
            for point in boundary {
                *point = viewport_point(*point, viewport);
            }
        }
        Entity::Circle { center, radius, .. } => {
            *center = viewport_point(*center, viewport);
            *radius *= viewport.height / viewport.view_height.max(1.0);
        }
        Entity::Table { origin, .. } => {
            *origin = viewport_point(*origin, viewport);
        }
    }
    copy
}

fn draw_selection_box(cr: &gtk::cairo::Context, selection_box: SelectionBox) {
    let x = selection_box.start_x.min(selection_box.current_x);
    let y = selection_box.start_y.min(selection_box.current_y);
    let width = (selection_box.current_x - selection_box.start_x).abs();
    let height = (selection_box.current_y - selection_box.start_y).abs();
    cr.set_source_rgba(0.55, 0.75, 1.0, 0.18);
    cr.rectangle(x, y, width, height);
    let _ = cr.fill_preserve();
    cr.set_line_width(1.2);
    cr.set_source_rgba(0.72, 0.86, 1.0, 0.86);
    let _ = cr.stroke();
}

fn parse_hex_color(value: &str) -> Option<(f64, f64, f64)> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if value.len() == 6 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&value[0..2], 16).ok()? as f64 / 255.0;
        let g = u8::from_str_radix(&value[2..4], 16).ok()? as f64 / 255.0;
        let b = u8::from_str_radix(&value[4..6], 16).ok()? as f64 / 255.0;
        return Some((r, g, b));
    }

    let rgb = value
        .strip_prefix("rgb(")
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(value);
    let parts: Vec<&str> = rgb
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].parse::<u8>().ok()? as f64 / 255.0;
    let g = parts[1].parse::<u8>().ok()? as f64 / 255.0;
    let b = parts[2].parse::<u8>().ok()? as f64 / 255.0;
    Some((r, g, b))
}

fn draw_pending_point(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    point: Point,
) {
    let screen = world_to_screen(point, width, height, camera);
    cr.set_source_rgba(0.95, 0.80, 1.0, 0.8);
    cr.arc(screen.x, screen.y, 4.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}

fn draw_preview_segment(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    start: Point,
    end: Point,
) {
    let a = world_to_screen(start, width, height, camera);
    let b = world_to_screen(end, width, height, camera);
    cr.set_line_width(1.2);
    cr.set_source_rgba(0.65, 0.95, 1.0, 0.54);
    cr.move_to(a.x, a.y);
    cr.line_to(b.x, b.y);
    let _ = cr.stroke();
}

fn draw_polyline_preview(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    vertices: &[Point],
    hover_point: Option<Point>,
) {
    if vertices.is_empty() {
        return;
    }
    cr.set_line_width(1.6);
    cr.set_source_rgba(0.65, 0.95, 1.0, 0.78);
    let first = world_to_screen(vertices[0], width, height, camera);
    cr.move_to(first.x, first.y);
    for vertex in vertices.iter().skip(1) {
        let p = world_to_screen(*vertex, width, height, camera);
        cr.line_to(p.x, p.y);
    }
    if let Some(hover) = hover_point {
        let p = world_to_screen(hover, width, height, camera);
        cr.line_to(p.x, p.y);
    }
    let _ = cr.stroke();
    for vertex in vertices {
        let screen = world_to_screen(*vertex, width, height, camera);
        cr.set_source_rgba(0.95, 0.80, 1.0, 0.8);
        cr.arc(screen.x, screen.y, 4.0, 0.0, std::f64::consts::TAU);
        let _ = cr.fill();
    }
}

fn draw_snap_marker(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    snap: SnapTarget,
) {
    let p = world_to_screen(snap.point, width, height, camera);
    cr.set_line_width(1.4);
    cr.set_source_rgba(0.65, 0.95, 1.0, 0.96);
    match snap.kind {
        SnapKind::Endpoint => {
            cr.rectangle(p.x - 5.0, p.y - 5.0, 10.0, 10.0);
            let _ = cr.stroke();
        }
        SnapKind::Midpoint => {
            cr.move_to(p.x, p.y - 6.0);
            cr.line_to(p.x + 6.0, p.y + 5.0);
            cr.line_to(p.x - 6.0, p.y + 5.0);
            cr.close_path();
            let _ = cr.stroke();
        }
        SnapKind::Center => {
            cr.arc(p.x, p.y, 6.0, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
        }
        SnapKind::Quadrant => {
            cr.move_to(p.x - 6.0, p.y);
            cr.line_to(p.x + 6.0, p.y);
            cr.move_to(p.x, p.y - 6.0);
            cr.line_to(p.x, p.y + 6.0);
            let _ = cr.stroke();
        }
        SnapKind::CircleEdge => {
            cr.arc(p.x, p.y, 5.5, 0.0, std::f64::consts::TAU);
            cr.move_to(p.x - 7.0, p.y);
            cr.line_to(p.x + 7.0, p.y);
            cr.move_to(p.x, p.y - 7.0);
            cr.line_to(p.x, p.y + 7.0);
            let _ = cr.stroke();
        }
        SnapKind::Intersection => {
            cr.move_to(p.x - 6.0, p.y - 6.0);
            cr.line_to(p.x + 6.0, p.y + 6.0);
            cr.move_to(p.x + 6.0, p.y - 6.0);
            cr.line_to(p.x - 6.0, p.y + 6.0);
            let _ = cr.stroke();
        }
        SnapKind::Perpendicular | SnapKind::Ortho => {
            cr.rectangle(p.x - 5.0, p.y - 5.0, 10.0, 10.0);
            cr.move_to(p.x - 5.0, p.y);
            cr.line_to(p.x, p.y);
            cr.line_to(p.x, p.y - 5.0);
            let _ = cr.stroke();
        }
        SnapKind::Parallel => {
            cr.move_to(p.x - 7.0, p.y + 5.0);
            cr.line_to(p.x - 1.0, p.y - 5.0);
            cr.move_to(p.x + 2.0, p.y + 5.0);
            cr.line_to(p.x + 8.0, p.y - 5.0);
            let _ = cr.stroke();
        }
    }
    cr.move_to(p.x + 9.0, p.y - 9.0);
    let _ = cr.show_text(snap.kind.label());
}

fn draw_grid(cr: &gtk::cairo::Context, width: f64, height: f64, camera: Camera) {
    cr.set_source_rgb(0.035, 0.035, 0.055);
    let _ = cr.paint();
    let top_left = screen_to_world(0.0, 0.0, width, height, camera);
    let bottom_right = screen_to_world(width, height, width, height, camera);
    let mut minor_world = 10.0;
    while minor_world * camera.zoom < 18.0 {
        minor_world *= 2.0;
    }
    while minor_world * camera.zoom > 72.0 {
        minor_world /= 2.0;
    }

    cr.set_line_width(1.0);
    cr.set_source_rgba(0.48, 0.50, 0.62, 0.16);
    let mut world_x = (top_left.x / minor_world).floor() * minor_world;
    while world_x < bottom_right.x {
        let screen = world_to_screen(Point { x: world_x, y: 0.0 }, width, height, camera);
        cr.move_to(screen.x, 0.0);
        cr.line_to(screen.x, height);
        world_x += minor_world;
    }
    let mut world_y = (bottom_right.y / minor_world).floor() * minor_world;
    while world_y < top_left.y {
        let screen = world_to_screen(Point { x: 0.0, y: world_y }, width, height, camera);
        cr.move_to(0.0, screen.y);
        cr.line_to(width, screen.y);
        world_y += minor_world;
    }
    let _ = cr.stroke();

    let origin = world_to_screen(Point::default(), width, height, camera);
    cr.set_line_width(1.5);
    cr.set_source_rgba(0.80, 0.65, 0.97, 0.58);
    cr.move_to(0.0, origin.y);
    cr.line_to(width, origin.y);
    cr.move_to(origin.x, 0.0);
    cr.line_to(origin.x, height);
    let _ = cr.stroke();

    cr.set_source_rgba(0.80, 0.65, 0.97, 0.9);
    cr.arc(origin.x, origin.y, 3.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}

fn draw_paper_background(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    camera: Camera,
) {
    let Some(layout) = document.active_layout_ref() else {
        return;
    };
    cr.set_source_rgb(0.56, 0.56, 0.54);
    let _ = cr.paint();
    let min = world_to_screen(Point { x: 0.0, y: 0.0 }, width, height, camera);
    let max = world_to_screen(
        Point {
            x: layout.paper.width,
            y: layout.paper.height,
        },
        width,
        height,
        camera,
    );
    let x = min.x.min(max.x);
    let y = min.y.min(max.y);
    let paper_width = (max.x - min.x).abs();
    let paper_height = (max.y - min.y).abs();
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.18);
    cr.rectangle(x + 5.0, y + 6.0, paper_width, paper_height);
    let _ = cr.fill();
    cr.set_source_rgb(0.98, 0.98, 0.96);
    cr.rectangle(x, y, paper_width, paper_height);
    let _ = cr.fill_preserve();
    cr.set_line_width(1.0);
    cr.set_source_rgba(0.12, 0.12, 0.12, 0.34);
    let _ = cr.stroke();
    cr.set_line_width(1.0);
    cr.set_dash(&[7.0, 7.0], 0.0);
    cr.set_source_rgba(0.12, 0.12, 0.12, 0.22);
    let left = layout.page_setup.margin_left * camera.zoom;
    let right = layout.page_setup.margin_right * camera.zoom;
    let top = layout.page_setup.margin_top * camera.zoom;
    let bottom = layout.page_setup.margin_bottom * camera.zoom;
    cr.rectangle(
        x + left,
        y + top,
        (paper_width - left - right).max(1.0),
        (paper_height - top - bottom).max(1.0),
    );
    let _ = cr.stroke();
    cr.set_dash(&[], 0.0);
}

fn draw_import_placeholders(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    camera: Camera,
) {
    if document.imported_references.is_empty() && document.mesh_references.is_empty() {
        return;
    }
    if !document.imported_references.is_empty() {
        cr.set_line_width(1.2);
        cr.set_source_rgba(0.54, 0.78, 0.92, 0.42);
        let top_left = world_to_screen(Point { x: -140.0, y: 92.0 }, width, height, camera);
        cr.rectangle(
            top_left.x,
            top_left.y,
            280.0 * camera.zoom,
            184.0 * camera.zoom,
        );
        let _ = cr.stroke();
    }
    draw_mesh_reference_boxes(cr, width, height, document, camera);
}

fn draw_mesh_reference_boxes(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    camera: Camera,
) {
    for mesh in &document.mesh_references {
        let Some(bbox) = mesh.bounding_box else {
            continue;
        };
        let points = mesh_box_points(bbox.min, bbox.max)
            .map(|point| {
                project_mesh_point(point, mesh.transform.scale, mesh.transform.translation)
            })
            .map(|point| world_to_screen(point, width, height, camera));
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        cr.set_line_width(1.4);
        cr.set_source_rgba(0.40, 0.91, 0.98, 0.62);
        for (a, b) in edges {
            cr.move_to(points[a].x, points[a].y);
            cr.line_to(points[b].x, points[b].y);
        }
        let _ = cr.stroke();

        let center = Point3 {
            x: (bbox.min.x + bbox.max.x) * 0.5,
            y: (bbox.min.y + bbox.max.y) * 0.5,
            z: (bbox.min.z + bbox.max.z) * 0.5,
        };
        draw_3d_axis(
            cr,
            width,
            height,
            camera,
            center,
            mesh.transform.scale,
            mesh.transform.translation,
        );
    }
}

fn draw_3d_axis(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    origin: Point3,
    scale: f64,
    translation: Point3,
) {
    let length = 42.0 / scale.max(0.0001);
    let origin_screen = world_to_screen(
        project_mesh_point(origin, scale, translation),
        width,
        height,
        camera,
    );
    let axes = [
        (
            Point3 {
                x: origin.x + length,
                ..origin
            },
            (0.95, 0.25, 0.25),
            "X",
        ),
        (
            Point3 {
                y: origin.y + length,
                ..origin
            },
            (0.25, 0.85, 0.38),
            "Y",
        ),
        (
            Point3 {
                z: origin.z + length,
                ..origin
            },
            (0.38, 0.65, 1.0),
            "Z",
        ),
    ];
    cr.set_line_width(1.8);
    for (end, color, label) in axes {
        let end_screen = world_to_screen(
            project_mesh_point(end, scale, translation),
            width,
            height,
            camera,
        );
        cr.set_source_rgba(color.0, color.1, color.2, 0.82);
        cr.move_to(origin_screen.x, origin_screen.y);
        cr.line_to(end_screen.x, end_screen.y);
        let _ = cr.stroke();
        cr.move_to(end_screen.x + 4.0, end_screen.y - 4.0);
        let _ = cr.show_text(label);
    }
}

fn mesh_box_points(min: Point3, max: Point3) -> [Point3; 8] {
    [
        Point3 {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        Point3 {
            x: max.x,
            y: min.y,
            z: min.z,
        },
        Point3 {
            x: max.x,
            y: max.y,
            z: min.z,
        },
        Point3 {
            x: min.x,
            y: max.y,
            z: min.z,
        },
        Point3 {
            x: min.x,
            y: min.y,
            z: max.z,
        },
        Point3 {
            x: max.x,
            y: min.y,
            z: max.z,
        },
        Point3 {
            x: max.x,
            y: max.y,
            z: max.z,
        },
        Point3 {
            x: min.x,
            y: max.y,
            z: max.z,
        },
    ]
}

fn project_mesh_point(point: Point3, scale: f64, translation: Point3) -> Point {
    let x = point.x * scale + translation.x;
    let y = point.y * scale + translation.y;
    let z = point.z * scale + translation.z;
    Point {
        x: x - z * 0.42,
        y: y - z * 0.30,
    }
}

fn draw_entity(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    entity: &Entity,
    selected: bool,
    hovered: bool,
    color: Option<&str>,
    line_type: &str,
    line_weight: f64,
) {
    if hovered && !selected {
        cr.set_line_width(5.0);
        cr.set_source_rgba(0.22, 0.78, 1.0, 0.28);
        stroke_entity_path(cr, width, height, camera, entity);
    }

    let base_line_width = if selected {
        2.8
    } else if hovered {
        2.2
    } else {
        1.6
    };
    cr.set_line_width((base_line_width * (0.85 + line_weight.max(0.01))).clamp(0.8, 7.0));
    configure_line_type(cr, line_type);
    if selected {
        cr.set_source_rgba(1.0, 0.90, 0.45, 0.98);
    } else if hovered {
        cr.set_source_rgba(0.50, 0.86, 1.0, 0.98);
    } else if let Some((r, g, b)) = parse_hex_color(color.unwrap_or_default()) {
        cr.set_source_rgba(r, g, b, 0.95);
    } else {
        cr.set_source_rgba(0.64, 0.67, 0.72, 0.92);
    }
    match entity {
        Entity::Line { start, end, .. } | Entity::Guideline { start, end, .. } => {
            let a = world_to_screen(*start, width, height, camera);
            let b = world_to_screen(*end, width, height, camera);
            cr.move_to(a.x, a.y);
            cr.line_to(b.x, b.y);
            let _ = cr.stroke();
        }
        Entity::Dimension {
            start,
            end,
            label,
            style,
            ..
        } => {
            let (_, offset) = parse_dimension_style(style);
            let (ext_start, ext_end, dim_start, dim_end) = dimension_segments(*start, *end, offset);
            cr.set_source_rgba(0.88, 0.86, 1.0, 0.9);
            draw_preview_segment(cr, width, height, camera, ext_start, dim_start);
            draw_preview_segment(cr, width, height, camera, ext_end, dim_end);
            draw_preview_segment(cr, width, height, camera, dim_start, dim_end);
            let a = world_to_screen(dim_start, width, height, camera);
            let b = world_to_screen(dim_end, width, height, camera);
            cr.move_to((a.x + b.x) / 2.0 + 5.0, (a.y + b.y) / 2.0 - 5.0);
            let _ = cr.show_text(label);
        }
        Entity::Polyline { points, closed, .. } => {
            stroke_points(cr, width, height, camera, points, *closed);
        }
        Entity::Spline {
            control_points,
            closed,
            ..
        } => {
            let points = spline_display_points(control_points, *closed);
            if let Some(first) = points.first() {
                let _ = first;
                stroke_points(cr, width, height, camera, &points, *closed);
            }
        }
        Entity::Circle { center, radius, .. } => {
            let c = world_to_screen(*center, width, height, camera);
            cr.arc(c.x, c.y, *radius * camera.zoom, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
        }
        Entity::Point { point, .. } => {
            let p = world_to_screen(*point, width, height, camera);
            let radius = if camera.zoom < 0.08 {
                1.2
            } else if camera.zoom < 0.25 {
                1.8
            } else {
                3.0
            };
            cr.arc(p.x, p.y, radius, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }
        Entity::Text {
            origin,
            text,
            height: text_height,
            ..
        } => {
            if !selected && !hovered && text_height * camera.zoom < MIN_TEXT_SCREEN_HEIGHT {
                return;
            }
            let p = world_to_screen(*origin, width, height, camera);
            cr.set_font_size((text_height * camera.zoom).clamp(8.0, 42.0));
            cr.move_to(p.x, p.y);
            let _ = cr.show_text(text);
        }
        Entity::Hatch { boundary, .. } => {
            if !boundary.is_empty() {
                if let Some((r, g, b)) = parse_hex_color(color.unwrap_or_default()) {
                    cr.set_source_rgba(r, g, b, 0.30);
                } else {
                    cr.set_source_rgba(0.64, 0.67, 0.72, 0.28);
                }
                append_points_path(cr, width, height, camera, boundary, true);
                cr.close_path();
                let _ = cr.fill_preserve();
                if let Some((r, g, b)) = parse_hex_color(color.unwrap_or_default()) {
                    cr.set_source_rgba(r, g, b, 0.70);
                } else {
                    cr.set_source_rgba(0.64, 0.67, 0.72, 0.60);
                }
                let _ = cr.stroke();
            }
        }
        Entity::Table {
            origin,
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } => {
            let o = world_to_screen(*origin, width, height, camera);
            let w = *cell_width * *columns as f64 * camera.zoom;
            let h = *cell_height * *rows as f64 * camera.zoom;
            cr.set_source_rgba(0.78, 0.88, 1.0, 0.86);
            cr.rectangle(o.x, o.y, w, h);
            let _ = cr.stroke();
            for row in 1..*rows {
                let y = o.y + *cell_height * row as f64 * camera.zoom;
                cr.move_to(o.x, y);
                cr.line_to(o.x + w, y);
            }
            for column in 1..*columns {
                let x = o.x + *cell_width * column as f64 * camera.zoom;
                cr.move_to(x, o.y);
                cr.line_to(x, o.y + h);
            }
            let _ = cr.stroke();
        }
        Entity::BlockReference {
            insertion, name, ..
        } => {
            let p = world_to_screen(*insertion, width, height, camera);
            cr.set_source_rgba(0.96, 0.76, 0.45, 0.92);
            cr.rectangle(p.x - 8.0, p.y - 8.0, 16.0, 16.0);
            let _ = cr.stroke();
            cr.move_to(p.x + 12.0, p.y + 4.0);
            let _ = cr.show_text(name);
        }
    }
    cr.set_dash(&[], 0.0);
}

fn configure_line_type(cr: &gtk::cairo::Context, line_type: &str) {
    match line_type.to_ascii_lowercase().as_str() {
        "dashed" => cr.set_dash(&[10.0, 7.0], 0.0),
        "dotted" => cr.set_dash(&[2.0, 6.0], 0.0),
        "center" => cr.set_dash(&[14.0, 5.0, 3.0, 5.0], 0.0),
        _ => cr.set_dash(&[], 0.0),
    }
}

fn stroke_entity_path(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    entity: &Entity,
) {
    match entity {
        Entity::Line { start, end, .. }
        | Entity::Guideline { start, end, .. }
        | Entity::Dimension { start, end, .. } => {
            let a = world_to_screen(*start, width, height, camera);
            let b = world_to_screen(*end, width, height, camera);
            cr.move_to(a.x, a.y);
            cr.line_to(b.x, b.y);
            let _ = cr.stroke();
        }
        Entity::Polyline { points, closed, .. } => {
            stroke_points(cr, width, height, camera, points, *closed)
        }
        Entity::Spline {
            control_points,
            closed,
            ..
        } => {
            let points = spline_display_points(control_points, *closed);
            stroke_points(cr, width, height, camera, &points, *closed);
        }
        Entity::Circle { center, radius, .. } => {
            let c = world_to_screen(*center, width, height, camera);
            cr.arc(c.x, c.y, *radius * camera.zoom, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
        }
        Entity::Point { point, .. } | Entity::Text { origin: point, .. } => {
            let p = world_to_screen(*point, width, height, camera);
            cr.arc(p.x, p.y, 7.0, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }
        Entity::Hatch { boundary, .. } => stroke_points(cr, width, height, camera, boundary, true),
        Entity::Table {
            origin,
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } => {
            let o = world_to_screen(*origin, width, height, camera);
            cr.rectangle(
                o.x,
                o.y,
                *cell_width * *columns as f64 * camera.zoom,
                *cell_height * *rows as f64 * camera.zoom,
            );
            let _ = cr.stroke();
        }
        Entity::BlockReference { insertion, .. } => {
            let p = world_to_screen(*insertion, width, height, camera);
            cr.rectangle(p.x - 10.0, p.y - 10.0, 20.0, 20.0);
            let _ = cr.stroke();
        }
    }
}

fn stroke_points(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    points: &[Point],
    closed: bool,
) {
    if append_points_path(cr, width, height, camera, points, closed) {
        let _ = cr.stroke();
    }
}

fn append_points_path(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    points: &[Point],
    closed: bool,
) -> bool {
    let Some(first_world) = points.first().copied() else {
        return false;
    };
    let first = world_to_screen(first_world, width, height, camera);
    cr.move_to(first.x, first.y);

    let min_world_distance = MIN_SCREEN_VERTEX_DISTANCE / camera.zoom.max(MIN_ZOOM);
    let min_world_distance_sq = min_world_distance * min_world_distance;
    let min_screen_distance_sq = MIN_SCREEN_VERTEX_DISTANCE * MIN_SCREEN_VERTEX_DISTANCE;
    let mut last_world = first_world;
    let mut last_screen = first;
    let mut appended = 1usize;

    for (index, point) in points.iter().enumerate().skip(1) {
        let is_last = index + 1 == points.len();
        let world_dx = point.x - last_world.x;
        let world_dy = point.y - last_world.y;
        if !is_last && world_dx * world_dx + world_dy * world_dy < min_world_distance_sq {
            continue;
        }
        let screen = world_to_screen(*point, width, height, camera);
        let screen_dx = screen.x - last_screen.x;
        let screen_dy = screen.y - last_screen.y;
        if !is_last && screen_dx * screen_dx + screen_dy * screen_dy < min_screen_distance_sq {
            continue;
        }
        cr.line_to(screen.x, screen.y);
        last_world = *point;
        last_screen = screen;
        appended += 1;
    }

    if closed {
        cr.close_path();
    }
    appended > 1 || closed
}

pub(crate) fn update_selection_label(label: &gtk::Label, document: &Document, selected: &[u64]) {
    let text = match selected {
        [] => "Selection: none".to_string(),
        [id] => document
            .entity_summary(*id)
            .unwrap_or_else(|| "Selection: none".to_string()),
        ids => format!("Selection: {} entities", ids.len()),
    };
    label.set_text(&text);
}

fn hit_test(document: &Document, point: Point, tolerance: f64) -> Option<u64> {
    crate::cad::selection::legacy_hit::legacy_hit_entity_at(document, point, tolerance)
}

fn hit_distance(entity: &Entity, point: Point, tolerance: f64) -> Option<f64> {
    match entity {
        Entity::Point { point: p, .. } => Some(point.distance_to(*p)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(distance_to_segment(point, *start, *end)),
        Entity::Polyline { points, closed, .. } => {
            polyline_distance(point, points, *closed, Some(tolerance))
        }
        Entity::Spline {
            control_points,
            closed,
            ..
        } => polyline_distance(
            point,
            &spline_display_points(control_points, *closed),
            *closed,
            Some(tolerance),
        ),
        Entity::Circle { center, radius, .. } => Some((point.distance_to(*center) - *radius).abs()),
        Entity::Text {
            origin,
            text,
            height,
            ..
        } => {
            let width = (text.chars().count() as f64 * height.max(1.0) * 0.6).max(6.0);
            let h = height.max(2.5);
            let center = Point {
                x: origin.x + width * 0.5,
                y: origin.y + h * 0.5,
            };
            let rx = width * 0.5 + tolerance;
            let ry = h * 0.5 + tolerance;
            let dx = ((point.x - center.x) / rx).abs();
            let dy = ((point.y - center.y) / ry).abs();
            if dx <= 1.0 && dy <= 1.0 {
                Some(point.distance_to(*origin).min(tolerance * 0.35))
            } else {
                Some(point.distance_to(*origin))
            }
        }
        Entity::Table { origin, .. }
        | Entity::BlockReference {
            insertion: origin, ..
        } => Some(point.distance_to(*origin)),
        Entity::Hatch { boundary, .. } => polyline_distance(point, boundary, true, Some(tolerance)),
    }
}

fn polyline_distance(
    point: Point,
    points: &[Point],
    closed: bool,
    tolerance: Option<f64>,
) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut best = f64::INFINITY;
    for pair in points.windows(2) {
        if segment_near_point_bounds(pair[0], pair[1], point, tolerance) {
            best = best.min(distance_to_segment(point, pair[0], pair[1]));
        }
    }
    if closed && points.len() > 2 {
        let last = *points.last().unwrap_or(&points[0]);
        if segment_near_point_bounds(last, points[0], point, tolerance) {
            best = best.min(distance_to_segment(point, last, points[0]));
        }
    }
    let vertex_best = points
        .iter()
        .filter(|candidate| {
            tolerance
                .map(|tol| {
                    (candidate.x - point.x).abs() <= tol && (candidate.y - point.y).abs() <= tol
                })
                .unwrap_or(true)
        })
        .map(|candidate| point.distance_to(*candidate))
        .fold(f64::INFINITY, f64::min);
    let best = best.min(vertex_best);
    best.is_finite().then_some(best)
}

fn segment_near_point_bounds(a: Point, b: Point, point: Point, tolerance: Option<f64>) -> bool {
    let Some(tolerance) = tolerance else {
        return true;
    };
    point.x >= a.x.min(b.x) - tolerance
        && point.x <= a.x.max(b.x) + tolerance
        && point.y >= a.y.min(b.y) - tolerance
        && point.y <= a.y.max(b.y) + tolerance
}

fn spline_display_points(points: &[Point], closed: bool) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut display = Vec::with_capacity(points.len() * 8);
    let count = points.len();
    let segment_count = if closed { count } else { count - 1 };
    for index in 0..segment_count {
        let p0 = if index == 0 {
            if closed {
                points[count - 1]
            } else {
                points[0]
            }
        } else {
            points[index - 1]
        };
        let p1 = points[index];
        let p2 = points[(index + 1) % count];
        let p3 = if index + 2 < count {
            points[index + 2]
        } else if closed {
            points[(index + 2) % count]
        } else {
            points[count - 1]
        };

        for step in 0..8 {
            let t = step as f64 / 8.0;
            display.push(catmull_rom_point(p0, p1, p2, p3, t));
        }
    }
    if !closed {
        display.push(*points.last().unwrap_or(&points[0]));
    }
    display
}

fn catmull_rom_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;
    Point {
        x: 0.5
            * ((2.0 * p1.x)
                + (-p0.x + p2.x) * t
                + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3),
        y: 0.5
            * ((2.0 * p1.y)
                + (-p0.y + p2.y) * t
                + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3),
    }
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return point.distance_to(start);
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    let closest = Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    };
    point.distance_to(closest)
}

fn find_snap(
    document: &Document,
    point: Point,
    pending_start: Option<Point>,
    tolerance: f64,
) -> Option<SnapTarget> {
    let mut candidates = Vec::new();
    collect_point_snaps(document, point, tolerance, &mut candidates);
    if document.entities.len() <= FULL_SNAP_ENTITY_LIMIT {
        collect_intersection_snaps(document, &mut candidates);
        collect_derived_midpoint_snaps(document, &mut candidates);
    }
    collect_circle_edge_snaps(document, point, &mut candidates);
    if let Some(start) = pending_start {
        collect_circle_radius_snaps(document, point, start, &mut candidates);
        collect_perpendicular_snaps(document, point, start, &mut candidates);
        collect_ortho_snap(point, start, &mut candidates);
    }
    let primary_snap = candidates
        .into_iter()
        .filter_map(|snap| {
            let distance = snap.point.distance_to(point);
            (distance <= tolerance).then_some((snap, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(snap, _)| snap);
    if primary_snap.is_some() {
        return primary_snap;
    }

    pending_start.and_then(|start| find_parallel_snap(document, point, start, tolerance * 0.45))
}

fn collect_point_snaps(
    document: &Document,
    pointer: Point,
    tolerance: f64,
    out: &mut Vec<SnapTarget>,
) {
    let query_bounds = (
        Point {
            x: pointer.x - tolerance,
            y: pointer.y - tolerance,
        },
        Point {
            x: pointer.x + tolerance,
            y: pointer.y + tolerance,
        },
    );
    for entity in document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
        .filter(|entity| {
            document
                .cached_entity_bounds(entity)
                .map(|bounds| bounds_intersect(bounds, query_bounds))
                .unwrap_or(true)
        })
    {
        match entity {
            Entity::Point { point, .. } => out.push(snap(*point, SnapKind::Endpoint)),
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => {
                out.push(snap(*start, SnapKind::Endpoint));
                out.push(snap(*end, SnapKind::Endpoint));
                out.push(snap(midpoint(*start, *end), SnapKind::Midpoint));
            }
            Entity::Polyline { points, closed, .. } => {
                for point in points {
                    if point.distance_to(pointer) <= tolerance {
                        out.push(snap(*point, SnapKind::Endpoint));
                    }
                }
                for pair in points.windows(2) {
                    let midpoint = midpoint(pair[0], pair[1]);
                    if midpoint.distance_to(pointer) <= tolerance {
                        out.push(snap(midpoint, SnapKind::Midpoint));
                    }
                }
                if *closed && points.len() > 2 {
                    let midpoint = midpoint(*points.last().unwrap_or(&points[0]), points[0]);
                    if midpoint.distance_to(pointer) <= tolerance {
                        out.push(snap(midpoint, SnapKind::Midpoint));
                    }
                }
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                for point in control_points {
                    if point.distance_to(pointer) <= tolerance {
                        out.push(snap(*point, SnapKind::Endpoint));
                    }
                }
                let points = spline_display_points(control_points, *closed);
                for pair in points.windows(2) {
                    let midpoint = midpoint(pair[0], pair[1]);
                    if midpoint.distance_to(pointer) <= tolerance {
                        out.push(snap(midpoint, SnapKind::Midpoint));
                    }
                }
            }
            Entity::Hatch {
                boundary: points, ..
            } => {
                for point in points {
                    if point.distance_to(pointer) <= tolerance {
                        out.push(snap(*point, SnapKind::Endpoint));
                    }
                }
                for pair in points.windows(2) {
                    let midpoint = midpoint(pair[0], pair[1]);
                    if midpoint.distance_to(pointer) <= tolerance {
                        out.push(snap(midpoint, SnapKind::Midpoint));
                    }
                }
                if points.len() > 2 {
                    let midpoint = midpoint(*points.last().unwrap_or(&points[0]), points[0]);
                    if midpoint.distance_to(pointer) <= tolerance {
                        out.push(snap(midpoint, SnapKind::Midpoint));
                    }
                }
            }
            Entity::Circle { center, radius, .. } => {
                out.push(snap(*center, SnapKind::Center));
                out.push(snap(
                    Point {
                        x: center.x + radius,
                        y: center.y,
                    },
                    SnapKind::Quadrant,
                ));
                out.push(snap(
                    Point {
                        x: center.x - radius,
                        y: center.y,
                    },
                    SnapKind::Quadrant,
                ));
                out.push(snap(
                    Point {
                        x: center.x,
                        y: center.y + radius,
                    },
                    SnapKind::Quadrant,
                ));
                out.push(snap(
                    Point {
                        x: center.x,
                        y: center.y - radius,
                    },
                    SnapKind::Quadrant,
                ));
            }
            Entity::Text { origin, .. }
            | Entity::Table { origin, .. }
            | Entity::BlockReference {
                insertion: origin, ..
            } => out.push(snap(*origin, SnapKind::Endpoint)),
        }
    }
}

fn collect_intersection_snaps(document: &Document, out: &mut Vec<SnapTarget>) {
    let segments = entity_segments(document);
    for (index, first) in segments.iter().enumerate() {
        for second in segments.iter().skip(index + 1) {
            if let Some(point) = segment_intersection(first.0, first.1, second.0, second.1) {
                out.push(snap(point, SnapKind::Intersection));
            }
        }
    }
}

fn collect_derived_midpoint_snaps(document: &Document, out: &mut Vec<SnapTarget>) {
    let segments = entity_segments(document);
    for (index, &(a, b)) in segments.iter().enumerate() {
        let mut cuts = vec![0.0, 1.0];
        for (other_index, &(c, d)) in segments.iter().enumerate() {
            if index == other_index {
                continue;
            }
            if let Some((_, t)) = segment_intersection_with_t(a, b, c, d) {
                if t > 1e-6 && t < 1.0 - 1e-6 {
                    cuts.push(t);
                }
            }
        }
        cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for pair in cuts.windows(2) {
            let t = (pair[0] + pair[1]) * 0.5;
            out.push(snap(point_on_segment(a, b, t), SnapKind::Midpoint));
        }
    }
}

fn collect_circle_edge_snaps(document: &Document, point: Point, out: &mut Vec<SnapTarget>) {
    for entity in document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
    {
        if let Entity::Circle { center, radius, .. } = entity {
            if let Some(edge) = circle_edge_point(*center, *radius, point) {
                out.push(snap(edge, SnapKind::CircleEdge));
            }
        }
    }
}

fn collect_circle_radius_snaps(
    document: &Document,
    point: Point,
    start: Point,
    out: &mut Vec<SnapTarget>,
) {
    for entity in document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
    {
        if let Entity::Circle { center, radius, .. } = entity {
            if start.distance_to(*center) < radius.abs().max(1.0) * 0.02 {
                if let Some(edge) = circle_edge_point(*center, *radius, point) {
                    out.push(snap(edge, SnapKind::CircleEdge));
                }
            }
        }
    }
}

fn collect_perpendicular_snaps(
    document: &Document,
    point: Point,
    start: Point,
    out: &mut Vec<SnapTarget>,
) {
    for (a, b) in entity_segments(document) {
        let projected = project_point_to_segment(start, a, b);
        if projected.distance_to(point) < point.distance_to(start).max(1.0) {
            out.push(snap(projected, SnapKind::Perpendicular));
        }
    }
}

fn find_parallel_snap(
    document: &Document,
    point: Point,
    start: Point,
    tolerance: f64,
) -> Option<SnapTarget> {
    let pointer_dx = point.x - start.x;
    let pointer_dy = point.y - start.y;
    let pointer_length = (pointer_dx * pointer_dx + pointer_dy * pointer_dy).sqrt();
    if pointer_length <= 1e-9 {
        return None;
    }

    let mut best = None::<(SnapTarget, f64)>;
    for (a, b) in entity_segments(document) {
        let segment_dx = b.x - a.x;
        let segment_dy = b.y - a.y;
        let segment_length = (segment_dx * segment_dx + segment_dy * segment_dy).sqrt();
        if segment_length <= 1e-9 {
            continue;
        }

        let ux = segment_dx / segment_length;
        let uy = segment_dy / segment_length;
        let dot = pointer_dx * ux + pointer_dy * uy;
        let projected = Point {
            x: start.x + ux * dot,
            y: start.y + uy * dot,
        };
        let distance = projected.distance_to(point);
        if distance <= tolerance.min(pointer_length.max(1.0) * 0.02) {
            let candidate = snap(projected, SnapKind::Parallel);
            if best
                .as_ref()
                .map(|(_, best_distance)| distance < *best_distance)
                .unwrap_or(true)
            {
                best = Some((candidate, distance));
            }
        }
    }
    best.map(|(snap, _)| snap)
}

fn collect_ortho_snap(point: Point, start: Point, out: &mut Vec<SnapTarget>) {
    let dx = (point.x - start.x).abs();
    let dy = (point.y - start.y).abs();
    if dx < dy {
        out.push(snap(
            Point {
                x: start.x,
                y: point.y,
            },
            SnapKind::Ortho,
        ));
    } else {
        out.push(snap(
            Point {
                x: point.x,
                y: start.y,
            },
            SnapKind::Ortho,
        ));
    }
}

fn entity_segments(document: &Document) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for entity in document
        .entities
        .iter()
        .filter(|entity| document.entity_visible_in_active_layout(entity.id()))
    {
        match entity {
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => segments.push((*start, *end)),
            Entity::Polyline { points, closed, .. } => {
                push_polyline_segments(&mut segments, points, *closed)
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                let points = spline_display_points(control_points, *closed);
                push_polyline_segments(&mut segments, &points, *closed);
            }
            Entity::Hatch { boundary, .. } => push_polyline_segments(&mut segments, boundary, true),
            Entity::Table {
                origin,
                rows,
                columns,
                cell_width,
                cell_height,
                ..
            } => {
                let width = *columns as f64 * *cell_width;
                let height = *rows as f64 * *cell_height;
                let a = *origin;
                let b = Point {
                    x: origin.x + width,
                    y: origin.y,
                };
                let c = Point {
                    x: origin.x + width,
                    y: origin.y + height,
                };
                let d = Point {
                    x: origin.x,
                    y: origin.y + height,
                };
                segments.extend([(a, b), (b, c), (c, d), (d, a)]);
            }
            _ => {}
        }
    }
    segments
}

fn push_polyline_segments(segments: &mut Vec<(Point, Point)>, points: &[Point], closed: bool) {
    for pair in points.windows(2) {
        segments.push((pair[0], pair[1]));
    }
    if closed && points.len() > 2 {
        segments.push((*points.last().unwrap_or(&points[0]), points[0]));
    }
}

fn segment_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    segment_intersection_with_t(a, b, c, d).map(|(point, _)| point)
}

fn segment_intersection_with_t(a: Point, b: Point, c: Point, d: Point) -> Option<(Point, f64)> {
    let denominator = (a.x - b.x) * (c.y - d.y) - (a.y - b.y) * (c.x - d.x);
    if denominator.abs() < 1e-9 {
        return None;
    }
    let t = ((a.x - c.x) * (c.y - d.y) - (a.y - c.y) * (c.x - d.x)) / denominator;
    let u = -((a.x - b.x) * (a.y - c.y) - (a.y - b.y) * (a.x - c.x)) / denominator;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((point_on_segment(a, b, t), t))
    } else {
        None
    }
}

fn point_on_segment(start: Point, end: Point, t: f64) -> Point {
    Point {
        x: start.x + t * (end.x - start.x),
        y: start.y + t * (end.y - start.y),
    }
}

fn circle_edge_point(center: Point, radius: f64, point: Point) -> Option<Point> {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON || radius <= 0.0 {
        return None;
    }
    Some(Point {
        x: center.x + dx / length * radius,
        y: center.y + dy / length * radius,
    })
}

fn project_point_to_segment(point: Point, start: Point, end: Point) -> Point {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return start;
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    }
}

fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    }
}

fn snap(point: Point, kind: SnapKind) -> SnapTarget {
    SnapTarget { point, kind }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_for_creation_rejects_empty() {
        assert!(normalize_text_for_creation("").is_none());
        assert!(normalize_text_for_creation("   ").is_none());
    }

    #[test]
    fn normalize_text_for_creation_accepts_content() {
        assert_eq!(
            normalize_text_for_creation("  Hello LixCAD  "),
            Some("Hello LixCAD".to_string())
        );
    }

    #[test]
    fn build_text_entity_uses_input_content() {
        let entity = build_text_entity(44, Point { x: 10.0, y: 20.0 }, "Note".to_string());
        let Entity::Text {
            id, origin, text, ..
        } = entity
        else {
            panic!("expected text entity");
        };
        assert_eq!(id, 44);
        assert_eq!(text, "Note");
        assert!((origin.x - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn inline_edit_same_text_skips_change() {
        assert!(!should_apply_inline_text_edit("Hello", " Hello "));
    }

    #[test]
    fn inline_edit_new_text_applies_change() {
        assert!(should_apply_inline_text_edit("Hello", "Hello 2"));
    }
}

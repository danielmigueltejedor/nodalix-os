use crate::{
    document::{Document, Entity},
    geometry::Point,
    tools::Tool,
};
use gtk::{prelude::*, DrawingArea};
use std::{cell::RefCell, rc::Rc};

const MIN_ZOOM: f64 = 0.08;
const MAX_ZOOM: f64 = 24.0;
const ZOOM_STEP: f64 = 1.18;
const SNAP_SCREEN_TOLERANCE: f64 = 13.0;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
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
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct CadCanvas {
    area: DrawingArea,
    cursor: Rc<RefCell<Point>>,
    camera: Rc<RefCell<Camera>>,
    pending_start: Rc<RefCell<Option<Point>>>,
    polyline_vertices: Rc<RefCell<Vec<Point>>>,
    hover_point: Rc<RefCell<Option<Point>>>,
}

impl CadCanvas {
    pub fn new(
        document: Rc<RefCell<Document>>,
        active_tool: Rc<RefCell<Tool>>,
        selected_entity: Rc<RefCell<Option<u64>>>,
        selection_label: gtk::Label,
    ) -> Self {
        let area = DrawingArea::new();
        area.add_css_class("cad-canvas");
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_content_width(900);
        area.set_content_height(620);

        let cursor = Rc::new(RefCell::new(Point::default()));
        let camera = Rc::new(RefCell::new(Camera::default()));
        let pointer = Rc::new(RefCell::new(None::<(f64, f64)>));
        let snap_target = Rc::new(RefCell::new(None::<SnapTarget>));
        let pending_start = Rc::new(RefCell::new(None));
        let polyline_vertices = Rc::new(RefCell::new(Vec::<Point>::new()));
        let hover_point = Rc::new(RefCell::new(None::<Point>));
        let draw_document = document.clone();
        let draw_pending = pending_start.clone();
        let draw_polyline = polyline_vertices.clone();
        let draw_hover = hover_point.clone();
        let draw_camera = camera.clone();
        let draw_selected = selected_entity.clone();
        let draw_snap = snap_target.clone();
        area.set_draw_func(move |_, cr, width, height| {
            draw_scene(
                cr,
                width as f64,
                height as f64,
                &draw_document.borrow(),
                *draw_pending.borrow(),
                &draw_polyline.borrow(),
                *draw_hover.borrow(),
                *draw_camera.borrow(),
                *draw_selected.borrow(),
                *draw_snap.borrow(),
            );
        });

        let click = gtk::GestureClick::new();
        click.set_button(1);
        let click_document = document.clone();
        let click_tool = active_tool.clone();
        let click_pending = pending_start.clone();
        let click_polyline = polyline_vertices.clone();
        let queue_area = area.clone();
        let click_camera = camera.clone();
        let click_selected = selected_entity.clone();
        let click_selection_label = selection_label.clone();
        let click_snap = snap_target.clone();
        click.connect_pressed(move |_, _, x, y| {
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
                *click_selected.borrow_mut() = hit;
                update_selection_label(&click_selection_label, &click_document.borrow(), hit);
                *click_pending.borrow_mut() = None;
            } else {
                handle_click(
                    &mut click_document.borrow_mut(),
                    tool,
                    point,
                    &click_pending,
                    &click_polyline,
                );
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
            let area = area.clone();
            motion.connect_motion(move |_, x, y| {
                *pointer.borrow_mut() = Some((x, y));
                let camera = *camera.borrow();
                let point =
                    screen_to_world(x, y, area.width() as f64, area.height() as f64, camera);
                let snap = find_snap(
                    &document.borrow(),
                    point,
                    pending_start
                        .borrow()
                        .as_ref()
                        .copied()
                        .or_else(|| polyline_vertices.borrow().last().copied()),
                    SNAP_SCREEN_TOLERANCE / camera.zoom,
                );
                let effective = snap.map(|snap| snap.point).unwrap_or(point);
                *hover_point.borrow_mut() = Some(effective);
                *snap_target.borrow_mut() = snap;
                area.queue_draw();
            });
        }
        {
            let pointer = pointer.clone();
            let snap_target = snap_target.clone();
            let hover_point = hover_point.clone();
            motion.connect_leave(move |_| {
                *pointer.borrow_mut() = None;
                *snap_target.borrow_mut() = None;
                *hover_point.borrow_mut() = None;
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

        let move_drag = gtk::GestureDrag::new();
        move_drag.set_button(1);
        let drag_last_world = Rc::new(RefCell::new(None::<Point>));
        {
            let document = document.clone();
            let active_tool = active_tool.clone();
            let selected_entity = selected_entity.clone();
            let selection_label = selection_label.clone();
            let camera = camera.clone();
            let drag_last_world = drag_last_world.clone();
            let area = area.clone();
            move_drag.connect_drag_begin(move |_, x, y| {
                if !matches!(*active_tool.borrow(), Tool::Select | Tool::Modify) {
                    *drag_last_world.borrow_mut() = None;
                    return;
                }
                let point = screen_to_world(
                    x,
                    y,
                    area.width() as f64,
                    area.height() as f64,
                    *camera.borrow(),
                );
                let hit = hit_test(&document.borrow(), point, 10.0 / camera.borrow().zoom);
                if hit.is_some() {
                    *selected_entity.borrow_mut() = hit;
                    update_selection_label(&selection_label, &document.borrow(), hit);
                    *drag_last_world.borrow_mut() = Some(point);
                    area.queue_draw();
                }
            });
        }
        {
            let document = document.clone();
            let selected_entity = selected_entity.clone();
            let camera = camera.clone();
            let drag_last_world = drag_last_world.clone();
            let area = area.clone();
            move_drag.connect_drag_update(move |gesture, dx, dy| {
                let Some(id) = *selected_entity.borrow() else {
                    return;
                };
                let Some((start_x, start_y)) = gesture.start_point() else {
                    return;
                };
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
                document.borrow_mut().translate_entity(
                    id,
                    point.x - previous.x,
                    point.y - previous.y,
                );
                *drag_last_world.borrow_mut() = Some(point);
                area.queue_draw();
            });
        }
        {
            let drag_last_world = drag_last_world.clone();
            move_drag.connect_drag_end(move |_, _, _| {
                *drag_last_world.borrow_mut() = None;
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
                camera.pan_x = start.pan_x - dx / start.zoom;
                camera.pan_y = start.pan_y + dy / start.zoom;
                area.queue_draw();
            });
        }
        area.add_controller(drag);

        Self {
            area,
            cursor,
            camera,
            pending_start,
            polyline_vertices,
            hover_point,
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

    pub fn finish_polyline(&self, document: &Rc<RefCell<Document>>) {
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
        }
        vertices.clear();
        *self.pending_start.borrow_mut() = None;
        self.area.queue_draw();
    }

    pub fn cancel_interaction(&self) {
        self.polyline_vertices.borrow_mut().clear();
        *self.pending_start.borrow_mut() = None;
        *self.hover_point.borrow_mut() = None;
        self.area.queue_draw();
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
    Point {
        x: (x - width / 2.0) / camera.zoom + camera.pan_x,
        y: (height / 2.0 - y) / camera.zoom + camera.pan_y,
    }
}

fn world_to_screen(point: Point, width: f64, height: f64, camera: Camera) -> Point {
    Point {
        x: width / 2.0 + (point.x - camera.pan_x) * camera.zoom,
        y: height / 2.0 - (point.y - camera.pan_y) * camera.zoom,
    }
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
    tool: Tool,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    polyline_vertices: &Rc<RefCell<Vec<Point>>>,
) {
    match tool {
        Tool::Line => two_point_entity(document, point, pending, |id, start, end| Entity::Line {
            id,
            layer: "Default".to_string(),
            start,
            end,
        }),
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
        Tool::Rectangle => two_point_entity(document, point, pending, |id, start, end| {
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
        }),
        Tool::Circle => {
            two_point_entity(document, point, pending, |id, start, end| Entity::Circle {
                id,
                layer: "Default".to_string(),
                center: start,
                radius: start.distance_to(end),
            })
        }
        Tool::Arc => two_point_entity(document, point, pending, |id, start, end| Entity::Circle {
            id,
            layer: "Default".to_string(),
            center: start,
            radius: start.distance_to(end),
        }),
        Tool::Text => {
            document.add_entity(Entity::Text {
                id: document.next_id(),
                layer: "Default".to_string(),
                origin: point,
                text: "Text".to_string(),
                height: 2.5,
                rotation: 0.0,
            });
        }
        Tool::Table => {
            document.add_entity(Entity::Table {
                id: document.next_id(),
                layer: "Default".to_string(),
                origin: point,
                rows: 3,
                columns: 4,
                cell_width: 18.0,
                cell_height: 7.0,
            });
        }
        Tool::Dimension | Tool::Measure => {
            two_point_entity(document, point, pending, |id, start, end| {
                Entity::Dimension {
                    id,
                    layer: "Dimensions".to_string(),
                    start,
                    end,
                    label: format!("{:.2}", start.distance_to(end)),
                    style: "ISO-25".to_string(),
                    precision: 2,
                }
            });
        }
        Tool::Hatch => two_point_entity(document, point, pending, |id, start, end| Entity::Hatch {
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
        }),
        Tool::Block => {
            document.add_entity(Entity::BlockReference {
                id: document.next_id(),
                layer: "Default".to_string(),
                name: "Block".to_string(),
                insertion: point,
                scale: 1.0,
                rotation: 0.0,
            });
        }
        Tool::Guideline | Tool::Parametric => {
            two_point_entity(document, point, pending, |id, start, end| {
                Entity::Guideline {
                    id,
                    layer: "Construction".to_string(),
                    start,
                    end,
                    construction: true,
                }
            });
        }
        _ => {
            *pending.borrow_mut() = None;
        }
    }
}

fn two_point_entity<F>(
    document: &mut Document,
    point: Point,
    pending: &Rc<RefCell<Option<Point>>>,
    build: F,
) where
    F: FnOnce(u64, Point, Point) -> Entity,
{
    let mut pending_ref = pending.borrow_mut();
    if let Some(start) = *pending_ref {
        let entity = build(document.next_id(), start, point);
        document.add_entity(entity);
        *pending_ref = None;
    } else {
        *pending_ref = Some(point);
    }
}

fn draw_scene(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    document: &Document,
    pending: Option<Point>,
    polyline_vertices: &[Point],
    hover_point: Option<Point>,
    camera: Camera,
    selected_entity: Option<u64>,
    snap: Option<SnapTarget>,
) {
    draw_grid(cr, width, height, camera);
    draw_import_placeholders(cr, width, height, document, camera);
    for entity in &document.entities {
        draw_entity(
            cr,
            width,
            height,
            camera,
            entity,
            selected_entity == Some(entity.id()),
        );
    }
    if let Some(point) = pending {
        draw_pending_point(cr, width, height, camera, point);
        if let Some(hover) = hover_point {
            draw_preview_segment(cr, width, height, camera, point, hover);
        }
    }
    draw_polyline_preview(cr, width, height, camera, polyline_vertices, hover_point);
    if let Some(snap) = snap {
        draw_snap_marker(cr, width, height, camera, snap);
    }
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

fn draw_entity(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: Camera,
    entity: &Entity,
    selected: bool,
) {
    cr.set_line_width(if selected { 2.6 } else { 1.6 });
    if selected {
        cr.set_source_rgba(1.0, 0.90, 0.45, 0.98);
    } else {
        cr.set_source_rgba(0.80, 0.65, 0.97, 0.95);
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
            start, end, label, ..
        } => {
            let a = world_to_screen(*start, width, height, camera);
            let b = world_to_screen(*end, width, height, camera);
            cr.set_source_rgba(0.88, 0.86, 1.0, 0.9);
            cr.move_to(a.x, a.y);
            cr.line_to(b.x, b.y);
            let _ = cr.stroke();
            cr.move_to((a.x + b.x) / 2.0 + 5.0, (a.y + b.y) / 2.0 - 5.0);
            let _ = cr.show_text(label);
        }
        Entity::Polyline { points, closed, .. } => {
            if let Some(first) = points.first() {
                let first = world_to_screen(*first, width, height, camera);
                cr.move_to(first.x, first.y);
                for point in points.iter().skip(1) {
                    let p = world_to_screen(*point, width, height, camera);
                    cr.line_to(p.x, p.y);
                }
                if *closed {
                    cr.close_path();
                }
                let _ = cr.stroke();
            }
        }
        Entity::Circle { center, radius, .. } => {
            let c = world_to_screen(*center, width, height, camera);
            cr.arc(c.x, c.y, *radius * camera.zoom, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
        }
        Entity::Point { point, .. } => {
            let p = world_to_screen(*point, width, height, camera);
            cr.arc(p.x, p.y, 3.0, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }
        Entity::Text {
            origin,
            text,
            height: text_height,
            ..
        } => {
            let p = world_to_screen(*origin, width, height, camera);
            cr.set_font_size((text_height * camera.zoom).clamp(8.0, 42.0));
            cr.move_to(p.x, p.y);
            let _ = cr.show_text(text);
        }
        Entity::Hatch {
            boundary, pattern, ..
        } => {
            if let Some(first) = boundary.first() {
                let first = world_to_screen(*first, width, height, camera);
                cr.set_source_rgba(0.80, 0.65, 0.97, 0.22);
                cr.move_to(first.x, first.y);
                for point in boundary.iter().skip(1) {
                    let p = world_to_screen(*point, width, height, camera);
                    cr.line_to(p.x, p.y);
                }
                cr.close_path();
                let _ = cr.fill_preserve();
                cr.set_source_rgba(0.80, 0.65, 0.97, 0.72);
                let _ = cr.stroke();
                cr.move_to(first.x + 6.0, first.y + 16.0);
                let _ = cr.show_text(pattern);
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
}

fn update_selection_label(label: &gtk::Label, document: &Document, selected: Option<u64>) {
    let text = selected
        .and_then(|id| document.entity_summary(id))
        .unwrap_or_else(|| "Selection: none".to_string());
    label.set_text(&text);
}

fn hit_test(document: &Document, point: Point, tolerance: f64) -> Option<u64> {
    document
        .entities
        .iter()
        .filter_map(|entity| hit_distance(entity, point).map(|distance| (entity.id(), distance)))
        .filter(|(_, distance)| *distance <= tolerance)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

fn hit_distance(entity: &Entity, point: Point) -> Option<f64> {
    match entity {
        Entity::Point { point: p, .. } => Some(point.distance_to(*p)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(distance_to_segment(point, *start, *end)),
        Entity::Polyline { points, closed, .. } => polyline_distance(point, points, *closed),
        Entity::Circle { center, radius, .. } => Some((point.distance_to(*center) - *radius).abs()),
        Entity::Text { origin, .. }
        | Entity::Table { origin, .. }
        | Entity::BlockReference {
            insertion: origin, ..
        } => Some(point.distance_to(*origin)),
        Entity::Hatch { boundary, .. } => polyline_distance(point, boundary, true),
    }
}

fn polyline_distance(point: Point, points: &[Point], closed: bool) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut best = points
        .windows(2)
        .map(|pair| distance_to_segment(point, pair[0], pair[1]))
        .fold(f64::INFINITY, f64::min);
    if closed && points.len() > 2 {
        best = best.min(distance_to_segment(
            point,
            *points.last().unwrap_or(&points[0]),
            points[0],
        ));
    }
    Some(
        best.min(
            points
                .iter()
                .map(|candidate| point.distance_to(*candidate))
                .fold(f64::INFINITY, f64::min),
        ),
    )
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
    collect_point_snaps(document, &mut candidates);
    collect_intersection_snaps(document, &mut candidates);
    collect_derived_midpoint_snaps(document, &mut candidates);
    collect_circle_edge_snaps(document, point, &mut candidates);
    if let Some(start) = pending_start {
        collect_circle_radius_snaps(document, point, start, &mut candidates);
        collect_perpendicular_snaps(document, point, start, &mut candidates);
        collect_ortho_snap(point, start, &mut candidates);
    }
    candidates
        .into_iter()
        .filter_map(|snap| {
            let distance = snap.point.distance_to(point);
            (distance <= tolerance).then_some((snap, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(snap, _)| snap)
}

fn collect_point_snaps(document: &Document, out: &mut Vec<SnapTarget>) {
    for entity in &document.entities {
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
                    out.push(snap(*point, SnapKind::Endpoint));
                }
                for pair in points.windows(2) {
                    out.push(snap(midpoint(pair[0], pair[1]), SnapKind::Midpoint));
                }
                if *closed && points.len() > 2 {
                    out.push(snap(
                        midpoint(*points.last().unwrap_or(&points[0]), points[0]),
                        SnapKind::Midpoint,
                    ));
                }
            }
            Entity::Hatch {
                boundary: points, ..
            } => {
                for point in points {
                    out.push(snap(*point, SnapKind::Endpoint));
                }
                for pair in points.windows(2) {
                    out.push(snap(midpoint(pair[0], pair[1]), SnapKind::Midpoint));
                }
                if points.len() > 2 {
                    out.push(snap(
                        midpoint(*points.last().unwrap_or(&points[0]), points[0]),
                        SnapKind::Midpoint,
                    ));
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
    for entity in &document.entities {
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
    for entity in &document.entities {
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
    for entity in &document.entities {
        match entity {
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => segments.push((*start, *end)),
            Entity::Polyline { points, closed, .. } => {
                push_polyline_segments(&mut segments, points, *closed)
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

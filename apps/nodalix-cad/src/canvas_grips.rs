//! Selection grip drawing and hit testing for the canvas.

use crate::{
    cad::geometry::{collect_grips, grip_is_editable, hit_test_grip, Grip, GripKind},
    document::Document,
    geometry::Point,
};
use gtk::prelude::*;

pub fn selection_grips(document: &Document, selected: &[u64]) -> Vec<Grip> {
    selected
        .iter()
        .copied()
        .filter(|id| document.entity_visible_in_active_layout(*id))
        .filter_map(|id| document.entities.iter().find(|entity| entity.id() == id))
        .flat_map(collect_grips)
        .collect()
}

pub fn grip_locked(document: &Document, grip: &Grip) -> bool {
    document.entity_layer_locked(grip.entity_id)
}

pub fn draw_selection_grips(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    camera: crate::canvas::Camera,
    document: &Document,
    selected: &[u64],
) {
    let grips = selection_grips(document, selected);
    if grips.is_empty() {
        return;
    }
    for grip in grips {
        let screen = crate::canvas::world_to_screen(grip.position, width, height, camera);
        let x = screen.x;
        let y = screen.y;
        let locked = grip_locked(document, &grip);
        let editable = grip_is_editable(grip.kind) && !locked;
        let (r, g, b, size) = grip_style(grip.kind, editable);
        cr.set_source_rgb(r, g, b);
        if matches!(grip.kind, GripKind::LineMid) {
            draw_diamond(cr, x, y, size);
        } else if matches!(grip.kind, GripKind::CircleRadius) {
            cr.arc(x, y, size * 0.9, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
        } else {
            cr.rectangle(x - size, y - size, size * 2.0, size * 2.0);
            cr.fill().ok();
        }
    }
}

fn grip_style(kind: GripKind, editable: bool) -> (f64, f64, f64, f64) {
    if !editable {
        return (0.55, 0.55, 0.55, 4.0);
    }
    match kind {
        GripKind::LineMid => (0.35, 0.85, 0.45, 5.0),
        GripKind::CircleRadius => (0.95, 0.75, 0.25, 5.0),
        _ => (0.25, 0.65, 1.0, 5.0),
    }
}

fn draw_diamond(cr: &gtk::cairo::Context, x: f64, y: f64, size: f64) {
    cr.move_to(x, y - size);
    cr.line_to(x + size, y);
    cr.line_to(x, y + size);
    cr.line_to(x - size, y);
    cr.close_path();
    cr.fill().ok();
}

pub fn hit_test_selection_grip(
    document: &Document,
    selected: &[u64],
    point: Point,
    tolerance: f64,
) -> Option<Grip> {
    let grips = selection_grips(document, selected);
    hit_test_grip(&grips, point, tolerance).filter(|grip| {
        grip_is_editable(grip.kind) && document.entity_visible_in_active_layout(grip.entity_id)
    })
}

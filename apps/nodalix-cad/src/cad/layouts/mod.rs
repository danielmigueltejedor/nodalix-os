//! Layout / paper-space helpers (pure logic, testable without GTK).

use crate::{
    document::{Document, LayoutKind, LayoutViewport},
    geometry::Point,
};

/// Log layout import diagnostics when `LIXCAD_LAYOUT_DEBUG=1`.
pub fn layout_debug_log(message: &str) {
    if std::env::var("LIXCAD_LAYOUT_DEBUG")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        eprintln!("[lixcad-layout] {message}");
    }
}

/// Normalize layout tab names (Model, paper layouts).
pub fn normalize_layout_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() || name.eq_ignore_ascii_case("model") {
        "Model".to_string()
    } else {
        name.to_string()
    }
}

/// Whether an entity belongs to model space (not assigned to a paper layout).
pub fn is_model_space_entity(document: &Document, entity_id: u64) -> bool {
    document.entity_layout(entity_id) == "Model"
}

/// Entities owned by a specific layout name (paper space); model space uses `"Model"`.
pub fn entities_in_layout<'a>(
    document: &'a Document,
    layout_name: &str,
) -> impl Iterator<Item = &'a crate::document::Entity> + 'a {
    let layout_name = normalize_layout_name(layout_name);
    document.entities.iter().filter(move |entity| {
        normalize_layout_name(document.entity_layout(entity.id())) == layout_name
    })
}

/// Viewports for a layout tab.
pub fn viewports_for_layout<'a>(
    document: &'a Document,
    layout_name: &str,
) -> impl Iterator<Item = &'a LayoutViewport> + 'a {
    let layout_name = normalize_layout_name(layout_name);
    document
        .layout_viewports
        .iter()
        .filter(move |viewport| normalize_layout_name(&viewport.layout) == layout_name)
}

/// World bounds used to fit the camera on the active layout tab.
pub fn active_layout_fit_bounds(document: &Document) -> Option<(Point, Point)> {
    if document.active_layout_kind() == LayoutKind::Paper {
        let layout = document.active_layout_ref()?;
        let paper = (
            Point { x: 0.0, y: 0.0 },
            Point {
                x: layout.paper.width,
                y: layout.paper.height,
            },
        );
        let entity_bounds = entities_in_layout(document, &document.active_layout)
            .filter_map(|entity| document.cached_entity_bounds(entity))
            .reduce(merge_bounds);
        let viewport_bounds = viewports_for_layout(document, &document.active_layout)
            .filter(|viewport| viewport.visible)
            .map(|viewport| viewport_paper_bounds(viewport))
            .reduce(merge_bounds);
        return [Some(paper), entity_bounds, viewport_bounds]
            .into_iter()
            .flatten()
            .reduce(merge_bounds);
    }
    entities_in_layout(document, "Model")
        .filter_map(|entity| document.cached_entity_bounds(entity))
        .reduce(merge_bounds)
}

pub fn viewport_paper_bounds(viewport: &LayoutViewport) -> (Point, Point) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;

    fn sample_line(doc: &mut Document, layout: Option<&str>) -> u64 {
        let id = doc.next_id();
        doc.add_entity_on_layout(
            Entity::Line {
                id,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
            layout,
        );
        id
    }

    #[test]
    fn model_entity_only_in_model() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc, None);
        assert!(is_model_space_entity(&doc, id));
        doc.set_active_layout("Layout1");
        assert!(!doc.entity_visible_in_active_layout(id));
    }

    #[test]
    fn paper_entity_only_in_its_layout() {
        let mut doc = Document::new_empty();
        doc.ensure_layout("Layout1");
        doc.ensure_layout("Layout2");
        let id1 = sample_line(&mut doc, Some("Layout1"));
        let id2 = sample_line(&mut doc, Some("Layout2"));
        doc.set_active_layout("Layout1");
        assert!(doc.entity_visible_in_active_layout(id1));
        assert!(!doc.entity_visible_in_active_layout(id2));
        doc.set_active_layout("Layout2");
        assert!(!doc.entity_visible_in_active_layout(id1));
        assert!(doc.entity_visible_in_active_layout(id2));
    }

    #[test]
    fn changing_active_layout_changes_visible_set() {
        let mut doc = Document::new_empty();
        doc.ensure_layout("Layout1");
        let model = sample_line(&mut doc, None);
        let paper = sample_line(&mut doc, Some("Layout1"));
        doc.set_active_layout("Model");
        assert!(doc.entity_visible_in_active_layout(model));
        assert!(!doc.entity_visible_in_active_layout(paper));
        doc.set_active_layout("Layout1");
        assert!(!doc.entity_visible_in_active_layout(model));
        assert!(doc.entity_visible_in_active_layout(paper));
    }

    #[test]
    fn layout_viewport_listed_per_layout() {
        let mut doc = Document::new_empty();
        doc.ensure_layout("Layout1");
        doc.ensure_layout("Layout2");
        doc.add_layout_viewport(LayoutViewport {
            id: 1,
            layout: "Layout1".to_string(),
            center: Point { x: 50.0, y: 50.0 },
            width: 100.0,
            height: 80.0,
            view_center: Point::default(),
            view_height: 100.0,
            model_zoom: 1.0,
            scale_paper_units: 1.0,
            scale_model_units: 1.0,
            twist: 0.0,
            locked: false,
            visible: true,
            border_visible: true,
            visible_layers: Vec::new(),
            hidden_layers: Vec::new(),
        });
        assert_eq!(viewports_for_layout(&doc, "Layout1").count(), 1);
        assert_eq!(viewports_for_layout(&doc, "Layout2").count(), 0);
    }

    #[test]
    fn normalize_layout_name_model_alias() {
        assert_eq!(normalize_layout_name("model"), "Model");
        assert_eq!(normalize_layout_name("  Layout1  "), "Layout1");
    }

    #[test]
    fn viewport_scale_maps_model_into_paper_rect() {
        use crate::document::LayoutViewport;
        let viewport = LayoutViewport {
            id: 1,
            layout: "Layout1".to_string(),
            center: Point { x: 100.0, y: 80.0 },
            width: 100.0,
            height: 50.0,
            view_center: Point { x: 0.0, y: 0.0 },
            view_height: 100.0,
            model_zoom: 0.5,
            scale_paper_units: 1.0,
            scale_model_units: 2.0,
            twist: 0.0,
            visible: true,
            locked: false,
            border_visible: true,
            visible_layers: Vec::new(),
            hidden_layers: Vec::new(),
        };
        let scale = viewport.height / viewport.view_height.max(1.0);
        let mapped_x = viewport.center.x + (10.0 - viewport.view_center.x) * scale;
        let mapped_y = viewport.center.y + (20.0 - viewport.view_center.y) * scale;
        assert!((mapped_x - 105.0).abs() < 1e-6);
        assert!((mapped_y - 90.0).abs() < 1e-6);
    }
}

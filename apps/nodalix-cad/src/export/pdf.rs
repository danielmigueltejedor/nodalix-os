//! PDF export / basic plot via Cairo `PdfSurface` (GTK/cairo, no document mutation).

use crate::{
    cad::layouts::{entities_in_layout, viewports_for_layout},
    canvas::{export_draw_layout_content, world_to_screen, Camera},
    document::{Document, LayoutKind, PaperSetup},
    geometry::Point,
};
use cairo::PdfSurface;
use gtk::cairo;
use std::path::{Path, PathBuf};

pub const PT_PER_MM: f64 = 72.0 / 25.4;
pub const DEFAULT_A4_WIDTH_MM: f64 = 297.0;
pub const DEFAULT_A4_HEIGHT_MM: f64 = 210.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfExportTarget {
    ActiveView,
    ModelSpace,
    ActiveLayout,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdfExportOptions {
    pub target: PdfExportTarget,
    pub paper_width_mm: f64,
    pub paper_height_mm: f64,
    pub margin_mm: f64,
    pub scale: Option<f64>,
    pub fit_to_page: bool,
    pub include_background: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            target: PdfExportTarget::ActiveView,
            paper_width_mm: DEFAULT_A4_WIDTH_MM,
            paper_height_mm: DEFAULT_A4_HEIGHT_MM,
            margin_mm: 10.0,
            scale: None,
            fit_to_page: true,
            include_background: true,
        }
    }
}

impl PdfExportOptions {
    pub fn for_document(document: &Document, target: PdfExportTarget) -> Self {
        let mut options = Self::default();
        options.target = target;
        if let Some((width, height)) = paper_size_for_target(document, target) {
            options.paper_width_mm = width;
            options.paper_height_mm = height;
        }
        options
    }
}

pub fn mm_to_pt(mm: f64) -> f64 {
    mm * PT_PER_MM
}

pub fn default_export_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(&home);
        let docs = home_path.join("Documents");
        if docs.is_dir() || std::fs::create_dir_all(&docs).is_ok() {
            return docs.join("lixcad-export.pdf");
        }
        return home_path.join("lixcad-export.pdf");
    }
    PathBuf::from("lixcad-export.pdf")
}

pub fn try_parse_export_command(command: &str) -> Option<PdfExportTarget> {
    match command.trim().to_ascii_lowercase().as_str() {
        "pdf" | "exportpdf" | "plot" => Some(PdfExportTarget::ActiveView),
        "pdf model" | "pdf modelspace" => Some(PdfExportTarget::ModelSpace),
        "pdf layout" | "pdf paper" => Some(PdfExportTarget::ActiveLayout),
        _ => None,
    }
}

pub fn export_pdf(
    document: &Document,
    path: &Path,
    options: PdfExportOptions,
) -> Result<(), String> {
    if options.paper_width_mm <= 0.0 || options.paper_height_mm <= 0.0 {
        return Err("invalid paper size".to_string());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
    }
    let page_w_pt = mm_to_pt(options.paper_width_mm);
    let page_h_pt = mm_to_pt(options.paper_height_mm);
    let surface = PdfSurface::new(page_w_pt, page_h_pt, path).map_err(|err| err.to_string())?;
    let cr = cairo::Context::new(&surface).map_err(|err| err.to_string())?;
    if options.include_background {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.paint();
    }
    let layout_name = resolve_layout_name(document, options.target);
    let margin_pt = mm_to_pt(options.margin_mm.max(0.0));
    let camera = fit_camera_for_layout(
        document,
        &layout_name,
        page_w_pt,
        page_h_pt,
        margin_pt,
        options.fit_to_page,
        options.scale,
    );
    export_draw_layout_content(&cr, page_w_pt, page_h_pt, document, camera, &layout_name);
    let _ = cr.show_page();
    surface.finish();
    Ok(())
}

fn resolve_layout_name(document: &Document, target: PdfExportTarget) -> String {
    match target {
        PdfExportTarget::ModelSpace => "Model".to_string(),
        PdfExportTarget::ActiveLayout | PdfExportTarget::ActiveView => {
            document.active_layout.clone()
        }
    }
}

pub fn paper_size_for_target(document: &Document, target: PdfExportTarget) -> Option<(f64, f64)> {
    let layout_name = resolve_layout_name(document, target);
    if layout_name == "Model" {
        return Some((DEFAULT_A4_WIDTH_MM, DEFAULT_A4_HEIGHT_MM));
    }
    document
        .layouts
        .iter()
        .find(|layout| {
            crate::document::normalized_layout_name(&layout.name)
                == crate::document::normalized_layout_name(&layout_name)
        })
        .map(|layout| valid_paper_size(&layout.paper))
}

pub fn valid_paper_size(paper: &PaperSetup) -> (f64, f64) {
    let width = paper.width;
    let height = paper.height;
    if width > 1.0 && height > 1.0 {
        (width, height)
    } else {
        (DEFAULT_A4_WIDTH_MM, DEFAULT_A4_HEIGHT_MM)
    }
}

pub fn model_space_bounds(document: &Document) -> Option<(Point, Point)> {
    entities_in_layout(document, "Model")
        .filter(|entity| document.entity_layer_visible(entity.id()))
        .filter_map(|entity| document.cached_entity_bounds(entity))
        .reduce(merge_bounds)
}

pub fn layout_export_bounds(document: &Document, layout_name: &str) -> Option<(Point, Point)> {
    let layout_name = crate::document::normalized_layout_name(layout_name);
    let is_paper = document
        .layouts
        .iter()
        .find(|layout| crate::document::normalized_layout_name(&layout.name) == layout_name)
        .is_some_and(|layout| layout.kind == LayoutKind::Paper);
    if is_paper {
        let paper = document
            .layouts
            .iter()
            .find(|layout| crate::document::normalized_layout_name(&layout.name) == layout_name)
            .map(|layout| valid_paper_size(&layout.paper));
        let paper_bounds = paper.map(|(w, h)| (Point { x: 0.0, y: 0.0 }, Point { x: w, y: h }));
        let entity_bounds = entities_in_layout(document, &layout_name)
            .filter(|entity| document.entity_layer_visible(entity.id()))
            .filter_map(|entity| document.cached_entity_bounds(entity))
            .reduce(merge_bounds);
        let viewport_bounds = viewports_for_layout(document, &layout_name)
            .filter(|viewport| viewport.visible)
            .map(|viewport| {
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
            })
            .reduce(merge_bounds);
        return [paper_bounds, entity_bounds, viewport_bounds]
            .into_iter()
            .flatten()
            .reduce(merge_bounds);
    }
    entities_in_layout(document, &layout_name)
        .filter(|entity| document.entity_layer_visible(entity.id()))
        .filter_map(|entity| document.cached_entity_bounds(entity))
        .reduce(merge_bounds)
}

pub fn fit_camera_for_bounds(
    bounds: (Point, Point),
    page_width_pt: f64,
    page_height_pt: f64,
    margin_pt: f64,
    fit_to_page: bool,
    scale_override: Option<f64>,
) -> Camera {
    let (min, max) = bounds;
    let span_x = (max.x - min.x).abs().max(1.0);
    let span_y = (max.y - min.y).abs().max(1.0);
    let content_w = (page_width_pt - 2.0 * margin_pt).max(1.0);
    let content_h = (page_height_pt - 2.0 * margin_pt).max(1.0);
    let zoom = if let Some(scale) = scale_override {
        scale.max(0.001) * PT_PER_MM
    } else if fit_to_page {
        (content_w / span_x).min(content_h / span_y)
    } else {
        PT_PER_MM
    };
    let cx = (min.x + max.x) * 0.5;
    let cy = (min.y + max.y) * 0.5;
    Camera {
        zoom,
        pan_x: cx,
        pan_y: cy,
        rotation: 0.0,
    }
}

pub fn fit_camera_for_layout(
    document: &Document,
    layout_name: &str,
    page_width_pt: f64,
    page_height_pt: f64,
    margin_pt: f64,
    fit_to_page: bool,
    scale_override: Option<f64>,
) -> Camera {
    let bounds = layout_export_bounds(document, layout_name)
        .unwrap_or((Point { x: -50.0, y: -50.0 }, Point { x: 50.0, y: 50.0 }));
    fit_camera_for_bounds(
        bounds,
        page_width_pt,
        page_height_pt,
        margin_pt,
        fit_to_page,
        scale_override,
    )
}

pub fn viewport_export_scale(viewport_height_mm: f64, view_height_world: f64) -> f64 {
    if view_height_world > f64::EPSILON {
        viewport_height_mm / view_height_world
    } else {
        1.0
    }
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
    use crate::document::{BlockDefinition, Entity};

    #[test]
    fn pdf_export_options_default() {
        let options = PdfExportOptions::default();
        assert_eq!(options.target, PdfExportTarget::ActiveView);
        assert!(options.fit_to_page);
        assert_eq!(options.paper_width_mm, DEFAULT_A4_WIDTH_MM);
    }

    #[test]
    fn mm_to_pt_conversion() {
        assert!((mm_to_pt(25.4) - 72.0).abs() < 1e-6);
        assert!((mm_to_pt(10.0) - 10.0 * PT_PER_MM).abs() < 1e-9);
    }

    #[test]
    fn fit_to_page_transform() {
        let bounds = (Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 50.0 });
        let camera = fit_camera_for_bounds(bounds, 800.0, 600.0, 20.0, true, None);
        let corner = world_to_screen(Point { x: 0.0, y: 0.0 }, 800.0, 600.0, camera);
        let opposite = world_to_screen(Point { x: 100.0, y: 50.0 }, 800.0, 600.0, camera);
        assert!(corner.x >= 15.0 && corner.y >= 15.0);
        assert!(opposite.x <= 785.0 && opposite.y <= 585.0);
    }

    #[test]
    fn hidden_layers_excluded_from_model_bounds() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        assert!(model_space_bounds(&doc).is_some());
        doc.set_layer_visible("Default", false);
        assert!(model_space_bounds(&doc).is_none());
    }

    #[test]
    fn locked_layers_still_in_export_bounds() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Locked".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 5.0 },
        });
        assert!(model_space_bounds(&doc).is_some());
    }

    #[test]
    fn a4_fallback_for_invalid_paper() {
        let (w, h) = valid_paper_size(&PaperSetup {
            width: 0.0,
            height: 0.0,
            unit: "mm".to_string(),
            orientation: "landscape".to_string(),
            preset: "A4".to_string(),
        });
        assert_eq!(w, DEFAULT_A4_WIDTH_MM);
        assert_eq!(h, DEFAULT_A4_HEIGHT_MM);
    }

    #[test]
    fn block_and_hatch_in_model_bounds() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "B".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
            }],
        });
        let block_id = doc.next_id();
        doc.add_entity(Entity::BlockReference {
            id: block_id,
            layer: "Default".to_string(),
            name: "B".to_string(),
            insertion: Point { x: 0.0, y: 0.0 },
            scale: 1.0,
            scale_y: None,
            rotation: 0.0,
        });
        let hatch_id = doc.next_id();
        doc.add_entity(Entity::Hatch {
            id: hatch_id,
            layer: "Default".to_string(),
            boundary: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 10.0, y: 0.0 },
                Point { x: 10.0, y: 10.0 },
            ],
            pattern: "SOLID".to_string(),
            scale: 1.0,
            angle: 0.0,
            solid: true,
        });
        let bounds = model_space_bounds(&doc).expect("bounds");
        assert!(bounds.1.x >= 10.0);
    }

    #[test]
    fn command_parser_pdf() {
        assert_eq!(
            try_parse_export_command("pdf"),
            Some(PdfExportTarget::ActiveView)
        );
        assert_eq!(
            try_parse_export_command("exportpdf"),
            Some(PdfExportTarget::ActiveView)
        );
        assert_eq!(
            try_parse_export_command("pdf model"),
            Some(PdfExportTarget::ModelSpace)
        );
    }

    #[test]
    fn invalid_path_returns_err() {
        let doc = Document::new_empty();
        let options = PdfExportOptions::default();
        let path = Path::new("/\0invalid/lixcad.pdf");
        assert!(export_pdf(&doc, path, options).is_err());
    }

    #[test]
    fn export_pdf_writes_non_empty_file() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 100.0, y: 50.0 },
        });
        let dir = std::env::temp_dir().join(format!("lixcad-pdf-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.pdf");
        let options = PdfExportOptions {
            target: PdfExportTarget::ModelSpace,
            ..PdfExportOptions::default()
        };
        export_pdf(&doc, &path, options).expect("export");
        let meta = std::fs::metadata(&path).expect("metadata");
        assert!(meta.len() > 200);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn viewport_export_scale_basic() {
        let scale = viewport_export_scale(100.0, 50.0);
        assert!((scale - 2.0).abs() < 1e-9);
    }

    #[test]
    fn active_layout_target_uses_paper_size() {
        let mut doc = Document::new_empty();
        let name = doc.create_paper_layout();
        doc.set_active_layout(&name);
        let (w, h) = paper_size_for_target(&doc, PdfExportTarget::ActiveLayout).unwrap();
        assert_eq!(w, DEFAULT_A4_WIDTH_MM);
        assert_eq!(h, DEFAULT_A4_HEIGHT_MM);
    }
}

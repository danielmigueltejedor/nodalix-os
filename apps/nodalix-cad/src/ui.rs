use crate::{canvas::CadCanvas, document::Document, geometry::Point, tools::Tool};
use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

pub fn build(app: &adw::Application) {
    load_css();

    let document = Rc::new(RefCell::new(Document::new_empty()));
    let active_tool = Rc::new(RefCell::new(Tool::Select));
    let selected_entity = Rc::new(RefCell::new(None::<u64>));
    let current_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    let selection_label = gtk::Label::new(Some("Selection: none"));
    selection_label.add_css_class("attribute-label");
    let cursor_label = gtk::Label::new(Some("X 0.00  Y 0.00"));
    cursor_label.add_css_class("status-label");
    let tool_label = gtk::Label::new(Some("Tool: Select"));
    tool_label.add_css_class("status-label");
    let modified_label = gtk::Label::new(Some("Saved"));
    modified_label.add_css_class("status-label");

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Lix CAD")
        .default_width(1460)
        .default_height(900)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("cad-root");
    window.set_content(Some(&root));

    let canvas = CadCanvas::new(
        document.clone(),
        active_tool.clone(),
        selected_entity.clone(),
        selection_label.clone(),
    );
    let tool_context = tool_context_panel(*active_tool.borrow());
    let properties = properties_panel(&document.borrow());
    let right_panel = right_panel(tool_context.clone(), properties.clone());

    root.append(&horizontal_scroll(&top_toolbar(
        &window,
        document.clone(),
        current_path.clone(),
        canvas.clone(),
        properties.clone(),
        modified_label.clone(),
    )));

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.set_hexpand(true);
    body.set_vexpand(true);
    root.append(&body);

    body.append(&vertical_scroll(&tool_palette(
        active_tool.clone(),
        tool_label.clone(),
        tool_context.clone(),
    )));
    let cursor = canvas.cursor();
    attach_cursor_tracking(
        canvas.widget(),
        cursor,
        cursor_label.clone(),
        canvas.camera(),
    );
    body.append(canvas.widget());
    body.append(&right_panel);

    root.append(&attribute_bar(
        selection_label.clone(),
        document.clone(),
        selected_entity.clone(),
    ));
    root.append(&status_bar(
        cursor_label,
        tool_label,
        modified_label.clone(),
        document.clone(),
    ));

    attach_keyboard_shortcuts(
        &window,
        canvas.clone(),
        document.clone(),
        active_tool.clone(),
        modified_label.clone(),
    );

    window.present();
}

fn horizontal_scroll<W: IsA<gtk::Widget>>(child: &W) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.add_css_class("toolbar-scroll");
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    scroll.set_child(Some(child));
    scroll
}

fn vertical_scroll<W: IsA<gtk::Widget>>(child: &W) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.add_css_class("side-scroll");
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(child));
    scroll
}

fn top_toolbar(
    window: &adw::ApplicationWindow,
    document: Rc<RefCell<Document>>,
    current_path: Rc<RefCell<Option<PathBuf>>>,
    cad_canvas: CadCanvas,
    properties: gtk::Box,
    modified_label: gtk::Label,
) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("top-toolbar");
    let canvas = cad_canvas.widget().clone();

    let new_button = toolbar_button("New");
    {
        let document = document.clone();
        let current_path = current_path.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        new_button.connect_clicked(move |_| {
            *document.borrow_mut() = Document::new_empty();
            *current_path.borrow_mut() = None;
            refresh_properties(&properties, &document.borrow());
            modified_label.set_text("New document");
            canvas.queue_draw();
        });
    }
    bar.append(&new_button);

    let open_button = toolbar_button("Open");
    {
        let window = window.clone();
        let document = document.clone();
        let current_path = current_path.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        open_button.connect_clicked(move |_| {
            choose_file(&window, "Open Lix CAD", gtk::FileChooserAction::Open, {
                let document = document.clone();
                let current_path = current_path.clone();
                let canvas = canvas.clone();
                let properties = properties.clone();
                let modified_label = modified_label.clone();
                let error_window = window.clone();
                move |path| {
                    if is_native_document(&path) {
                        match Document::open_nodcad(&path) {
                            Ok(opened) => {
                                *document.borrow_mut() = opened;
                                *current_path.borrow_mut() = Some(path.clone());
                                refresh_properties(&properties, &document.borrow());
                                modified_label.set_text("Opened");
                                canvas.queue_draw();
                            }
                            Err(error) => show_error(&error_window, "Open failed", &error),
                        }
                    } else {
                        import_and_refresh(
                            &error_window,
                            &document,
                            &path,
                            &canvas,
                            &properties,
                            &modified_label,
                        );
                    }
                }
            });
        });
    }
    bar.append(&open_button);

    let save_button = toolbar_button("Save");
    {
        let window = window.clone();
        let document = document.clone();
        let current_path = current_path.clone();
        let modified_label = modified_label.clone();
        save_button.connect_clicked(move |_| {
            if let Some(path) = current_path.borrow().clone() {
                save_document(&window, &document, &path, &modified_label);
            } else {
                choose_file(&window, "Save Lix CAD", gtk::FileChooserAction::Save, {
                    let window = window.clone();
                    let document = document.clone();
                    let current_path = current_path.clone();
                    let modified_label = modified_label.clone();
                    move |path| {
                        save_document(&window, &document, &path, &modified_label);
                        *current_path.borrow_mut() = Some(path);
                    }
                });
            }
        });
    }
    bar.append(&save_button);

    let import_button = toolbar_button("Import");
    {
        let window = window.clone();
        let document = document.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        import_button.connect_clicked(move |_| {
            choose_file(
                &window,
                "Import CAD/Mesh File",
                gtk::FileChooserAction::Open,
                {
                    let window = window.clone();
                    let document = document.clone();
                    let canvas = canvas.clone();
                    let properties = properties.clone();
                    let modified_label = modified_label.clone();
                    move |path| {
                        import_and_refresh(
                            &window,
                            &document,
                            &path,
                            &canvas,
                            &properties,
                            &modified_label,
                        );
                    }
                },
            );
        });
    }
    bar.append(&import_button);

    let sample_step = toolbar_button("Import sample STEP");
    {
        let window = window.clone();
        let document = document.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        sample_step.connect_clicked(move |_| {
            let path = PathBuf::from("/mnt/data/5621-Separador.stp");
            import_and_refresh(
                &window,
                &document,
                &path,
                &canvas,
                &properties,
                &modified_label,
            );
        });
    }
    bar.append(&sample_step);

    let scale_button = toolbar_button("Scale factor");
    {
        let window = window.clone();
        let document = document.clone();
        let properties = properties.clone();
        let canvas = canvas.clone();
        let modified_label = modified_label.clone();
        scale_button.connect_clicked(move |_| {
            show_scale_dialog(
                &window,
                document.clone(),
                properties.clone(),
                canvas.clone(),
                modified_label.clone(),
            );
        });
    }
    bar.append(&scale_button);

    let settings_button = toolbar_button("Settings");
    {
        let window = window.clone();
        settings_button.connect_clicked(move |_| show_settings_dialog(&window));
    }
    bar.append(&settings_button);

    let export_dxf = toolbar_button("Export DXF");
    {
        let window = window.clone();
        let document = document.clone();
        export_dxf.connect_clicked(move |_| {
            choose_file(&window, "Export DXF", gtk::FileChooserAction::Save, {
                let window = window.clone();
                let document = document.clone();
                move |path| match crate::export::dxf::export(&document.borrow(), &path) {
                    Ok(()) => show_info(&window, "DXF exported", &path.display().to_string()),
                    Err(error) => show_error(&window, "DXF export failed", &error),
                }
            });
        });
    }
    bar.append(&export_dxf);

    let export_dwg = toolbar_button("Export DWG");
    {
        let window = window.clone();
        let document = document.clone();
        export_dwg.connect_clicked(move |_| {
            choose_file(&window, "Export DWG", gtk::FileChooserAction::Save, {
                let window = window.clone();
                let document = document.clone();
                move |path| match crate::export::dwg::export_via_dxf(&document.borrow(), &path) {
                    Ok(()) => show_info(&window, "DWG exported", &path.display().to_string()),
                    Err(error) => show_error(&window, "DWG export bridge", &error),
                }
            });
        });
    }
    bar.append(&export_dwg);

    let export_svg = toolbar_button("Export SVG");
    {
        let window = window.clone();
        let document = document.clone();
        export_svg.connect_clicked(move |_| {
            choose_file(&window, "Export SVG", gtk::FileChooserAction::Save, {
                let window = window.clone();
                let document = document.clone();
                move |path| match crate::export::svg::export(&document.borrow(), &path) {
                    Ok(()) => show_info(&window, "SVG exported", &path.display().to_string()),
                    Err(error) => show_error(&window, "SVG export failed", &error),
                }
            });
        });
    }
    bar.append(&export_svg);

    for label in ["Undo", "Redo", "View: Top", "Units: mm"] {
        let button = toolbar_button(label);
        if label == "View: Top" {
            button.set_tooltip_text(Some(crate::drawing::projection::projection_status()));
        }
        bar.append(&button);
    }

    let zoom_out = toolbar_button("−");
    zoom_out.set_tooltip_text(Some("Zoom out"));
    {
        let cad_canvas = cad_canvas.clone();
        zoom_out.connect_clicked(move |_| cad_canvas.zoom_out());
    }
    bar.append(&zoom_out);

    let zoom_reset = toolbar_button("100%");
    zoom_reset.set_tooltip_text(Some("Reset view"));
    {
        let cad_canvas = cad_canvas.clone();
        zoom_reset.connect_clicked(move |_| cad_canvas.reset_view());
    }
    bar.append(&zoom_reset);

    let zoom_in = toolbar_button("+");
    zoom_in.set_tooltip_text(Some("Zoom in"));
    {
        let cad_canvas = cad_canvas.clone();
        zoom_in.connect_clicked(move |_| cad_canvas.zoom_in());
    }
    bar.append(&zoom_in);

    bar
}

fn toolbar_button(label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("tool-button");
    button
}

fn is_native_document(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("nodcad") || ext.eq_ignore_ascii_case("lixcad"))
        .unwrap_or(false)
}

fn tool_palette(
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
) -> gtk::Box {
    let palette = gtk::Box::new(gtk::Orientation::Vertical, 8);
    palette.add_css_class("tool-palette");
    let buttons: Rc<RefCell<Vec<gtk::ToggleButton>>> = Rc::new(RefCell::new(Vec::new()));

    for (tool, icon, label) in Tool::all() {
        let button = gtk::ToggleButton::with_label(icon);
        button.set_tooltip_text(Some(label));
        button.add_css_class("palette-button");
        if *tool == Tool::Select {
            button.set_active(true);
        }
        let current = *tool;
        let active_tool = active_tool.clone();
        let tool_label = tool_label.clone();
        let tool_context = tool_context.clone();
        let buttons_for_click = buttons.clone();
        button.connect_clicked(move |button| {
            *active_tool.borrow_mut() = current;
            tool_label.set_text(&format!("Tool: {}", current.label()));
            refresh_tool_context(&tool_context, current);
            for other in buttons_for_click.borrow().iter() {
                other.set_active(false);
            }
            button.set_active(true);
        });
        buttons.borrow_mut().push(button.clone());
        palette.append(&button);
    }

    palette
}

fn right_panel(tool_context: gtk::Box, properties: gtk::Box) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.add_css_class("right-panel");
    let tool_scroll = gtk::ScrolledWindow::new();
    tool_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    tool_scroll.set_min_content_height(210);
    tool_scroll.set_child(Some(&tool_context));
    panel.append(&tool_scroll);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("panel-separator");
    panel.append(&separator);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_child(Some(&properties));
    panel.append(&scroll);
    panel
}

fn attribute_bar(
    selection_label: gtk::Label,
    document: Rc<RefCell<Document>>,
    selected_entity: Rc<RefCell<Option<u64>>>,
) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("attribute-bar");
    bar.append(&selection_label);
    let layer = gtk::Entry::new();
    layer.set_placeholder_text(Some("Layer"));
    layer.set_text("Default");
    layer.add_css_class("attribute-entry");
    let lineweight = gtk::Entry::new();
    lineweight.set_placeholder_text(Some("Lineweight"));
    lineweight.set_text("0.25 mm");
    lineweight.add_css_class("attribute-entry");
    let linetype = gtk::Entry::new();
    linetype.set_placeholder_text(Some("Linetype"));
    linetype.set_text("Continuous");
    linetype.add_css_class("attribute-entry");
    let apply = gtk::Button::with_label("Apply");
    apply.add_css_class("context-chip");
    {
        let document = document.clone();
        let selected_entity = selected_entity.clone();
        let selection_label = selection_label.clone();
        let layer = layer.clone();
        apply.connect_clicked(move |_| {
            let Some(id) = *selected_entity.borrow() else {
                return;
            };
            let layer_name = layer.text().to_string();
            if layer_name.trim().is_empty() {
                return;
            }
            if document
                .borrow_mut()
                .set_entity_layer(id, layer_name.trim())
            {
                if let Some(summary) = document.borrow().entity_summary(id) {
                    selection_label.set_text(&summary);
                }
            }
        });
    }
    bar.append(&gtk::Label::new(Some("Layer")));
    bar.append(&layer);
    bar.append(&gtk::Label::new(Some("Weight")));
    bar.append(&lineweight);
    bar.append(&gtk::Label::new(Some("Type")));
    bar.append(&linetype);
    bar.append(&apply);
    bar
}

fn tool_context_panel(tool: Tool) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    panel.add_css_class("tool-context-panel");
    refresh_tool_context(&panel, tool);
    panel
}

fn refresh_tool_context(panel: &gtk::Box, tool: Tool) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    section_title(panel, "Tool parameters");
    property(panel, "Active tool", tool.label());

    match tool {
        Tool::Select => {
            compact_note(
                panel,
                "Select objects to inspect and edit their properties.",
            );
            context_buttons(panel, &["Move", "Copy", "Rotate"]);
        }
        Tool::Polyline => {
            compact_note(panel, "Click to add connected vertices. Enter finishes the polyline. Esc cancels the current polyline.");
            context_entry(panel, "Layer", "Default");
            context_entry(panel, "Lineweight", "0.25 mm");
            context_entry(panel, "Linetype", "Continuous");
            context_entry(panel, "Snap", "Endpoint / midpoint / perpendicular");
            context_buttons(panel, &["Close later", "Object snap", "Construction"]);
        }
        Tool::Line | Tool::Rectangle | Tool::Circle | Tool::Arc => {
            context_entry(panel, "Layer", "Default");
            context_entry(panel, "Lineweight", "0.25 mm");
            context_entry(panel, "Linetype", "Continuous");
            context_entry(panel, "Snap", "Grid");
            context_buttons(panel, &["Ortho", "Object snap", "Construction"]);
        }
        Tool::Dimension => {
            context_entry(panel, "Style", "ISO-25");
            context_entry(panel, "Precision", "0.00");
            context_entry(panel, "Arrow", "Closed filled");
            context_entry(panel, "Text height", "2.5 mm");
            context_buttons(panel, &["Linear", "Aligned", "Radius"]);
        }
        Tool::Text => {
            context_entry(panel, "Text style", "Technical");
            context_entry(panel, "Height", "2.5 mm");
            context_entry(panel, "Alignment", "Left");
            context_entry(panel, "Rotation", "0 deg");
            context_buttons(panel, &["Single line", "Multiline", "Annotative"]);
        }
        Tool::Modify => {
            context_buttons(
                panel,
                &["Move", "Copy", "Offset", "Trim", "Extend", "Mirror"],
            );
            context_entry(panel, "Base point", "Pick on canvas");
            context_entry(panel, "Distance", "By cursor");
        }
        Tool::Block => {
            context_entry(panel, "Block name", "New block");
            context_entry(panel, "Insertion", "Pick point");
            context_entry(panel, "Scale", "1.000");
            context_buttons(panel, &["Create", "Insert", "Explode"]);
        }
        Tool::Hatch => {
            context_entry(panel, "Pattern", "ANSI31");
            context_entry(panel, "Scale", "1.0");
            context_entry(panel, "Angle", "0 deg");
            context_buttons(panel, &["Pick boundary", "Associative"]);
        }
        Tool::Table => {
            context_entry(panel, "Rows", "3");
            context_entry(panel, "Columns", "4");
            context_entry(panel, "Cell height", "6 mm");
            context_buttons(panel, &["Insert table", "Title row"]);
        }
        Tool::Parametric => {
            context_buttons(panel, &["Horizontal", "Vertical", "Parallel", "Coincident"]);
            context_entry(panel, "Parameter", "d1");
            context_entry(panel, "Expression", "10 mm");
        }
        Tool::Guideline => {
            context_entry(panel, "Type", "Construction line");
            context_entry(panel, "Angle", "0 deg");
            context_buttons(panel, &["Horizontal", "Vertical", "Bisector"]);
        }
        Tool::Measure => {
            context_entry(panel, "Mode", "Distance");
            context_entry(panel, "Units", "mm");
            context_buttons(panel, &["Distance", "Area", "Angle"]);
        }
        Tool::Pan | Tool::Orbit => {
            context_entry(panel, "View", "Top");
            context_entry(panel, "Zoom", "Mouse wheel / toolbar");
            context_buttons(panel, &["Fit", "Previous", "Reset"]);
        }
        Tool::SectionFromMesh | Tool::FitCurve | Tool::Extrude => {
            context_entry(panel, "Workflow", "Reverse engineering");
            context_entry(panel, "Tolerance", "0.10 mm");
            context_buttons(panel, &["Preview", "Apply", "Create sketch"]);
        }
    }
}

fn context_entry(panel: &gtk::Box, label: &str, value: &str) {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 5);
    let title = gtk::Label::new(Some(label));
    title.set_xalign(0.0);
    title.add_css_class("prop-label");
    let entry = gtk::Entry::new();
    entry.set_text(value);
    entry.add_css_class("context-entry");
    row.append(&title);
    row.append(&entry);
    panel.append(&row);
}

fn context_buttons(panel: &gtk::Box, labels: &[&str]) {
    let row = gtk::FlowBox::new();
    row.set_selection_mode(gtk::SelectionMode::None);
    row.set_column_spacing(6);
    row.set_row_spacing(6);
    for label in labels {
        let button = gtk::Button::with_label(label);
        button.add_css_class("context-chip");
        row.insert(&button, -1);
    }
    panel.append(&row);
}

fn compact_note(panel: &gtk::Box, text: &str) {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.add_css_class("context-note");
    panel.append(&label);
}

fn properties_panel(document: &Document) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 14);
    panel.add_css_class("properties-panel");
    refresh_properties(&panel, document);
    panel
}

fn refresh_properties(panel: &gtk::Box, document: &Document) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    section_title(panel, "Properties");
    property(panel, "Document", &document.name);
    property(panel, "Units", document.units.label());
    property(panel, "Entities", &document.entities.len().to_string());
    property(
        panel,
        "Modified",
        if document.modified { "yes" } else { "no" },
    );

    section_title(panel, "Layers");
    for layer in &document.layers {
        property(
            panel,
            &layer.name,
            if layer.visible { "visible" } else { "hidden" },
        );
    }

    section_title(panel, "Imported files");
    if document.imported_references.is_empty() {
        property(panel, "References", "none");
    }
    for reference in &document.imported_references {
        property(panel, &reference.format, &reference.summary);
    }

    section_title(panel, "Mesh info");
    if document.mesh_references.is_empty() {
        property(panel, "Meshes", "none");
    }
    for mesh in &document.mesh_references {
        property(panel, "Triangles", &mesh.triangle_count.to_string());
        property(
            panel,
            "Dimensions",
            &crate::mesh::analysis::dimensions_label(mesh.bounding_box),
        );
        property(panel, "Scale", &format!("{:.6}", mesh.transform.scale));
    }

    section_title(panel, "Flujo de ingeniería inversa");
    reverse_workflow(panel, document);

    section_title(panel, "Technical drawing");
    property(panel, "Sheet", crate::drawing::sheet::sheet_status());
    property(
        panel,
        "Projection",
        crate::drawing::projection::projection_status(),
    );
    property(panel, "Views", crate::drawing::views::view_status());
    property(
        panel,
        "Dimensions",
        crate::drawing::dimensions::dimension_status(),
    );
    property(panel, "PDF", crate::export::pdf::export_pdf_placeholder());
}

fn reverse_workflow(panel: &gtk::Box, document: &Document) {
    for step in [
        "1. Importar malla STL / modelo STEP",
        "2. Limpiar o recortar malla",
        "3. Medir distancia conocida",
        "4. Calcular factor de escala",
        "5. Escalar modelo",
        "6. Alinear con ejes",
        "7. Crear sección/boceto desde malla",
        "8. Ajustar curvas",
        "9. Reconstruir perfil",
        "10. Extruir",
        "11. Crear vistas normalizadas",
        "12. Acotar plano",
        "13. Exportar DXF/PDF",
    ] {
        let label = gtk::Label::new(Some(step));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.add_css_class("workflow-step");
        panel.append(&label);
    }
    property(
        panel,
        "Scale factor",
        &format!("real / measured = {:.6}", document.metadata.scale_factor),
    );
    property(
        panel,
        "Mesh cleanup",
        crate::mesh::cleanup::cleanup_status(),
    );
    property(panel, "Section", crate::mesh::section::section_status());
    property(panel, "Fit curves", crate::reverse::fit_curves::status());
    property(panel, "Sketch", crate::reverse::sketch_from_mesh::status());
    property(panel, "Extrude", crate::reverse::extrude::status());
}

fn section_title(panel: &gtk::Box, text: &str) {
    let title = gtk::Label::new(Some(text));
    title.set_xalign(0.0);
    title.add_css_class("panel-title");
    panel.append(&title);
}

fn property(panel: &gtk::Box, label: &str, value: &str) {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let name = gtk::Label::new(Some(label));
    name.set_xalign(0.0);
    name.add_css_class("prop-label");
    let value = gtk::Label::new(Some(value));
    value.set_xalign(0.0);
    value.set_wrap(true);
    value.add_css_class("prop-value");
    row.append(&name);
    row.append(&value);
    panel.append(&row);
}

fn status_bar(
    cursor_label: gtk::Label,
    tool_label: gtk::Label,
    modified_label: gtk::Label,
    document: Rc<RefCell<Document>>,
) -> gtk::Box {
    let status = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    status.add_css_class("status-bar");
    status.append(&gtk::Label::new(Some(&format!(
        "Document: {}",
        document.borrow().name
    ))));
    status.append(&cursor_label);
    status.append(&gtk::Label::new(Some(&format!(
        "Units: {}",
        document.borrow().units.label()
    ))));
    status.append(&gtk::Label::new(Some(&format!(
        "Scale: {:.4}",
        document.borrow().metadata.scale_factor
    ))));
    status.append(&tool_label);
    status.append(&gtk::Label::new(Some(
        "Snap: endpoint · midpoint · center · perpendicular",
    )));
    status.append(&modified_label);
    status
}

fn attach_cursor_tracking(
    area: &gtk::DrawingArea,
    cursor: Rc<RefCell<Point>>,
    label: gtk::Label,
    camera: Rc<RefCell<crate::canvas::Camera>>,
) {
    let motion = gtk::EventControllerMotion::new();
    let area_for_motion = area.clone();
    motion.connect_motion(move |_, x, y| {
        let point = crate::canvas::screen_to_world(
            x,
            y,
            area_for_motion.width() as f64,
            area_for_motion.height() as f64,
            *camera.borrow(),
        );
        *cursor.borrow_mut() = point;
        label.set_text(&format!("X {:.2}  Y {:.2}", point.x, point.y));
    });
    area.add_controller(motion);
}

fn import_and_refresh(
    window: &adw::ApplicationWindow,
    document: &Rc<RefCell<Document>>,
    path: &PathBuf,
    canvas: &gtk::DrawingArea,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
) {
    let result = {
        let mut document = document.borrow_mut();
        crate::import::import_path(path, &mut document)
    };

    match result {
        Ok(summary) => {
            refresh_properties(properties, &document.borrow());
            modified_label.set_text("Modified");
            canvas.queue_draw();
            show_import_summary(window, &summary);
        }
        Err(error) if error.starts_with(crate::import::dwg::DWG_SETUP_REQUIRED) => {
            show_dwg_setup_dialog(window, &error)
        }
        Err(error) => show_error(window, "Import failed", &error),
    }
}

fn attach_keyboard_shortcuts(
    window: &adw::ApplicationWindow,
    canvas: CadCanvas,
    document: Rc<RefCell<Document>>,
    active_tool: Rc<RefCell<Tool>>,
    modified_label: gtk::Label,
) {
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, _| match key {
        gdk::Key::Return | gdk::Key::KP_Enter => {
            if *active_tool.borrow() == Tool::Polyline {
                canvas.finish_polyline(&document);
                modified_label.set_text("Modified");
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        }
        gdk::Key::Escape => {
            canvas.cancel_interaction();
            gtk::glib::Propagation::Stop
        }
        _ => gtk::glib::Propagation::Proceed,
    });
    window.add_controller(keys);
}

fn save_document(
    window: &adw::ApplicationWindow,
    document: &Rc<RefCell<Document>>,
    path: &PathBuf,
    modified_label: &gtk::Label,
) {
    match crate::export::native::save(&mut document.borrow_mut(), path) {
        Ok(()) => modified_label.set_text("Saved"),
        Err(error) => show_error(window, "Save failed", &error),
    }
}

#[allow(deprecated)]
fn show_scale_dialog(
    window: &adw::ApplicationWindow,
    document: Rc<RefCell<Document>>,
    properties: gtk::Box,
    canvas: gtk::DrawingArea,
    modified_label: gtk::Label,
) {
    let dialog = gtk::Dialog::builder()
        .transient_for(window)
        .modal(true)
        .title("Scale factor calculator")
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Apply", gtk::ResponseType::Accept);
    let area = dialog.content_area();
    area.set_spacing(10);
    area.set_margin_top(14);
    area.set_margin_bottom(14);
    area.set_margin_start(14);
    area.set_margin_end(14);
    let measured = gtk::Entry::new();
    measured.set_placeholder_text(Some("Measured distance in mesh"));
    let real = gtk::Entry::new();
    real.set_placeholder_text(Some("Real known distance"));
    let formula = gtk::Label::new(Some("factor = real_distance / measured_distance"));
    formula.set_xalign(0.0);
    area.append(&formula);
    area.append(&measured);
    area.append(&real);
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let parsed = measured
                .text()
                .parse::<f64>()
                .ok()
                .zip(real.text().parse::<f64>().ok());
            if let Some((measured, real)) = parsed {
                if measured > 0.0 {
                    let factor = real / measured;
                    let mut doc = document.borrow_mut();
                    doc.metadata.measured_distance = Some(measured);
                    doc.metadata.real_distance = Some(real);
                    doc.metadata.scale_factor = factor;
                    for mesh in &mut doc.mesh_references {
                        mesh.transform.scale = factor;
                    }
                    doc.modified = true;
                    refresh_properties(&properties, &doc);
                    modified_label.set_text("Modified");
                    canvas.queue_draw();
                }
            }
        }
        dialog.close();
    });
    dialog.present();
}

#[allow(deprecated)]
fn show_dwg_setup_dialog(window: &adw::ApplicationWindow, import_error: &str) {
    let dialog = gtk::Dialog::builder()
        .transient_for(window)
        .modal(true)
        .title("DWG import requires a converter")
        .build();
    dialog.add_button("Detect converters", gtk::ResponseType::Other(1));
    dialog.add_button("Configure converter path", gtk::ResponseType::Other(2));
    dialog.add_button("Open documentation", gtk::ResponseType::Other(3));
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let area = dialog.content_area();
    area.set_spacing(12);
    area.set_margin_top(16);
    area.set_margin_bottom(16);
    area.set_margin_start(16);
    area.set_margin_end(16);

    let intro = gtk::Label::new(Some(
        "DWG is a proprietary CAD format. Lix CAD can import DWG by converting it internally to DXF using a local converter.",
    ));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    intro.add_css_class("context-note");
    area.append(&intro);

    let status = gtk::Label::new(Some(
        import_error
            .trim_start_matches(crate::import::dwg::DWG_SETUP_REQUIRED)
            .trim(),
    ));
    status.set_wrap(true);
    status.set_xalign(0.0);
    status.add_css_class("prop-value");
    area.append(&status);

    let window_for_response = window.clone();
    dialog.connect_response(move |dialog, response| match response {
        gtk::ResponseType::Other(1) => {
            show_info(
                &window_for_response,
                "DWG converter detection",
                &crate::import::dwg::converter_status_text(),
            );
        }
        gtk::ResponseType::Other(2) => {
            show_info(
                &window_for_response,
                "Configure DWG converter",
                &format!(
                    "Create or edit:\n{}\n\nExample:\n[dwg_import]\nbackend = \"oda\"\nconverter_path = \"/usr/bin/oda-file-converter\"\n\nAlso supported:\n~/.config/lixcad/settings.toml\n~/.config/nodalix-cad/settings.toml\n\nLix CAD will use this path on the next DWG import.",
                    crate::import::dwg::settings_path().display()
                ),
            );
        }
        gtk::ResponseType::Other(3) => {
            show_info(
                &window_for_response,
                "DWG import documentation",
                "See README.md and INTEGRATION_RESEARCH.md. ODA File Converter and LibreDWG are detected from settings/PATH. ODA is accepted with backend = \"oda\" and converter_path = \"/usr/bin/oda-file-converter\".",
            );
        }
        _ => dialog.close(),
    });
    dialog.present();
}

#[allow(deprecated)]
fn show_settings_dialog(window: &adw::ApplicationWindow) {
    let dialog = gtk::Dialog::builder()
        .transient_for(window)
        .modal(true)
        .title("Lix CAD Settings")
        .default_width(520)
        .build();
    dialog.add_button("Close", gtk::ResponseType::Close);
    let area = dialog.content_area();
    area.set_spacing(12);
    area.set_margin_top(16);
    area.set_margin_bottom(16);
    area.set_margin_start(16);
    area.set_margin_end(16);

    section_title(&area, "General");
    property(&area, "Native format", ".nodcad / .lixcad JSON project");
    section_title(&area, "Units");
    property(&area, "Default units", "millimeters");
    section_title(&area, "Snaps");
    property(
        &area,
        "Active snaps",
        "grid, endpoint, midpoint, center, quadrant, intersection, perpendicular, ortho",
    );
    section_title(&area, "DWG Import");
    property(
        &area,
        "Converter status",
        &crate::import::dwg::converter_status_text(),
    );
    section_title(&area, "External Tools");
    property(
        &area,
        "ODA File Converter",
        "detected from settings/PATH; backend = \"oda\" is supported",
    );
    property(
        &area,
        "LibreDWG",
        "dwg2dxf can be used for DWG -> DXF import",
    );
    property(
        &area,
        "FreeCAD",
        "detected as future bridge for CAD conversions",
    );
    section_title(&area, "Experimental");
    property(
        &area,
        "STEP",
        "metadata/reference import now; B-Rep/OCCT planned",
    );

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

#[allow(deprecated)]
fn choose_file<F>(
    window: &adw::ApplicationWindow,
    title: &str,
    action: gtk::FileChooserAction,
    on_path: F,
) where
    F: Fn(PathBuf) + 'static,
{
    let accept = match action {
        gtk::FileChooserAction::Save => "Save",
        _ => "Open",
    };
    let dialog = gtk::FileChooserNative::new(
        Some(title),
        Some(window),
        action,
        Some(accept),
        Some("Cancel"),
    );
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                on_path(path);
            }
        }
        dialog.destroy();
    });
    dialog.show();
}

#[allow(deprecated)]
fn show_import_summary(window: &adw::ApplicationWindow, summary: &crate::import::ImportSummary) {
    let mut text = format!("{}\n\n{}", summary.file_name, summary.summary);
    for (label, value) in &summary.details {
        text.push_str(&format!("\n{label}: {value}"));
    }
    if !summary.warnings.is_empty() {
        text.push_str("\n\nWarnings:");
        for warning in &summary.warnings {
            text.push_str(&format!("\n- {warning}"));
        }
    }
    show_info(window, &format!("Imported {}", summary.format), &text);
}

#[allow(deprecated)]
fn show_error(window: &adw::ApplicationWindow, title: &str, text: &str) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .message_type(gtk::MessageType::Error)
        .buttons(gtk::ButtonsType::Close)
        .text(title)
        .secondary_text(text)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

#[allow(deprecated)]
fn show_info(window: &adw::ApplicationWindow, title: &str, text: &str) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .message_type(gtk::MessageType::Info)
        .buttons(gtk::ButtonsType::Close)
        .text(title)
        .secondary_text(text)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/nodalix-cad.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

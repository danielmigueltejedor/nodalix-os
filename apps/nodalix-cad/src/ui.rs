use crate::tool_parameters::{ArcUiMode, DimensionCreationMode};
use crate::{
    cad::commands::command_registry::{CommandRegistry, ParsedCommand},
    cad::commands::{build_geometry_from_command_parts, GeometryBuildResult},
    cad::geometry::{CircleCreationMode, LineCreationMode, RectangleCreationMode},
    cad::precision::PrecisionState,
    cad::snapping::OsnapState,
    canvas::{update_selection_label, CadCanvas, CanvasInteractionContext},
    document::{Document, Entity},
    geometry::Point,
    tools::Tool,
    ui_context::{DocumentTab, UiCadContext, UiDocumentTabsContext, UiViewContext},
    ui_history::{
        clear_document_history, delete_selected_entities, document_active_layout,
        duplicate_entity_with_history, paste_entities_at, perform_redo, perform_undo,
        refresh_after_history_change, try_add_entity_with_history,
        update_entity_properties_with_history, update_layers_with_history, DocumentBusy,
    },
    units::Unit,
};
use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

const COMMANDS: &[(&str, &str)] = &[
    ("line 1,4 5,7", "línea por dos puntos"),
    ("line 3 1,4 90", "línea por longitud, punto y ángulo"),
    ("pline 0,0 5,0 5,3", "polilínea por puntos"),
    ("rectangle 0,0 10,6", "rectángulo por esquinas"),
    ("circle 0,0 4", "círculo centro radio"),
    ("point 2,3", "punto"),
    ("arc", "arco"),
    ("select", "selección"),
    ("move", "mover"),
    ("copy", "copiar"),
    ("delete", "borrar"),
    ("dimension", "cota"),
    ("text", "texto"),
    ("hatch", "sombreado"),
    ("table", "tabla"),
    ("viewport", "crear viewport"),
    ("vpscale 100", "escala viewport 1:100"),
    ("vplock", "bloquear viewport"),
    ("newlayout", "crear presentación"),
    ("duplayout", "duplicar presentación"),
    ("deletelayout", "borrar presentación"),
    ("model", "espacio modelo"),
    ("layout ", "activar presentación"),
    ("fit", "ajustar vista"),
    ("zoomin", "acercar"),
    ("zoomout", "alejar"),
    ("reset", "reset vista"),
    ("pan", "desplazar vista"),
    ("orbit", "vista 3D"),
    ("extrude", "extruir"),
    ("section", "sección"),
    ("parametric", "restricciones"),
];

pub fn build(app: &adw::Application) {
    load_css();

    let document = Rc::new(RefCell::new(Document::new_empty()));
    let cad = UiCadContext::new(document);
    let active_tool = Rc::new(RefCell::new(Tool::Select));
    let inline_text_entry = gtk::Entry::new();
    inline_text_entry.set_placeholder_text(Some("Text"));
    inline_text_entry.set_visible(false);
    inline_text_entry.set_halign(gtk::Align::Start);
    inline_text_entry.set_valign(gtk::Align::Start);
    inline_text_entry.add_css_class("inline-text-entry");

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

    let document_tabs_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    document_tabs_bar.add_css_class("document-tabs-bar");
    let document_tabs_ctx = UiDocumentTabsContext {
        tabs: Rc::new(RefCell::new(vec![DocumentTab {
            title: "Untitled".to_string(),
            path: None,
            document: cad.document.borrow().clone(),
        }])),
        active_tab: Rc::new(RefCell::new(0usize)),
        bar: document_tabs_bar.clone(),
    };

    let attribute_layer_entry = gtk::Entry::new();
    attribute_layer_entry.set_placeholder_text(Some("Layer"));
    attribute_layer_entry.add_css_class("attribute-entry");

    let canvas = CadCanvas::new(
        CanvasInteractionContext {
            document: cad.document.clone(),
            active_tool: active_tool.clone(),
            selected_entity: cad.selected_entity.clone(),
            history: cad.history.clone(),
            tool_parameters: cad.tool_parameters.clone(),
            osnap: cad.osnap.clone(),
            precision: cad.precision.clone(),
            inline_text_entry: inline_text_entry.clone(),
            attribute_layer_entry: attribute_layer_entry.clone(),
        },
        selection_label.clone(),
    );
    let tool_context = tool_context_panel(*active_tool.borrow(), &cad, &canvas);
    let properties = properties_panel(&cad.document.borrow());
    let layout_tabs = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    layout_tabs.add_css_class("layout-tabs-bar");
    let layer_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    layer_panel.add_css_class("layer-panel");

    let view = Rc::new(UiViewContext {
        canvas: canvas.clone(),
        properties: properties.clone(),
        layout_tabs: layout_tabs.clone(),
        modified_label: modified_label.clone(),
        layer_panel: layer_panel.clone(),
        attribute_layer_entry: attribute_layer_entry.clone(),
    });
    let attributes = attribute_bar(selection_label.clone(), cad.clone(), view.clone());
    let right_panel = right_panel(attributes, properties.clone(), layer_panel.clone());
    refresh_layer_panel(
        &view.layer_panel,
        &cad,
        &view.canvas,
        &view.properties,
        &selection_label,
        &view.modified_label,
    );
    sync_attribute_layer_entry(
        &view.attribute_layer_entry,
        &cad.document.borrow(),
        &cad.selected_entity.borrow(),
    );

    let toolbar = top_toolbar(&window, &cad, &document_tabs_ctx, &view);
    attach_toolbar_context_menu(
        &toolbar,
        &window,
        cad.clone(),
        canvas.clone(),
        active_tool.clone(),
        tool_label.clone(),
        tool_context.clone(),
    );
    root.append(&horizontal_scroll(&toolbar));
    root.append(&document_tabs_bar);

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.set_hexpand(true);
    body.set_vexpand(true);
    root.append(&body);

    body.append(&vertical_scroll(&tool_palette(
        active_tool.clone(),
        tool_label.clone(),
        tool_context.clone(),
        cad.clone(),
        canvas.clone(),
    )));
    let cursor = canvas.cursor();
    attach_cursor_tracking(
        canvas.widget(),
        cursor,
        cursor_label.clone(),
        canvas.camera(),
    );
    attach_canvas_context_menu(
        window.clone(),
        canvas.clone(),
        cad.clone(),
        view.clone(),
        selection_label.clone(),
        active_tool.clone(),
        tool_label.clone(),
        tool_context.clone(),
    );
    let canvas_overlay = gtk::Overlay::new();
    canvas_overlay.set_hexpand(true);
    canvas_overlay.set_vexpand(true);
    canvas_overlay.set_child(Some(canvas.widget()));
    canvas_overlay.add_overlay(canvas.inline_text_editor());
    let command_bar = floating_command_bar(
        cad.clone(),
        view.clone(),
        selection_label.clone(),
        active_tool.clone(),
        tool_label.clone(),
        tool_context.clone(),
    );
    canvas_overlay.add_overlay(&command_bar);
    let compass = view_compass(canvas.clone(), cad.document.clone());
    canvas_overlay.add_overlay(&compass);
    body.append(&canvas_overlay);
    body.append(&right_panel);

    refresh_layout_tabs(
        &layout_tabs,
        cad.document.clone(),
        canvas.clone(),
        properties.clone(),
    );
    refresh_document_tabs(&document_tabs_ctx, &cad, &*view);
    root.append(&layout_tabs);
    root.append(&horizontal_scroll(&tool_context));
    let osnap_status = gtk::Label::new(None);
    osnap_status.add_css_class("status-label");
    sync_osnap_status_label(&osnap_status, &cad.osnap.borrow());
    root.append(&osnap_toolbar(
        cad.clone(),
        canvas.clone(),
        osnap_status.clone(),
    ));
    let precision_status = gtk::Label::new(None);
    precision_status.add_css_class("status-label");
    sync_precision_status_label(&precision_status, &cad.precision.borrow());
    root.append(&precision_toolbar(
        cad.clone(),
        canvas.clone(),
        precision_status.clone(),
    ));
    root.append(&status_bar(
        cursor_label,
        tool_label,
        modified_label.clone(),
        cad.document.clone(),
        osnap_status,
        precision_status,
    ));

    attach_keyboard_shortcuts(
        &window,
        &cad,
        &*view,
        selection_label.clone(),
        active_tool.clone(),
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

fn title_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Drawing")
        .to_string()
}

fn sync_active_tab(tabs_ctx: &UiDocumentTabsContext, cad: &UiCadContext) {
    let index = *tabs_ctx.active_tab.borrow();
    if let Some(tab) = tabs_ctx.tabs.borrow_mut().get_mut(index) {
        tab.document = cad.document.borrow().clone();
        tab.path = cad.current_path.borrow().clone();
        if let Some(path) = &tab.path {
            tab.title = title_for_path(path);
        } else {
            tab.title = tab.document.name.clone();
        }
    }
}

fn open_document_tab(
    tab: DocumentTab,
    tabs_ctx: &UiDocumentTabsContext,
    cad: &UiCadContext,
    view: &UiViewContext,
) {
    sync_active_tab(tabs_ctx, cad);
    let mut tabs_mut = tabs_ctx.tabs.borrow_mut();
    tabs_mut.push(tab);
    *tabs_ctx.active_tab.borrow_mut() = tabs_mut.len().saturating_sub(1);
    drop(tabs_mut);
    load_document_tab(tabs_ctx, cad, view);
    refresh_document_tabs(tabs_ctx, cad, view);
}

fn load_document_tab(tabs_ctx: &UiDocumentTabsContext, cad: &UiCadContext, view: &UiViewContext) {
    let index = *tabs_ctx.active_tab.borrow();
    let Some(tab) = tabs_ctx.tabs.borrow().get(index).cloned() else {
        return;
    };
    *cad.document.borrow_mut() = tab.document;
    *cad.current_path.borrow_mut() = tab.path;
    clear_document_history(cad);
    refresh_properties(&view.properties, &cad.document.borrow());
    refresh_layout_tabs(
        &view.layout_tabs,
        cad.document.clone(),
        view.canvas.clone(),
        view.properties.clone(),
    );
    view.canvas.fit_document(&cad.document.borrow());
}

fn refresh_document_tabs(
    tabs_ctx: &UiDocumentTabsContext,
    cad: &UiCadContext,
    view: &UiViewContext,
) {
    let bar = &tabs_ctx.bar;
    while let Some(child) = bar.first_child() {
        bar.remove(&child);
    }
    let tab_snapshot = tabs_ctx.tabs.borrow().clone();
    let active = *tabs_ctx.active_tab.borrow();
    for (index, tab) in tab_snapshot.into_iter().enumerate() {
        let tab_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        tab_box.add_css_class("document-tab");
        if index == active {
            tab_box.add_css_class("document-tab-active");
        }

        let button = gtk::Button::with_label(&tab.title);
        button.add_css_class("document-tab-title");
        button.set_has_frame(false);
        button.add_css_class("document-tab");
        if index == active {
            button.add_css_class("document-tab-active");
        }
        {
            let tabs_ctx = tabs_ctx.clone();
            let cad = cad.clone();
            let view = view.clone();
            button.connect_clicked(move |_| {
                sync_active_tab(&tabs_ctx, &cad);
                *tabs_ctx.active_tab.borrow_mut() = index;
                load_document_tab(&tabs_ctx, &cad, &view);
                refresh_document_tabs(&tabs_ctx, &cad, &view);
            });
        }
        let close = gtk::Button::with_label("×");
        close.add_css_class("document-tab-close");
        close.set_tooltip_text(Some("Close drawing"));
        {
            let tabs_ctx = tabs_ctx.clone();
            let cad = cad.clone();
            let view = view.clone();
            close.connect_clicked(move |_| {
                sync_active_tab(&tabs_ctx, &cad);
                close_document_tab(index, &tabs_ctx, &cad, &view);
            });
        }
        tab_box.append(&button);
        tab_box.append(&close);
        bar.append(&tab_box);
    }
}

fn close_document_tab(
    index: usize,
    tabs_ctx: &UiDocumentTabsContext,
    cad: &UiCadContext,
    view: &UiViewContext,
) {
    let Some(tab) = tabs_ctx.tabs.borrow().get(index).cloned() else {
        return;
    };
    let close_now = {
        let tabs_ctx = tabs_ctx.clone();
        let cad = cad.clone();
        let view = view.clone();
        move || {
            if tabs_ctx.tabs.borrow().len() <= 1 {
                let mut tab = tabs_ctx.tabs.borrow_mut();
                tab[0] = DocumentTab {
                    title: "Untitled".to_string(),
                    path: None,
                    document: Document::new_empty(),
                };
                *tabs_ctx.active_tab.borrow_mut() = 0;
            } else {
                tabs_ctx.tabs.borrow_mut().remove(index);
                let len = tabs_ctx.tabs.borrow().len();
                if *tabs_ctx.active_tab.borrow() >= len {
                    *tabs_ctx.active_tab.borrow_mut() = len.saturating_sub(1);
                }
            }
            load_document_tab(&tabs_ctx, &cad, &view);
            refresh_document_tabs(&tabs_ctx, &cad, &view);
        }
    };
    if tab.document.modified {
        show_unsaved_close_dialog(&tab.title, close_now);
    } else {
        close_now();
    }
}

#[allow(deprecated)]
fn show_unsaved_close_dialog(title: &str, close_now: impl Fn() + 'static) {
    let dialog = gtk::MessageDialog::builder()
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text("Hay cambios sin guardar")
        .secondary_text(format!(
            "{title} tiene cambios sin guardar. Guarda desde la toolbar si quieres conservarlos."
        ))
        .build();
    dialog.add_button("Cancelar", gtk::ResponseType::Cancel);
    dialog.add_button("Cerrar sin guardar", gtk::ResponseType::Accept);
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            close_now();
        }
        dialog.close();
    });
    dialog.present();
}

fn top_toolbar(
    window: &adw::ApplicationWindow,
    cad: &UiCadContext,
    document_tabs_ctx: &UiDocumentTabsContext,
    view: &UiViewContext,
) -> gtk::Box {
    let document = cad.document.clone();
    let current_path = cad.current_path.clone();
    let cad_canvas = view.canvas.clone();
    let properties = view.properties.clone();
    let modified_label = view.modified_label.clone();
    let layout_tabs = view.layout_tabs.clone();

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("top-toolbar");
    let canvas = cad_canvas.widget().clone();
    let canvas_controller = cad_canvas.clone();

    let new_button = toolbar_button("New");
    {
        let tabs_ctx = document_tabs_ctx.clone();
        let cad_ctx = cad.clone();
        let view_ctx = view.clone();
        let canvas = canvas.clone();
        let modified_label = modified_label.clone();
        new_button.connect_clicked(move |_| {
            open_document_tab(
                DocumentTab {
                    title: "Untitled".to_string(),
                    path: None,
                    document: Document::new_empty(),
                },
                &tabs_ctx,
                &cad_ctx,
                &view_ctx,
            );
            modified_label.set_text("New document");
            canvas.queue_draw();
        });
    }
    bar.append(&new_button);

    let open_button = toolbar_button("Open");
    {
        let window = window.clone();
        let canvas = canvas.clone();
        let modified_label = modified_label.clone();
        let tabs_ctx = document_tabs_ctx.clone();
        let cad_ctx = cad.clone();
        let view_ctx = view.clone();
        open_button.connect_clicked(move |_| {
            choose_file(&window, "Open Lix CAD", gtk::FileChooserAction::Open, {
                let canvas = canvas.clone();
                let modified_label = modified_label.clone();
                let error_window = window.clone();
                let tabs_ctx = tabs_ctx.clone();
                let cad_ctx = cad_ctx.clone();
                let view_ctx = view_ctx.clone();
                move |path| {
                    if is_native_document(&path) {
                        match Document::open_nodcad(&path) {
                            Ok(opened) => {
                                open_document_tab(
                                    DocumentTab {
                                        title: title_for_path(&path),
                                        path: Some(path.clone()),
                                        document: opened,
                                    },
                                    &tabs_ctx,
                                    &cad_ctx,
                                    &view_ctx,
                                );
                                modified_label.set_text("Opened");
                                canvas.queue_draw();
                            }
                            Err(error) => show_error(&error_window, "Open failed", &error),
                        }
                    } else {
                        match import_as_new_document(&path) {
                            Ok((opened, summary)) => {
                                open_document_tab(
                                    DocumentTab {
                                        title: title_for_path(&path),
                                        path: None,
                                        document: opened,
                                    },
                                    &tabs_ctx,
                                    &cad_ctx,
                                    &view_ctx,
                                );
                                modified_label.set_text("Opened");
                                canvas.queue_draw();
                                show_import_summary(&error_window, &summary);
                            }
                            Err(error)
                                if error.starts_with(crate::import::dwg::DWG_SETUP_REQUIRED) =>
                            {
                                show_dwg_setup_dialog(&error_window, &error)
                            }
                            Err(error) => show_error(&error_window, "Open failed", &error),
                        }
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
        let tabs_ctx = document_tabs_ctx.clone();
        let cad_ctx = cad.clone();
        let view_ctx = view.clone();
        save_button.connect_clicked(move |_| {
            if let Some(path) = current_path.borrow().clone() {
                save_document(&window, &document, &path, &modified_label);
                sync_active_tab(&tabs_ctx, &cad_ctx);
                refresh_document_tabs(&tabs_ctx, &cad_ctx, &view_ctx);
            } else {
                choose_file(&window, "Save Lix CAD", gtk::FileChooserAction::Save, {
                    let window = window.clone();
                    let document = document.clone();
                    let current_path = current_path.clone();
                    let modified_label = modified_label.clone();
                    let tabs_ctx = tabs_ctx.clone();
                    let cad_ctx = cad_ctx.clone();
                    let view_ctx = view_ctx.clone();
                    move |path| {
                        save_document(&window, &document, &path, &modified_label);
                        *current_path.borrow_mut() = Some(path);
                        sync_active_tab(&tabs_ctx, &cad_ctx);
                        refresh_document_tabs(&tabs_ctx, &cad_ctx, &view_ctx);
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
        let canvas_controller = canvas_controller.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        let layout_tabs = layout_tabs.clone();
        import_button.connect_clicked(move |_| {
            choose_file(
                &window,
                "Import CAD/Mesh File",
                gtk::FileChooserAction::Open,
                {
                    let window = window.clone();
                    let document = document.clone();
                    let canvas_controller = canvas_controller.clone();
                    let properties = properties.clone();
                    let modified_label = modified_label.clone();
                    let layout_tabs = layout_tabs.clone();
                    move |path| {
                        import_and_refresh(
                            &window,
                            &document,
                            &path,
                            &canvas_controller,
                            &properties,
                            &modified_label,
                            &layout_tabs,
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
        let canvas_controller = canvas_controller.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        let layout_tabs = layout_tabs.clone();
        sample_step.connect_clicked(move |_| {
            let path = PathBuf::from("/mnt/data/5621-Separador.stp");
            import_and_refresh(
                &window,
                &document,
                &path,
                &canvas_controller,
                &properties,
                &modified_label,
                &layout_tabs,
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

    let undo_button = toolbar_button("Undo");
    {
        let cad = cad.clone();
        let canvas = cad_canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        undo_button.connect_clicked(move |_| {
            if perform_undo(&cad) {
                refresh_properties(&properties, &cad.document.borrow());
                modified_label.set_text("Modified");
                canvas.widget().queue_draw();
            }
        });
    }
    bar.append(&undo_button);

    let redo_button = toolbar_button("Redo");
    {
        let cad = cad.clone();
        let canvas = cad_canvas.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        redo_button.connect_clicked(move |_| {
            if perform_redo(&cad) {
                refresh_properties(&properties, &cad.document.borrow());
                modified_label.set_text("Modified");
                canvas.widget().queue_draw();
            }
        });
    }
    bar.append(&redo_button);

    let top_view_button = toolbar_button("View: Top");
    top_view_button.set_tooltip_text(Some(crate::drawing::projection::projection_status()));
    {
        let canvas = cad_canvas.clone();
        top_view_button.connect_clicked(move |_| canvas.set_view_rotation(0.0));
    }
    bar.append(&top_view_button);

    let minimize_button = toolbar_button("Window Minimize");
    minimize_button.set_tooltip_text(Some("Minimize"));
    if !is_hyprland_session() {
        let window = window.clone();
        minimize_button.connect_clicked(move |_| window.minimize());
        bar.append(&minimize_button);
    }

    let fullscreen_button = toolbar_button("Window Fullscreen");
    fullscreen_button.set_tooltip_text(Some("Toggle fullscreen"));
    {
        let window = window.clone();
        fullscreen_button.connect_clicked(move |_| {
            if window.is_fullscreen() {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
        });
    }
    bar.append(&fullscreen_button);

    let close_button = toolbar_button("Window Close");
    close_button.set_tooltip_text(Some("Close window"));
    {
        let window = window.clone();
        close_button.connect_clicked(move |_| window.close());
    }
    bar.append(&close_button);

    let units_button = toolbar_button("Units");
    units_button.set_tooltip_text(Some("Units"));
    {
        let document = document.clone();
        let properties = properties.clone();
        let modified_label = modified_label.clone();
        let canvas = canvas.clone();
        let units_button_for_click = units_button.clone();
        units_button.connect_clicked(move |_| {
            let document = document.clone();
            let properties = properties.clone();
            let modified_label = modified_label.clone();
            let canvas = canvas.clone();
            popup_menu(
                &units_button_for_click,
                0.0,
                units_button_for_click.height() as f64,
                vec![
                    unit_menu_item(
                        "Millimeters",
                        Unit::Millimeters,
                        document.clone(),
                        properties.clone(),
                        canvas.clone(),
                        modified_label.clone(),
                    ),
                    unit_menu_item(
                        "Centimeters",
                        Unit::Centimeters,
                        document.clone(),
                        properties.clone(),
                        canvas.clone(),
                        modified_label.clone(),
                    ),
                    unit_menu_item(
                        "Meters",
                        Unit::Meters,
                        document.clone(),
                        properties.clone(),
                        canvas.clone(),
                        modified_label.clone(),
                    ),
                    unit_menu_item(
                        "Inches",
                        Unit::Inches,
                        document.clone(),
                        properties.clone(),
                        canvas.clone(),
                        modified_label.clone(),
                    ),
                ],
            );
        });
    }
    bar.append(&units_button);

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
    let button = gtk::Button::new();
    button.add_css_class("tool-button");
    if let Some(icon_name) = toolbar_icon_name(label) {
        button.set_child(Some(&app_icon(icon_name, 18)));
    } else {
        let fallback = gtk::Label::new(Some(label));
        fallback.add_css_class("toolbar-symbol");
        button.set_child(Some(&fallback));
    }
    button.set_tooltip_text(Some(label));
    button
}

fn toolbar_icon_name(label: &str) -> Option<&'static str> {
    match label {
        "New" => Some("new"),
        "Open" => Some("open"),
        "Save" => Some("save"),
        "Import" => Some("import"),
        "Export DXF" => Some("export-dxf"),
        "Export DWG" => Some("export-dwg"),
        "Export SVG" => Some("export-svg"),
        "Settings" => Some("settings"),
        "Scale factor" => Some("scale"),
        "View: Top" => Some("view-top"),
        "Units" => Some("units"),
        "−" => Some("zoom-out"),
        "+" => Some("zoom-in"),
        "100%" => Some("zoom-fit"),
        "Undo" => Some("undo"),
        "Redo" => Some("redo"),
        "Window Close" => Some("window-close"),
        "Window Fullscreen" => Some("window-fullscreen"),
        "Window Minimize" => Some("window-minimize"),
        _ => None,
    }
}

fn is_hyprland_session() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || std::env::var("XDG_CURRENT_DESKTOP")
            .map(|value| value.to_ascii_lowercase().contains("hyprland"))
            .unwrap_or(false)
}

fn app_icon(name: &str, size: i32) -> gtk::Image {
    crate::assets::load_icon_image(name, size)
}

fn layer_icon_button(icon_name: &str, tooltip: &str, size: i32) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("layer-icon-button");
    button.set_child(Some(&app_icon(icon_name, size)));
    button
}

fn osnap_toggle_button(
    icon_name: &str,
    label: &str,
    tooltip: &str,
    active: bool,
) -> gtk::ToggleButton {
    let button = gtk::ToggleButton::new();
    button.set_tooltip_text(Some(tooltip));
    button.set_active(active);
    button.add_css_class("osnap-toggle");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    content.append(&app_icon(icon_name, 16));
    let text = gtk::Label::new(Some(label));
    text.add_css_class("osnap-toggle-label");
    content.append(&text);
    button.set_child(Some(&content));
    button
}

fn sync_osnap_status_label(label: &gtk::Label, state: &OsnapState) {
    label.set_text(&osnap_status_text(state));
}

fn osnap_status_text(state: &OsnapState) -> String {
    if !state.enabled {
        return "OSNAP: off".to_string();
    }
    let mut parts = Vec::new();
    if state.endpoint {
        parts.push("END");
    }
    if state.midpoint {
        parts.push("MID");
    }
    if state.center {
        parts.push("CEN");
    }
    if state.intersection {
        parts.push("INT");
    }
    if state.quadrant {
        parts.push("QUAD");
    }
    if state.nearest {
        parts.push("NEAR");
    }
    if state.node {
        parts.push("NODE");
    }
    if state.perpendicular {
        parts.push("PERP");
    }
    if parts.is_empty() {
        "OSNAP: on (none)".to_string()
    } else {
        format!("OSNAP: {}", parts.join(" · "))
    }
}

fn osnap_toolbar(cad: UiCadContext, canvas: CadCanvas, status_label: gtk::Label) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.add_css_class("osnap-bar");
    bar.set_margin_start(12);
    bar.set_margin_end(12);
    bar.set_margin_top(4);
    bar.set_margin_bottom(4);

    let state = cad.osnap.borrow().clone();
    let master = osnap_toggle_button(
        "snap-toggle",
        "OSNAP",
        "Toggle object snap (F3)",
        state.enabled,
    );
    let end = osnap_toggle_button("snap-endpoint", "END", "Endpoint snap", state.endpoint);
    let mid = osnap_toggle_button("snap-midpoint", "MID", "Midpoint snap", state.midpoint);
    let cen = osnap_toggle_button("snap-center", "CEN", "Center snap", state.center);
    let int = osnap_toggle_button(
        "snap-intersection",
        "INT",
        "Intersection snap",
        state.intersection,
    );
    let quad = osnap_toggle_button("snap-quadrant", "QUAD", "Quadrant snap", state.quadrant);
    let near = osnap_toggle_button("snap-nearest", "NEAR", "Nearest snap", state.nearest);
    let node = osnap_toggle_button("snap-node", "NODE", "Node snap (points, text)", state.node);

    let wire_toggle = |button: &gtk::ToggleButton, mutator: fn(&mut OsnapState, bool)| {
        let cad = cad.clone();
        let canvas = canvas.clone();
        let status_label = status_label.clone();
        button.connect_toggled(move |btn| {
            let active = btn.is_active();
            {
                let mut state = cad.osnap.borrow_mut();
                mutator(&mut state, active);
            }
            sync_osnap_status_label(&status_label, &cad.osnap.borrow());
            canvas.widget().queue_draw();
        });
    };

    wire_toggle(&master, |s, v| s.enabled = v);
    wire_toggle(&end, |s, v| s.endpoint = v);
    wire_toggle(&mid, |s, v| s.midpoint = v);
    wire_toggle(&cen, |s, v| s.center = v);
    wire_toggle(&int, |s, v| s.intersection = v);
    wire_toggle(&quad, |s, v| s.quadrant = v);
    wire_toggle(&near, |s, v| s.nearest = v);
    wire_toggle(&node, |s, v| s.node = v);

    bar.append(&master);
    bar.append(&end);
    bar.append(&mid);
    bar.append(&cen);
    bar.append(&int);
    bar.append(&quad);
    bar.append(&near);
    bar.append(&node);
    bar
}

fn sync_precision_status_label(label: &gtk::Label, state: &PrecisionState) {
    let mut parts = Vec::new();
    parts.push(if state.ortho_enabled {
        "ORTHO"
    } else {
        "ortho off"
    });
    parts.push(if state.polar_enabled {
        "POLAR"
    } else {
        "polar off"
    });
    parts.push(if state.dynamic_input_enabled {
        "DYN"
    } else {
        "dyn off"
    });
    label.set_text(&parts.join(" · "));
}

fn precision_toolbar(cad: UiCadContext, canvas: CadCanvas, status_label: gtk::Label) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.add_css_class("precision-bar");
    bar.set_margin_start(12);
    bar.set_margin_end(12);
    bar.set_margin_top(2);
    bar.set_margin_bottom(4);

    let state = cad.precision.borrow().clone();
    let ortho = osnap_toggle_button(
        "precision/precision-ortho",
        "ORTHO",
        "Orthogonal mode — lock cursor to horizontal/vertical (F8)",
        state.ortho_enabled,
    );
    let polar = osnap_toggle_button(
        "precision/precision-polar",
        "POLAR",
        "Polar tracking — snap cursor to standard angles (F10)",
        state.polar_enabled,
    );
    let dynamic = osnap_toggle_button(
        "precision/precision-dynamic-input",
        "DYN",
        "Dynamic input — show length and angle near cursor (F12)",
        state.dynamic_input_enabled,
    );

    let wire = |button: &gtk::ToggleButton, mutator: fn(&mut PrecisionState, bool)| {
        let cad = cad.clone();
        let canvas = canvas.clone();
        let status_label = status_label.clone();
        button.connect_toggled(move |btn| {
            let active = btn.is_active();
            {
                let mut state = cad.precision.borrow_mut();
                mutator(&mut state, active);
            }
            sync_precision_status_label(&status_label, &cad.precision.borrow());
            canvas.widget().queue_draw();
        });
    };

    wire(&ortho, |s, v| s.ortho_enabled = v);
    wire(&polar, |s, v| s.polar_enabled = v);
    wire(&dynamic, |s, v| s.dynamic_input_enabled = v);

    bar.append(&ortho);
    bar.append(&polar);
    bar.append(&dynamic);
    bar
}

fn try_execute_osnap_command(
    command: &str,
    osnap: &Rc<RefCell<OsnapState>>,
    history: &gtk::Label,
) -> bool {
    match command {
        "osnap" => {
            let enabled = {
                let mut state = osnap.borrow_mut();
                state.enabled = !state.enabled;
                state.enabled
            };
            history.set_text(if enabled { "OSNAP: on" } else { "OSNAP: off" });
            true
        }
        "osnap on" => {
            osnap.borrow_mut().enabled = true;
            history.set_text("OSNAP: on");
            true
        }
        "osnap off" => {
            osnap.borrow_mut().enabled = false;
            history.set_text("OSNAP: off");
            true
        }
        "snap end" => {
            toggle_osnap_mode(osnap, |s| &mut s.endpoint);
            history.set_text("SNAP END toggled");
            true
        }
        "snap mid" => {
            toggle_osnap_mode(osnap, |s| &mut s.midpoint);
            history.set_text("SNAP MID toggled");
            true
        }
        "snap cen" => {
            toggle_osnap_mode(osnap, |s| &mut s.center);
            history.set_text("SNAP CEN toggled");
            true
        }
        "snap int" => {
            toggle_osnap_mode(osnap, |s| &mut s.intersection);
            history.set_text("SNAP INT toggled");
            true
        }
        "snap near" => {
            toggle_osnap_mode(osnap, |s| &mut s.nearest);
            history.set_text("SNAP NEAR toggled");
            true
        }
        "snap node" => {
            toggle_osnap_mode(osnap, |s| &mut s.node);
            history.set_text("SNAP NODE toggled");
            true
        }
        _ => false,
    }
}

fn toggle_osnap_mode(osnap: &Rc<RefCell<OsnapState>>, field: fn(&mut OsnapState) -> &mut bool) {
    let mut state = osnap.borrow_mut();
    let value = field(&mut state);
    *value = !*value;
}

pub(crate) fn sync_attribute_layer_entry(
    entry: &gtk::Entry,
    document: &Document,
    selected: &[u64],
) {
    let text = document.selection_layer_field_text(selected);
    if entry.text().as_str() != text {
        entry.set_text(&text);
    }
}

fn is_native_document(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("nodcad") || ext.eq_ignore_ascii_case("lixcad"))
        .unwrap_or(false)
}

fn tool_palette(
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
    cad: UiCadContext,
    canvas: CadCanvas,
) -> gtk::Box {
    let palette = gtk::Box::new(gtk::Orientation::Vertical, 8);
    palette.add_css_class("tool-palette");
    let buttons: Rc<RefCell<Vec<gtk::ToggleButton>>> = Rc::new(RefCell::new(Vec::new()));

    for (tool, icon, label) in Tool::all() {
        let button = gtk::ToggleButton::new();
        button.set_child(Some(&app_icon(icon, 20)));
        button.set_tooltip_text(Some(label));
        button.add_css_class("palette-button");
        if *tool == Tool::Select {
            button.set_active(true);
        }
        let current = *tool;
        let active_tool = active_tool.clone();
        let tool_label = tool_label.clone();
        let tool_context = tool_context.clone();
        let cad = cad.clone();
        let canvas = canvas.clone();
        let buttons_for_click = buttons.clone();
        button.connect_clicked(move |button| {
            set_active_tool(
                &active_tool,
                &tool_label,
                &tool_context,
                &cad,
                &canvas,
                current,
            );
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

fn right_panel(attributes: gtk::Box, properties: gtk::Box, layer_panel: gtk::Box) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.add_css_class("right-panel");
    panel.append(&panel_header(
        "Propiedades",
        "Parámetros editables de la selección",
    ));
    panel.append(&attributes);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("panel-separator");
    panel.append(&separator);

    panel.append(&panel_header("Layers", "Gestión de capas"));
    let layer_scroll = gtk::ScrolledWindow::new();
    layer_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    layer_scroll.set_vexpand(true);
    layer_scroll.set_child(Some(&layer_panel));
    panel.append(&layer_scroll);

    panel.append(&panel_header("Documento", "Estado, capas y referencias"));
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_child(Some(&properties));
    panel.append(&scroll);
    panel
}

fn panel_header(title: &str, subtitle: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Vertical, 2);
    header.add_css_class("panel-header");
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.add_css_class("panel-heading");
    let subtitle = gtk::Label::new(Some(subtitle));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("panel-subtitle");
    header.append(&title);
    header.append(&subtitle);
    header
}

#[allow(deprecated)]
fn attribute_bar(
    selection_label: gtk::Label,
    cad: UiCadContext,
    view: Rc<UiViewContext>,
) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 10);
    bar.add_css_class("attribute-bar");
    bar.append(&selection_label);

    let group = gtk::Box::new(gtk::Orientation::Vertical, 6);
    group.add_css_class("attribute-group");

    let layer = view.attribute_layer_entry.clone();

    let lineweight = gtk::Entry::new();
    lineweight.set_placeholder_text(Some("Lineweight"));
    lineweight.set_text("0.25 mm");
    lineweight.add_css_class("attribute-entry");

    let linetype = gtk::Entry::new();
    linetype.set_placeholder_text(Some("Linetype"));
    linetype.set_text("Continuous");
    linetype.add_css_class("attribute-entry");

    let color_button = gtk::ColorButton::new();
    color_button.add_css_class("color-picker");
    color_button.set_rgba(&gdk::RGBA::new(0.61, 0.64, 0.69, 1.0));
    color_button.set_tooltip_text(Some("Elegir color"));

    let red = rgb_spin(156);
    let green = rgb_spin(163);
    let blue = rgb_spin(175);
    for spin in [&red, &green, &blue] {
        spin.add_css_class("rgb-spin");
    }

    {
        let red = red.clone();
        let green = green.clone();
        let blue = blue.clone();
        color_button.connect_rgba_notify(move |button| {
            let rgba = button.rgba();
            red.set_value((rgba.red() * 255.0).round() as f64);
            green.set_value((rgba.green() * 255.0).round() as f64);
            blue.set_value((rgba.blue() * 255.0).round() as f64);
        });
    }

    for spin in [&red, &green, &blue] {
        let color_button = color_button.clone();
        let red = red.clone();
        let green = green.clone();
        let blue = blue.clone();
        spin.connect_value_changed(move |_| {
            color_button.set_rgba(&gdk::RGBA::new(
                red.value_as_int() as f32 / 255.0,
                green.value_as_int() as f32 / 255.0,
                blue.value_as_int() as f32 / 255.0,
                1.0,
            ));
        });
    }

    let apply = gtk::Button::with_label("Aplicar");
    apply.add_css_class("context-chip");
    {
        let cad = cad.clone();
        let view = view.clone();
        let selection_label = selection_label.clone();
        let layer = layer.clone();
        let lineweight = lineweight.clone();
        let linetype = linetype.clone();
        let red = red.clone();
        let green = green.clone();
        let blue = blue.clone();
        apply.connect_clicked(move |_| {
            let color_value = rgb_hex(&red, &green, &blue);
            let line_weight = parse_lineweight_value(lineweight.text().as_str());
            let line_type = {
                let value = linetype.text().to_string();
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            };
            apply_selected_attributes(
                &cad,
                &view,
                &selection_label,
                Some(layer.text().as_str()),
                Some(&color_value),
                line_weight,
                line_type.as_deref(),
            );
        });
    }

    let reset_color = gtk::Button::with_label("Color capa");
    reset_color.add_css_class("context-chip");
    {
        let cad = cad.clone();
        let view = view.clone();
        let selection_label = selection_label.clone();
        let layer = layer.clone();
        reset_color.connect_clicked(move |_| {
            apply_selected_attributes(
                &cad,
                &view,
                &selection_label,
                Some(layer.text().as_str()),
                Some("default"),
                None,
                None,
            );
        });
    }

    group.append(&field_label("Layer"));
    group.append(&layer);
    group.append(&field_label("Weight"));
    group.append(&lineweight);
    group.append(&field_label("Type"));
    group.append(&linetype);
    bar.append(&group);

    let color_group = gtk::Box::new(gtk::Orientation::Vertical, 7);
    color_group.add_css_class("color-control");
    color_group.append(&field_label("Color"));
    color_group.append(&color_button);

    let rgb_group = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    rgb_group.append(&field_label("R"));
    rgb_group.append(&red);
    rgb_group.append(&field_label("G"));
    rgb_group.append(&green);
    rgb_group.append(&field_label("B"));
    rgb_group.append(&blue);
    color_group.append(&rgb_group);

    let action_group = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    action_group.append(&apply);
    action_group.append(&reset_color);
    color_group.append(&action_group);

    bar.append(&color_group);
    bar
}

fn refresh_layout_tabs(
    bar: &gtk::Box,
    document: Rc<RefCell<Document>>,
    canvas: CadCanvas,
    properties: gtk::Box,
) {
    while let Some(child) = bar.first_child() {
        bar.remove(&child);
    }
    let layouts = document.borrow().layouts.clone();
    let active = document.borrow().active_layout.clone();
    for layout in layouts {
        let button = gtk::Button::with_label(&layout.name);
        button.add_css_class("layout-tab");
        if layout.name == active {
            button.add_css_class("layout-tab-active");
        }
        {
            let document = document.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let bar = bar.clone();
            let name = layout.name.clone();
            button.connect_clicked(move |_| {
                document.borrow_mut().set_active_layout(&name);
                refresh_properties(&properties, &document.borrow());
                refresh_layout_tabs(&bar, document.clone(), canvas.clone(), properties.clone());
                canvas.fit_document(&document.borrow());
            });
        }
        {
            let document = document.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let bar = bar.clone();
            let name = layout.name.clone();
            let tab_widget = button.clone();
            let click = gtk::GestureClick::new();
            click.set_button(3);
            click.connect_pressed(move |_, _, x, y| {
                let mut items = vec![
                    menu_item("Activate layout", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let properties = properties.clone();
                        let bar = bar.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().set_active_layout(&name);
                            refresh_properties(&properties, &document.borrow());
                            refresh_layout_tabs(
                                &bar,
                                document.clone(),
                                canvas.clone(),
                                properties.clone(),
                            );
                            canvas.widget().queue_draw();
                        }
                    }),
                    menu_item("Duplicate layout", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let properties = properties.clone();
                        let bar = bar.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().duplicate_layout(&name);
                            refresh_properties(&properties, &document.borrow());
                            refresh_layout_tabs(
                                &bar,
                                document.clone(),
                                canvas.clone(),
                                properties.clone(),
                            );
                            canvas.fit_document(&document.borrow());
                        }
                    }),
                    menu_item("Fit layout", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().set_active_layout(&name);
                            canvas.fit_document(&document.borrow());
                        }
                    }),
                ];
                if name != "Model" {
                    items.push(menu_item("New viewport", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let properties = properties.clone();
                        let bar = bar.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().set_active_layout(&name);
                            document.borrow_mut().create_viewport_for_active_layout();
                            refresh_properties(&properties, &document.borrow());
                            refresh_layout_tabs(
                                &bar,
                                document.clone(),
                                canvas.clone(),
                                properties.clone(),
                            );
                            canvas.fit_document(&document.borrow());
                        }
                    }));
                    items.push(menu_item("Toggle viewport lock", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let properties = properties.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().set_active_layout(&name);
                            document.borrow_mut().toggle_active_layout_viewport_lock();
                            refresh_properties(&properties, &document.borrow());
                            canvas.widget().queue_draw();
                        }
                    }));
                    items.push(menu_item("Delete layout", {
                        let document = document.clone();
                        let canvas = canvas.clone();
                        let properties = properties.clone();
                        let bar = bar.clone();
                        let name = name.clone();
                        move || {
                            document.borrow_mut().delete_layout(&name);
                            refresh_properties(&properties, &document.borrow());
                            refresh_layout_tabs(
                                &bar,
                                document.clone(),
                                canvas.clone(),
                                properties.clone(),
                            );
                            canvas.widget().queue_draw();
                        }
                    }));
                }
                popup_menu(&tab_widget, x, y, items);
            });
            button.add_controller(click);
        }
        bar.append(&button);
    }

    let add_button = gtk::Button::with_label("+");
    add_button.add_css_class("layout-tab");
    add_button.add_css_class("layout-add-tab");
    add_button.set_tooltip_text(Some("Crear presentación"));
    {
        let document = document.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let bar = bar.clone();
        add_button.connect_clicked(move |_| {
            document.borrow_mut().create_paper_layout();
            refresh_properties(&properties, &document.borrow());
            refresh_layout_tabs(&bar, document.clone(), canvas.clone(), properties.clone());
            canvas.widget().queue_draw();
        });
    }
    bar.append(&add_button);
}

fn field_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("field-label");
    label
}

fn rgb_spin(value: u8) -> gtk::SpinButton {
    let spin = gtk::SpinButton::with_range(0.0, 255.0, 1.0);
    spin.set_value(value as f64);
    spin.set_numeric(true);
    spin.set_width_chars(3);
    spin
}

fn rgb_hex(red: &gtk::SpinButton, green: &gtk::SpinButton, blue: &gtk::SpinButton) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        red.value_as_int().clamp(0, 255),
        green.value_as_int().clamp(0, 255),
        blue.value_as_int().clamp(0, 255)
    )
}

fn parse_lineweight_value(value: &str) -> Option<f64> {
    let normalized = value.trim().to_ascii_lowercase();
    let numeric = normalized.trim_end_matches("mm").trim();
    let weight = numeric.parse::<f64>().ok()?;
    if weight.is_finite() && weight > 0.0 {
        Some(weight)
    } else {
        None
    }
}

fn parse_positive_f64(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn apply_selected_attributes(
    cad: &UiCadContext,
    view: &UiViewContext,
    selection_label: &gtk::Label,
    layer_name: Option<&str>,
    color_value: Option<&str>,
    line_weight: Option<f64>,
    line_type: Option<&str>,
) {
    let selected_ids = cad.selected_entity.borrow().clone();
    if selected_ids.is_empty() {
        if let Some(layer_name) = layer_name {
            let trimmed = layer_name.trim();
            if trimmed.is_empty() || trimmed == "(mixed)" {
                return;
            }
            if update_layers_with_history(cad, |doc| doc.set_active_layer_name(trimmed)) {
                let doc = cad.document.borrow();
                sync_attribute_layer_entry(&view.attribute_layer_entry, &doc, &[]);
                refresh_layer_panel(
                    &view.layer_panel,
                    cad,
                    &view.canvas,
                    &view.properties,
                    selection_label,
                    &view.modified_label,
                );
                view.modified_label.set_text("Modified");
                view.canvas.widget().queue_draw();
            }
        }
        return;
    }

    let changed = update_entity_properties_with_history(cad, &selected_ids, |doc, id| {
        let mut entity_changed = false;
        if let Some(layer_name) = layer_name {
            let trimmed = layer_name.trim();
            if !trimmed.is_empty() && trimmed != "(mixed)" {
                entity_changed |= doc.set_entity_layer(id, trimmed);
            }
        }
        if let Some(color_value) = color_value {
            entity_changed |= doc.set_entity_color(id, color_value);
        }
        if let Some(weight) = line_weight {
            entity_changed |= doc.set_entity_line_weight(id, weight);
        }
        if let Some(line_type) = line_type {
            if !line_type.trim().is_empty() {
                entity_changed |= doc.set_entity_line_type(id, line_type.trim());
            }
        }
        entity_changed
    });

    if changed {
        let doc = cad.document.borrow();
        let selected = cad.selected_entity.borrow();
        update_selection_label(selection_label, &doc, &selected);
        sync_attribute_layer_entry(&view.attribute_layer_entry, &doc, &selected);
        refresh_properties(&view.properties, &doc);
        view.modified_label.set_text("Modified");
        view.canvas.widget().queue_draw();
    }
}

fn tool_context_panel(tool: Tool, cad: &UiCadContext, canvas: &CadCanvas) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    panel.add_css_class("tool-context-panel");
    refresh_tool_context(&panel, tool, cad, canvas);
    panel
}

fn refresh_tool_context(panel: &gtk::Box, tool: Tool, cad: &UiCadContext, canvas: &CadCanvas) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    section_title(panel, "Tool parameters");
    property(panel, "Active tool", tool.label());

    match tool {
        Tool::Select => {
            compact_note(
                panel,
                "Pick, drag, box-select. Drag grip to edit entity. Del removes selection.",
            );
            context_buttons(panel, &["Move", "Copy", "Rotate"]);
        }
        Tool::Polyline => {
            compact_note(panel, "Click vertices. Enter finishes. Esc cancels.");
            context_inline(
                panel,
                &[
                    ("Layer", "Default"),
                    ("Weight", "0.25"),
                    ("Type", "Continuous"),
                    ("Snap", "Object"),
                ],
            );
            context_buttons(panel, &["Close later", "Object snap", "Construction"]);
        }
        Tool::Line | Tool::Rectangle | Tool::Circle | Tool::Arc => {
            context_inline(
                panel,
                &[
                    ("Layer", "Default"),
                    ("Weight", "0.25"),
                    ("Type", "Continuous"),
                    ("Snap", "Grid"),
                ],
            );
            context_buttons(panel, &["Ortho", "Object snap", "Construction"]);
            add_mode_controls(panel, tool, cad, canvas);
        }
        Tool::Dimension => {
            context_entry(panel, "Style", "ISO-25");
            context_entry(panel, "Precision", "0.00");
            context_entry(panel, "Arrow", "Closed filled");
            context_entry(panel, "Text height", "2.5 mm");
            add_mode_controls(panel, tool, cad, canvas);
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
        Tool::Move => {
            compact_note(
                panel,
                "Select entities first. Base point, then destination.",
            );
            context_entry(panel, "Mode", "Move selection");
            context_buttons(panel, &["Ortho", "Polar", "OSNAP"]);
        }
        Tool::Copy => {
            compact_note(
                panel,
                "Select entities first. Base point, then destination.",
            );
            context_entry(panel, "Mode", "Copy selection");
            context_entry(panel, "Multiple copies", "TODO");
            context_buttons(panel, &["Ortho", "Polar", "OSNAP"]);
        }
        Tool::Rotate => {
            compact_note(
                panel,
                "Select entities first. Base point, then angle point.",
            );
            context_entry(panel, "Angle", "By cursor / polar");
            context_buttons(panel, &["Ortho", "Polar", "OSNAP"]);
        }
        Tool::Scale => {
            compact_note(
                panel,
                "Select entities first. Base point, then factor point.",
            );
            context_entry(panel, "Factor", "By cursor distance");
            context_buttons(panel, &["Ortho", "Polar", "OSNAP"]);
        }
        Tool::Mirror => {
            compact_note(
                panel,
                "Select entities first. Two points define mirror axis.",
            );
            context_entry(panel, "Mode", "Transform original");
            context_buttons(panel, &["OSNAP"]);
        }
        Tool::Offset => {
            compact_note(
                panel,
                "Click line/circle/polyline, move to side, click to create offset.",
            );
            let distance_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let distance_entry = gtk::Entry::new();
            distance_entry.set_placeholder_text(Some("Distance"));
            distance_entry.set_width_chars(8);
            distance_entry.set_text(&format!(
                "{:.4}",
                cad.tool_parameters.borrow().offset_distance
            ));
            {
                let cad = cad.clone();
                distance_entry.connect_changed(move |entry| {
                    if let Some(value) = parse_positive_f64(entry.text().as_str()) {
                        cad.tool_parameters.borrow_mut().offset_distance = value;
                    }
                });
            }
            distance_row.append(&field_label("Offset Distance"));
            distance_row.append(&distance_entry);
            panel.append(&distance_row);
            context_entry(panel, "Through point", "TODO");
            context_entry(panel, "Erase source", "TODO");
            context_buttons(panel, &["OSNAP"]);
        }
        Tool::Trim => {
            compact_note(
                panel,
                "Click cutting edge, then click the side of the entity to trim.",
            );
            context_entry(panel, "Boundary", "Explicit edge");
            context_buttons(panel, &["OSNAP"]);
        }
        Tool::Extend => {
            compact_note(
                panel,
                "Click boundary edge, then click the entity segment to extend.",
            );
            context_entry(panel, "Boundary", "Explicit edge");
            context_buttons(panel, &["OSNAP"]);
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
            context_inline(panel, &[("Tolerance", "0.10 mm"), ("Mode", "Sketch")]);
            context_buttons(panel, &["Preview", "Apply"]);
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

fn context_inline(panel: &gtk::Box, fields: &[(&str, &str)]) {
    let row = gtk::FlowBox::new();
    row.set_selection_mode(gtk::SelectionMode::None);
    row.set_column_spacing(6);
    row.set_row_spacing(4);
    for (label, value) in fields {
        let item = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        item.add_css_class("tool-param-chip");
        let label = gtk::Label::new(Some(label));
        label.add_css_class("prop-label");
        let value = gtk::Label::new(Some(value));
        value.add_css_class("prop-value");
        item.append(&label);
        item.append(&value);
        row.insert(&item, -1);
    }
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

fn add_mode_controls(panel: &gtk::Box, tool: Tool, cad: &UiCadContext, canvas: &CadCanvas) {
    match tool {
        Tool::Circle => {
            section_title(panel, "Circle mode");
            let row = gtk::FlowBox::new();
            row.set_selection_mode(gtk::SelectionMode::None);
            row.set_column_spacing(6);
            row.set_row_spacing(6);
            let mode = cad.tool_parameters.borrow().circle_mode;
            row.insert(
                &mode_button(
                    "tool-modes/circle-center-radius",
                    "Center + Radius",
                    "Circle from center and radius",
                    mode == CircleCreationMode::CenterRadius,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().circle_mode =
                                CircleCreationMode::CenterRadius;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Circle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/circle-center-diameter",
                    "Center + Diameter",
                    "Circle from center and diameter",
                    mode == CircleCreationMode::CenterDiameter,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().circle_mode =
                                CircleCreationMode::CenterDiameter;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Circle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/circle-2-point",
                    "2 Points",
                    "Circle using two diameter points",
                    mode == CircleCreationMode::TwoPointDiameter,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().circle_mode =
                                CircleCreationMode::TwoPointDiameter;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Circle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/circle-3-point",
                    "3 Points",
                    "Circle through three points",
                    mode == CircleCreationMode::ThreePoint,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().circle_mode =
                                CircleCreationMode::ThreePoint;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Circle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            panel.append(&row);
        }
        Tool::Rectangle => {
            section_title(panel, "Rectangle mode");
            let row = gtk::FlowBox::new();
            row.set_selection_mode(gtk::SelectionMode::None);
            row.set_column_spacing(6);
            row.set_row_spacing(6);
            let mode = cad.tool_parameters.borrow().rectangle_mode;
            row.insert(
                &mode_button(
                    "tool-modes/rectangle-2-corners",
                    "2 Corners",
                    "Rectangle from opposite corners",
                    mode == RectangleCreationMode::TwoCorners,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().rectangle_mode =
                                RectangleCreationMode::TwoCorners;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Rectangle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/rectangle-size",
                    "Corner + Size",
                    "Coming soon",
                    mode == RectangleCreationMode::CornerDimensions,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().rectangle_mode =
                                RectangleCreationMode::CornerDimensions;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Rectangle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/rectangle-center-size",
                    "Center + Size",
                    "Coming soon",
                    mode == RectangleCreationMode::CenterDimensions,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().rectangle_mode =
                                RectangleCreationMode::CenterDimensions;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Rectangle, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            panel.append(&row);
            let dimensions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let width_entry = gtk::Entry::new();
            let height_entry = gtk::Entry::new();
            width_entry.set_placeholder_text(Some("Width"));
            height_entry.set_placeholder_text(Some("Height"));
            width_entry.set_width_chars(6);
            height_entry.set_width_chars(6);
            width_entry.set_text(&format!(
                "{:.2}",
                cad.tool_parameters.borrow().rectangle_width
            ));
            height_entry.set_text(&format!(
                "{:.2}",
                cad.tool_parameters.borrow().rectangle_height
            ));
            {
                let cad = cad.clone();
                width_entry.connect_changed(move |entry| {
                    if let Some(value) = parse_positive_f64(entry.text().as_str()) {
                        cad.tool_parameters.borrow_mut().rectangle_width = value;
                    }
                });
            }
            {
                let cad = cad.clone();
                height_entry.connect_changed(move |entry| {
                    if let Some(value) = parse_positive_f64(entry.text().as_str()) {
                        cad.tool_parameters.borrow_mut().rectangle_height = value;
                    }
                });
            }
            dimensions.append(&field_label("W"));
            dimensions.append(&width_entry);
            dimensions.append(&field_label("H"));
            dimensions.append(&height_entry);
            panel.append(&dimensions);
        }
        Tool::Line => {
            section_title(panel, "Line mode");
            let row = gtk::FlowBox::new();
            row.set_selection_mode(gtk::SelectionMode::None);
            row.set_column_spacing(6);
            row.set_row_spacing(6);
            let mode = cad.tool_parameters.borrow().line_mode;
            row.insert(
                &mode_button(
                    "tool-modes/line-2-points",
                    "2 Points",
                    "Line from two points",
                    mode == LineCreationMode::TwoPoints,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().line_mode =
                                LineCreationMode::TwoPoints;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Line, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/line-length-angle",
                    "Length + Angle",
                    "Coming soon",
                    mode == LineCreationMode::PointLengthAngle,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().line_mode =
                                LineCreationMode::PointLengthAngle;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Line, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            panel.append(&row);
        }
        Tool::Arc => {
            section_title(panel, "Arc mode");
            let row = gtk::FlowBox::new();
            row.set_selection_mode(gtk::SelectionMode::None);
            row.set_column_spacing(6);
            row.set_row_spacing(6);
            let mode = cad.tool_parameters.borrow().arc_mode;
            row.insert(
                &mode_button(
                    "tool-modes/arc-3-point",
                    "3 Points",
                    "Arc through three points (polyline approximation)",
                    mode == ArcUiMode::ThreePoint,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().arc_mode = ArcUiMode::ThreePoint;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Arc, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            let disabled = mode_button(
                "tool-modes/arc-center-start-end",
                "Center + Start + End",
                "Coming soon",
                mode == ArcUiMode::CenterStartEnd,
                || {},
            );
            disabled.set_sensitive(false);
            row.insert(&disabled, -1);
            panel.append(&row);
        }
        Tool::Dimension => {
            section_title(panel, "Dimension mode");
            let row = gtk::FlowBox::new();
            row.set_selection_mode(gtk::SelectionMode::None);
            row.set_column_spacing(6);
            row.set_row_spacing(6);
            let mode = cad.tool_parameters.borrow().dimension_mode;
            row.insert(
                &mode_button(
                    "tool-modes/dimension-linear",
                    "Linear",
                    "Linear dimension with offset",
                    mode == DimensionCreationMode::Linear,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().dimension_mode =
                                DimensionCreationMode::Linear;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Dimension, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/dimension-aligned",
                    "Aligned",
                    "Aligned dimension parallel to segment",
                    mode == DimensionCreationMode::Aligned,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().dimension_mode =
                                DimensionCreationMode::Aligned;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Dimension, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/dimension-radius",
                    "Radius",
                    "Radius dimension from circle",
                    mode == DimensionCreationMode::Radius,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().dimension_mode =
                                DimensionCreationMode::Radius;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Dimension, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            row.insert(
                &mode_button(
                    "tool-modes/dimension-diameter",
                    "Diameter",
                    "Diameter dimension from circle",
                    mode == DimensionCreationMode::Diameter,
                    {
                        let cad = cad.clone();
                        let panel = panel.clone();
                        let canvas = canvas.clone();
                        move || {
                            cad.tool_parameters.borrow_mut().dimension_mode =
                                DimensionCreationMode::Diameter;
                            canvas.cancel_interaction();
                            refresh_tool_context(&panel, Tool::Dimension, &cad, &canvas);
                        }
                    },
                ),
                -1,
            );
            panel.append(&row);
        }
        _ => {}
    }
}

fn mode_button<F>(
    icon_name: &str,
    label: &str,
    tooltip: &str,
    selected: bool,
    on_click: F,
) -> gtk::Button
where
    F: Fn() + 'static,
{
    let button = gtk::Button::new();
    button.add_css_class("context-chip");
    button.add_css_class("tool-mode-button");
    if selected {
        button.add_css_class("suggested-action");
    }
    button.set_tooltip_text(Some(tooltip));
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    content.append(&app_icon(icon_name, 16));
    let text = gtk::Label::new(Some(label));
    text.add_css_class("prop-value");
    content.append(&text);
    button.set_child(Some(&content));
    button.connect_clicked(move |_| on_click());
    button
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

pub(crate) fn refresh_properties(panel: &gtk::Box, document: &Document) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    section_title(panel, "Properties");
    property(panel, "Document", &document.name);
    property(panel, "Units", document.units.label());
    property(panel, "Active layout", &document.active_layout);
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

    section_title(panel, "Presentaciones");
    for layout in &document.layouts {
        let kind = match layout.kind {
            crate::document::LayoutKind::Model => "modelo",
            crate::document::LayoutKind::Paper => "papel",
        };
        property(panel, &layout.name, kind);
        if layout.kind == crate::document::LayoutKind::Paper {
            property(
                panel,
                "Papel",
                &format!(
                    "{} {}x{} {}",
                    layout.paper.preset, layout.paper.width, layout.paper.height, layout.paper.unit
                ),
            );
            property(
                panel,
                "Márgenes",
                &format!(
                    "{}/{}/{}/{} {}",
                    layout.page_setup.margin_top,
                    layout.page_setup.margin_right,
                    layout.page_setup.margin_bottom,
                    layout.page_setup.margin_left,
                    layout.paper.unit
                ),
            );
        }
    }

    let active_viewports = document
        .layout_viewports
        .iter()
        .filter(|viewport| viewport.layout == document.active_layout)
        .collect::<Vec<_>>();
    if !active_viewports.is_empty() {
        section_title(panel, "Viewports");
        for viewport in active_viewports {
            property(
                panel,
                &format!("Viewport #{}", viewport.id),
                &format!(
                    "{}x{} @ 1:{:.3} {}",
                    viewport.width,
                    viewport.height,
                    viewport.scale_model_units,
                    if viewport.locked {
                        "bloqueado"
                    } else {
                        "editable"
                    }
                ),
            );
            property(
                panel,
                "Centro modelo",
                &format!(
                    "{:.2}, {:.2}",
                    viewport.view_center.x, viewport.view_center.y
                ),
            );
        }
    }

    section_title(panel, "Imported files");
    if document.imported_references.is_empty() {
        property(panel, "References", "none");
    }
    for reference in &document.imported_references {
        property(panel, &reference.format, &reference.summary);
    }

    section_title(panel, "Edición rápida");
    compact_note(
        panel,
        "Usa la sección Propiedades para cambiar capa, tipo de línea y color RGB de la selección. La command bar acepta line, pline, viewport, vpscale, vplock, copy, delete y más.",
    );
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

pub(crate) fn refresh_layer_panel(
    panel: &gtk::Box,
    cad: &UiCadContext,
    canvas: &CadCanvas,
    properties: &gtk::Box,
    selection_label: &gtk::Label,
    modified_label: &gtk::Label,
) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    let create_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let create_entry = gtk::Entry::new();
    create_entry.set_placeholder_text(Some("New layer"));
    let create_btn = layer_icon_button("layer-add", "Create layer", 16);
    {
        let cad = cad.clone();
        let panel = panel.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let selection_label = selection_label.clone();
        let modified_label = modified_label.clone();
        let create_entry_for_click = create_entry.clone();
        create_btn.connect_clicked(move |_| {
            let name = create_entry_for_click.text().to_string();
            if update_layers_with_history(&cad, |doc| doc.create_layer(name.trim())) {
                refresh_layer_panel(
                    &panel,
                    &cad,
                    &canvas,
                    &properties,
                    &selection_label,
                    &modified_label,
                );
                refresh_properties(&properties, &cad.document.borrow());
                modified_label.set_text("Modified");
                canvas.widget().queue_draw();
            }
        });
    }
    create_row.append(&create_entry);
    create_row.append(&create_btn);
    panel.append(&create_row);

    let move_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let move_entry = gtk::Entry::new();
    move_entry.set_placeholder_text(Some("Move selection to layer"));
    let move_btn = gtk::Button::with_label("Move");
    {
        let cad = cad.clone();
        let canvas = canvas.clone();
        let properties = properties.clone();
        let selection_label = selection_label.clone();
        let modified_label = modified_label.clone();
        let move_entry_for_click = move_entry.clone();
        move_btn.connect_clicked(move |_| {
            let target = move_entry_for_click.text().to_string();
            let ids = cad.selected_entity.borrow().clone();
            if ids.is_empty() {
                return;
            }
            let changed = update_entity_properties_with_history(&cad, &ids, |doc, id| {
                doc.set_entity_layer(id, target.trim())
            });
            if changed {
                update_selection_label(&selection_label, &cad.document.borrow(), &ids);
                refresh_properties(&properties, &cad.document.borrow());
                modified_label.set_text("Modified");
                canvas.widget().queue_draw();
            }
        });
    }
    move_row.append(&move_entry);
    move_row.append(&move_btn);
    panel.append(&move_row);

    for layer in cad.document.borrow().layers.clone() {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
        row.add_css_class("layer-row");
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let is_active = cad.document.borrow().active_layer_name == layer.name;
        let active = layer_icon_button(
            if is_active { "layer-active" } else { "layer" },
            if is_active {
                "Active layer"
            } else {
                "Set active layer"
            },
            16,
        );
        let visible = layer_icon_button(
            if layer.visible { "eye" } else { "eye-off" },
            if layer.visible {
                "Hide layer"
            } else {
                "Show layer"
            },
            16,
        );
        let locked = layer_icon_button(
            if layer.locked { "lock" } else { "unlock" },
            if layer.locked {
                "Unlock layer"
            } else {
                "Lock layer"
            },
            16,
        );
        let delete = layer_icon_button("layer-delete", "Delete empty layer", 16);
        delete.set_sensitive(cad.document.borrow().can_delete_layer(&layer.name));
        let count = cad
            .document
            .borrow()
            .entities
            .iter()
            .filter(|entity| entity.layer() == layer.name)
            .count();
        top.append(&active);
        top.append(&visible);
        top.append(&locked);
        top.append(&gtk::Label::new(Some(&format!("{} ({count})", layer.name))));
        top.append(&delete);

        let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let name_entry = gtk::Entry::new();
        name_entry.set_text(&layer.name);
        let color_entry = gtk::Entry::new();
        color_entry.set_text(&layer.color);
        let weight_entry = gtk::Entry::new();
        weight_entry.set_text(&format!("{:.2}", layer.line_weight));
        let type_entry = gtk::Entry::new();
        type_entry.set_text(&layer.line_type);
        bottom.append(&name_entry);
        bottom.append(&color_entry);
        bottom.append(&weight_entry);
        bottom.append(&type_entry);

        {
            let cad = cad.clone();
            let panel = panel.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let selection_label = selection_label.clone();
            let modified_label = modified_label.clone();
            let layer_name = layer.name.clone();
            active.connect_clicked(move |_| {
                if update_layers_with_history(&cad, |doc| doc.set_active_layer_name(&layer_name)) {
                    refresh_layer_panel(
                        &panel,
                        &cad,
                        &canvas,
                        &properties,
                        &selection_label,
                        &modified_label,
                    );
                    refresh_properties(&properties, &cad.document.borrow());
                    canvas.widget().queue_draw();
                }
            });
        }
        {
            let cad = cad.clone();
            let panel = panel.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let selection_label = selection_label.clone();
            let modified_label = modified_label.clone();
            let layer_name = layer.name.clone();
            visible.connect_clicked(move |_| {
                let target = !cad.document.borrow().layer_visible(&layer_name);
                if update_layers_with_history(&cad, |doc| {
                    doc.set_layer_visible(&layer_name, target)
                }) {
                    refresh_layer_panel(
                        &panel,
                        &cad,
                        &canvas,
                        &properties,
                        &selection_label,
                        &modified_label,
                    );
                    refresh_properties(&properties, &cad.document.borrow());
                    canvas.widget().queue_draw();
                }
            });
        }
        {
            let cad = cad.clone();
            let panel = panel.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let selection_label = selection_label.clone();
            let modified_label = modified_label.clone();
            let layer_name = layer.name.clone();
            locked.connect_clicked(move |_| {
                let target = !cad.document.borrow().layer_locked(&layer_name);
                if update_layers_with_history(&cad, |doc| doc.set_layer_locked(&layer_name, target))
                {
                    refresh_layer_panel(
                        &panel,
                        &cad,
                        &canvas,
                        &properties,
                        &selection_label,
                        &modified_label,
                    );
                    refresh_properties(&properties, &cad.document.borrow());
                    canvas.widget().queue_draw();
                }
            });
        }
        {
            let cad = cad.clone();
            let panel = panel.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let selection_label = selection_label.clone();
            let modified_label = modified_label.clone();
            let layer_name = layer.name.clone();
            delete.connect_clicked(move |_| {
                if update_layers_with_history(&cad, |doc| doc.delete_layer_if_empty(&layer_name)) {
                    refresh_layer_panel(
                        &panel,
                        &cad,
                        &canvas,
                        &properties,
                        &selection_label,
                        &modified_label,
                    );
                    refresh_properties(&properties, &cad.document.borrow());
                    canvas.widget().queue_draw();
                }
            });
        }
        {
            let cad = cad.clone();
            let panel = panel.clone();
            let canvas = canvas.clone();
            let properties = properties.clone();
            let selection_label = selection_label.clone();
            let modified_label = modified_label.clone();
            let old_name = layer.name.clone();
            let color_entry = color_entry.clone();
            let weight_entry = weight_entry.clone();
            let type_entry = type_entry.clone();
            name_entry.connect_activate(move |entry| {
                let new_name = entry.text().to_string();
                let changed = update_layers_with_history(&cad, |doc| {
                    let mut any = false;
                    any |= doc.rename_layer(&old_name, new_name.trim());
                    any |= doc.set_layer_color(new_name.trim(), color_entry.text().as_str());
                    any |= doc.set_layer_line_weight(
                        new_name.trim(),
                        weight_entry.text().as_str().parse::<f64>().unwrap_or(0.25),
                    );
                    any |= doc.set_layer_line_type(new_name.trim(), type_entry.text().as_str());
                    any
                });
                if changed {
                    refresh_layer_panel(
                        &panel,
                        &cad,
                        &canvas,
                        &properties,
                        &selection_label,
                        &modified_label,
                    );
                    refresh_properties(&properties, &cad.document.borrow());
                    canvas.widget().queue_draw();
                }
            });
        }

        row.append(&top);
        row.append(&bottom);
        panel.append(&row);
    }
}

fn status_bar(
    cursor_label: gtk::Label,
    tool_label: gtk::Label,
    modified_label: gtk::Label,
    document: Rc<RefCell<Document>>,
    osnap_status: gtk::Label,
    precision_status: gtk::Label,
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
    status.append(&osnap_status);
    status.append(&precision_status);
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

fn attach_canvas_context_menu(
    window: adw::ApplicationWindow,
    canvas: CadCanvas,
    cad: UiCadContext,
    view: Rc<UiViewContext>,
    selection_label: gtk::Label,
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
) {
    let document = cad.document.clone();
    let selected_entity = cad.selected_entity.clone();
    let properties = view.properties.clone();
    let modified_label = view.modified_label.clone();
    let click = gtk::GestureClick::new();
    click.set_button(3);
    let controller_area = canvas.widget().clone();
    let area = controller_area.clone();
    let layer_entry = view.attribute_layer_entry.clone();
    click.connect_pressed(move |_, _, x, y| {
        let hit = canvas.entity_at_screen(&document.borrow(), x, y);
        if let Some(id) = hit {
            *selected_entity.borrow_mut() = vec![id];
            crate::canvas::update_selection_ui(
                &selection_label,
                &layer_entry,
                &document.borrow(),
                &selected_entity.borrow(),
            );
            area.queue_draw();
            popup_menu(
                &area,
                x,
                y,
                vec![
                    menu_item("Duplicate", {
                        let cad = cad.clone();
                        let document = document.clone();
                        let selected_entity = selected_entity.clone();
                        let selection_label = selection_label.clone();
                        let properties = properties.clone();
                        let modified_label = modified_label.clone();
                        let area = area.clone();
                        move || {
                            if let Some(copy_id) = duplicate_entity_with_history(&cad, id) {
                                *selected_entity.borrow_mut() = vec![copy_id];
                                update_selection_label(
                                    &selection_label,
                                    &document.borrow(),
                                    &selected_entity.borrow(),
                                );
                                refresh_properties(&properties, &document.borrow());
                                modified_label.set_text("Modified");
                                area.queue_draw();
                            }
                        }
                    }),
                    menu_item("Delete", {
                        let cad = cad.clone();
                        let view = view.clone();
                        let selected_entity = selected_entity.clone();
                        let selection_label = selection_label.clone();
                        move || {
                            if delete_selected_entities(&cad, &[id]) {
                                selected_entity.borrow_mut().clear();
                                refresh_after_history_change(
                                    &cad,
                                    &view,
                                    &selection_label,
                                    &selected_entity,
                                );
                            }
                        }
                    }),
                    menu_item("Move to Default layer", {
                        let cad = cad.clone();
                        let selected_entity = selected_entity.clone();
                        let selection_label = selection_label.clone();
                        let properties = properties.clone();
                        let modified_label = modified_label.clone();
                        let area = area.clone();
                        move || {
                            if update_entity_properties_with_history(
                                &cad,
                                &[id],
                                |doc, entity_id| doc.set_entity_layer(entity_id, "Default"),
                            ) {
                                update_selection_label(
                                    &selection_label,
                                    &cad.document.borrow(),
                                    &selected_entity.borrow(),
                                );
                                refresh_properties(&properties, &cad.document.borrow());
                                modified_label.set_text("Modified");
                                area.queue_draw();
                            }
                        }
                    }),
                    menu_item("Fit drawing", {
                        let canvas = canvas.clone();
                        let document = document.clone();
                        move || canvas.fit_document(&document.borrow())
                    }),
                    menu_item("Edit Text", {
                        let window = window.clone();
                        let cad = cad.clone();
                        let selected_entity = selected_entity.clone();
                        let selection_label = selection_label.clone();
                        let properties = properties.clone();
                        let modified_label = modified_label.clone();
                        let area = area.clone();
                        move || {
                            open_text_edit_dialog(
                                &window,
                                &cad,
                                id,
                                &selected_entity,
                                &selection_label,
                                &properties,
                                &modified_label,
                                &area,
                            );
                        }
                    }),
                ],
            );
        } else {
            selected_entity.borrow_mut().clear();
            update_selection_label(
                &selection_label,
                &document.borrow(),
                &selected_entity.borrow(),
            );
            area.queue_draw();
            popup_menu(
                &area,
                x,
                y,
                vec![
                    menu_item("Fit drawing", {
                        let canvas = canvas.clone();
                        let document = document.clone();
                        move || canvas.fit_document(&document.borrow())
                    }),
                    menu_item("Reset view", {
                        let canvas = canvas.clone();
                        move || canvas.reset_view()
                    }),
                    menu_item("Cancel current command", {
                        let canvas = canvas.clone();
                        move || canvas.cancel_interaction()
                    }),
                    menu_item("Select tool", {
                        let active_tool = active_tool.clone();
                        let tool_label = tool_label.clone();
                        let tool_context = tool_context.clone();
                        let cad = cad.clone();
                        let canvas = canvas.clone();
                        move || {
                            set_active_tool(
                                &active_tool,
                                &tool_label,
                                &tool_context,
                                &cad,
                                &canvas,
                                Tool::Select,
                            )
                        }
                    }),
                    menu_item("Line tool", {
                        let active_tool = active_tool.clone();
                        let tool_label = tool_label.clone();
                        let tool_context = tool_context.clone();
                        let cad = cad.clone();
                        let canvas = canvas.clone();
                        move || {
                            set_active_tool(
                                &active_tool,
                                &tool_label,
                                &tool_context,
                                &cad,
                                &canvas,
                                Tool::Line,
                            )
                        }
                    }),
                    menu_item("Polyline tool", {
                        let active_tool = active_tool.clone();
                        let tool_label = tool_label.clone();
                        let tool_context = tool_context.clone();
                        let cad = cad.clone();
                        let canvas = canvas.clone();
                        move || {
                            set_active_tool(
                                &active_tool,
                                &tool_label,
                                &tool_context,
                                &cad,
                                &canvas,
                                Tool::Polyline,
                            )
                        }
                    }),
                    menu_item("Circle tool", {
                        let active_tool = active_tool.clone();
                        let tool_label = tool_label.clone();
                        let tool_context = tool_context.clone();
                        let cad = cad.clone();
                        let canvas = canvas.clone();
                        move || {
                            set_active_tool(
                                &active_tool,
                                &tool_label,
                                &tool_context,
                                &cad,
                                &canvas,
                                Tool::Circle,
                            )
                        }
                    }),
                ],
            );
        }
    });
    controller_area.add_controller(click);
}

fn open_text_edit_dialog(
    window: &adw::ApplicationWindow,
    cad: &UiCadContext,
    id: u64,
    selected_entity: &Rc<RefCell<Vec<u64>>>,
    selection_label: &gtk::Label,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
    area: &gtk::DrawingArea,
) {
    let current_text = {
        let document = cad.document.borrow();
        document.entities.iter().find_map(|entity| match entity {
            Entity::Text {
                id: entity_id,
                text,
                ..
            } if *entity_id == id => Some(text.clone()),
            _ => None,
        })
    };
    let Some(current_text) = current_text else {
        return;
    };

    let dialog = gtk::Dialog::builder()
        .title("Edit text")
        .transient_for(window)
        .modal(true)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Apply", gtk::ResponseType::Accept);
    let entry = gtk::Entry::new();
    entry.set_text(&current_text);
    entry.set_activates_default(true);
    dialog.set_default_response(gtk::ResponseType::Accept);
    dialog.content_area().append(&entry);

    let cad = cad.clone();
    let selected_entity = selected_entity.clone();
    let selection_label = selection_label.clone();
    let properties = properties.clone();
    let modified_label = modified_label.clone();
    let area = area.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let new_text = entry.text().to_string();
            if !new_text.trim().is_empty()
                && update_entity_properties_with_history(&cad, &[id], |doc, entity_id| {
                    doc.set_text_entity_text(entity_id, new_text.trim())
                })
            {
                if let Ok(document) = cad.document.try_borrow() {
                    update_selection_label(&selection_label, &document, &selected_entity.borrow());
                    refresh_properties(&properties, &document);
                }
                modified_label.set_text("Modified");
                area.queue_draw();
            }
        }
        dialog.close();
    });
    dialog.present();
}

fn floating_command_bar(
    cad: UiCadContext,
    view: Rc<UiViewContext>,
    selection_label: gtk::Label,
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 6);
    bar.add_css_class("floating-command-bar");
    bar.set_halign(gtk::Align::Center);
    bar.set_valign(gtk::Align::End);
    bar.set_margin_bottom(18);
    bar.set_margin_start(24);
    bar.set_margin_end(24);

    let history = gtk::Label::new(Some("Command: ready"));
    history.add_css_class("command-history");
    history.set_xalign(0.0);
    history.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let prompt = gtk::Label::new(Some(">"));
    prompt.add_css_class("command-prompt");
    let entry = gtk::Entry::new();
    entry.add_css_class("command-entry");
    entry.set_placeholder_text(Some(
        "LINE, PLINE, CIRCLE, SELECT, FIT, ZOOMIN, ZOOMOUT, RESET, DELETE",
    ));
    entry.set_hexpand(true);
    let suggestions = gtk::Label::new(Some(
        "Sugerencias: line, pline, circle, viewport, vpscale 100",
    ));
    suggestions.add_css_class("command-suggestions");
    suggestions.set_xalign(0.0);
    suggestions.set_ellipsize(gtk::pango::EllipsizeMode::End);

    {
        let suggestions = suggestions.clone();
        entry.connect_changed(move |entry| {
            suggestions.set_text(&command_suggestions(&entry.text()));
        });
    }

    {
        let cad = cad.clone();
        let view = view.clone();
        let selection_label = selection_label.clone();
        let active_tool = active_tool.clone();
        let tool_label = tool_label.clone();
        let tool_context = tool_context.clone();
        let command_status = history.clone();
        entry.connect_activate(move |entry| {
            let raw_command = entry.text().to_string();
            if raw_command.trim().is_empty() {
                return;
            }
            entry.set_text("");
            schedule_command_execution(
                raw_command,
                CommandExecutionContext {
                    cad: cad.clone(),
                    view: view.clone(),
                    selection_label: selection_label.clone(),
                    active_tool: active_tool.clone(),
                    tool_label: tool_label.clone(),
                    tool_context: tool_context.clone(),
                    command_status: command_status.clone(),
                },
            );
        });
    }

    row.append(&prompt);
    row.append(&entry);
    bar.append(&history);
    bar.append(&row);
    bar.append(&suggestions);
    bar
}

fn command_suggestions(input: &str) -> String {
    let query = input.trim().to_ascii_lowercase();
    if query.is_empty() {
        return "Sugerencias: line, pline, circle, rectangle, select, viewport, vpscale 100"
            .to_string();
    }
    let matches = COMMANDS
        .iter()
        .filter(|(command, description)| {
            command.starts_with(&query) || description.contains(&query)
        })
        .take(6)
        .map(|(command, description)| format!("{command} · {description}"))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        "Sin coincidencias. Enter ejecuta el comando escrito.".to_string()
    } else {
        format!("Sugerencias: {}", matches.join("   "))
    }
}

fn view_compass(canvas: CadCanvas, document: Rc<RefCell<Document>>) -> gtk::Box {
    let compass = gtk::Box::new(gtk::Orientation::Vertical, 6);
    compass.add_css_class("view-compass");
    compass.set_halign(gtk::Align::End);
    compass.set_valign(gtk::Align::Start);
    compass.set_margin_top(18);
    compass.set_margin_end(18);

    let title = gtk::Label::new(Some("VIEW"));
    title.add_css_class("view-compass-title");
    compass.append(&title);

    let rose = gtk::Grid::new();
    rose.add_css_class("view-compass-rose");
    rose.set_row_spacing(4);
    rose.set_column_spacing(4);

    rose.attach(
        &compass_button("N", {
            let canvas = canvas.clone();
            move || canvas.set_view_rotation(0.0)
        }),
        1,
        0,
        1,
        1,
    );
    rose.attach(
        &compass_button("W", {
            let canvas = canvas.clone();
            move || canvas.set_view_rotation(std::f64::consts::FRAC_PI_2)
        }),
        0,
        1,
        1,
        1,
    );
    rose.attach(
        &compass_button("FIT", {
            let canvas = canvas.clone();
            let document = document.clone();
            move || canvas.fit_document(&document.borrow())
        }),
        1,
        1,
        1,
        1,
    );
    rose.attach(
        &compass_button("E", {
            let canvas = canvas.clone();
            move || canvas.set_view_rotation(-std::f64::consts::FRAC_PI_2)
        }),
        2,
        1,
        1,
        1,
    );
    rose.attach(
        &compass_button("S", {
            let canvas = canvas.clone();
            move || canvas.set_view_rotation(std::f64::consts::PI)
        }),
        1,
        2,
        1,
        1,
    );
    compass.append(&rose);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    row.append(&compass_button("↺", {
        let canvas = canvas.clone();
        move || canvas.rotate_view_by(std::f64::consts::FRAC_PI_4)
    }));
    row.append(&compass_button("0°", {
        let canvas = canvas.clone();
        move || canvas.set_view_rotation(0.0)
    }));
    row.append(&compass_button("ISO", {
        let canvas = canvas.clone();
        move || canvas.set_view_rotation(-std::f64::consts::FRAC_PI_4)
    }));
    row.append(&compass_button("↻", {
        let canvas = canvas.clone();
        move || canvas.rotate_view_by(-std::f64::consts::FRAC_PI_4)
    }));
    compass.append(&row);
    compass
}

fn compass_button<F>(label: &str, action: F) -> gtk::Button
where
    F: Fn() + 'static,
{
    let button = gtk::Button::with_label(label);
    button.add_css_class("view-compass-button");
    button.connect_clicked(move |_| action());
    button
}

fn next_command_entity_id(cad: &UiCadContext) -> u64 {
    cad.document
        .try_borrow()
        .map(|document| document.next_id())
        .unwrap_or(1)
}

fn execute_geometry_command(
    command: &str,
    cad: &UiCadContext,
    selection_label: &gtk::Label,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
    canvas: &gtk::DrawingArea,
    command_status: &gtk::Label,
) -> bool {
    let active_layout = document_active_layout(cad);
    let parts = command
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let part_refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    let next_id = next_command_entity_id(cad);

    match build_geometry_from_command_parts(&part_refs, next_id) {
        GeometryBuildResult::Success { entity, message } => {
            add_command_entity(
                cad,
                entity,
                active_layout,
                selection_label,
                properties,
                modified_label,
                canvas,
                message,
                command_status,
            );
            true
        }
        GeometryBuildResult::Error { message } => {
            command_status.set_text(message);
            true
        }
        GeometryBuildResult::NotHandled => false,
    }
}

fn add_command_entity(
    cad: &UiCadContext,
    mut entity: Entity,
    active_layout: String,
    selection_label: &gtk::Label,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
    canvas: &gtk::DrawingArea,
    message: &'static str,
    command_status: &gtk::Label,
) {
    let active_layer = cad.document.borrow().active_layer_name.clone();
    entity.set_layer(&active_layer);
    match try_add_entity_with_history(cad, entity.clone(), &active_layout) {
        Ok(id) => {
            *cad.selected_entity.borrow_mut() = vec![id];
            if let Ok(document) = cad.document.try_borrow() {
                let selected = cad.selected_entity.borrow();
                update_selection_label(selection_label, &document, &selected);
                refresh_properties(properties, &document);
            }
            modified_label.set_text("Modified");
            canvas.queue_draw();
            command_status.set_text(message);
        }
        Err(DocumentBusy) => {
            let cad = cad.clone();
            let selection_label = selection_label.clone();
            let properties = properties.clone();
            let modified_label = modified_label.clone();
            let canvas = canvas.clone();
            let command_status = command_status.clone();
            gtk::glib::idle_add_local_once(move || {
                add_command_entity(
                    &cad,
                    entity,
                    active_layout,
                    &selection_label,
                    &properties,
                    &modified_label,
                    &canvas,
                    message,
                    &command_status,
                );
            });
        }
    }
}

#[derive(Clone)]
struct CommandExecutionContext {
    cad: UiCadContext,
    view: Rc<UiViewContext>,
    selection_label: gtk::Label,
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
    command_status: gtk::Label,
}

const COMMAND_BUSY_MAX_ATTEMPTS: u32 = 64;

fn schedule_command_execution(raw_command: String, ctx: CommandExecutionContext) {
    gtk::glib::idle_add_local_once(move || run_scheduled_command(raw_command, ctx, 0));
}

fn run_scheduled_command(raw_command: String, ctx: CommandExecutionContext, attempt: u32) {
    if ctx.cad.document.try_borrow_mut().is_err() {
        if attempt < COMMAND_BUSY_MAX_ATTEMPTS {
            gtk::glib::idle_add_local_once(move || {
                run_scheduled_command(raw_command, ctx, attempt + 1);
            });
        } else {
            ctx.command_status
                .set_text("Command: document busy, try again");
        }
        return;
    }
    execute_command(
        &raw_command,
        &ctx.cad,
        &ctx.view,
        &ctx.selection_label,
        &ctx.active_tool,
        &ctx.tool_label,
        &ctx.tool_context,
        &ctx.command_status,
    );
}

fn tool_from_registry_id(tool_id: &str) -> Tool {
    match tool_id {
        "line" => Tool::Line,
        "polyline" => Tool::Polyline,
        "rectangle" => Tool::Rectangle,
        "circle" => Tool::Circle,
        "move" => Tool::Move,
        "select" => Tool::Select,
        "pan" => Tool::Pan,
        _ => Tool::Select,
    }
}

fn registry_tool_message(tool_id: &str) -> &'static str {
    match tool_id {
        "line" => "LINE: specify first point",
        "polyline" => "PLINE: specify next point, Enter finishes",
        "rectangle" => "RECTANGLE: specify first corner",
        "circle" => "CIRCLE: specify center point",
        "move" => "MOVE: specify base point",
        _ => "Tool activated",
    }
}

/// Delegates bare commands to `CommandRegistry` parsing; legacy execution remains for
/// parameterized geometry commands and layout/view aliases.
fn try_execute_registry_command(
    raw_command: &str,
    cad: &UiCadContext,
    view: &UiViewContext,
    selection_label: &gtk::Label,
    active_tool: &Rc<RefCell<Tool>>,
    tool_label: &gtk::Label,
    tool_context: &gtk::Box,
    history: &gtk::Label,
) -> bool {
    let canvas = &view.canvas;
    let selected_entity = &cad.selected_entity;
    if !CommandRegistry::is_registry_delegated(raw_command) {
        return false;
    }
    match CommandRegistry::parse(raw_command) {
        ParsedCommand::ActivateTool(tool_id) => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                tool_from_registry_id(tool_id),
            );
            history.set_text(registry_tool_message(tool_id));
            true
        }
        ParsedCommand::DeleteSelection => {
            let selected_ids = selected_entity.borrow().clone();
            if selected_ids.is_empty() {
                history.set_text("DELETE: no entity selected");
                return true;
            }
            if delete_selected_entities(cad, &selected_ids) {
                selected_entity.borrow_mut().clear();
                refresh_after_history_change(cad, view, selection_label, selected_entity);
                history.set_text("DELETE: entity erased");
            }
            true
        }
        ParsedCommand::Undo => {
            if perform_undo(cad) {
                refresh_after_history_change(cad, view, selection_label, selected_entity);
                history.set_text("UNDO");
            } else {
                history.set_text("UNDO: nothing to undo");
            }
            true
        }
        ParsedCommand::Redo => {
            if perform_redo(cad) {
                refresh_after_history_change(cad, view, selection_label, selected_entity);
                history.set_text("REDO");
            } else {
                history.set_text("REDO: nothing to redo");
            }
            true
        }
        _ => false,
    }
}

fn execute_command(
    raw_command: &str,
    cad: &UiCadContext,
    view: &UiViewContext,
    selection_label: &gtk::Label,
    active_tool: &Rc<RefCell<Tool>>,
    tool_label: &gtk::Label,
    tool_context: &gtk::Box,
    history: &gtk::Label,
) {
    let canvas = &view.canvas;
    let properties = &view.properties;
    let modified_label = &view.modified_label;
    let document = &cad.document;
    let selected_entity = &cad.selected_entity;
    let command = raw_command.trim().to_ascii_lowercase();
    if command.is_empty() {
        history.set_text("Command: ready");
        return;
    }

    if try_execute_osnap_command(&command, &cad.osnap, history) {
        canvas.widget().queue_draw();
        return;
    }

    {
        let selected = cad.selected_entity.borrow().clone();
        if crate::canvas_modify::try_execute_modify_command(
            &command,
            &mut cad.document.borrow_mut(),
            &cad.history,
            &selected,
            Some(history),
        ) {
            refresh_after_history_change(cad, view, selection_label, &cad.selected_entity);
            canvas.widget().queue_draw();
            return;
        }
    }

    {
        let selected = cad.selected_entity.borrow().clone();
        match crate::canvas_offset::try_execute_offset_command(
            &command,
            &mut cad.document.borrow_mut(),
            &cad.history,
            &selected,
            &mut cad.tool_parameters.borrow_mut(),
            Some(history),
        ) {
            crate::canvas_offset::OffsetCommandResult::NotHandled => {}
            crate::canvas_offset::OffsetCommandResult::Applied => {
                refresh_after_history_change(cad, view, selection_label, &cad.selected_entity);
                canvas.widget().queue_draw();
                return;
            }
            crate::canvas_offset::OffsetCommandResult::SetDistance => {
                set_active_tool(
                    active_tool,
                    tool_label,
                    tool_context,
                    cad,
                    canvas,
                    Tool::Offset,
                );
                canvas.widget().queue_draw();
                return;
            }
            crate::canvas_offset::OffsetCommandResult::Handled => {
                canvas.widget().queue_draw();
                return;
            }
        }
    }

    if let Some(activation) = crate::canvas_trim_extend::try_execute_trim_extend_command(&command) {
        let tool = match activation {
            crate::canvas_trim_extend::ToolActivation::Trim => Tool::Trim,
            crate::canvas_trim_extend::ToolActivation::Extend => Tool::Extend,
        };
        set_active_tool(active_tool, tool_label, tool_context, cad, canvas, tool);
        let msg = match tool {
            Tool::Trim => "TRIM: pick cutting edge, then entity side",
            Tool::Extend => "EXTEND: pick boundary, then entity to extend",
            _ => "Tool activated",
        };
        history.set_text(msg);
        canvas.widget().queue_draw();
        return;
    }

    if try_execute_registry_command(
        raw_command,
        cad,
        view,
        selection_label,
        active_tool,
        tool_label,
        tool_context,
        history,
    ) {
        return;
    }

    if let Some(layout_name) = command.strip_prefix("layout ") {
        let layout_name = layout_name.trim();
        if layout_name.is_empty() {
            history.set_text("LAYOUT: missing layout name");
            return;
        }
        document.borrow_mut().set_active_layout(layout_name);
        refresh_properties(properties, &document.borrow());
        canvas.widget().queue_draw();
        history.set_text(&format!(
            "LAYOUT: active {}",
            document.borrow().active_layout
        ));
        return;
    }

    if let Some(scale) = command
        .strip_prefix("vpscale ")
        .or_else(|| command.strip_prefix("viewportscale "))
    {
        let scale = scale.trim().trim_start_matches("1:");
        match scale.parse::<f64>() {
            Ok(model_units) => {
                let applied = {
                    let mut doc = document.borrow_mut();
                    doc.set_active_layout_viewport_scale(model_units)
                };
                if applied {
                    refresh_properties(properties, &document.borrow());
                    canvas.widget().queue_draw();
                    history.set_text(&format!("VPSCALE: 1:{model_units:.3}"));
                } else {
                    history.set_text("VPSCALE: use in a presentation, e.g. vpscale 100");
                }
            }
            _ => history.set_text("VPSCALE: use in a presentation, e.g. vpscale 100"),
        }
        return;
    }

    if execute_geometry_command(
        &command,
        cad,
        selection_label,
        properties,
        modified_label,
        canvas.widget(),
        history,
    ) {
        return;
    }

    match command.as_str() {
        "l" | "line" | "li" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Line,
            );
            history.set_text("LINE: specify first point");
        }
        "pl" | "pline" | "polyline" | "pol" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Polyline,
            );
            history.set_text("PLINE: specify next point, Enter finishes");
        }
        "c" | "circle" | "ci" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Circle,
            );
            history.set_text("CIRCLE: specify center point");
        }
        "a" | "arc" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Arc,
            );
            history.set_text("ARC: specify arc geometry");
        }
        "rec" | "rect" | "rectangle" | "r" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Rectangle,
            );
            history.set_text("RECTANGLE: specify first corner");
        }
        "s" | "select" | "sel" | "pick" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Select,
            );
            history.set_text("SELECT: pick an entity");
        }
        "modify" | "mo" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Modify,
            );
            history.set_text("MODIFY: drag selected geometry");
        }
        "move" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Move,
            );
            history.set_text("MOVE: pick base point, then destination");
        }
        "m" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Modify,
            );
            history.set_text("MODIFY: drag selected geometry");
        }
        "copy" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Copy,
            );
            history.set_text("COPY: pick base point, then destination");
        }
        "rotate" | "ro" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Rotate,
            );
            history.set_text("ROTATE: pick base point, then angle");
        }
        "scale" | "sc" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Scale,
            );
            history.set_text("SCALE: pick base point, then scale factor");
        }
        "mirror" | "mi" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Mirror,
            );
            history.set_text("MIRROR: pick axis start, then axis end");
        }
        "offset" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Offset,
            );
            history.set_text("OFFSET: pick entity, then side point");
        }
        "trim" | "tr" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Trim,
            );
            history.set_text("TRIM: pick cutting edge, then entity side");
        }
        "extend" | "ex" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Extend,
            );
            history.set_text("EXTEND: pick boundary, then entity to extend");
        }
        "d" | "dim" | "dimension" | "linear" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Dimension,
            );
            history.set_text("DIMENSION: specify measured points");
        }
        "t" | "text" | "mtext" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Text,
            );
            history.set_text("TEXT: place text");
        }
        "h" | "hatch" | "bhatch" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Hatch,
            );
            history.set_text("HATCH: create hatch boundary");
        }
        "tbl" | "table" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Table,
            );
            history.set_text("TABLE: insert table");
        }
        "b" | "block" | "insert" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Block,
            );
            history.set_text("BLOCK: insert or create block");
        }
        "xline" | "ray" | "guide" | "guideline" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Guideline,
            );
            history.set_text("GUIDELINE: construction geometry");
        }
        "me" | "measure" | "dist" | "distance" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Measure,
            );
            history.set_text("MEASURE: pick two points");
        }
        "pan" | "p" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Pan,
            );
            history.set_text("PAN: drag the view");
        }
        "orbit" | "3dorbit" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Orbit,
            );
            history.set_text("ORBIT: 3D view placeholder");
        }
        "extrude" | "ext" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Extrude,
            );
            history.set_text("EXTRUDE: 3D extrusion placeholder");
        }
        "fitcurve" | "splinefit" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::FitCurve,
            );
            history.set_text("FITCURVE: fit curve placeholder");
        }
        "section" | "sectionmesh" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::SectionFromMesh,
            );
            history.set_text("SECTION: section from mesh placeholder");
        }
        "param" | "parametric" | "constraint" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Parametric,
            );
            history.set_text("PARAMETRIC: constraints placeholder");
        }
        "fit" | "zoomextents" | "ze" | "extents" => {
            canvas.fit_document(&document.borrow());
            history.set_text("FIT: drawing extents");
        }
        "model" | "ms" | "modelspace" => {
            document.borrow_mut().set_active_layout("Model");
            refresh_properties(properties, &document.borrow());
            canvas.widget().queue_draw();
            history.set_text("MODEL: active");
        }
        "newlayout" | "layoutnew" | "papernew" => {
            let name = document.borrow_mut().create_paper_layout();
            refresh_properties(properties, &document.borrow());
            canvas.widget().queue_draw();
            history.set_text(&format!("LAYOUT: created {name}"));
        }
        "deletelayout" | "layoutdelete" => {
            let name = document.borrow().active_layout.clone();
            if document.borrow_mut().delete_layout(&name) {
                refresh_properties(properties, &document.borrow());
                canvas.widget().queue_draw();
                history.set_text("LAYOUT: deleted");
            } else {
                history.set_text("LAYOUT: cannot delete Model");
            }
        }
        "duplayout" | "layoutcopy" => {
            let name = document.borrow().active_layout.clone();
            if let Some(new_name) = document.borrow_mut().duplicate_layout(&name) {
                refresh_properties(properties, &document.borrow());
                canvas.widget().queue_draw();
                history.set_text(&format!("LAYOUT: duplicated {new_name}"));
            } else {
                history.set_text("LAYOUT: cannot duplicate Model");
            }
        }
        "vp" | "viewport" | "mview" | "newviewport" => {
            if let Some(id) = document.borrow_mut().create_viewport_for_active_layout() {
                refresh_properties(properties, &document.borrow());
                canvas.fit_document(&document.borrow());
                history.set_text(&format!("VIEWPORT: created #{id}"));
            } else {
                history.set_text("VIEWPORT: switch to a presentation first");
            }
        }
        "vplock" | "viewportlock" => {
            match document.borrow_mut().toggle_active_layout_viewport_lock() {
                Some(true) => {
                    refresh_properties(properties, &document.borrow());
                    canvas.widget().queue_draw();
                    history.set_text("VIEWPORT: locked");
                }
                Some(false) => {
                    refresh_properties(properties, &document.borrow());
                    canvas.widget().queue_draw();
                    history.set_text("VIEWPORT: unlocked");
                }
                None => history.set_text("VIEWPORT: no viewport in current presentation"),
            }
        }
        "north" | "top" | "plan" | "wcs" => {
            canvas.set_view_rotation(0.0);
            history.set_text("VIEW: north / plan");
        }
        "east" | "right" => {
            canvas.set_view_rotation(-std::f64::consts::FRAC_PI_2);
            history.set_text("VIEW: east");
        }
        "west" | "left" => {
            canvas.set_view_rotation(std::f64::consts::FRAC_PI_2);
            history.set_text("VIEW: west");
        }
        "south" | "bottom" => {
            canvas.set_view_rotation(std::f64::consts::PI);
            history.set_text("VIEW: south");
        }
        "rotl" | "rotateleft" => {
            canvas.rotate_view_by(std::f64::consts::FRAC_PI_4);
            history.set_text("VIEW: rotated left");
        }
        "rotr" | "rotateright" => {
            canvas.rotate_view_by(-std::f64::consts::FRAC_PI_4);
            history.set_text("VIEW: rotated right");
        }
        "z" | "zoomin" | "zi" => {
            canvas.zoom_in();
            history.set_text("ZOOMIN");
        }
        "zoomout" | "zo" => {
            canvas.zoom_out();
            history.set_text("ZOOMOUT");
        }
        "reset" | "100" | "home" => {
            canvas.reset_view();
            history.set_text("RESET VIEW");
        }
        "esc" | "cancel" => {
            canvas.cancel_interaction();
            history.set_text("Command canceled");
        }
        "enter" | "finish" => {
            if *active_tool.borrow() == Tool::Polyline {
                canvas.finish_polyline(document, &cad.history);
                refresh_properties(properties, &document.borrow());
                modified_label.set_text("Modified");
                history.set_text("PLINE finished");
            } else {
                history.set_text("Nothing to finish");
            }
        }
        "del" | "delete" | "erase" | "e" => {
            let selected_ids = selected_entity.borrow().clone();
            if selected_ids.is_empty() {
                history.set_text("DELETE: no entity selected");
                return;
            }
            if delete_selected_entities(cad, &selected_ids) {
                selected_entity.borrow_mut().clear();
                refresh_after_history_change(cad, view, selection_label, selected_entity);
                history.set_text("DELETE: entity erased");
            }
        }
        "duplicate" | "dup" => {
            let selected_ids = selected_entity.borrow().clone();
            if selected_ids.is_empty() {
                history.set_text("DUPLICATE: no entity selected");
                return;
            }
            let mut copies = Vec::new();
            for id in selected_ids {
                if let Some(copy_id) = duplicate_entity_with_history(cad, id) {
                    copies.push(copy_id);
                }
            }
            if !copies.is_empty() {
                *selected_entity.borrow_mut() = copies;
                update_selection_label(
                    selection_label,
                    &document.borrow(),
                    &selected_entity.borrow(),
                );
                refresh_properties(properties, &document.borrow());
                modified_label.set_text("Modified");
                canvas.widget().queue_draw();
                history.set_text("DUPLICATE: offset copy of selection");
            }
        }
        "co" | "cp" => {
            set_active_tool(
                active_tool,
                tool_label,
                tool_context,
                cad,
                canvas,
                Tool::Copy,
            );
            history.set_text("COPY: pick base point, then destination");
        }
        _ => {
            history.set_text(&format!("Unknown command: {}", raw_command.trim()));
        }
    }
}

fn attach_toolbar_context_menu(
    toolbar: &gtk::Box,
    window: &adw::ApplicationWindow,
    cad: UiCadContext,
    canvas: CadCanvas,
    active_tool: Rc<RefCell<Tool>>,
    tool_label: gtk::Label,
    tool_context: gtk::Box,
) {
    let click = gtk::GestureClick::new();
    click.set_button(3);
    let toolbar_widget = toolbar.clone();
    let window = window.clone();
    click.connect_pressed(move |_, _, x, y| {
        popup_menu(
            &toolbar_widget,
            x,
            y,
            vec![
                menu_item("Zoom in", {
                    let canvas = canvas.clone();
                    move || canvas.zoom_in()
                }),
                menu_item("Zoom out", {
                    let canvas = canvas.clone();
                    move || canvas.zoom_out()
                }),
                menu_item("Reset view", {
                    let canvas = canvas.clone();
                    move || canvas.reset_view()
                }),
                menu_item("Select tool", {
                    let active_tool = active_tool.clone();
                    let tool_label = tool_label.clone();
                    let tool_context = tool_context.clone();
                    let cad = cad.clone();
                    let canvas = canvas.clone();
                    move || {
                        set_active_tool(
                            &active_tool,
                            &tool_label,
                            &tool_context,
                            &cad,
                            &canvas,
                            Tool::Select,
                        )
                    }
                }),
                menu_item("Measure tool", {
                    let active_tool = active_tool.clone();
                    let tool_label = tool_label.clone();
                    let tool_context = tool_context.clone();
                    let cad = cad.clone();
                    let canvas = canvas.clone();
                    move || {
                        set_active_tool(
                            &active_tool,
                            &tool_label,
                            &tool_context,
                            &cad,
                            &canvas,
                            Tool::Measure,
                        )
                    }
                }),
                menu_item("Settings", {
                    let window = window.clone();
                    move || show_settings_dialog(&window)
                }),
            ],
        );
    });
    toolbar.add_controller(click);
}

fn set_active_tool(
    active_tool: &Rc<RefCell<Tool>>,
    tool_label: &gtk::Label,
    tool_context: &gtk::Box,
    cad: &UiCadContext,
    canvas: &CadCanvas,
    tool: Tool,
) {
    *active_tool.borrow_mut() = tool;
    tool_label.set_text(&format!("Tool: {}", tool.label()));
    canvas.cancel_interaction();
    refresh_tool_context(tool_context, tool, cad, canvas);
}

fn menu_item<F>(label: &'static str, action: F) -> (&'static str, Box<dyn Fn() + 'static>)
where
    F: Fn() + 'static,
{
    (label, Box::new(action))
}

fn unit_menu_item(
    label: &'static str,
    unit: Unit,
    document: Rc<RefCell<Document>>,
    properties: gtk::Box,
    canvas: gtk::DrawingArea,
    modified_label: gtk::Label,
) -> (&'static str, Box<dyn Fn() + 'static>) {
    menu_item(label, move || {
        let mut doc = document.borrow_mut();
        doc.units = unit;
        doc.modified = true;
        refresh_properties(&properties, &doc);
        modified_label.set_text("Modified");
        canvas.queue_draw();
    })
}

fn popup_menu<W: IsA<gtk::Widget>>(
    parent: &W,
    x: f64,
    y: f64,
    items: Vec<(&'static str, Box<dyn Fn() + 'static>)>,
) {
    let popover = gtk::Popover::new();
    popover.add_css_class("context-popover");
    popover.set_has_arrow(true);
    popover.set_parent(parent);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(
        x.round() as i32,
        y.round() as i32,
        1,
        1,
    )));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);

    for (label, action) in items {
        let button = gtk::Button::with_label(label);
        button.add_css_class("context-menu-item");
        button.set_halign(gtk::Align::Fill);
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            action();
            popover.popdown();
        });
        content.append(&button);
    }

    popover.set_child(Some(&content));
    popover.popup();
}

fn import_and_refresh(
    window: &adw::ApplicationWindow,
    document: &Rc<RefCell<Document>>,
    path: &Path,
    canvas: &CadCanvas,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
    layout_tabs: &gtk::Box,
) {
    let result = {
        let mut document = document.borrow_mut();
        crate::import::import_path(path, &mut document)
    };

    match result {
        Ok(summary) => {
            refresh_properties(properties, &document.borrow());
            refresh_layout_tabs(
                layout_tabs,
                document.clone(),
                canvas.clone(),
                properties.clone(),
            );
            modified_label.set_text("Modified");
            canvas.fit_document(&document.borrow());
            show_import_summary(window, &summary);
        }
        Err(error) if error.starts_with(crate::import::dwg::DWG_SETUP_REQUIRED) => {
            show_dwg_setup_dialog(window, &error)
        }
        Err(error) => show_error(window, "Import failed", &error),
    }
}

fn import_as_new_document(path: &Path) -> Result<(Document, crate::import::ImportSummary), String> {
    let mut document = Document::new_empty();
    document.name = title_for_path(path);
    let summary = crate::import::import_path(path, &mut document)?;
    Ok((document, summary))
}

fn attach_keyboard_shortcuts(
    window: &adw::ApplicationWindow,
    cad: &UiCadContext,
    view: &UiViewContext,
    selection_label: gtk::Label,
    active_tool: Rc<RefCell<Tool>>,
) {
    let cad = cad.clone();
    let view = view.clone();
    let clipboard = cad.clipboard.clone();
    let canvas = view.canvas.clone();
    let modified_label = view.modified_label.clone();
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let shortcut_window = window.clone();
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if gtk::prelude::GtkWindowExt::focus(&shortcut_window)
            .map(|widget| widget.is::<gtk::Entry>())
            .unwrap_or(false)
        {
            return gtk::glib::Propagation::Proceed;
        }
        let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        match key {
            gdk::Key::Delete => {
                let ids = cad.selected_entity.borrow().clone();
                if ids.is_empty() {
                    return gtk::glib::Propagation::Proceed;
                }
                if delete_selected_entities(&cad, &ids) {
                    cad.selected_entity.borrow_mut().clear();
                    refresh_after_history_change(
                        &cad,
                        &view,
                        &selection_label,
                        &cad.selected_entity,
                    );
                }
                gtk::glib::Propagation::Stop
            }
            gdk::Key::z if ctrl => {
                let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
                if shift {
                    if perform_redo(&cad) {
                        refresh_after_history_change(
                            &cad,
                            &view,
                            &selection_label,
                            &cad.selected_entity,
                        );
                        gtk::glib::Propagation::Stop
                    } else {
                        gtk::glib::Propagation::Proceed
                    }
                } else if perform_undo(&cad) {
                    refresh_after_history_change(
                        &cad,
                        &view,
                        &selection_label,
                        &cad.selected_entity,
                    );
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            }
            gdk::Key::y if ctrl => {
                if perform_redo(&cad) {
                    refresh_after_history_change(
                        &cad,
                        &view,
                        &selection_label,
                        &cad.selected_entity,
                    );
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            }
            gdk::Key::F3 => {
                {
                    let mut state = cad.osnap.borrow_mut();
                    state.enabled = !state.enabled;
                }
                canvas.widget().queue_draw();
                gtk::glib::Propagation::Stop
            }
            gdk::Key::F8 => {
                {
                    let mut state = cad.precision.borrow_mut();
                    state.ortho_enabled = !state.ortho_enabled;
                }
                canvas.widget().queue_draw();
                gtk::glib::Propagation::Stop
            }
            gdk::Key::F10 => {
                {
                    let mut state = cad.precision.borrow_mut();
                    state.polar_enabled = !state.polar_enabled;
                }
                canvas.widget().queue_draw();
                gtk::glib::Propagation::Stop
            }
            gdk::Key::F12 => {
                {
                    let mut state = cad.precision.borrow_mut();
                    state.dynamic_input_enabled = !state.dynamic_input_enabled;
                }
                canvas.widget().queue_draw();
                gtk::glib::Propagation::Stop
            }
            gdk::Key::c if ctrl => {
                let ids = cad.selected_entity.borrow().clone();
                *clipboard.borrow_mut() = cad.document.borrow().entities_by_ids(&ids);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::v if ctrl => {
                let entities = clipboard.borrow().clone();
                if entities.is_empty() {
                    return gtk::glib::Propagation::Proceed;
                }
                let target = *canvas.cursor().borrow();
                let ids = paste_entities_at(&cad, &entities, target);
                *cad.selected_entity.borrow_mut() = ids;
                refresh_after_history_change(&cad, &view, &selection_label, &cad.selected_entity);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                if *active_tool.borrow() == Tool::Polyline {
                    canvas.finish_polyline(&cad.document, &cad.history);
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
        }
    });
    window.add_controller(keys);
}

fn save_document(
    window: &adw::ApplicationWindow,
    document: &Rc<RefCell<Document>>,
    path: &Path,
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
        .modal(false)
        .destroy_with_parent(true)
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
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-cad.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

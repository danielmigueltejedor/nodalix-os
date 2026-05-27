use crate::{
    pages,
    system::displays::{self, Monitor},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const CANVAS_W: f64 = 640.0;
const CANVAS_H: f64 = 280.0;

pub fn build_displays_page() -> gtk::Widget {
    let page = pages::page(
        "Pantallas",
        "Resolución por monitor y disposición espacial (arrastra los monitores en el lienzo).",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let layout_card = pages::card("Disposición");
    let hint = gtk::Label::new(Some(
        "Arrastra cada pantalla para colocarla. Pulsa «Aplicar disposición» para guardar en Hyprland.",
    ));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    hint.add_css_class("home-card-detail");
    layout_card.append(&hint);

    let canvas_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    canvas_host.add_css_class("monitor-canvas-host");
    layout_card.append(&canvas_host);

    let apply_layout_btn = gtk::Button::with_label("Aplicar disposición");
    apply_layout_btn.add_css_class("pill-button");
    apply_layout_btn.add_css_class("pill-button-accent");
    layout_card.append(&apply_layout_btn);
    page.append(&layout_card);

    let monitors_card = pages::card("Monitores");
    let monitors_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    monitors_card.append(&monitors_box);
    page.append(&monitors_card);

    let positions: Rc<RefCell<HashMap<String, (i64, i64)>>> =
        Rc::new(RefCell::new(HashMap::new()));

    let reload: Rc<dyn Fn()> = Rc::new({
        let canvas_host = canvas_host.clone();
        let monitors_box = monitors_box.clone();
        let status = status.clone();
        let positions = positions.clone();
        move || {
            status.set_loading("Detectando pantallas…");
            run_bg(
                displays::list_monitors,
                {
                    let canvas_host = canvas_host.clone();
                    let monitors_box = monitors_box.clone();
                    let status = status.clone();
                    let positions = positions.clone();
                    move |result| match result {
                        Ok(monitors) => {
                            *positions.borrow_mut() = monitors
                                .iter()
                                .map(|m| (m.name.clone(), (m.x, m.y)))
                                .collect();
                            rebuild_monitors_ui(
                                &monitors_box,
                                &canvas_host,
                                &positions,
                                &status,
                                &monitors,
                            );
                            status.set_success("Pantallas detectadas");
                        }
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        }
    });

    reload();

    let status_apply = status.clone();
    let positions_apply = positions.clone();
    let reload_after = reload.clone();
    apply_layout_btn.connect_clicked(move |_| {
        let layout: Vec<(String, i64, i64)> = positions_apply
            .borrow()
            .iter()
            .map(|(n, (x, y))| (n.clone(), *x, *y))
            .collect();
        status_apply.set_loading("Aplicando disposición…");
        let status_apply = status_apply.clone();
        let reload_after = reload_after.clone();
        run_bg(
            move || displays::apply_layout(&layout),
            move |result| {
                match result {
                    Ok(()) => status_apply.set_success("Disposición aplicada"),
                    Err(err) => status_apply.set_error(&err),
                }
                reload_after();
            },
        );
    });

    pages::scrolled_page(page)
}

fn rebuild_monitors_ui(
    monitors_box: &gtk::Box,
    canvas_host: &gtk::Box,
    positions: &Rc<RefCell<HashMap<String, (i64, i64)>>>,
    status: &StatusStrip,
    monitors: &[Monitor],
) {
    while let Some(child) = monitors_box.first_child() {
        monitors_box.remove(&child);
    }
    while let Some(child) = canvas_host.first_child() {
        canvas_host.remove(&child);
    }

    let (scale, offset_x, offset_y) = layout_metrics(monitors);
    let fixed = gtk::Fixed::new();
    fixed.set_size_request(CANVAS_W as i32, CANVAS_H as i32);
    fixed.add_css_class("monitor-canvas");

    for monitor in monitors {
        let w = (monitor.width as f64 * scale).max(96.0) as i32;
        let h = (monitor.height as f64 * scale * 0.55).max(56.0) as i32;
        let px = offset_x + monitor.x as f64 * scale;
        let py = offset_y + monitor.y as f64 * scale;

        let frame = gtk::Frame::new(None);
        frame.add_css_class("monitor-block");
        frame.set_size_request(w, h);

        let inner = gtk::Box::new(gtk::Orientation::Vertical, 4);
        inner.set_margin_top(8);
        inner.set_margin_bottom(8);
        inner.set_margin_start(10);
        inner.set_margin_end(10);
        let title = gtk::Label::new(Some(&monitor.description));
        title.add_css_class("monitor-block-title");
        let res = gtk::Label::new(Some(&format!("{}×{}", monitor.width, monitor.height)));
        res.add_css_class("monitor-block-res");
        inner.append(&title);
        inner.append(&res);
        frame.set_child(Some(&inner));

        fixed.put(&frame, px, py);

        let name = monitor.name.clone();
        let positions = positions.clone();
        let fixed_drag = fixed.clone();
        let frame_drag = frame.clone();
        let canvas_pos = Rc::new(Cell::new((px, py)));

        let gesture = gtk::GestureDrag::new();
        let fixed_update = fixed_drag.clone();
        let frame_update = frame_drag.clone();
        let canvas_pos_update = canvas_pos.clone();
        gesture.connect_drag_update(move |g, dx, dy| {
            let (ox, oy) = canvas_pos_update.get();
            let (sx, sy) = g.start_point().unwrap_or((0.0, 0.0));
            let nx = ox + sx + dx;
            let ny = oy + sy + dy;
            fixed_update.move_(&frame_update, nx, ny);
        });
        let fixed_end = fixed_drag.clone();
        let frame_end = frame_drag.clone();
        let canvas_pos_end = canvas_pos.clone();
        gesture.connect_drag_end(move |g, _, _| {
            let (ox, oy) = canvas_pos_end.get();
            let (dx, dy) = g.offset().unwrap_or((0.0, 0.0));
            let (sx, sy) = g.start_point().unwrap_or((0.0, 0.0));
            let final_x = ox + sx + dx;
            let final_y = oy + sy + dy;
            fixed_end.move_(&frame_end, final_x, final_y);
            canvas_pos_end.set((final_x, final_y));
            let logical_x = ((final_x - offset_x) / scale).round() as i64;
            let logical_y = ((final_y - offset_y) / scale).round() as i64;
            positions
                .borrow_mut()
                .insert(name.clone(), (logical_x.max(0), logical_y.max(0)));
        });
        frame.add_controller(gesture);
    }

    canvas_host.append(&fixed);

    for monitor in monitors {
        monitors_box.append(&monitor_row(monitor, status));
    }
}

fn layout_metrics(monitors: &[Monitor]) -> (f64, f64, f64) {
    if monitors.is_empty() {
        return (0.12, 20.0, 20.0);
    }
    let max_x = monitors.iter().map(|m| m.x + m.width).max().unwrap_or(1920);
    let max_y = monitors.iter().map(|m| m.y + m.height).max().unwrap_or(1080);
    let scale_x = (CANVAS_W - 40.0) / max_x.max(1) as f64;
    let scale_y = (CANVAS_H - 40.0) / max_y.max(1) as f64;
    let scale = scale_x.min(scale_y).min(0.2);
    (scale, 20.0, 20.0)
}

fn monitor_row(monitor: &Monitor, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 10);
    row.add_css_class("monitor-config-row");

    let head = gtk::Label::new(Some(&format!(
        "{} — {}×{} @ {:.0} Hz",
        monitor.description, monitor.width, monitor.height, monitor.refresh
    )));
    head.set_xalign(0.0);
    head.add_css_class("info-label");
    row.append(&head);

    let mode_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let mode_label = gtk::Label::new(Some("Resolución"));
    mode_label.add_css_class("info-label");
    mode_label.set_hexpand(true);

    let strings: Vec<String> = monitor.modes.iter().map(|m| m.label()).collect();
    let list = gtk::StringList::new(&strings.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let dropdown = gtk::DropDown::new(Some(list), gtk::Expression::NONE);
    dropdown.set_selected(0);
    mode_row.append(&mode_label);
    mode_row.append(&dropdown);
    row.append(&mode_row);

    let apply = gtk::Button::with_label("Aplicar resolución");
    apply.add_css_class("pill-button");
    let name = monitor.name.clone();
    let scale = monitor.scale;
    let x = monitor.x;
    let y = monitor.y;
    let modes = monitor.modes.clone();
    let status = status.clone();
    apply.connect_clicked(move |_| {
        let idx = dropdown.selected() as usize;
        let Some(mode) = modes.get(idx) else {
            return;
        };
        status.set_loading("Aplicando resolución…");
        let status = status.clone();
        let name = name.clone();
        let mode = mode.clone();
        run_bg(
            move || displays::apply_monitor(&name, &mode, x, y, scale),
            move |result| match result {
                Ok(()) => status.set_success("Resolución aplicada"),
                Err(err) => status.set_error(&err),
            },
        );
    });
    row
}

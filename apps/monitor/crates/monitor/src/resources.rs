//! CPU, memory and network graphs.

use std::cell::RefCell;
use std::collections::VecDeque;

use monitor_core::cpu::parse_model;
use monitor_core::format_bytes;
use nodalia_gtk::gtk::{self, prelude::*};

use crate::graph::Graph;
use crate::i18n::tr;
use crate::sampler::{push, Snapshot};

pub struct ResourceView {
    pub widget: gtk::ScrolledWindow,
    cpu_value: gtk::Label,
    cpu_graph: Graph,
    core_bars: Vec<gtk::LevelBar>,
    mem_value: gtk::Label,
    mem_graph: Graph,
    swap_value: gtk::Label,
    swap_graph: Graph,
    down_value: gtk::Label,
    up_value: gtk::Label,
    down_graph: Graph,
    up_graph: Graph,
    rate_history: RefCell<(VecDeque<f64>, VecDeque<f64>)>,
}

fn card(title: &str, subtitle: Option<&str>) -> (gtk::Box, gtk::Box) {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    outer.add_css_class("monitor-card");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let name = gtk::Label::new(Some(&title.to_uppercase()));
    name.add_css_class("ndl-micro");
    name.set_xalign(0.0);
    header.append(&name);
    if let Some(sub) = subtitle {
        let sub = gtk::Label::new(Some(sub));
        sub.add_css_class("dim-label");
        sub.set_xalign(0.0);
        sub.set_hexpand(true);
        sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
        header.append(&sub);
    }
    outer.append(&header);
    (outer.clone(), outer)
}

fn big_value() -> gtk::Label {
    let l = gtk::Label::new(None);
    l.add_css_class("monitor-value");
    l.set_xalign(0.0);
    l
}

fn dim() -> gtk::Label {
    let l = gtk::Label::new(None);
    l.add_css_class("dim-label");
    l.set_xalign(0.0);
    l
}

impl ResourceView {
    pub fn new(core_count: usize) -> Self {
        let model = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| parse_model(&t));

        let (cpu_card, _) = card(tr("CPU"), model.as_deref());
        let cpu_value = big_value();
        let cpu_graph = Graph::new("monitor-graph-accent", 1.0);
        cpu_card.append(&cpu_value);
        cpu_card.append(&cpu_graph.area);
        let cores_label = dim();
        cores_label.set_text(&format!("{}: {core_count}", tr("Cores")));
        cpu_card.append(&cores_label);
        let grid = gtk::FlowBox::new();
        grid.set_selection_mode(gtk::SelectionMode::None);
        grid.set_min_children_per_line(2);
        grid.set_max_children_per_line(16);
        grid.set_homogeneous(true);
        grid.set_column_spacing(12);
        grid.set_row_spacing(6);
        let core_bars: Vec<gtk::LevelBar> = (0..core_count)
            .map(|i| {
                let bar = gtk::LevelBar::for_interval(0.0, 1.0);
                bar.set_hexpand(true);
                bar.set_tooltip_text(Some(&format!("{} {}", tr("CPU"), i + 1)));
                grid.append(&bar);
                bar
            })
            .collect();
        cpu_card.append(&grid);

        let (mem_card, _) = card(tr("Memory"), None);
        let mem_value = big_value();
        let mem_graph = Graph::new("monitor-graph-success", 1.0);
        mem_card.append(&mem_value);
        mem_card.append(&mem_graph.area);
        let (swap_card, _) = card(tr("Swap"), None);
        let swap_value = big_value();
        let swap_graph = Graph::new("monitor-graph-warning", 1.0);
        swap_card.append(&swap_value);
        swap_card.append(&swap_graph.area);

        let (net_card, _) = card(tr("Network"), None);
        let down_value = dim();
        let up_value = dim();
        let down_graph = Graph::new("monitor-graph-info", 1.0);
        let up_graph = Graph::new("monitor-graph-warning", 1.0);
        let stack = gtk::Overlay::new();
        stack.set_child(Some(&down_graph.area));
        stack.add_overlay(&up_graph.area);
        net_card.append(&down_value);
        net_card.append(&up_value);
        net_card.append(&stack);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_start(8);
        content.set_margin_end(8);
        content.set_margin_bottom(12);
        content.append(&cpu_card);
        let pair = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        pair.set_homogeneous(true);
        pair.append(&mem_card);
        pair.append(&swap_card);
        content.append(&pair);
        content.append(&net_card);
        let widget = gtk::ScrolledWindow::builder()
            .child(&content)
            .vexpand(true)
            .build();

        Self {
            widget,
            cpu_value,
            cpu_graph,
            core_bars,
            mem_value,
            mem_graph,
            swap_value,
            swap_graph,
            down_value,
            up_value,
            down_graph,
            up_graph,
            rate_history: RefCell::new((VecDeque::new(), VecDeque::new())),
        }
    }

    pub fn update(&self, s: &Snapshot) {
        self.cpu_value
            .set_text(&format!("{:.0} %", s.cpu_total * 100.0));
        self.cpu_graph.push(s.cpu_total);
        for (bar, usage) in self.core_bars.iter().zip(&s.cores) {
            bar.set_value(*usage);
        }

        let m = &s.memory;
        self.mem_value.set_text(&format!(
            "{} / {}",
            format_bytes(m.used()),
            format_bytes(m.total)
        ));
        self.mem_graph.push(m.fraction());
        self.swap_value.set_text(&if m.swap_total == 0 {
            "—".to_string()
        } else {
            format!(
                "{} / {}",
                format_bytes(m.swap_used()),
                format_bytes(m.swap_total)
            )
        });
        self.swap_graph.push(m.swap_fraction());

        self.down_value.set_text(&format!(
            "{}  {}/s",
            tr("Download"),
            format_bytes(s.net.0 as u64)
        ));
        self.up_value.set_text(&format!(
            "{}  {}/s",
            tr("Upload"),
            format_bytes(s.net.1 as u64)
        ));
        {
            let mut h = self.rate_history.borrow_mut();
            push(&mut h.0, s.net.0);
            push(&mut h.1, s.net.1);
        }
        self.down_graph.push(s.net.0);
        self.up_graph.push(s.net.1);
        // One scale for both lines, at least 100 KiB/s so an idle link is not a flat roof.
        let top = self
            .down_graph
            .peak()
            .max(self.up_graph.peak())
            .max(100.0 * 1024.0);
        self.down_graph.set_max(top);
        self.up_graph.set_max(top);
    }
}

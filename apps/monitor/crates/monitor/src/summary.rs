//! The strip of small graphs above the process table: what the machine is doing at a glance,
//! so the app shows graphs whichever page it opens on.

use monitor_core::format_bytes;
use nodalia_gtk::gtk::{self, prelude::*};

use crate::graph::Graph;
use crate::i18n::tr;
use crate::sampler::Snapshot;

struct Tile {
    widget: gtk::Box,
    value: gtk::Label,
    graph: Graph,
}

fn tile(title: &str, class: &str, max: f64) -> Tile {
    let widget = gtk::Box::new(gtk::Orientation::Vertical, 4);
    widget.add_css_class("monitor-card");
    widget.add_css_class("monitor-tile");
    widget.set_hexpand(true);
    let name = gtk::Label::new(Some(&title.to_uppercase()));
    name.add_css_class("ndl-micro");
    name.set_xalign(0.0);
    let value = gtk::Label::new(None);
    value.add_css_class("monitor-tile-value");
    value.set_xalign(0.0);
    let graph = Graph::new(class, max).compact(44);
    widget.append(&name);
    widget.append(&value);
    widget.append(&graph.area);
    Tile {
        widget,
        value,
        graph,
    }
}

pub struct Summary {
    pub widget: gtk::Box,
    cpu: Tile,
    memory: Tile,
    network: Tile,
    disk: Tile,
}

impl Summary {
    pub fn new() -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        widget.set_homogeneous(true);
        widget.set_margin_start(8);
        widget.set_margin_end(8);
        widget.set_margin_bottom(12);
        let cpu = tile(tr("CPU"), "monitor-graph-accent", 1.0);
        let memory = tile(tr("Memory"), "monitor-graph-success", 1.0);
        let network = tile(tr("Network"), "monitor-graph-info", 1.0);
        let disk = tile(tr("Disk"), "monitor-graph-warning", 1.0);
        for t in [&cpu, &memory, &network, &disk] {
            widget.append(&t.widget);
        }
        Self {
            widget,
            cpu,
            memory,
            network,
            disk,
        }
    }

    pub fn update(&self, s: &Snapshot) {
        self.cpu
            .value
            .set_text(&format!("{:.0} %", s.cpu_total * 100.0));
        self.cpu.graph.push(s.cpu_total);
        self.memory
            .value
            .set_text(&format!("{:.0} %", s.memory.fraction() * 100.0));
        self.memory.graph.push(s.memory.fraction());
        self.network.value.set_text(&format!(
            "{} {}/s",
            tr("Download"),
            format_bytes(s.net.0 as u64)
        ));
        self.network.graph.push(s.net.0 + s.net.1);
        self.network
            .graph
            .set_max(self.network.graph.peak().max(100.0 * 1024.0));
        self.disk.value.set_text(&format!(
            "{} {}/s",
            tr("Read"),
            format_bytes(s.disk.0 as u64)
        ));
        self.disk.graph.push(s.disk.0 + s.disk.1);
        self.disk
            .graph
            .set_max(self.disk.graph.peak().max(1024.0 * 1024.0));
    }
}

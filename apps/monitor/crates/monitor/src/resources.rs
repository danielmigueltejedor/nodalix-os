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
    cores_label: gtk::Label,
    core_grid: gtk::Grid,
    /// One bar per online core, by core number; follows cores coming and going.
    core_bars: RefCell<Vec<(u32, gtk::ProgressBar)>>,
    mem_value: gtk::Label,
    mem_detail: gtk::Label,
    mem_graph: Graph,
    swap_value: gtk::Label,
    swap_detail: gtk::Label,
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

/// "of 15.7 GiB · 4 %": what the big number is a share of.
fn usage_detail(used: u64, total: u64) -> String {
    let percent = if total == 0 {
        0.0
    } else {
        used as f64 * 100.0 / total as f64
    };
    format!("{} {} · {:.0} %", tr("of"), format_bytes(total), percent)
}

impl ResourceView {
    pub fn new() -> Self {
        let model = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| parse_model(&t));

        let (cpu_card, _) = card(tr("CPU"), model.as_deref());
        let cpu_value = big_value();
        let cpu_graph = Graph::new("monitor-graph-accent", 1.0);
        cpu_card.append(&cpu_value);
        cpu_card.append(&cpu_graph.area);
        let cores_label = dim();
        cpu_card.append(&cores_label);
        let grid = gtk::Grid::new();
        grid.set_column_homogeneous(true);
        grid.set_column_spacing(12);
        grid.set_row_spacing(6);
        cpu_card.append(&grid);

        let (mem_card, _) = card(tr("Memory"), None);
        let mem_value = big_value();
        let mem_detail = dim();
        let mem_graph = Graph::new("monitor-graph-success", 1.0);
        mem_card.append(&mem_value);
        mem_card.append(&mem_detail);
        mem_card.append(&mem_graph.area);
        let (swap_card, _) = card(tr("Swap"), None);
        let swap_value = big_value();
        let swap_detail = dim();
        let swap_graph = Graph::new("monitor-graph-warning", 1.0);
        swap_card.append(&swap_value);
        swap_card.append(&swap_detail);
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
            cores_label,
            core_grid: grid,
            core_bars: RefCell::new(Vec::new()),
            mem_value,
            mem_detail,
            mem_graph,
            swap_value,
            swap_detail,
            swap_graph,
            down_value,
            up_value,
            down_graph,
            up_graph,
            rate_history: RefCell::new((VecDeque::new(), VecDeque::new())),
        }
    }

    /// Makes the bars match the cores in the reading: a first reading that failed leaves none,
    /// and a core can go offline or come back.
    fn sync_core_bars(&self, cores: &[(u32, f64)]) {
        let mut bars = self.core_bars.borrow_mut();
        let same =
            bars.len() == cores.len() && bars.iter().zip(cores).all(|((a, _), (b, _))| a == b);
        if !same {
            for (_, bar) in bars.drain(..) {
                if let Some(cell) = bar.parent() {
                    self.core_grid.remove(&cell);
                }
            }
            let columns = core_columns(cores.len());
            for (index, (id, _)) in cores.iter().enumerate() {
                let cell = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let number = gtk::Label::new(Some(&(id + 1).to_string()));
                number.add_css_class("dim-label");
                number.add_css_class("numeric");
                number.set_width_chars(2);
                number.set_xalign(1.0);
                let bar = gtk::ProgressBar::new();
                bar.set_hexpand(true);
                bar.set_valign(gtk::Align::Center);
                bar.set_size_request(120, -1);
                bar.set_tooltip_text(Some(&format!("{} {}", tr("CPU"), id + 1)));
                cell.append(&number);
                cell.append(&bar);
                self.core_grid.attach(
                    &cell,
                    (index % columns) as i32,
                    (index / columns) as i32,
                    1,
                    1,
                );
                bars.push((*id, bar));
            }
            self.cores_label
                .set_text(&format!("{}: {}", tr("Cores"), cores.len()));
        }
        for ((_, bar), (_, usage)) in bars.iter().zip(cores) {
            bar.set_fraction(*usage);
        }
    }

    pub fn update(&self, s: &Snapshot) {
        self.cpu_value
            .set_text(&format!("{:.0} %", s.cpu_total * 100.0));
        self.cpu_graph.push(s.cpu_total);
        self.sync_core_bars(&s.cores);

        let m = &s.memory;
        self.mem_value.set_text(&format_bytes(m.used()));
        self.mem_detail.set_text(&usage_detail(m.used(), m.total));
        self.mem_graph.push(m.fraction());
        if m.swap_total == 0 {
            self.swap_value.set_text("—");
            self.swap_detail.set_text(tr("This system has no swap"));
        } else {
            self.swap_value.set_text(&format_bytes(m.swap_used()));
            self.swap_detail
                .set_text(&usage_detail(m.swap_used(), m.swap_total));
        }
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

/// How many bars go on a row: as many rows as eight columns need, then the columns that spread
/// the cores evenly over them, so twelve cores are two rows of six and not five and five and two.
fn core_columns(cores: usize) -> usize {
    let rows = cores.div_ceil(8).max(1);
    cores.div_ceil(rows).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cores_fill_even_rows() {
        assert_eq!(core_columns(0), 1);
        assert_eq!(core_columns(4), 4);
        assert_eq!(core_columns(8), 8);
        assert_eq!(core_columns(12), 6);
        assert_eq!(core_columns(16), 8);
        assert_eq!(core_columns(20), 7);
    }

    #[test]
    fn the_detail_says_how_much_of_the_total_is_used() {
        assert!(usage_detail(1024 * 1024, 4 * 1024 * 1024).ends_with("25 %"));
        // No total (no swap) is not a division by zero.
        assert!(usage_detail(0, 0).ends_with("0 %"));
    }
}

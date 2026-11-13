//! A line graph with a filled area, drawn in the widget's CSS `color` so NDL's modes and
//! contrast settings reach it without any colour being named here.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use nodalia_gtk::gtk::{self, prelude::*};

use crate::sampler::{push, HISTORY};

#[derive(Clone)]
pub struct Graph {
    pub area: gtk::DrawingArea,
    data: Rc<RefCell<VecDeque<f64>>>,
    max: Rc<Cell<f64>>,
    guides: Rc<Cell<bool>>,
}

impl Graph {
    /// `class` is a CSS class on the area whose `color` is the line colour. `max` is the value
    /// at the top of the graph.
    pub fn new(class: &str, max: f64) -> Self {
        let area = gtk::DrawingArea::new();
        area.add_css_class("monitor-graph");
        area.add_css_class(class);
        area.set_content_height(120);
        area.set_hexpand(true);
        let data = Rc::new(RefCell::new(VecDeque::new()));
        let max = Rc::new(Cell::new(max.max(f64::MIN_POSITIVE)));
        let guides = Rc::new(Cell::new(true));
        let graph = Self {
            area,
            data,
            max,
            guides,
        };
        let (data, max, guides) = (graph.data.clone(), graph.max.clone(), graph.guides.clone());
        graph.area.set_draw_func(move |area, cr, width, height| {
            draw(
                area,
                cr,
                width as f64,
                height as f64,
                &data.borrow(),
                max.get(),
                guides.get(),
            );
        });
        graph
    }

    /// A graph for a small place: lower, and without the guide lines.
    pub fn compact(self, height: i32) -> Self {
        self.area.set_content_height(height);
        self.guides.set(false);
        self
    }

    pub fn push(&self, value: f64) {
        push(&mut self.data.borrow_mut(), value);
        self.area.queue_draw();
    }

    pub fn set_max(&self, max: f64) {
        self.max.set(max.max(f64::MIN_POSITIVE));
        self.area.queue_draw();
    }

    pub fn peak(&self) -> f64 {
        self.data.borrow().iter().copied().fold(0.0, f64::max)
    }
}

/// The fewest samples a graph spreads across its width: a graph that has just opened is not a
/// line in the corner of an empty box. Past this many the picture slowly zooms out, one sample
/// at a time, until it holds the whole history.
const MIN_SPAN: usize = 24;

/// X of sample `i` out of `n` shown, the newest at the right edge.
fn x_of(i: usize, n: usize, width: f64) -> f64 {
    let span = n.clamp(MIN_SPAN, HISTORY);
    let step = width / (span - 1) as f64;
    width - (n - 1 - i) as f64 * step
}

fn draw(
    area: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    w: f64,
    h: f64,
    data: &VecDeque<f64>,
    max: f64,
    guides: bool,
) {
    let color = area.color();
    // Four horizontal guides, faint.
    cr.set_source_rgba(
        color.red().into(),
        color.green().into(),
        color.blue().into(),
        0.10,
    );
    cr.set_line_width(1.0);
    for i in (1..4).filter(|_| guides) {
        let y = (h * i as f64 / 4.0).round() + 0.5;
        cr.move_to(0.0, y);
        cr.line_to(w, y);
        let _ = cr.stroke();
    }
    if data.is_empty() {
        return;
    }
    let n = data.len();
    let y_of = |v: f64| h - (v / max).clamp(0.0, 1.0) * (h - 2.0) - 1.0;
    let trace = |cr: &gtk::cairo::Context| {
        for (i, v) in data.iter().enumerate() {
            let (x, y) = (x_of(i, n, w), y_of(*v));
            if i == 0 {
                cr.move_to(x, y)
            } else {
                cr.line_to(x, y)
            }
        }
    };
    trace(cr);
    cr.line_to(w, h);
    cr.line_to(x_of(0, n, w), h);
    cr.close_path();
    // The fill fades towards the bottom, so the line reads first.
    let fade = gtk::cairo::LinearGradient::new(0.0, 0.0, 0.0, h);
    let (r, g, b) = (
        color.red().into(),
        color.green().into(),
        color.blue().into(),
    );
    fade.add_color_stop_rgba(0.0, r, g, b, 0.32);
    fade.add_color_stop_rgba(1.0, r, g, b, 0.02);
    let _ = cr.set_source(&fade);
    let _ = cr.fill();
    trace(cr);
    cr.set_source_rgba(
        color.red().into(),
        color.green().into(),
        color.blue().into(),
        color.alpha().into(),
    );
    cr.set_line_width(2.0);
    cr.set_line_join(gtk::cairo::LineJoin::Round);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    let _ = cr.stroke();
    // A dot on the newest sample.
    if let Some(last) = data.back() {
        cr.arc(
            (w - 3.0).max(0.0),
            y_of(*last).clamp(3.0, (h - 3.0).max(3.0)),
            3.0,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.fill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_sample_sits_at_the_right_edge() {
        assert_eq!(x_of(4, 5, 590.0), 590.0);
        assert_eq!(x_of(3, 5, 590.0), 590.0 - 590.0 / 23.0);
    }

    #[test]
    fn a_full_history_spans_the_width() {
        assert!(x_of(0, HISTORY, 590.0).abs() < 1e-9);
    }

    #[test]
    fn a_new_graph_still_spreads_over_the_width() {
        // Five samples are the right fifth of a graph made for twenty-four.
        let left = x_of(0, 5, 230.0);
        assert!((left - (230.0 - 4.0 * 10.0)).abs() < 1e-9);
        // From twenty-four on, the oldest sample is at the left edge.
        assert!(x_of(0, MIN_SPAN, 230.0).abs() < 1e-9);
    }
}

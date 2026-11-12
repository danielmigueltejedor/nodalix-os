//! The screens' layout, without GTK or D-Bus: what mutter reports (`GetCurrentState`) turned
//! into a configuration that can be edited — resolution, refresh rate, scale, orientation,
//! the primary screen and where each screen sits — and turned back into what
//! `ApplyMonitorsConfig` takes. Every rule that keeps a configuration valid lives here, where it
//! is tested: a wrong configuration can leave a screen black.

#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub refresh: f64,
    pub scales: Vec<f64>,
    pub preferred: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub connector: String,
    pub name: String,
    pub modes: Vec<Mode>,
}

/// A screen of the desktop: the monitor(s) it shows (more than one when mirrored), the mode each
/// uses, where it is, and how it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Logical {
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    /// 0 normal, 1 rotated 90°, 2 180°, 3 270°, 4–7 the same flipped.
    pub transform: u32,
    pub primary: bool,
    /// `(connector, mode id)`.
    pub monitors: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub serial: u32,
    pub monitors: Vec<Monitor>,
    pub logicals: Vec<Logical>,
}

impl Config {
    pub fn monitor(&self, connector: &str) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.connector == connector)
    }

    /// The mode a logical screen's first monitor uses.
    pub fn mode_of(&self, index: usize) -> Option<&Mode> {
        let (connector, id) = self.logicals.get(index)?.monitors.first()?;
        self.monitor(connector)?.modes.iter().find(|m| m.id == *id)
    }

    /// The monitors that are connected but not part of the desktop.
    pub fn disabled(&self) -> Vec<&Monitor> {
        self.monitors
            .iter()
            .filter(|m| {
                !self
                    .logicals
                    .iter()
                    .any(|l| l.monitors.iter().any(|(c, _)| *c == m.connector))
            })
            .collect()
    }

    /// The size a logical screen takes on the desktop: the mode's size over the scale, with width
    /// and height swapped when it is rotated a quarter turn.
    pub fn size(&self, index: usize) -> (i32, i32) {
        let (Some(mode), Some(logical)) = (self.mode_of(index), self.logicals.get(index)) else {
            return (0, 0);
        };
        let scale = if logical.scale > 0.0 {
            logical.scale
        } else {
            1.0
        };
        let (w, h) = (
            (f64::from(mode.width) / scale).round() as i32,
            (f64::from(mode.height) / scale).round() as i32,
        );
        if logical.transform % 2 == 1 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// The distinct resolutions of a screen's monitor, largest first.
    pub fn resolutions(&self, index: usize) -> Vec<(i32, i32)> {
        let Some(monitor) = self
            .logicals
            .get(index)
            .and_then(|l| l.monitors.first())
            .and_then(|(c, _)| self.monitor(c))
        else {
            return Vec::new();
        };
        let mut out: Vec<(i32, i32)> = Vec::new();
        for m in &monitor.modes {
            if !out.contains(&(m.width, m.height)) {
                out.push((m.width, m.height));
            }
        }
        out.sort_by_key(|(w, h)| std::cmp::Reverse(i64::from(*w) * i64::from(*h)));
        out
    }

    /// The modes of a screen's monitor at a resolution, highest refresh rate first.
    pub fn refresh_rates(&self, index: usize, size: (i32, i32)) -> Vec<&Mode> {
        let Some(monitor) = self
            .logicals
            .get(index)
            .and_then(|l| l.monitors.first())
            .and_then(|(c, _)| self.monitor(c))
        else {
            return Vec::new();
        };
        let mut out: Vec<&Mode> = monitor
            .modes
            .iter()
            .filter(|m| (m.width, m.height) == size)
            .collect();
        out.sort_by(|a, b| b.refresh.total_cmp(&a.refresh));
        out
    }

    /// Uses `mode` on every monitor of the screen that has a mode with its size and nearest
    /// refresh rate (a mirrored screen's monitors share one size). The scale is kept when the new
    /// mode supports it and falls back to the mode's preferred one when it does not.
    pub fn set_mode(&mut self, index: usize, mode_id: &str) -> bool {
        let Some(wanted) = self
            .logicals
            .get(index)
            .and_then(|l| l.monitors.first())
            .and_then(|(c, _)| self.monitor(c))
            .and_then(|m| m.modes.iter().find(|m| m.id == mode_id))
            .cloned()
        else {
            return false;
        };
        let scale = self.logicals[index].scale;
        let supported = wanted.scales.iter().any(|s| (s - scale).abs() < 0.001);
        let monitors: Vec<String> = self.logicals[index]
            .monitors
            .iter()
            .map(|(c, _)| c.clone())
            .collect();
        let mut chosen = Vec::new();
        for connector in monitors {
            let id = self
                .monitor(&connector)
                .and_then(|m| {
                    m.modes
                        .iter()
                        .filter(|x| (x.width, x.height) == (wanted.width, wanted.height))
                        .min_by(|a, b| {
                            (a.refresh - wanted.refresh)
                                .abs()
                                .total_cmp(&(b.refresh - wanted.refresh).abs())
                        })
                })
                .map(|m| m.id.clone());
            let Some(id) = id else { return false };
            chosen.push((connector, id));
        }
        let logical = &mut self.logicals[index];
        logical.monitors = chosen;
        if !supported {
            logical.scale = wanted.scales.first().copied().unwrap_or(1.0);
        }
        self.normalize();
        true
    }

    /// Picks a resolution: the mode with this size, keeping the refresh rate when it has one that
    /// near and otherwise the best one.
    pub fn set_resolution(&mut self, index: usize, size: (i32, i32)) -> bool {
        let current = self.mode_of(index).map(|m| m.refresh).unwrap_or(60.0);
        let Some(best) = self
            .refresh_rates(index, size)
            .into_iter()
            .min_by(|a, b| {
                (a.refresh - current)
                    .abs()
                    .total_cmp(&(b.refresh - current).abs())
            })
            .map(|m| m.id.clone())
        else {
            return false;
        };
        self.set_mode(index, &best)
    }

    /// The scales the screen's current mode supports.
    pub fn scales(&self, index: usize) -> Vec<f64> {
        self.mode_of(index)
            .map(|m| m.scales.clone())
            .unwrap_or_default()
    }

    pub fn set_scale(&mut self, index: usize, scale: f64) -> bool {
        if !self.scales(index).iter().any(|s| (s - scale).abs() < 0.001) {
            return false;
        }
        self.logicals[index].scale = scale;
        self.normalize();
        true
    }

    pub fn set_transform(&mut self, index: usize, transform: u32) -> bool {
        if transform > 7 || index >= self.logicals.len() {
            return false;
        }
        self.logicals[index].transform = transform;
        self.normalize();
        true
    }

    /// Makes a screen the primary one; only one is.
    pub fn set_primary(&mut self, index: usize) {
        for (i, l) in self.logicals.iter_mut().enumerate() {
            l.primary = i == index;
        }
    }

    /// Takes a screen off the desktop. The last one cannot go, and the primary role passes to
    /// the first that remains.
    pub fn disable(&mut self, index: usize) -> bool {
        if self.logicals.len() < 2 || index >= self.logicals.len() {
            return false;
        }
        let removed = self.logicals.remove(index);
        if removed.primary {
            self.logicals[0].primary = true;
        }
        self.normalize();
        true
    }

    /// Puts a disabled monitor back, to the right of the others, at its preferred mode.
    pub fn enable(&mut self, connector: &str) -> bool {
        let Some(monitor) = self.monitor(connector) else {
            return false;
        };
        let Some(mode) = monitor
            .modes
            .iter()
            .find(|m| m.preferred)
            .or_else(|| monitor.modes.first())
            .cloned()
        else {
            return false;
        };
        let right = (0..self.logicals.len())
            .map(|i| self.logicals[i].x + self.size(i).0)
            .max()
            .unwrap_or(0);
        self.logicals.push(Logical {
            x: right,
            y: 0,
            scale: mode.scales.first().copied().unwrap_or(1.0),
            transform: 0,
            primary: self.logicals.is_empty(),
            monitors: vec![(connector.to_owned(), mode.id)],
        });
        self.normalize();
        true
    }

    /// Moves a screen to where it was dropped, then snaps it against the others so no screen
    /// overlaps another and the desktop stays in one piece.
    pub fn move_to(&mut self, index: usize, x: i32, y: i32) {
        if index >= self.logicals.len() {
            return;
        }
        let (w, h) = self.size(index);
        let others: Vec<(i32, i32, i32, i32)> = (0..self.logicals.len())
            .filter(|i| *i != index)
            .map(|i| {
                let (ow, oh) = self.size(i);
                (self.logicals[i].x, self.logicals[i].y, ow, oh)
            })
            .collect();
        let (nx, ny) = snap((x, y), (w, h), &others);
        self.logicals[index].x = nx;
        self.logicals[index].y = ny;
        self.normalize();
    }

    /// Shifts the whole layout so its top-left corner is (0, 0), which mutter requires.
    pub fn normalize(&mut self) {
        let min_x = self.logicals.iter().map(|l| l.x).min().unwrap_or(0);
        let min_y = self.logicals.iter().map(|l| l.y).min().unwrap_or(0);
        for l in &mut self.logicals {
            l.x -= min_x;
            l.y -= min_y;
        }
    }

    /// A configuration is valid when it has a screen, exactly one primary, every monitor in at
    /// most one screen, and no two screens overlapping.
    pub fn is_valid(&self) -> bool {
        if self.logicals.is_empty() || self.logicals.iter().filter(|l| l.primary).count() != 1 {
            return false;
        }
        let mut seen = Vec::new();
        for l in &self.logicals {
            for (c, _) in &l.monitors {
                if seen.contains(c) {
                    return false;
                }
                seen.push(c.clone());
            }
        }
        for i in 0..self.logicals.len() {
            let (aw, ah) = self.size(i);
            for j in (i + 1)..self.logicals.len() {
                let (bw, bh) = self.size(j);
                let (a, b) = (&self.logicals[i], &self.logicals[j]);
                let overlap = a.x < b.x + bw && b.x < a.x + aw && a.y < b.y + bh && b.y < a.y + ah;
                if overlap {
                    return false;
                }
            }
        }
        true
    }
}

/// How near an edge has to be to line up with it, in desktop pixels.
const LINE_UP: i32 = 48;

/// The place nearest to `at` for a rectangle of size `(w, h)` among `others` (`x, y, w, h`) that
/// touches one of them along a full edge and overlaps none. With no other screen, `at` itself.
pub fn snap(at: (i32, i32), (w, h): (i32, i32), others: &[(i32, i32, i32, i32)]) -> (i32, i32) {
    if others.is_empty() {
        return at;
    }
    let mut best: Option<((i32, i32), i64)> = None;
    let mut consider = |p: (i32, i32)| {
        // Rejects a place that overlaps another screen.
        if others
            .iter()
            .any(|&(ox, oy, ow, oh)| p.0 < ox + ow && ox < p.0 + w && p.1 < oy + oh && oy < p.1 + h)
        {
            return;
        }
        let d = i64::from(p.0 - at.0).pow(2) + i64::from(p.1 - at.1).pow(2);
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((p, d));
        }
    };
    for &(ox, oy, ow, oh) in others {
        // Along a vertical edge the screen may slide, but must still touch: clamp, then line up
        // with the other's top or bottom when it is close.
        let slide_y = |y: i32| {
            let mut y = y.clamp(oy - h + 1, oy + oh - 1);
            for line in [oy, oy + oh - h] {
                if (y - line).abs() <= LINE_UP {
                    y = line;
                }
            }
            y
        };
        let slide_x = |x: i32| {
            let mut x = x.clamp(ox - w + 1, ox + ow - 1);
            for line in [ox, ox + ow - w] {
                if (x - line).abs() <= LINE_UP {
                    x = line;
                }
            }
            x
        };
        consider((ox + ow, slide_y(at.1)));
        consider((ox - w, slide_y(at.1)));
        consider((slide_x(at.0), oy + oh));
        consider((slide_x(at.0), oy - h));
    }
    best.map_or(at, |(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(id: &str, w: i32, h: i32, r: f64, scales: &[f64]) -> Mode {
        Mode {
            id: id.into(),
            width: w,
            height: h,
            refresh: r,
            scales: scales.to_vec(),
            preferred: false,
        }
    }

    fn laptop() -> Monitor {
        Monitor {
            connector: "eDP-1".into(),
            name: "Laptop".into(),
            modes: vec![
                Mode {
                    preferred: true,
                    ..mode("a120", 2880, 1800, 120.0, &[1.0, 1.5, 2.0])
                },
                mode("a60", 2880, 1800, 60.0, &[1.0, 1.5, 2.0]),
                mode("b60", 1920, 1200, 60.0, &[1.0, 2.0]),
            ],
        }
    }

    fn external() -> Monitor {
        Monitor {
            connector: "DP-2".into(),
            name: "Dell".into(),
            modes: vec![
                Mode {
                    preferred: true,
                    ..mode("c60", 3840, 2160, 60.0, &[1.0, 2.0])
                },
                mode("d60", 1920, 1080, 60.0, &[1.0]),
            ],
        }
    }

    fn two() -> Config {
        Config {
            serial: 1,
            monitors: vec![laptop(), external()],
            logicals: vec![
                Logical {
                    x: 0,
                    y: 0,
                    scale: 1.5,
                    transform: 0,
                    primary: true,
                    monitors: vec![("eDP-1".into(), "a120".into())],
                },
                Logical {
                    x: 1920,
                    y: 0,
                    scale: 2.0,
                    transform: 0,
                    primary: false,
                    monitors: vec![("DP-2".into(), "c60".into())],
                },
            ],
        }
    }

    #[test]
    fn a_screens_size_is_its_mode_over_the_scale() {
        let c = two();
        assert_eq!(c.size(0), (1920, 1200));
        assert_eq!(c.size(1), (1920, 1080));
    }

    #[test]
    fn a_quarter_turn_swaps_width_and_height() {
        let mut c = two();
        c.set_transform(1, 1);
        assert_eq!(c.size(1), (1080, 1920));
        assert!(!c.set_transform(1, 9));
    }

    #[test]
    fn resolutions_are_distinct_and_largest_first() {
        assert_eq!(two().resolutions(0), vec![(2880, 1800), (1920, 1200)]);
    }

    #[test]
    fn changing_resolution_keeps_the_refresh_rate_that_is_nearest() {
        let mut c = two();
        assert!(c.set_resolution(0, (1920, 1200)));
        assert_eq!(c.logicals[0].monitors[0].1, "b60");
    }

    #[test]
    fn a_scale_the_new_mode_does_not_support_falls_back_to_its_first() {
        let mut c = two();
        // 1.5 is not supported at 1920×1200.
        c.set_resolution(0, (1920, 1200));
        assert_eq!(c.logicals[0].scale, 1.0);
        // Only a supported scale can be set.
        assert!(!c.set_scale(0, 1.5));
        assert!(c.set_scale(0, 2.0));
    }

    #[test]
    fn rates_come_highest_first() {
        let c = two();
        let rates: Vec<f64> = c
            .refresh_rates(0, (2880, 1800))
            .iter()
            .map(|m| m.refresh)
            .collect();
        assert_eq!(rates, [120.0, 60.0]);
    }

    #[test]
    fn dropping_a_screen_near_another_snaps_it_to_the_edge() {
        let mut c = two();
        // Dropped a little inside the first screen and lower: it goes to the right of it.
        c.move_to(1, 1800, 200);
        assert!(c.is_valid());
        assert_eq!(c.logicals[1].x, 1920);
        assert_eq!(c.logicals[1].y, 200);
    }

    #[test]
    fn a_screen_lines_up_with_the_top_edge_when_close() {
        let mut c = two();
        c.move_to(1, 1950, 20);
        assert_eq!(c.logicals[1].y, 0);
    }

    #[test]
    fn a_screen_dropped_below_goes_below() {
        let mut c = two();
        c.move_to(1, 100, 1500);
        assert_eq!(c.logicals[1].y, 1200);
        assert!(c.is_valid());
    }

    #[test]
    fn the_layout_is_shifted_to_start_at_zero() {
        let mut c = two();
        // Left of the first screen: the whole layout moves so the left one is at x = 0.
        c.move_to(1, -2000, 0);
        assert_eq!(c.logicals.iter().map(|l| l.x).min(), Some(0));
        assert!(c.is_valid());
    }

    #[test]
    fn a_layout_with_overlap_or_two_primaries_is_not_valid() {
        let mut c = two();
        c.logicals[1].x = 100;
        assert!(!c.is_valid());
        let mut c = two();
        c.logicals[1].primary = true;
        assert!(!c.is_valid());
    }

    #[test]
    fn the_last_screen_cannot_be_turned_off_and_the_primary_moves_on() {
        let mut c = two();
        assert!(c.disable(0));
        assert!(c.logicals[0].primary);
        assert_eq!(c.logicals[0].x, 0);
        assert!(!c.disable(0));
        assert_eq!(c.disabled().len(), 1);
    }

    #[test]
    fn a_disabled_monitor_comes_back_at_its_preferred_mode_to_the_right() {
        let mut c = two();
        c.disable(1);
        assert!(c.enable("DP-2"));
        assert_eq!(c.logicals[1].monitors[0].1, "c60");
        assert_eq!(c.logicals[1].x, 1920);
        assert!(c.is_valid());
    }

    #[test]
    fn with_no_other_screen_a_drop_stays_where_it_is() {
        assert_eq!(snap((10, 20), (100, 100), &[]), (10, 20));
    }
}

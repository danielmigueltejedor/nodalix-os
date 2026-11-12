//! Memory and swap from `/proc/meminfo`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Memory {
    pub total: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}

impl Memory {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }
    pub fn swap_used(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_free)
    }
    /// Used fraction of RAM, 0.0 to 1.0.
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used() as f64 / self.total as f64
        }
    }
    pub fn swap_fraction(&self) -> f64 {
        if self.swap_total == 0 {
            0.0
        } else {
            self.swap_used() as f64 / self.swap_total as f64
        }
    }
}

/// Parses `/proc/meminfo` (values in kB) into bytes.
pub fn parse_meminfo(text: &str) -> Option<Memory> {
    let mut m = Memory::default();
    let mut have_total = false;
    let mut have_available = false;
    let mut free = 0;
    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let kb: u64 = match rest.split_whitespace().next().and_then(|v| v.parse().ok()) {
            Some(v) => v,
            None => continue,
        };
        let bytes = kb.saturating_mul(1024);
        match key {
            "MemTotal" => {
                m.total = bytes;
                have_total = true;
            }
            "MemAvailable" => {
                m.available = bytes;
                have_available = true;
            }
            "MemFree" => free = bytes,
            "SwapTotal" => m.swap_total = bytes,
            "SwapFree" => m.swap_free = bytes,
            _ => {}
        }
    }
    if !have_available {
        m.available = free; // kernels older than 3.14
    }
    have_total.then_some(m)
}

pub fn read() -> Option<Memory> {
    parse_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: &str = "MemTotal:       16000000 kB\nMemFree:         1000000 kB\nMemAvailable:    8000000 kB\nSwapTotal:       4000000 kB\nSwapFree:        3000000 kB\n";

    #[test]
    fn parses_into_bytes() {
        let m = parse_meminfo(INFO).unwrap();
        assert_eq!(m.total, 16_000_000 * 1024);
        assert_eq!(m.used(), 8_000_000 * 1024);
        assert_eq!(m.swap_used(), 1_000_000 * 1024);
        assert!((m.fraction() - 0.5).abs() < 1e-9);
        assert!((m.swap_fraction() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn falls_back_to_free_without_available() {
        let m = parse_meminfo("MemTotal: 100 kB\nMemFree: 40 kB\n").unwrap();
        assert_eq!(m.available, 40 * 1024);
    }

    #[test]
    fn no_swap_is_zero_not_nan() {
        let m = parse_meminfo("MemTotal: 100 kB\nMemAvailable: 50 kB\n").unwrap();
        assert_eq!(m.swap_fraction(), 0.0);
    }

    #[test]
    fn missing_total_is_an_error() {
        assert!(parse_meminfo("MemFree: 1 kB\n").is_none());
    }
}

//! Data sources for the Nodalix system monitor: CPU, memory, network, processes and mounted
//! file systems, read from `/proc` and the kernel's mount table. Every parser takes the text it
//! parses, so it is tested without a running system; the `read_*` functions only fetch that text.

pub mod cpu;
pub mod disk;
pub mod fs;
pub mod mem;
pub mod net;
pub mod procs;

/// Formats a byte count with binary prefixes (`1.5 GiB`).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.5 GiB");
        assert!(format_bytes(u64::MAX).ends_with("TiB"));
    }
}

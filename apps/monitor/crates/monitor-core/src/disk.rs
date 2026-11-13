//! Disk throughput from `/proc/diskstats`.

use std::collections::HashMap;

/// The sector size `/proc/diskstats` counts in, whatever the disk's own.
const SECTOR: u64 = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskTotals {
    pub read: u64,
    pub written: u64,
}

pub type Disks = HashMap<String, DiskTotals>;

/// Whether a device is a whole disk the kernel counts for real: not a partition (its disk
/// counts the same bytes), not a RAM or loop device, and not a device-mapper layer on top of
/// another disk (the disk under it counts the same bytes).
fn is_whole_disk(name: &str) -> bool {
    const SKIPPED: [&str; 6] = ["loop", "ram", "zram", "dm-", "md", "sr"];
    if SKIPPED.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    let ends_in_digit = name.chars().last().is_some_and(|c| c.is_ascii_digit());
    if !ends_in_digit {
        // sda, vda, xvda: whole disks.
        return true;
    }
    // nvme0n1 and mmcblk0 are whole disks that end in a digit, nvme0n1p1 and mmcblk0p1 are
    // partitions, and so is sda1.
    (name.starts_with("nvme") || name.starts_with("mmcblk")) && !is_partition_of_numbered(name)
}

/// `nvme0n1p2`, `mmcblk0p1`: a `p` and digits after a device that itself ends in a digit.
fn is_partition_of_numbered(name: &str) -> bool {
    match name.rfind('p') {
        Some(at) if at > 0 => {
            let (before, after) = name.split_at(at);
            let digits = &after[1..];
            !digits.is_empty()
                && digits.chars().all(|c| c.is_ascii_digit())
                && before.chars().last().is_some_and(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// Bytes read and written per whole disk.
pub fn parse(text: &str) -> Disks {
    let mut disks = Disks::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // major minor name reads merged sectors-read ms writes merged sectors-written …
        if fields.len() < 10 {
            continue;
        }
        let name = fields[2];
        if !is_whole_disk(name) {
            continue;
        }
        let (Ok(read), Ok(written)) = (fields[5].parse::<u64>(), fields[9].parse::<u64>()) else {
            continue;
        };
        disks.insert(
            name.to_owned(),
            DiskTotals {
                read: read.saturating_mul(SECTOR),
                written: written.saturating_mul(SECTOR),
            },
        );
    }
    disks
}

pub fn read() -> Option<Disks> {
    Some(parse(&std::fs::read_to_string("/proc/diskstats").ok()?))
}

/// Bytes per second read and written between two readings taken `seconds` apart.
pub fn rate(previous: &Disks, now: &Disks, seconds: f64) -> (f64, f64) {
    if seconds <= 0.0 {
        return (0.0, 0.0);
    }
    let (mut read, mut written) = (0u64, 0u64);
    for (name, totals) in now {
        if let Some(before) = previous.get(name) {
            read = read.saturating_add(totals.read.saturating_sub(before.read));
            written = written.saturating_add(totals.written.saturating_sub(before.written));
        }
    }
    (read as f64 / seconds, written as f64 / seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATS: &str = "\
   7       0 loop0 10 0 100 0 0 0 0 0 0 0 0 0 0 0 0 0 0
 259       0 nvme0n1 500 0 2000 10 300 0 1000 5 0 0 0 0 0 0 0 0 0
 259       1 nvme0n1p1 100 0 800 1 50 0 400 1 0 0 0 0 0 0 0 0 0
 259       2 nvme0n1p2 400 0 1200 9 250 0 600 4 0 0 0 0 0 0 0 0 0
   8       0 sda 40 0 160 3 20 0 80 2 0 0 0 0 0 0 0 0 0
   8       1 sda1 30 0 120 2 10 0 40 1 0 0 0 0 0 0 0 0 0
 253       0 dm-0 90 0 360 1 70 0 280 1 0 0 0 0 0 0 0 0 0
  11       0 sr0 1 0 8 0 0 0 0 0 0 0 0 0 0 0 0 0 0
 179       0 mmcblk0 5 0 40 1 5 0 40 1 0 0 0 0 0 0 0 0 0
 179       1 mmcblk0p1 4 0 30 1 4 0 30 1 0 0 0 0 0 0 0 0 0
";

    #[test]
    fn only_whole_disks_count() {
        let disks = parse(STATS);
        let mut names: Vec<&str> = disks.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["mmcblk0", "nvme0n1", "sda"]);
        assert_eq!(
            disks["nvme0n1"],
            DiskTotals {
                read: 2000 * 512,
                written: 1000 * 512
            }
        );
    }

    #[test]
    fn short_or_damaged_lines_are_skipped() {
        assert!(parse("8 0 sda 1 2\n").is_empty());
        assert!(parse("8 0 sda 1 2 x 4 5 6 y\n").is_empty());
        assert!(parse("").is_empty());
    }

    #[test]
    fn the_rate_is_bytes_per_second_and_survives_resets() {
        let before = parse(STATS);
        let mut after = before.clone();
        after.get_mut("sda").unwrap().read += 4096;
        after.get_mut("nvme0n1").unwrap().written += 1024;
        after.get_mut("mmcblk0").unwrap().read = 0; // a counter that went back
        let (read, written) = rate(&before, &after, 2.0);
        assert_eq!(read, 2048.0);
        assert_eq!(written, 512.0);
        assert_eq!(rate(&before, &after, 0.0), (0.0, 0.0));
    }
}

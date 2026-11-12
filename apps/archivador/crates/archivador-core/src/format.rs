//! Text for sizes.

/// A size the way GNOME writes it: powers of 1000, one decimal from 10 kB up only when it helps.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["kB", "MB", "GB", "TB", "PB"];
    if bytes < 1000 {
        return if bytes == 1 {
            "1 byte".into()
        } else {
            format!("{bytes} bytes")
        };
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// `rwxr-xr-x`: the permission bits of a mode, as `ls -l` writes them.
pub fn permissions(mode: u32) -> String {
    [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ]
    .iter()
    .map(|&(bit, letter)| if mode & bit != 0 { letter } else { '-' })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_are_written_like_ls() {
        assert_eq!(permissions(0o755), "rwxr-xr-x");
        assert_eq!(permissions(0o644), "rw-r--r--");
        assert_eq!(permissions(0o100600), "rw-------");
        assert_eq!(permissions(0), "---------");
    }

    #[test]
    fn sizes_use_powers_of_a_thousand() {
        assert_eq!(size(0), "0 bytes");
        assert_eq!(size(1), "1 byte");
        assert_eq!(size(999), "999 bytes");
        assert_eq!(size(1000), "1.0 kB");
        assert_eq!(size(1536), "1.5 kB");
        assert_eq!(size(250_000), "250 kB");
        assert_eq!(size(4_200_000), "4.2 MB");
        assert_eq!(size(3_500_000_000), "3.5 GB");
        assert_eq!(size(u64::MAX), "18447 PB");
    }
}

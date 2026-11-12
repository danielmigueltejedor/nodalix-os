//! Network counters from `/proc/net/dev`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetTotals {
    pub rx: u64,
    pub tx: u64,
}

/// Sums every interface except loopback. Virtual links such as bridges stay in the total: hiding
/// a real link is worse than double counting a bridge.
pub fn parse_dev(text: &str) -> NetTotals {
    let mut totals = NetTotals::default();
    for line in text.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == "lo" {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 9 {
            continue;
        }
        totals.rx = totals.rx.saturating_add(fields[0].parse().unwrap_or(0));
        totals.tx = totals.tx.saturating_add(fields[8].parse().unwrap_or(0));
    }
    totals
}

pub fn read() -> Option<NetTotals> {
    Some(parse_dev(&std::fs::read_to_string("/proc/net/dev").ok()?))
}

/// Bytes per second between two readings taken `seconds` apart.
pub fn rate(previous: NetTotals, now: NetTotals, seconds: f64) -> (f64, f64) {
    if seconds <= 0.0 {
        return (0.0, 0.0);
    }
    (
        now.rx.saturating_sub(previous.rx) as f64 / seconds,
        now.tx.saturating_sub(previous.tx) as f64 / seconds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV: &str = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    lo: 9999 10 0 0 0 0 0 0 9999 10 0 0 0 0 0 0\n  eth0: 1000 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n wlan0: 2000 20 0 0 0 0 0 0 100 2 0 0 0 0 0 0\n";

    #[test]
    fn sums_interfaces_but_not_loopback() {
        assert_eq!(parse_dev(DEV), NetTotals { rx: 3000, tx: 600 });
    }

    #[test]
    fn ignores_short_lines() {
        assert_eq!(parse_dev("a\nb\n eth0: 1 2 3\n"), NetTotals::default());
    }

    #[test]
    fn rate_is_per_second_and_never_negative() {
        let a = NetTotals { rx: 1000, tx: 100 };
        let b = NetTotals { rx: 3000, tx: 50 };
        assert_eq!(rate(a, b, 2.0), (1000.0, 0.0));
        assert_eq!(rate(a, b, 0.0), (0.0, 0.0));
    }
}

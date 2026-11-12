//! Network counters from `/proc/net/dev`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetTotals {
    pub rx: u64,
    pub tx: u64,
}

use std::collections::HashMap;

pub type Interfaces = HashMap<String, NetTotals>;

/// Reads counters by interface, excluding loopback and retaining virtual links.
pub fn parse_dev(text: &str) -> Interfaces {
    let mut interfaces = Interfaces::new();
    for line in text.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name == "lo" {
            continue;
        }
        let mut fields = rest.split_whitespace();
        let rx = fields.next().and_then(|value| value.parse().ok());
        let tx = fields.nth(7).and_then(|value| value.parse().ok());
        if let (Some(rx), Some(tx)) = (rx, tx) {
            interfaces.insert(name.to_owned(), NetTotals { rx, tx });
        }
    }
    interfaces
}

pub fn read() -> Option<Interfaces> {
    Some(parse_dev(&std::fs::read_to_string("/proc/net/dev").ok()?))
}

/// Bytes per second between two readings taken `seconds` apart.
pub fn rate(previous: &Interfaces, now: &Interfaces, seconds: f64) -> (f64, f64) {
    if seconds <= 0.0 {
        return (0.0, 0.0);
    }
    let mut delta = NetTotals::default();
    for (name, counters) in now {
        if let Some(before) = previous.get(name) {
            delta.rx = delta.rx.saturating_add(counters.rx.saturating_sub(before.rx));
            delta.tx = delta.tx.saturating_add(counters.tx.saturating_sub(before.tx));
        }
    }
    (delta.rx as f64 / seconds, delta.tx as f64 / seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV: &str = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    lo: 9999 10 0 0 0 0 0 0 9999 10 0 0 0 0 0 0\n  eth0: 1000 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n wlan0: 2000 20 0 0 0 0 0 0 100 2 0 0 0 0 0 0\n";

    #[test]
    fn reads_interfaces_but_not_loopback() {
        let interfaces = parse_dev(DEV);
        assert_eq!(interfaces.len(), 2);
        assert_eq!(interfaces["eth0"], NetTotals { rx: 1000, tx: 500 });
        assert_eq!(interfaces["wlan0"], NetTotals { rx: 2000, tx: 100 });
    }

    #[test]
    fn ignores_short_lines() {
        assert!(parse_dev("a\nb\n eth0: 1 2 3\n").is_empty());
    }

    #[test]
    fn rate_uses_matching_interfaces_and_ignores_counter_resets() {
        let before = parse_dev(DEV);
        let after = Interfaces::from([
            ("eth0".into(), NetTotals { rx: 3000, tx: 50 }),
            ("tun0".into(), NetTotals { rx: 9000, tx: 1000 }),
        ]);
        assert_eq!(rate(&before, &after, 2.0), (1000.0, 0.0));
        assert_eq!(rate(&before, &after, 0.0), (0.0, 0.0));
    }
}

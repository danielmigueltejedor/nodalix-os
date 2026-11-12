//! Takes readings on a timer and turns the counters into rates, keeping a short history for the
//! graphs.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use monitor_core::cpu::{self, CpuSample};
use monitor_core::mem::{self, Memory};
use monitor_core::net::{self, Interfaces};
use monitor_core::procs::{self, Process};

/// Samples kept for each graph; one per second.
pub const HISTORY: usize = 60;

pub struct Snapshot {
    pub cpu_total: f64,
    /// Online cores by number, each with its busy fraction.
    pub cores: Vec<(u32, f64)>,
    pub memory: Memory,
    /// Bytes per second, download then upload.
    pub net: (f64, f64),
    /// Every process with its CPU percentage of one core.
    pub processes: Vec<(Process, f64)>,
}

#[derive(Default)]
pub struct Sampler {
    last_at: Option<Instant>,
    cpu: Option<CpuSample>,
    net: Option<Interfaces>,
    /// CPU ticks of the previous reading, by pid and start time: a reused pid is a new process.
    ticks: HashMap<(u32, u64), u64>,
}

/// Appends to a bounded history.
pub fn push(history: &mut VecDeque<f64>, value: f64) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

impl Sampler {
    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let seconds = self
            .last_at
            .map_or(0.0, |t| now.duration_since(t).as_secs_f64());
        self.last_at = Some(now);

        // A failed reading is no baseline either: zeros would make the next good one read as
        // the whole time since boot.
        let cpu_now = cpu::read_sample();
        let (cpu_total, cores) = match (&self.cpu, &cpu_now) {
            (Some(prev), Some(cpu_now)) => (
                cpu::usage(prev.total, cpu_now.total),
                // By core number: when a core goes offline the others keep their own baseline.
                cpu_now
                    .cores
                    .iter()
                    .map(|(id, now)| {
                        let usage = prev
                            .cores
                            .iter()
                            .find(|(pid, _)| pid == id)
                            .map_or(0.0, |(_, before)| cpu::usage(*before, *now));
                        (*id, usage)
                    })
                    .collect(),
            ),
            (None, Some(now)) => (0.0, now.cores.iter().map(|(id, _)| (*id, 0.0)).collect()),
            (_, None) => (0.0, Vec::new()),
        };
        self.cpu = cpu_now;

        // A failed reading is no baseline: storing zeros would make the next good one look like
        // everything since boot arrived in one interval.
        let net_now = net::read();
        let rate = match (&self.net, &net_now) {
            (Some(before), Some(now)) => net::rate(before, now, seconds),
            _ => (0.0, 0.0),
        };
        self.net = net_now;

        let all = procs::read_all();
        let mut ticks = HashMap::with_capacity(all.len());
        let processes = all
            .into_iter()
            .map(|p| {
                ticks.insert((p.pid, p.start_time), p.cpu_ticks);
                // A process we have not seen before has no interval yet: show 0, not its lifetime.
                let percent = self
                    .ticks
                    .get(&(p.pid, p.start_time))
                    .map_or(0.0, |before| {
                        procs::cpu_percent(*before, p.cpu_ticks, seconds)
                    });
                (p, percent)
            })
            .collect();
        self.ticks = ticks;

        Snapshot {
            cpu_total,
            cores,
            memory: mem::read().unwrap_or_default(),
            net: rate,
            processes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_bounded() {
        let mut h = VecDeque::new();
        for i in 0..(HISTORY + 10) {
            push(&mut h, i as f64);
        }
        assert_eq!(h.len(), HISTORY);
        assert_eq!(*h.front().unwrap(), 10.0);
        assert_eq!(*h.back().unwrap(), (HISTORY + 9) as f64);
    }

    #[test]
    fn the_first_sample_has_no_rates_and_the_second_does() {
        let mut s = Sampler::default();
        let first = s.sample();
        assert_eq!(first.cpu_total, 0.0);
        assert_eq!(first.net, (0.0, 0.0));
        assert!(first.processes.iter().all(|(_, pct)| *pct == 0.0));
        assert!(first.memory.total > 0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let second = s.sample();
        assert!((0.0..=1.0).contains(&second.cpu_total));
        assert!(second
            .cores
            .iter()
            .all(|(_, usage)| (0.0..=1.0).contains(usage)));
    }
}

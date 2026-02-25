use std::{fs, time::Duration};

use opentelemetry::metrics::{Meter, ObservableCounter, ObservableGauge};

const SCHEDSTAT_PATH: &str = "/proc/self/schedstat";
const STATUS_PATH: &str = "/proc/self/status";
const BYTES_PER_KIBIBYTE: u64 = 1024;

pub struct Instruments {
    _cpu_time: ObservableCounter<f64>,
    _memory_usage: ObservableGauge<u64>,
    _thread_count: ObservableGauge<u64>,
}

fn cpu_time() -> Option<Duration> {
    let schedstat = fs::read_to_string(SCHEDSTAT_PATH).ok()?;
    let running_ns = schedstat.split_whitespace().next()?.parse().ok()?;
    return Some(Duration::from_nanos(running_ns));
}

fn status_field(status: &str, key: &str) -> Option<u64> {
    let line = status
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))?;
    return line.split_whitespace().next()?.parse().ok();
}

fn resident_bytes() -> Option<u64> {
    let status = fs::read_to_string(STATUS_PATH).ok()?;
    return status_field(&status, "VmRSS").map(|kibibytes| kibibytes * BYTES_PER_KIBIBYTE);
}

fn thread_count() -> Option<u64> {
    let status = fs::read_to_string(STATUS_PATH).ok()?;
    return status_field(&status, "Threads");
}

impl Instruments {
    pub fn register(meter: &Meter) -> Self {
        return Self {
            _cpu_time: meter
                .f64_observable_counter("process.cpu.time")
                .with_unit("s")
                .with_description("CPU time the process spent running.")
                .with_callback(|observer| {
                    if let Some(time) = cpu_time() {
                        observer.observe(time.as_secs_f64(), &[]);
                    }
                })
                .build(),
            _memory_usage: meter
                .u64_observable_gauge("process.memory.usage")
                .with_unit("By")
                .with_description("Resident set size of the process.")
                .with_callback(|observer| {
                    if let Some(bytes) = resident_bytes() {
                        observer.observe(bytes, &[]);
                    }
                })
                .build(),
            _thread_count: meter
                .u64_observable_gauge("process.thread.count")
                .with_unit("{thread}")
                .with_description("Threads of the process.")
                .with_callback(|observer| {
                    if let Some(count) = thread_count() {
                        observer.observe(count, &[]);
                    }
                })
                .build(),
        };
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const STATUS: &str = "Name:\tdemo\nVmRSS:\t    2048 kB\nThreads:\t3\n";

    #[test]
    fn status_field_reads_the_number_after_the_key() {
        assert_eq!(status_field(STATUS, "VmRSS"), Some(2048));
        assert_eq!(status_field(STATUS, "Threads"), Some(3));
        assert_eq!(status_field(STATUS, "VmSwap"), None);
    }

    #[test]
    fn the_running_process_reports_every_value() {
        assert!(cpu_time().is_some());
        assert!(resident_bytes().is_some_and(|bytes| bytes > 0));
        assert!(thread_count().is_some_and(|count| count >= 1));
    }
}

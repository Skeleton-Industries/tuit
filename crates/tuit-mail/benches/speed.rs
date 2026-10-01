//! Speed benchmark. Today it measures one thing: the time to run the `tuit`
//! command from start to exit. That is the whole command, not yet the "first
//! screen" the target is about, because there is no screen yet. To add a
//! measurement, write a function that returns its timings and add a `report`
//! line for it in `main`.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const WARM_UP_RUNS: usize = 10;
const TIMED_RUNS: usize = 100;
const START_UP_TARGET_MS: f64 = 100.0;

/// Times whole runs of the `tuit` command. Panics if any run fails.
fn start_up() -> Vec<Duration> {
    let run = || {
        let started = Instant::now();
        let status = Command::new(env!("CARGO_BIN_EXE_tuit"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("the tuit command failed to start");
        let elapsed = started.elapsed();
        assert!(status.success(), "the tuit command failed: {status}");
        elapsed
    };
    for _ in 0..WARM_UP_RUNS {
        run();
    }
    (0..TIMED_RUNS).map(|_| run()).collect()
}

/// Turns timings into a "median, min, max" line for the named measurement.
fn report(name: &str, mut timings: Vec<Duration>, target_ms: f64) -> String {
    timings.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let n = timings.len();
    let median = (ms(timings[(n - 1) / 2]) + ms(timings[n / 2])) / 2.0;
    let mut line = format!(
        "{name}: median {median:.1} ms, min {:.1} ms, max {:.1} ms over {n} runs (target: under {target_ms:.0} ms)",
        ms(timings[0]),
        ms(timings[n - 1]),
    );
    if median > target_ms {
        line.push_str(" OVER TARGET");
    }
    line
}

fn main() {
    // Arguments are ignored: Cargo passes `--bench`.
    println!("tuit speed (rough: one machine, one moment)");
    println!("{}", report("start-up", start_up(), START_UP_TARGET_MS));
}

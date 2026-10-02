//! Speed benchmark. It measures two things. Start-up is the time to run the
//! `tuit` command from start to exit. That is the whole command, not yet the
//! "first screen" the target is about, because there is no screen yet. Listing
//! is the time to read and sort a Maildir of 10,000 messages. To add a
//! measurement, write a function that returns its timings and add a `report`
//! line for it in `main`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tuit_maildir::Maildir;

const WARM_UP_RUNS: usize = 10;
const TIMED_RUNS: usize = 100;
const START_UP_TARGET_MS: f64 = 100.0;
const LIST_MESSAGES: usize = 10_000;
const LIST_WARM_UP_RUNS: usize = 1;
const LIST_TIMED_RUNS: usize = 10;
const LIST_TARGET_MS: f64 = 200.0;

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

/// One made-up message of about 4 KB of headers and 3 KB of body.
fn made_up_message(n: usize) -> String {
    // A fixed spread of dates over about a year, so the sort has work to do.
    let seconds = 1_700_000_000 + (n * 7_919) % 31_536_000;
    let mut message = String::with_capacity(7_500);
    for hop in 1..=6 {
        write!(
            message,
            "Received: from mail{hop}.example.org (mail{hop}.example.org [192.0.2.{hop}])\n\
             \tby mx{hop}.example.com (Postfix) with ESMTPS id 4F{n:06X}{hop:02}\n\
             \tfor <user@example.com>; Tue, 14 Nov 2023 22:13:{hop:02} +0000 (UTC)\n"
        )
        .unwrap();
    }
    write!(
        message,
        "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.org;\n\
         \ts=selector1; t={seconds}; h=from:to:subject:date:message-id:mime-version;\n\
         \tbh=Zm9vYmFyZm9vYmFyZm9vYmFyZm9vYmFyZm9vYmFyZm9vYmFyZm9vYmFy;\n\
         \tb={sig}\n\
         Authentication-Results: mx.example.com; dkim=pass header.d=example.org;\n\
         \tspf=pass smtp.mailfrom=sender{n}@example.org\n\
         From: Sender Number {n} <sender{n}@example.org>\n\
         To: User <user@example.com>\n\
         Subject: Made-up message number {n}\n\
         Date: {date}\n\
         Message-ID: <{n}.{seconds}@example.org>\n\
         MIME-Version: 1.0\n\
         Content-Type: text/plain; charset=UTF-8\n\
         Content-Transfer-Encoding: 7bit\n",
        sig = "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo".repeat(10),
        date = rfc2822(seconds),
    )
    .unwrap();
    let mut filler = 0;
    while message.len() < 4_000 {
        filler += 1;
        writeln!(
            message,
            "X-Made-Up-{filler}: {}",
            "0123456789abcdef".repeat(4)
        )
        .unwrap();
    }
    message.push('\n');
    let body_start = message.len();
    while message.len() - body_start < 3_000 {
        message.push_str("This is a line of made-up text in the body of a made-up message.\n");
    }
    message
}

/// A date header value for the given seconds, in a fixed UTC form.
fn rfc2822(seconds: usize) -> String {
    const DAY_NAMES: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTH_NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (year, month, day) = tuit_core::Timestamp::from_unix_seconds(seconds as i64).utc_date();
    let day_of_week = DAY_NAMES[(seconds / 86_400) % 7];
    let time = seconds % 86_400;
    format!(
        "{day_of_week}, {day} {} {year} {:02}:{:02}:{:02} +0000",
        MONTH_NAMES[month as usize - 1],
        time / 3_600,
        time % 3_600 / 60,
        time % 60,
    )
}

/// Times listing a Maildir of 10,000 made-up messages. Building the folder is
/// not timed, and the folder is deleted afterwards. Panics if a run fails or
/// finds the wrong number of messages.
fn list() -> Vec<Duration> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bench-maildir");
    let _ = fs::remove_dir_all(&root);
    for folder in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join(folder)).expect("couldn't make the bench folder");
    }
    for n in 0..LIST_MESSAGES {
        let name = format!("{}.M{n}P1.host:2,S", 1_700_000_000 + n);
        fs::write(root.join("cur").join(name), made_up_message(n))
            .expect("couldn't write a bench message");
    }
    let maildir = Maildir::new(&root);
    let run = || {
        let started = Instant::now();
        let messages = tuit_core::list_newest_first(&maildir).expect("listing failed");
        let elapsed = started.elapsed();
        assert_eq!(messages.len(), LIST_MESSAGES);
        elapsed
    };
    for _ in 0..LIST_WARM_UP_RUNS {
        run();
    }
    let timings = (0..LIST_TIMED_RUNS).map(|_| run()).collect();
    let _ = fs::remove_dir_all(&root);
    timings
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
    if median >= target_ms {
        line.push_str(" OVER TARGET");
    }
    line
}

fn main() {
    // Arguments are ignored: Cargo passes `--bench`.
    println!("tuit speed (rough: one machine, one moment)");
    println!("{}", report("start-up", start_up(), START_UP_TARGET_MS));
    println!("{}", report("list 10,000", list(), LIST_TARGET_MS));
}

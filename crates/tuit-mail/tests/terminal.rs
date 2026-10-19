//! Runs the real `tuit` binary in a pseudo-terminal and checks that it takes the terminal over,
//! draws, and puts the terminal back when the user presses `q`. This is the one test that
//! touches a real terminal: the other tests of the list use an in-memory backend.
#![cfg(target_os = "linux")]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{Mode, OFlags, open};
use rustix::io::Errno;
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{LocalModes, Termios, Winsize, tcgetattr, tcsetwinsize};

/// How long any one wait may take. A hung child fails the test instead of hanging CI.
const DEADLINE: Duration = Duration::from_secs(10);

/// `ESC [ ? 1049 h` and `ESC [ ? 1049 l`: enter and leave the alternate screen. These are the
/// strings crossterm's `EnterAlternateScreen` and `LeaveAlternateScreen` write.
const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
const LEAVE_ALT_SCREEN: &[u8] = b"\x1b[?1049l";
/// `ESC [ ? 25 l` and `ESC [ ? 25 h`: hide and show the cursor.
const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";

/// What one try at reading the parent side found.
enum Got {
    /// Some bytes, now in the buffer.
    Data,
    /// Nothing arrived in time.
    Nothing,
    /// The child side is closed (end of file, or EIO, which is how Linux says so).
    Closed,
}

/// Reads from the parent side of the pty into `out`, waiting up to `wait` for something to
/// arrive.
fn read_some(parent: &mut File, out: &mut Vec<u8>, wait: Duration) -> Got {
    let timeout = Timespec {
        tv_sec: wait.as_secs() as _,
        tv_nsec: wait.subsec_nanos() as _,
    };
    loop {
        let mut fds = [PollFd::new(&*parent, PollFlags::IN)];
        match poll(&mut fds, Some(&timeout)) {
            Ok(0) => return Got::Nothing,
            Ok(_) => break,
            // A signal interrupted the wait. Try again.
            Err(Errno::INTR) => {}
            Err(e) => panic!("poll on the pty failed: {e}"),
        }
    }
    let mut buf = [0u8; 4096];
    match parent.read(&mut buf) {
        Ok(0) => Got::Closed,
        Ok(n) => {
            out.extend_from_slice(&buf[..n]);
            Got::Data
        }
        Err(e) if e.raw_os_error() == Some(Errno::IO.raw_os_error()) => Got::Closed,
        Err(e) => panic!("read from the pty failed: {e}"),
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

/// Owns the child, and on drop (including when an assertion fails) kills it and waits for it,
/// so a failing test leaves no process behind.
struct Tuit(Child);

impl Drop for Tuit {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Waits for the child to exit, reading the pty meanwhile so the child never blocks on a full
/// buffer.
fn wait_for_exit(child: &mut Tuit, parent: &mut File, out: &mut Vec<u8>) -> ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().expect("could not wait for tuit") {
            return status;
        }
        assert!(
            start.elapsed() < DEADLINE,
            "tuit did not exit within {DEADLINE:?} of `q`"
        );
        read_some(parent, out, Duration::from_millis(50));
    }
}

/// Reads whatever is still waiting after the child has exited, until a short wait finds nothing.
fn drain(parent: &mut File, out: &mut Vec<u8>) {
    let start = Instant::now();
    while let Got::Data = read_some(parent, out, Duration::from_millis(100)) {
        assert!(
            start.elapsed() < DEADLINE,
            "the pty never stopped giving output"
        );
    }
}

fn modes(fd: &OwnedFd) -> Termios {
    tcgetattr(fd).expect("could not read the terminal mode")
}

#[test]
fn takes_over_the_terminal_and_puts_it_back_on_q() {
    // 1. Open a pty pair, size it, and record its mode before tuit runs.
    let parent_fd = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).expect("openpt");
    grantpt(&parent_fd).expect("grantpt");
    unlockpt(&parent_fd).expect("unlockpt");
    let name = ptsname(&parent_fd, Vec::new()).expect("ptsname");
    let child_fd = open(&name, OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).expect("open child");
    tcsetwinsize(
        &child_fd,
        Winsize {
            ws_row: 40,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        },
    )
    .expect("set the window size");
    let before = modes(&child_fd);
    assert!(
        before
            .local_modes
            .contains(LocalModes::ICANON | LocalModes::ECHO),
        "a fresh pty should start in canonical mode with echo"
    );

    // 2. Start tuit with all three standard streams on the child side.
    //
    // `setsid --ctty` (from util-linux) starts tuit in a new session with its standard input,
    // this pty, as the controlling terminal. We can't do that ourselves without `unsafe`
    // (`pre_exec`). It matters: crossterm measures the window by opening /dev/tty, which is
    // the controlling terminal, so without it tuit would measure the developer's own terminal
    // and ignore the size set above. `setsid` does not fork here, because the child is not a
    // process group leader, so `Tuit` holds tuit itself.
    let stdio = |fd: &OwnedFd| Stdio::from(fd.try_clone().expect("dup the child side"));
    let mut child = Tuit(
        Command::new("setsid")
            .arg("--ctty")
            .arg(env!("CARGO_BIN_EXE_tuit"))
            .arg("--sample")
            .env("TERM", "xterm-256color")
            .stdin(stdio(&child_fd))
            .stdout(stdio(&child_fd))
            .stderr(stdio(&child_fd))
            .spawn()
            .expect("could not start tuit with setsid"),
    );
    let mut parent = File::from(parent_fd);
    let mut out = Vec::new();

    // 3. Read until the first screen has been drawn: a subject from the sample, which only
    // appears once the list is drawn.
    let start = Instant::now();
    while !contains(&out, b"Lunch on Thursday?") {
        if start.elapsed() >= DEADLINE {
            panic!(
                "no first screen within {DEADLINE:?}; got {:?}",
                String::from_utf8_lossy(&out)
            );
        }
        read_some(&mut parent, &mut out, Duration::from_millis(50));
    }
    assert!(
        contains(&out, ENTER_ALT_SCREEN),
        "tuit drew without entering the alternate screen"
    );
    assert!(
        !modes(&child_fd)
            .local_modes
            .intersects(LocalModes::ICANON | LocalModes::ECHO),
        "while running, tuit should have the terminal in raw mode"
    );

    // 4. Press `q`, and wait for a clean exit.
    parent.write_all(b"q").expect("could not write q");
    let status = wait_for_exit(&mut child, &mut parent, &mut out);
    assert!(status.success(), "tuit exited with {status}");
    // Collect everything it wrote, including the last bytes before it exited.
    drain(&mut parent, &mut out);

    // 5. The terminal is back: the same mode as before, and the alternate screen left.
    let after = modes(&child_fd);
    assert!(
        after
            .local_modes
            .contains(LocalModes::ICANON | LocalModes::ECHO),
        "canonical mode and echo should be back on, got {:?}",
        after.local_modes
    );
    assert_eq!(after.local_modes, before.local_modes, "local modes differ");
    assert_eq!(after.input_modes, before.input_modes, "input modes differ");
    assert_eq!(
        after.output_modes, before.output_modes,
        "output modes differ"
    );
    assert_eq!(
        after.control_modes, before.control_modes,
        "control modes differ"
    );
    // `SpecialCodes` has no `PartialEq`, but its `Debug` lists every code.
    assert_eq!(
        format!("{:?}", after.special_codes),
        format!("{:?}", before.special_codes),
        "special characters differ"
    );
    let entered = rfind(&out, ENTER_ALT_SCREEN).expect("never entered the alternate screen");
    let left = rfind(&out, LEAVE_ALT_SCREEN).expect("never left the alternate screen");
    assert!(
        left > entered,
        "the last alternate-screen switch was entering"
    );

    // The cursor is shown again after the last time it was hidden.
    let shown = rfind(&out, SHOW_CURSOR).expect("never showed the cursor");
    assert!(
        rfind(&out, HIDE_CURSOR).is_none_or(|hidden| shown > hidden),
        "the cursor is still hidden"
    );
}

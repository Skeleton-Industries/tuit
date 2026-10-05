use std::process::{Command, Stdio};

fn tuit(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tuit"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run the tuit binary")
}

#[test]
fn prints_its_version() {
    let expected = format!("tuit {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        let output = tuit(&[flag]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }
}

#[test]
fn an_unknown_argument_is_a_one_line_error() {
    for args in [&["--frobnicate"][..], &["--version", "extra"], &["list"]] {
        let output = tuit(args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.starts_with("tuit: "));
    }
}

#[test]
fn an_argument_after_the_version_flag_is_the_one_blamed() {
    for flag in ["--version", "-V"] {
        let output = tuit(&[flag, "extra"]);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.trim_end(), "tuit: unexpected argument 'extra'");
    }
}

#[cfg(unix)]
#[test]
fn an_argument_that_is_not_utf8_is_an_error_and_not_a_panic() {
    use std::os::unix::ffi::OsStrExt;

    let output = Command::new(env!("CARGO_BIN_EXE_tuit"))
        .arg(std::ffi::OsStr::from_bytes(b"\xff"))
        .stdin(Stdio::null())
        .output()
        .expect("failed to run the tuit binary");
    // A panic exits with 101 and prints several lines; the error path exits with 1 and prints one.
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("tuit: unrecognised argument"));
}

#[test]
fn without_a_terminal_it_says_so_and_does_not_hang() {
    // Standard input is null and the output is a pipe, so neither is a terminal.
    let output = tuit(&[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "nothing is drawn to a pipe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim_end(), "tuit: needs a terminal to draw on");
}

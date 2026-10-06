use std::fs;
use std::path::Path;
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
    for args in [
        &["--frobnicate"][..],
        &["--version", "extra"],
        &["one", "two"],
    ] {
        let output = tuit(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
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
fn a_path_that_is_not_utf8_is_an_error_and_not_a_panic() {
    use std::os::unix::ffi::OsStrExt;

    let output = Command::new(env!("CARGO_BIN_EXE_tuit"))
        .arg(std::ffi::OsStr::from_bytes(b"no-such-\xff"))
        .stdin(Stdio::null())
        .output()
        .expect("failed to run the tuit binary");
    // A panic exits with 101 and prints several lines; the error path exits with 1 and prints one.
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("tuit: "));
}

#[test]
fn without_a_path_it_prints_usage_and_does_not_open_the_screen() {
    let output = tuit(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("usage: tuit <maildir>"), "{stderr}");
    assert!(stderr.contains("--sample"), "{stderr}");
    assert!(!stderr.contains("needs a terminal"), "{stderr}");
}

#[test]
fn a_missing_folder_is_a_one_line_error_naming_it() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-maildir");
    let output = tuit(&[path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("tuit: "), "{stderr}");
    assert!(stderr.contains("no-such-maildir"), "{stderr}");
    assert!(!stderr.contains("needs a terminal"), "{stderr}");
}

#[test]
fn a_readable_maildir_gets_as_far_as_needing_a_terminal() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("maildir-reads-first");
    let _ = fs::remove_dir_all(&root);
    for folder in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    fs::write(
        root.join("cur").join("1.M1P1.example"),
        "From: Alice Example <alice@example.org>\nSubject: Hello\nDate: Tue, 14 Nov 2023 22:13:20 +0000\n\nBody.\n",
    )
    .unwrap();
    fs::write(
        root.join("new").join("2.M2P2.example"),
        "From: Bob Example <bob@example.org>\nSubject: Again\nDate: Wed, 15 Nov 2023 08:00:00 +0000\n\nBody.\n",
    )
    .unwrap();
    let output = tuit(&[root.to_str().unwrap()]);
    let _ = fs::remove_dir_all(&root);
    // The folder was read without error, so the only thing left to fail is the missing terminal.
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim_end(), "tuit: needs a terminal to draw on");
}

#[test]
fn sample_without_a_terminal_says_so_and_does_not_hang() {
    // Standard input is null and the output is a pipe, so neither is a terminal.
    let output = tuit(&["--sample"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "nothing is drawn to a pipe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim_end(), "tuit: needs a terminal to draw on");
}

#[test]
fn help_prints_the_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = tuit(&[flag]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.starts_with("usage: tuit <maildir>"), "{stdout}");
        assert!(stdout.contains("--sample") && stdout.contains("--version"));
    }
}

#[test]
fn a_path_with_a_newline_in_it_is_still_a_one_line_error() {
    let output = tuit(&["no-such\nfolder"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("no-such?folder"), "{stderr}");
}

use std::process::Command;

#[test]
fn prints_its_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_tuit"))
        .output()
        .expect("failed to run the tuit binary");

    assert!(output.status.success());
    let expected = format!("tuit {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

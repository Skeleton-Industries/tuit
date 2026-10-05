mod sample;

use std::process::ExitCode;

fn main() -> ExitCode {
    // `args_os`, not `args`, which panics on an argument that isn't valid UTF-8. Bytes that
    // aren't text are swapped for the replacement character, which is only ever shown back.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [] => match tuit_tui::run(sample::messages()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&error.to_string()),
        },
        ["--version" | "-V"] => {
            println!("tuit {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--version" | "-V", extra, ..] => fail(&format!("unexpected argument '{extra}'")),
        [first, ..] => fail(&format!("unrecognised argument '{first}' (try --version)")),
    }
}

/// Prints a one-line error to stderr.
fn fail(message: &str) -> ExitCode {
    eprintln!("tuit: {message}");
    ExitCode::FAILURE
}

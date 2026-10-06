mod sample;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use tuit_core::{MessageSummary, StoreError, Untrusted, list_newest_first};
use tuit_maildir::Maildir;

const USAGE: &str = "\
usage: tuit <maildir>    show the Maildir at that path
       tuit --sample     show made-up messages
       tuit --version    print the version
       tuit --help       print this";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Request {
    /// Show these messages in the list.
    Show(Source),
    /// Print the version.
    Version,
    /// Print the usage, because it was asked for.
    Help,
    /// Nothing was asked for: print the usage as an error.
    Usage,
}

/// Where the messages to show come from.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    /// The Maildir at this path.
    Maildir(PathBuf),
    /// The made-up messages built into the program.
    Sample,
}

/// Reads the arguments after the program name. An error is the one-line message to print.
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Request, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Request::Usage);
    };
    let request = if first == "--version" || first == "-V" {
        Request::Version
    } else if first == "--help" || first == "-h" {
        Request::Help
    } else if first == "--sample" {
        Request::Show(Source::Sample)
    } else if first.as_encoded_bytes().starts_with(b"-") {
        // Lossy text is only for showing an argument back.
        return Err(format!(
            "unrecognised option '{}' (try --help)",
            first.to_string_lossy()
        ));
    } else if first.is_empty() {
        // An empty path would quietly mean the current folder.
        return Err("the Maildir path is empty".to_string());
    } else {
        Request::Show(Source::Maildir(PathBuf::from(first)))
    };
    match args.next() {
        Some(extra) => Err(format!("unexpected argument '{}'", extra.to_string_lossy())),
        None => Ok(request),
    }
}

/// The messages from `source`, in the order the list shows them: newest first.
fn load(source: Source) -> Result<Vec<MessageSummary>, StoreError> {
    match source {
        Source::Maildir(path) => list_newest_first(&Maildir::new(path)),
        Source::Sample => Ok(sample::messages()),
    }
}

fn main() -> ExitCode {
    // `args_os`, not `args`, which panics on an argument that isn't valid UTF-8.
    let source = match parse(std::env::args_os().skip(1)) {
        Ok(Request::Show(source)) => source,
        Ok(Request::Version) => {
            println!("tuit {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Request::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Request::Usage) => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
        Err(message) => return fail(&message),
    };
    // The mail is read before the screen opens, so an error prints to a normal terminal.
    let shown = load(source)
        .map_err(|error| error.to_string())
        .and_then(|messages| tuit_tui::run(messages).map_err(|error| error.to_string()));
    match shown {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => fail(&message),
    }
}

/// Prints a one-line error to stderr. The message can carry a path or an argument as typed, so
/// control characters in it, a newline or an escape among them, are replaced by the one rule
/// for untrusted text.
fn fail(message: &str) -> ExitCode {
    eprintln!("tuit: {}", Untrusted::new(message).terminal_line());
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Request, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_asks_for_usage() {
        assert_eq!(parse_strs(&[]), Ok(Request::Usage));
    }

    #[test]
    fn a_path_asks_for_that_maildir() {
        assert_eq!(
            parse_strs(&["Mail/inbox"]),
            Ok(Request::Show(Source::Maildir(PathBuf::from("Mail/inbox"))))
        );
    }

    #[test]
    fn sample_asks_for_the_made_up_messages() {
        assert_eq!(parse_strs(&["--sample"]), Ok(Request::Show(Source::Sample)));
    }

    #[test]
    fn both_version_flags_ask_for_the_version() {
        assert_eq!(parse_strs(&["--version"]), Ok(Request::Version));
        assert_eq!(parse_strs(&["-V"]), Ok(Request::Version));
    }

    #[test]
    fn an_unknown_flag_is_an_error_that_names_it() {
        let error = parse_strs(&["--frobnicate"]).unwrap_err();
        assert!(error.contains("'--frobnicate'"), "{error}");
        assert!(parse_strs(&["-x"]).is_err());
    }

    #[test]
    fn a_second_path_is_an_error_that_names_it() {
        assert_eq!(
            parse_strs(&["one", "two"]),
            Err("unexpected argument 'two'".to_string())
        );
    }

    #[test]
    fn anything_after_a_flag_is_an_error_that_names_it() {
        for flag in ["--version", "-V", "--help", "-h", "--sample"] {
            assert_eq!(
                parse_strs(&[flag, "extra"]),
                Err("unexpected argument 'extra'".to_string()),
                "{flag}"
            );
        }
    }

    #[test]
    fn a_flag_after_a_path_is_an_unexpected_argument() {
        assert_eq!(
            parse_strs(&["inbox", "--version"]),
            Err("unexpected argument '--version'".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_utf8_is_kept_byte_for_byte() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = OsStr::from_bytes(b"Mail/caf\xe9");
        assert_eq!(
            parse([path.to_os_string()]),
            Ok(Request::Show(Source::Maildir(PathBuf::from(path))))
        );
    }

    #[test]
    fn both_help_flags_ask_for_help() {
        assert_eq!(parse_strs(&["--help"]), Ok(Request::Help));
        assert_eq!(parse_strs(&["-h"]), Ok(Request::Help));
    }

    #[test]
    fn an_empty_path_is_an_error() {
        assert_eq!(
            parse_strs(&[""]),
            Err("the Maildir path is empty".to_string())
        );
    }

    #[test]
    fn a_maildir_is_loaded_newest_first() {
        let root = std::env::temp_dir().join(format!("tuit-load-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for folder in ["cur", "new", "tmp"] {
            std::fs::create_dir_all(root.join(folder)).unwrap();
        }
        // File names sort the opposite way to the dates, and the newest is in `cur`.
        for (file, day, subject) in [
            ("new/1.older", "01", "Older"),
            ("new/2.oldest", "00", "Oldest"),
            ("cur/3.newest:2,S", "02", "Newest"),
        ] {
            let day = if day == "00" {
                "30 Sep"
            } else {
                &format!("{day} Oct")
            };
            let text = format!(
                "From: Made Up <made.up@example.org>\r\nSubject: {subject}\r\n\
                 Date: {day} 2026 09:30:00 +0000\r\n\r\nInvented.\r\n"
            );
            std::fs::write(root.join(file), text).unwrap();
        }
        let loaded = load(Source::Maildir(root.clone()));
        std::fs::remove_dir_all(&root).unwrap();
        let subjects: Vec<String> = loaded.unwrap().into_iter().map(|m| m.subject).collect();
        assert_eq!(subjects, ["Newest", "Older", "Oldest"]);
    }

    #[test]
    fn the_sample_source_loads_the_made_up_messages() {
        assert_eq!(load(Source::Sample).unwrap(), sample::messages());
        assert!(!sample::messages().is_empty());
    }
}

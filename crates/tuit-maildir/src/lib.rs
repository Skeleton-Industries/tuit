//! Reads a Maildir folder from disk.
//! Of our crates, it may depend only on `tuit-core`. File access for mail
//! belongs here.

use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use mail_parser::{HeaderName, MessageParser};
use tuit_core::{MailStore, MessageId, MessageSummary, StoreError, Timestamp, Untrusted};

/// How much of a file is read, at most, when looking for the end of its headers.
/// A safety limit, set by judgement and not by measurement: real headers are
/// far smaller, and a file with no blank line would otherwise be read to its
/// end.
const MAX_HEADER_BYTES: u64 = 1024 * 1024;

/// The parser, built once and shared. It decodes only the three headers a
/// summary is made from, and passes over the rest without decoding them. A
/// summary that reads a fourth header needs it added here too, or it comes back
/// empty; the test `the_parser_gives_what_the_full_parser_gives` catches that for
/// the common headers it includes.
static PARSER: LazyLock<MessageParser> = LazyLock::new(|| {
    MessageParser::new()
        .header_date(HeaderName::Date)
        .header_text(HeaderName::Subject)
        .header_address(HeaderName::From)
        .default_header_ignore()
});

/// One Maildir folder on disk.
#[derive(Debug, Clone)]
pub struct Maildir {
    path: PathBuf,
}

impl Maildir {
    /// Doesn't touch the disk; a missing folder is reported by `list`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl MailStore for Maildir {
    fn list(&self) -> Result<Vec<MessageSummary>, StoreError> {
        let mut messages = Vec::new();
        // `new` before `cur`. Mail programs move messages from `new` to `cur`.
        // One moved between the two reads then shows twice, not never. Other
        // renames while we read can still hide a message or repeat it.
        for folder in ["new", "cur"] {
            read_folder(&self.path, folder, &mut messages)?;
        }
        Ok(messages)
    }
}

/// Adds a summary for every message file directly inside the `name` folder of
/// the Maildir at `root`.
fn read_folder(
    root: &Path,
    name: &str,
    messages: &mut Vec<MessageSummary>,
) -> Result<(), StoreError> {
    let folder = root.join(name);
    let entries = fs::read_dir(&folder).map_err(|e| open_error(root, &folder, name, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_error(&folder, &e))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let headers = match read_message_headers(&entry, &path) {
            Ok(Some(headers)) => headers,
            // Gone, or not a file: not a message.
            Ok(None) => continue,
            Err(e) => return Err(io_error(&path, &e)),
        };
        let id = file_name.split(':').next().unwrap_or_default();
        messages.push(summarise(MessageId::new(id), &headers));
    }
    Ok(())
}

/// The error for a Maildir folder that couldn't be opened. When it is missing
/// or isn't a folder, looks at what is there to say why, and names the path the
/// user gave. The look happens only after the failure, so it can't change what
/// gets read. Like every `StoreError`, the text is cleaned when shown.
fn open_error(root: &Path, folder: &Path, name: &str, error: &io::Error) -> StoreError {
    use io::ErrorKind::{NotADirectory, NotFound};
    let path = root.display();
    if !matches!(error.kind(), NotFound | NotADirectory) {
        return io_error(folder, error);
    }
    match fs::metadata(root) {
        // `metadata` follows links, so a link to nothing looks missing. Ask about the link.
        Err(e) if e.kind() == NotFound && fs::symlink_metadata(root).is_ok() => {
            StoreError::new(format!("{path} is a link to something that doesn't exist"))
        }
        Err(e) if e.kind() == NotFound => StoreError::new(format!("{path} doesn't exist")),
        Ok(meta) if !meta.is_dir() => {
            StoreError::new(format!("{path} isn't a Maildir: it isn't a folder"))
        }
        // The folder inside can be a link to nothing too, and then `ls` shows it.
        Ok(_) if error.kind() == NotFound && fs::symlink_metadata(folder).is_ok() => {
            StoreError::new(format!(
                "{path} isn't a Maildir: \"{name}\" is a link to something that doesn't exist"
            ))
        }
        Ok(_) if error.kind() == NotFound => StoreError::new(format!(
            "{path} isn't a Maildir: it has no \"{name}\" folder"
        )),
        Ok(_) => StoreError::new(format!("{path} isn't a Maildir: \"{name}\" isn't a folder")),
        // Couldn't look at the path itself, as with `file/`: say what failed there.
        Err(e) => io_error(root, &e),
    }
}

fn io_error(path: &Path, error: &io::Error) -> StoreError {
    StoreError::new(format!("can't read {}: {error}", path.display()))
}

/// The headers of a folder entry, or `None` if it isn't a message: it has
/// gone, or it isn't a regular file or a link to one.
fn read_message_headers(entry: &DirEntry, path: &Path) -> io::Result<Option<Vec<u8>>> {
    match is_regular_file(entry, path) {
        Ok(true) => read_headers(path),
        Ok(false) => Ok(None),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Whether a folder entry is a regular file, or a link to one. This is asked
/// before the file is opened, because opening a named pipe waits until
/// something writes to it, which may be never.
fn is_regular_file(entry: &DirEntry, path: &Path) -> io::Result<bool> {
    let file_type = entry.file_type()?;
    if file_type.is_symlink() {
        // `fs::metadata` follows the link; the entry's own type doesn't.
        return Ok(fs::metadata(path)?.is_file());
    }
    Ok(file_type.is_file())
}

/// The header block of the file at `path`, without the blank line that ends
/// it. `None` if the file has gone.
///
/// Reads line by line and stops at the first empty line. The file is read in
/// chunks of 8 KB, so a large body is never read, and a small message is read
/// whole. If there's no empty line in the first `MAX_HEADER_BYTES`, whatever was
/// read counts as the headers.
fn read_headers(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut reader = BufReader::new(file.take(MAX_HEADER_BYTES));
    let mut headers = Vec::new();
    loop {
        let line_start = headers.len();
        if reader.read_until(b'\n', &mut headers)? == 0 {
            break;
        }
        let line = headers.get(line_start..).unwrap_or_default();
        if line == b"\n" || line == b"\r\n" {
            headers.truncate(line_start);
            break;
        }
    }
    // The parser ignores a last header that has no line ending, which is what a
    // file with no body, or one cut at the limit, ends with.
    if headers.last().is_some_and(|&byte| byte != b'\n') {
        headers.push(b'\n');
    }
    Ok(Some(headers))
}

/// Picks the summary fields out of a header block. Anything that can't be read
/// is left empty.
fn summarise(id: MessageId, headers: &[u8]) -> MessageSummary {
    summarise_with(&PARSER, id, headers)
}

/// `summarise`, with the parser given, so a test can compare parsers.
fn summarise_with(parser: &MessageParser, id: MessageId, headers: &[u8]) -> MessageSummary {
    let mut summary = MessageSummary {
        id,
        from: Untrusted::default(),
        subject: Untrusted::default(),
        date: None,
    };
    let Some(message) = parser.parse_headers(headers) else {
        return summary;
    };
    if let Some(sender) = message.from().and_then(|from| from.first()) {
        let name = sender.name().filter(|n| !n.trim().is_empty());
        let address = sender.address().filter(|a| !a.trim().is_empty());
        summary.from = Untrusted::new(name.or(address).unwrap_or_default());
    }
    summary.subject = Untrusted::new(message.subject().unwrap_or_default());
    summary.date = message
        .date()
        .filter(|date| date.is_valid())
        .map(|date| Timestamp::from_unix_seconds(date.to_timestamp()));
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Header lines, some well formed and some not, to be put together in every
    /// order. Several leave a quote, comment or bracket open, to test where a
    /// header the parser passes over is taken to end. The well-formed headers
    /// that aren't shown are there so that a summary reading one fails the test.
    const PIECES: [&str; 18] = [
        "From: Ada Example <ada@example.org>\n",
        "Subject: =?UTF-8?B?w6l0w6k=?= plans\n",
        "Date: Fri, 2 Oct 2026 09:30:00 +0100\n",
        "FROM: \"Bo, Sample\" <bo@example.org>\n",
        "subject: folded\n over two lines\n",
        "To: \"unclosed <to@example.org>\n",
        "Content-Type: text/plain; charset=\"utf-8\n",
        "Received: from x (comment (nested\n",
        "Message-ID: <unclosed@example.org\n",
        "From-Foo: not a sender\n",
        "Reply-To: reply@example.org\n",
        "Message-ID: <id@example.org>\n",
        "To: Cy <cy@example.org>\n",
        "Cc: Di <di@example.org>\n",
        "Sender: Ed <ed@example.org>\n",
        "In-Reply-To: <earlier@example.org>\n",
        "References: <first@example.org> <earlier@example.org>\n",
        "a line with no colon\n",
    ];

    #[test]
    fn the_parser_gives_what_the_full_parser_gives() {
        let full = MessageParser::default();
        for a in PIECES {
            for b in PIECES {
                for c in PIECES {
                    let headers = format!("{a}{b}{c}\n");
                    let id = || MessageId::new("x");
                    assert_eq!(
                        summarise_with(&PARSER, id(), headers.as_bytes()),
                        summarise_with(&full, id(), headers.as_bytes()),
                        "{headers}"
                    );
                }
            }
        }
    }
}

//! Reads a Maildir folder from disk.
//! Of our crates, it may depend only on `tuit-core`. File access for mail
//! belongs here.

use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use mail_parser::MessageParser;
use tuit_core::{MailStore, MessageId, MessageSummary, StoreError, Timestamp};

/// How much of a file is read, at most, when looking for the end of its headers.
const MAX_HEADER_BYTES: u64 = 1024 * 1024;

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
            read_folder(&self.path.join(folder), &mut messages)?;
        }
        Ok(messages)
    }
}

/// Adds a summary for every message file directly inside `folder`.
fn read_folder(folder: &Path, messages: &mut Vec<MessageSummary>) -> Result<(), StoreError> {
    let entries = fs::read_dir(folder).map_err(|e| io_error(folder, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_error(folder, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let headers = match read_message_headers(&entry, &path) {
            Ok(Some(headers)) => headers,
            // Gone, or not a file: not a message.
            Ok(None) => continue,
            Err(e) => return Err(io_error(&path, &e)),
        };
        let id = name.split(':').next().unwrap_or_default();
        messages.push(summarise(MessageId::new(id), &headers));
    }
    Ok(())
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
    let mut summary = MessageSummary {
        id,
        from: String::new(),
        subject: String::new(),
        date: None,
    };
    let Some(message) = MessageParser::default().parse_headers(headers) else {
        return summary;
    };
    if let Some(sender) = message.from().and_then(|from| from.first()) {
        let name = sender.name().filter(|n| !n.trim().is_empty());
        let address = sender.address().filter(|a| !a.trim().is_empty());
        summary.from = name.or(address).unwrap_or_default().to_owned();
    }
    summary.subject = message.subject().unwrap_or_default().to_owned();
    summary.date = message
        .date()
        .filter(|date| date.is_valid())
        .map(|date| Timestamp::from_unix_seconds(date.to_timestamp()));
    summary
}

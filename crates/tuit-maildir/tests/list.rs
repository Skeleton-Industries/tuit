//! Builds small Maildir folders at run time and lists them.

use std::fs;
use std::path::{Path, PathBuf};

use tuit_core::{MailStore, MessageSummary, list_newest_first};
use tuit_maildir::Maildir;

/// A Maildir under Cargo's test directory, removed when dropped.
struct TestMaildir {
    root: PathBuf,
}

impl TestMaildir {
    /// `name` must be different in every test, so tests don't share a folder.
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("maildir-{name}"));
        let _ = fs::remove_dir_all(&root);
        for folder in ["cur", "new", "tmp"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        Self { root }
    }

    fn add(&self, folder: &str, file_name: &str, contents: impl AsRef<[u8]>) {
        fs::write(self.root.join(folder).join(file_name), contents).unwrap();
    }

    fn list(&self) -> Vec<MessageSummary> {
        list_newest_first(&Maildir::new(&self.root)).unwrap()
    }
}

impl Drop for TestMaildir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn the_one_message(mut messages: Vec<MessageSummary>) -> MessageSummary {
    assert_eq!(messages.len(), 1, "{messages:?}");
    messages.remove(0)
}

fn unix(summary: &MessageSummary) -> Option<i64> {
    summary.date.map(|d| d.unix_seconds())
}

/// The date in `PLAIN`, and in most other test messages, as seconds since
/// 1970: Tuesday 14 November 2023 at 22:13:20 UTC. Chosen for being round.
const PLAIN_DATE: i64 = 1_700_000_000;

const PLAIN: &str = "From: Alice Example <alice@example.com>\n\
Subject: Hello there\n\
Date: Tue, 14 Nov 2023 22:13:20 +0000\n\
\n\
Body text.\n";

#[test]
fn a_plain_message() {
    let maildir = TestMaildir::new("plain");
    maildir.add("cur", "1.M1P1.host", PLAIN);
    let m = the_one_message(maildir.list());
    assert_eq!(m.id.as_str(), "1.M1P1.host");
    assert_eq!(m.from, "Alice Example");
    assert_eq!(m.subject, "Hello there");
    assert_eq!(unix(&m), Some(PLAIN_DATE));
}

#[test]
fn a_date_with_an_offset_is_converted_to_utc() {
    let maildir = TestMaildir::new("offset");
    maildir.add(
        "cur",
        "a",
        "From: a@example.com\nDate: Wed, 15 Nov 2023 00:13:20 +0200\n\n",
    );
    assert_eq!(unix(&the_one_message(maildir.list())), Some(PLAIN_DATE));
}

#[test]
fn encoded_words_are_decoded() {
    let maildir = TestMaildir::new("encoded");
    // "Café ☕" and "Zoë Example", in base64 and quoted-printable.
    maildir.add(
        "cur",
        "a",
        "From: =?UTF-8?Q?Zo=C3=AB_Example?= <zoe@example.org>\n\
         Subject: =?UTF-8?B?Q2Fmw6kg4piV?=\n\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "Zoë Example");
    assert_eq!(m.subject, "Café ☕");
}

#[test]
fn an_address_with_no_name() {
    let maildir = TestMaildir::new("no-name");
    maildir.add("cur", "a", "From: bob@example.org\nSubject: Hi\n\n");
    maildir.add("cur", "b", "From: <carol@example.org>\nSubject: Hi\n\n");
    let messages = maildir.list();
    let froms: Vec<_> = messages.iter().map(|m| m.from.as_str()).collect();
    assert_eq!(froms, ["bob@example.org", "carol@example.org"]);
}

#[test]
fn no_from_and_no_subject() {
    let maildir = TestMaildir::new("no-subject");
    maildir.add("cur", "a", "Date: Tue, 14 Nov 2023 22:13:20 +0000\n\n");
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "");
    assert_eq!(m.subject, "");
}

#[test]
fn no_date_and_a_garbage_date() {
    let maildir = TestMaildir::new("dates");
    maildir.add("cur", "none", "Subject: a\n\n");
    maildir.add(
        "cur",
        "garbage",
        "Subject: b\nDate: last Tuesday, probably\n\n",
    );
    maildir.add(
        "cur",
        "dated",
        "Subject: c\nDate: Tue, 14 Nov 2023 22:13:20 +0000\n\n",
    );
    let messages = maildir.list();
    let ids: Vec<_> = messages.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["dated", "garbage", "none"]);
    assert_eq!(unix(&messages[1]), None);
    assert_eq!(unix(&messages[2]), None);
}

#[test]
fn messages_in_new_and_cur_are_both_listed() {
    let maildir = TestMaildir::new("new-and-cur");
    maildir.add(
        "new",
        "fresh",
        "Subject: new\nDate: Tue, 14 Nov 2023 22:13:20 +0000\n\n",
    );
    maildir.add(
        "cur",
        "old:2,S",
        "Subject: old\nDate: Mon, 13 Nov 2023 22:13:20 +0000\n\n",
    );
    let messages = maildir.list();
    let subjects: Vec<_> = messages.iter().map(|m| m.subject.as_str()).collect();
    assert_eq!(subjects, ["new", "old"]);
}

#[test]
fn tmp_dot_files_and_subfolders_are_ignored() {
    let maildir = TestMaildir::new("ignored");
    maildir.add("tmp", "half-delivered", PLAIN);
    maildir.add("cur", ".hidden", PLAIN);
    maildir.add("new", ".another", PLAIN);
    fs::create_dir(maildir.root.join("cur").join("subfolder")).unwrap();
    maildir.add("cur", "real", PLAIN);
    assert_eq!(the_one_message(maildir.list()).id.as_str(), "real");
}

#[test]
fn the_id_is_cut_at_the_first_colon() {
    let maildir = TestMaildir::new("id");
    maildir.add("cur", "1700000000.M1P1.host:2,S", PLAIN);
    maildir.add("new", "1700000001.M2P1.host", PLAIN);
    maildir.add("cur", "odd:name:2,S", PLAIN);
    let mut ids: Vec<_> = maildir
        .list()
        .iter()
        .map(|m| m.id.as_str().to_owned())
        .collect();
    ids.sort();
    assert_eq!(ids, ["1700000000.M1P1.host", "1700000001.M2P1.host", "odd"]);
}

#[test]
fn headers_only_with_no_body_and_no_blank_line() {
    let maildir = TestMaildir::new("no-blank-line");
    maildir.add("cur", "a", "From: dave@example.com\nSubject: Short");
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "dave@example.com");
    assert_eq!(m.subject, "Short");
}

#[test]
fn crlf_line_endings() {
    let maildir = TestMaildir::new("crlf");
    maildir.add(
        "cur",
        "a",
        "From: Eve Example <eve@example.com>\r\nSubject: Windows\r\n\
         Date: Tue, 14 Nov 2023 22:13:20 +0000\r\n\r\nSubject: not this\r\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "Eve Example");
    assert_eq!(m.subject, "Windows");
    assert_eq!(unix(&m), Some(PLAIN_DATE));
}

#[test]
fn a_folded_subject_is_joined() {
    let maildir = TestMaildir::new("folded");
    maildir.add("cur", "a", "Subject: one\n two\n\n");
    assert_eq!(the_one_message(maildir.list()).subject, "one two");
}

#[test]
fn a_folded_from_and_a_folded_date_are_joined() {
    let maildir = TestMaildir::new("folded-from");
    maildir.add(
        "cur",
        "a",
        "From: Heidi Example\n <heidi@example.com>\n\
         Date: Tue, 14 Nov 2023\n\t22:13:20 +0000\n\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "Heidi Example");
    assert_eq!(unix(&m), Some(PLAIN_DATE));
}

#[test]
fn header_names_match_in_any_case() {
    let maildir = TestMaildir::new("case");
    maildir.add(
        "cur",
        "a",
        "FROM: Ivan Example <ivan@example.com>\n\
         subject: Shouting\n\
         dAtE: Tue, 14 Nov 2023 22:13:20 +0000\n\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "Ivan Example");
    assert_eq!(m.subject, "Shouting");
    assert_eq!(unix(&m), Some(PLAIN_DATE));
}

/// A message should have one of each. When it has two, the parser gives the
/// last. This pins that, so a change to how headers are read can't alter
/// which sender is shown without a test failing.
#[test]
fn the_last_of_a_repeated_header_wins() {
    let maildir = TestMaildir::new("repeated");
    maildir.add(
        "cur",
        "a",
        "Subject: first\nSubject: second\n\
         From: Judy Example <judy@example.com>\nFrom: Karl Example <karl@example.org>\n\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.subject, "second");
    assert_eq!(m.from, "Karl Example");
}

#[test]
fn header_lookalikes_in_the_body_are_not_headers() {
    let maildir = TestMaildir::new("lookalikes");
    maildir.add(
        "cur",
        "a",
        "From: Frank Example <frank@example.com>\n\
         \n\
         Subject: not the subject\n\
         From: Mallory <mallory@example.org>\n\
         Date: Tue, 14 Nov 2023 22:13:20 +0000\n",
    );
    let m = the_one_message(maildir.list());
    assert_eq!(m.from, "Frank Example");
    assert_eq!(m.subject, "");
    assert_eq!(m.date, None);
}

#[test]
fn headers_are_read_for_one_mebibyte_and_no_further() {
    let maildir = TestMaildir::new("endless");
    let mut contents = String::from("Subject: Early\n");
    while contents.len() < 2 * 1024 * 1024 {
        contents.push_str("X-Filler: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n");
    }
    contents.push_str("From: Late Example <late@example.com>\n\n");
    maildir.add("cur", "a", contents);
    let m = the_one_message(maildir.list());
    assert_eq!(m.subject, "Early");
    assert_eq!(m.from, "");
}

/// Opening a named pipe waits until something writes to it, so a reader that
/// opened one would never return. The listing runs on its own thread, so that
/// if it does hang, this test fails instead of hanging too.
#[cfg(unix)]
#[test]
fn entries_that_are_not_files_are_skipped() {
    use std::os::unix::fs::symlink;
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let maildir = TestMaildir::new("not-files");
    let cur = maildir.root.join("cur");
    let made = Command::new("mkfifo").arg(cur.join("pipe")).status();
    assert!(made.unwrap().success(), "couldn't make a named pipe");
    symlink(cur.join("pipe"), cur.join("link-to-a-pipe")).unwrap();
    symlink(maildir.root.join("tmp"), cur.join("link-to-a-folder")).unwrap();
    symlink(cur.join("nothing-here"), cur.join("link-to-nothing")).unwrap();
    maildir.add("cur", "real", PLAIN);

    let (sender, receiver) = mpsc::channel();
    let root = maildir.root.clone();
    thread::spawn(move || sender.send(list_newest_first(&Maildir::new(root))));
    let listed = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the listing hung");
    assert_eq!(the_one_message(listed.unwrap()).id.as_str(), "real");
}

#[cfg(unix)]
#[test]
fn a_link_to_a_message_is_listed() {
    use std::os::unix::fs::symlink;

    let maildir = TestMaildir::new("link");
    maildir.add("tmp", "elsewhere", PLAIN);
    symlink(
        maildir.root.join("tmp").join("elsewhere"),
        maildir.root.join("cur").join("linked"),
    )
    .unwrap();
    let m = the_one_message(maildir.list());
    assert_eq!(m.id.as_str(), "linked");
    assert_eq!(m.subject, "Hello there");
}

#[test]
fn a_time_that_cannot_exist_is_no_date() {
    let maildir = TestMaildir::new("impossible-time");
    maildir.add(
        "cur",
        "a",
        "Subject: a\nDate: Tue, 14 Nov 2023 25:13:20 +0000\n\n",
    );
    assert_eq!(the_one_message(maildir.list()).date, None);
}

#[test]
fn a_blank_name_falls_back_to_the_address() {
    let maildir = TestMaildir::new("blank-name");
    maildir.add("cur", "a", "From: \"  \" <nina@example.org>\n\n");
    assert_eq!(the_one_message(maildir.list()).from, "nina@example.org");
}

#[test]
fn a_missing_folder_is_an_error_naming_the_path() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("maildir-does-not-exist");
    let error = Maildir::new(&root).list().unwrap_err();
    assert!(
        error.to_string().contains("maildir-does-not-exist"),
        "{error}"
    );
}

#[test]
fn a_missing_new_folder_is_an_error_naming_the_path() {
    let maildir = TestMaildir::new("no-new");
    fs::remove_dir(maildir.root.join("new")).unwrap();
    let error = Maildir::new(&maildir.root).list().unwrap_err();
    let message = error.to_string();
    let missing = maildir.root.join("new");
    assert!(
        message.contains(&missing.display().to_string()),
        "{message}"
    );
}

#[test]
fn random_bytes_do_not_panic() {
    let maildir = TestMaildir::new("random");
    // A fixed pseudo-random sequence, so a failure can be repeated.
    let mut state: u32 = 12345;
    let bytes: Vec<u8> = (0..100_000)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect();
    maildir.add("cur", "noise", &bytes);
    maildir.add("cur", "empty", b"");
    maildir.add("cur", "newlines", b"\n\n\n\r\n");
    maildir.add("cur", "colons", b":::::\n: :\n=?UTF-8?B?\xff?=: =?\n\n");
    let messages = maildir.list();
    assert_eq!(messages.len(), 4);
}

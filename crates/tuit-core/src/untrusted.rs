//! Text that came from outside, and the one way it gets to a terminal.

/// Text from outside tuit: a sender, a subject, a file name, a path. It can contain anything,
/// including the control characters that drive a terminal.
///
/// It has no `Display`, no `Deref` or `AsRef<str>`, and no method that hands the text over, so
/// showing it without cleaning it doesn't compile. `Debug` is kept for test failures, and
/// escapes control characters by its own rule. Comparing for equality still sees the raw text,
/// and code that set out to rebuild it by guessing could: the type stops forgetting, not
/// determination. The way out is [`terminal_line`](Self::terminal_line) (or
/// [`terminal_line_prefix`](Self::terminal_line_prefix)), which makes the text safe to write to
/// a terminal on one line by one rule:
///
/// 1. a tab and every character that breaks a line (U+0009 to U+000D, U+0085, U+2028 and
///    U+2029) becomes a space;
/// 2. every other control character (`char::is_control`: the rest of C0, DEL and C1, so the
///    escape that starts a terminal sequence among them) becomes U+FFFD, the replacement
///    character;
/// 3. the explicit text-direction characters U+202A to U+202E and U+2066 to U+2069 are removed,
///    because they can reorder what the reader sees on the line. Ordinary mail carries them too,
///    around a name or a number, so they go quietly and leave no mark.
///
/// Nothing else changes. That leaves in some characters that can't start a terminal sequence
/// but aren't plain text either: zero-width characters and the direction marks (U+200E, U+200F,
/// U+061C).
///
/// It can't be printed with `{}`:
///
/// ```compile_fail
/// use tuit_core::Untrusted;
/// let _ = format!("{}", Untrusted::new("x"));
/// ```
///
/// It can't stand in for a `&str`:
///
/// ```compile_fail
/// use tuit_core::Untrusted;
/// let _: &str = &Untrusted::new("x");
/// ```
///
/// And it can't be handed to something that takes any text, as ratatui's drawing calls do:
///
/// ```compile_fail
/// use tuit_core::Untrusted;
/// fn draw(_text: impl AsRef<str>) {}
/// draw(Untrusted::new("x"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Untrusted(String);

impl Untrusted {
    /// Wraps text from outside. Apart from the empty `Default`, this is the only way to make one.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Whether there is no text at all. Text that is only spaces or control characters is not
    /// empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// All of the text, made safe to write to a terminal on one line.
    pub fn terminal_line(&self) -> String {
        make_safe(&self.0)
    }

    /// Like [`terminal_line`](Self::terminal_line), for at most the first `max_bytes` bytes of
    /// the raw text, cut back to a character boundary. The `bool` says whether any text was left
    /// out. The text past the prefix is never looked at, so the cost doesn't depend on its length.
    pub fn terminal_line_prefix(&self, max_bytes: usize) -> (String, bool) {
        let end = self.0.floor_char_boundary(max_bytes);
        (make_safe(&self.0[..end]), end < self.0.len())
    }
}

fn make_safe(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .map(|c| match c {
            '\t'..='\r' | '\u{85}' | '\u{2028}' | '\u{2029}' => ' ',
            c if c.is_control() => '\u{fffd}',
            c => c,
        })
        .collect()
}

impl PartialEq<str> for Untrusted {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Untrusted {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_unchanged() {
        let text = "Zoë 日本語 e\u{301}";
        assert_eq!(Untrusted::new(text).terminal_line(), text);
    }

    #[test]
    fn an_escape_sequence_loses_its_escape() {
        let text = Untrusted::new("a\x1b[2Jb\u{9b}31mc\x7f\0d");
        assert_eq!(
            text.terminal_line(),
            "a\u{fffd}[2Jb\u{fffd}31mc\u{fffd}\u{fffd}d"
        );
    }

    #[test]
    fn a_tab_and_every_line_break_become_a_space() {
        for code in [0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029] {
            let breaker = char::from_u32(code).unwrap();
            assert_eq!(
                Untrusted::new(format!("a{breaker}b")).terminal_line(),
                "a b",
                "U+{code:04X}"
            );
        }
    }

    #[test]
    fn every_other_control_character_is_replaced() {
        // C0, then DEL and C1, without the tab and the line breaks.
        for code in (0x00..=0x1f).chain(0x7f..=0x9f) {
            if matches!(code, 0x09..=0x0d | 0x85) {
                continue;
            }
            let control = char::from_u32(code).unwrap();
            assert_eq!(
                Untrusted::new(format!("a{control}b")).terminal_line(),
                "a\u{fffd}b",
                "U+{code:04X}"
            );
        }
    }

    #[test]
    fn text_direction_characters_are_removed() {
        let text = Untrusted::new(
            "a\u{202a}b\u{202b}c\u{202c}d\u{202d}e\u{202e}f\u{2066}g\u{2067}h\u{2068}i\u{2069}j",
        );
        assert_eq!(text.terminal_line(), "abcdefghij");
    }

    #[test]
    fn nearby_characters_are_left_alone() {
        // Just outside the ranges that are removed or become a space, a zero-width space and a
        // direction mark.
        let text = "\u{2027}\u{202f}\u{2065}\u{206a}\u{200b}\u{200e}";
        assert_eq!(Untrusted::new(text).terminal_line(), text);
    }

    #[test]
    fn a_prefix_is_cut_back_to_a_character_boundary() {
        let text = Untrusted::new("a日b");
        assert_eq!(text.terminal_line_prefix(0), (String::new(), true));
        assert_eq!(text.terminal_line_prefix(1), ("a".to_owned(), true));
        assert_eq!(text.terminal_line_prefix(2), ("a".to_owned(), true));
        assert_eq!(text.terminal_line_prefix(3), ("a".to_owned(), true));
        assert_eq!(text.terminal_line_prefix(4), ("a日".to_owned(), true));
        assert_eq!(text.terminal_line_prefix(5), ("a日b".to_owned(), false));
        assert_eq!(text.terminal_line_prefix(500), ("a日b".to_owned(), false));
    }

    #[test]
    fn a_prefix_is_made_safe_too() {
        // The prefix of six bytes ends after the direction character, which takes three.
        let text = Untrusted::new("a\x1bb\u{202e}c");
        assert_eq!(
            text.terminal_line_prefix(6),
            ("a\u{fffd}b".to_owned(), true)
        );
    }

    #[test]
    fn debug_escapes_control_characters() {
        let shown = format!("{:?}", Untrusted::new("a\x1b[2Jb\nc"));
        assert!(!shown.contains('\x1b'));
        assert!(!shown.contains('\n'));
        assert!(shown.contains("\\u{1b}"));
    }

    #[test]
    fn it_compares_with_text_and_knows_when_it_is_empty() {
        assert_eq!(Untrusted::new("x"), "x");
        assert!(Untrusted::new("x") == *"x");
        assert!(Untrusted::new("x") != "y");
        assert!(Untrusted::new("x") != *"y");
        assert!(Untrusted::new("X") != "x");
        // Wrapping keeps the text exactly, spaces at the ends included.
        assert_eq!(Untrusted::new(" x "), " x ");
        assert_ne!(Untrusted::new(" x "), "x");
        // Comparing sees the text as it came, not as it would be shown.
        assert_ne!(Untrusted::new("a\tb"), "a b");
        assert!(Untrusted::new("a\tb") != *"a b");
        assert_eq!(Untrusted::new(String::from(" x ")), " x ");
        assert!(Untrusted::default().is_empty());
        assert!(Untrusted::new("").is_empty());
        assert!(!Untrusted::new(" ").is_empty());
    }
}

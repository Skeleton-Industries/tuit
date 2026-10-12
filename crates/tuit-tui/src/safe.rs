//! The rule for text that is going to a line of a terminal.

use tuit_core::Sanitizer;

/// The rule that makes text from outside safe to write to a terminal on one line:
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
/// Use it as `text.sanitize_with(&TerminalLine)`. All of the text goes through the rule; none
/// is cut off.
#[derive(Debug, Clone, Copy)]
pub struct TerminalLine;

impl Sanitizer for TerminalLine {
    type Output = String;

    fn sanitize(&self, raw: &str) -> String {
        clean(raw)
    }
}

/// [`TerminalLine`] for at most the first `max_bytes` bytes of the raw text, cut back to a
/// character boundary. The `bool` in the output says whether any text was left out. The text
/// past the prefix is never looked at, so the cost doesn't depend on its length.
pub(crate) struct TerminalLinePrefix {
    pub(crate) max_bytes: usize,
}

impl Sanitizer for TerminalLinePrefix {
    type Output = (String, bool);

    fn sanitize(&self, raw: &str) -> (String, bool) {
        let end = raw.floor_char_boundary(self.max_bytes);
        (clean(&raw[..end]), end < raw.len())
    }
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .map(|c| match c {
            '\t'..='\r' | '\u{85}' | '\u{2028}' | '\u{2029}' => ' ',
            c if c.is_control() => '\u{fffd}',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tuit_core::Untrusted;

    fn line(text: &str) -> String {
        Untrusted::new(text).sanitize_with(&TerminalLine)
    }

    fn prefix(text: &str, max_bytes: usize) -> (String, bool) {
        Untrusted::new(text).sanitize_with(&TerminalLinePrefix { max_bytes })
    }

    #[test]
    fn plain_text_is_unchanged() {
        let text = "Zoë 日本語 e\u{301}";
        assert_eq!(line(text), text);
    }

    #[test]
    fn an_escape_sequence_loses_its_escape() {
        assert_eq!(
            line("a\x1b[2Jb\u{9b}31mc\x7f\0d"),
            "a\u{fffd}[2Jb\u{fffd}31mc\u{fffd}\u{fffd}d"
        );
    }

    #[test]
    fn a_tab_and_every_line_break_become_a_space() {
        for code in [0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029] {
            let breaker = char::from_u32(code).unwrap();
            assert_eq!(line(&format!("a{breaker}b")), "a b", "U+{code:04X}");
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
            assert_eq!(line(&format!("a{control}b")), "a\u{fffd}b", "U+{code:04X}");
        }
    }

    #[test]
    fn text_direction_characters_are_removed() {
        let text =
            "a\u{202a}b\u{202b}c\u{202c}d\u{202d}e\u{202e}f\u{2066}g\u{2067}h\u{2068}i\u{2069}j";
        assert_eq!(line(text), "abcdefghij");
    }

    #[test]
    fn nearby_characters_are_left_alone() {
        // Just outside the ranges that are removed or become a space, a zero-width space and a
        // direction mark.
        let text = "\u{2027}\u{202f}\u{2065}\u{206a}\u{200b}\u{200e}";
        assert_eq!(line(text), text);
    }

    #[test]
    fn a_prefix_is_cut_back_to_a_character_boundary() {
        assert_eq!(prefix("a日b", 0), (String::new(), true));
        assert_eq!(prefix("a日b", 1), ("a".to_owned(), true));
        assert_eq!(prefix("a日b", 2), ("a".to_owned(), true));
        assert_eq!(prefix("a日b", 3), ("a".to_owned(), true));
        assert_eq!(prefix("a日b", 4), ("a日".to_owned(), true));
        assert_eq!(prefix("a日b", 5), ("a日b".to_owned(), false));
        assert_eq!(prefix("a日b", 500), ("a日b".to_owned(), false));
    }

    #[test]
    fn a_prefix_is_made_safe_too() {
        // The prefix of six bytes ends after the direction character, which takes three.
        assert_eq!(
            prefix("a\x1bb\u{202e}c", 6),
            ("a\u{fffd}b".to_owned(), true)
        );
    }
}

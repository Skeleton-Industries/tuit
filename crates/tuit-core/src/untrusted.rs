//! Text that came from outside, marked so that it can't be shown by accident.

/// Text from outside tuit: a sender, a subject, a file name, a path. It can contain anything,
/// control characters included, and hasn't been made safe for anywhere.
///
/// The core only marks text. What "safe" means depends on where the text is going, and the edge
/// that sends it there decides that and writes the rule. So the type has no `Display` and no
/// `Deref` or `AsRef<str>`: showing it by accident doesn't compile. The way to the text is
/// [`expose_raw`](Self::expose_raw), which has to be called on purpose and is meant for that
/// edge's rule.
///
/// Two things still see the text. `Debug` is kept for test failures and escapes control
/// characters by its own rule, which is not a rule for showing text to a person. And comparing
/// for equality sees the raw text, so code that set out to rebuild it by guessing could. The
/// type stops forgetting, not determination.
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
/// And it can't be handed to something that takes any text:
///
/// ```compile_fail
/// use tuit_core::Untrusted;
/// fn draw(_text: impl AsRef<str>) {}
/// draw(Untrusted::new("x"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Untrusted(String);

impl Untrusted {
    /// Wraps text from outside. Apart from the empty `Default`, this is the only way to make one
    /// from text.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Whether there is no text at all. Text that is only spaces or control characters is not
    /// empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The text exactly as it came, control characters included.
    ///
    /// It exists so that an edge can write the rule that makes the text safe for its own
    /// destination. Nothing else should call it, and outside this crate a lint refuses the call
    /// unless a function switches the lint off: see `docs/ARCHITECTURE.md`.
    pub fn expose_raw(&self) -> &str {
        &self.0
    }
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
    fn debug_escapes_control_characters() {
        let shown = format!("{:?}", Untrusted::new("a\x1b[2Jb\nc"));
        assert!(!shown.contains('\x1b'));
        assert!(!shown.contains('\n'));
        assert!(shown.contains("\\u{1b}"));
    }

    #[test]
    fn expose_raw_gives_the_text_as_it_came() {
        let text = "a\x1b[2J\tb \u{202e}";
        assert_eq!(Untrusted::new(text).expose_raw(), text);
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

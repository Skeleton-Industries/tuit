//! Text that came from outside, marked so that it can't be shown by accident.

/// A rule that makes text from outside safe for one destination.
///
/// The core defines no rules and doesn't know the destinations. An edge that knows a
/// destination writes a named type for it and implements this trait for that type. An
/// [`Untrusted`] applies the rule itself, through [`Untrusted::sanitize_with`].
///
/// An implementation receives the raw text, so it is the place a reviewer looks. One that hands
/// the text back unchanged, or keeps it, defeats the type, on purpose or by mistake, and only
/// review catches it. So does one that wraps a closure and calls it: the rule is then wherever
/// the closure is written, and no longer has one place.
///
/// A closure by itself is not a rule, because nothing implements this trait for closures:
///
/// ```compile_fail
/// use tuit_core::Untrusted;
/// let _ = Untrusted::new("x").sanitize_with(&|raw: &str| raw.to_owned());
/// ```
pub trait Sanitizer {
    /// What this destination calls safe.
    type Output;

    /// Applies the rule to the text exactly as it came.
    fn sanitize(&self, raw: &str) -> Self::Output;
}

/// Text from outside tuit: a sender, a subject, a file name, a path. It can contain anything,
/// control characters included, and hasn't been made safe for anywhere.
///
/// The core only marks text. What "safe" means depends on where the text is going, and the edge
/// that sends it there decides that and writes the rule, as a [`Sanitizer`]. So the type has no
/// `Display` and no `Deref` or `AsRef<str>`: showing it by accident doesn't compile. The way to
/// the text is [`sanitize_with`](Self::sanitize_with), which hands the raw text to a rule and
/// returns what the rule makes of it. No method hands the raw text to a caller.
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

    /// Makes the text safe with `sanitizer`, which is given it exactly as it came, and returns
    /// what the rule makes of it.
    pub fn sanitize_with<S: Sanitizer>(&self, sanitizer: &S) -> S::Output {
        sanitizer.sanitize(&self.0)
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

    struct Length;

    impl Sanitizer for Length {
        type Output = usize;

        fn sanitize(&self, raw: &str) -> usize {
            raw.len()
        }
    }

    struct Upper;

    impl Sanitizer for Upper {
        type Output = String;

        fn sanitize(&self, raw: &str) -> String {
            raw.to_uppercase()
        }
    }

    struct Same;

    impl Sanitizer for Same {
        type Output = String;

        fn sanitize(&self, raw: &str) -> String {
            raw.to_owned()
        }
    }

    #[test]
    fn debug_escapes_control_characters() {
        let shown = format!("{:?}", Untrusted::new("a\x1b[2Jb\nc"));
        assert!(!shown.contains('\x1b'));
        assert!(!shown.contains('\n'));
        assert!(shown.contains("\\u{1b}"));
    }

    #[test]
    fn a_rule_sees_the_text_as_it_came() {
        // Control characters, a direction character and spaces at the ends all arrive.
        let text = " a\x1b[2J\tb \u{202e}\n";
        assert_eq!(Untrusted::new(text).sanitize_with(&Same), text);
        assert_eq!(Untrusted::new(text).sanitize_with(&Length), text.len());
    }

    #[test]
    fn the_output_of_the_rule_is_returned() {
        assert_eq!(Untrusted::new("abc").sanitize_with(&Upper), "ABC");
        assert_eq!(Untrusted::default().sanitize_with(&Length), 0);
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

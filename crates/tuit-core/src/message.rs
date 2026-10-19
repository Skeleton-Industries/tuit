//! What a message is: its name in a store, a moment in time, and what a message
//! list shows.

use crate::untrusted::Untrusted;

/// A store's own name for one message. Only the store that issued it can interpret it.
///
/// It can come from a file name, so it can contain anything. It is a key, not something to
/// show: it can be compared, ordered and hashed, and has no `Display` and no method that reads it
/// as text. `Debug` shows it with control characters escaped. A store that has to find a message
/// again will need to read it; that method is added here then, on purpose.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(String);

impl MessageId {
    /// Wraps a store's name for a message.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// A moment in time: whole seconds since 1970-01-01 00:00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Makes a timestamp from seconds since 1970-01-01 00:00:00 UTC. Negative
    /// values are before 1970.
    pub fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Seconds since 1970-01-01 00:00:00 UTC.
    pub fn unix_seconds(self) -> i64 {
        self.0
    }
}

/// What a message list shows for one message.
///
/// `from` and `subject` are whatever the message said. They can contain
/// anything, control characters included, so they are [`Untrusted`]: whoever
/// shows them decides what safe means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSummary {
    /// The store's name for the message.
    pub id: MessageId,
    /// The sender's name if the message gives one, otherwise their address. Empty if neither.
    pub from: Untrusted,
    /// Empty if the message has no subject.
    pub subject: Untrusted,
    /// `None` if the message has no date or the date can't be read.
    pub date: Option<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_compare_and_order_by_their_text() {
        assert_eq!(MessageId::new("abc:2,S"), MessageId::new("abc:2,S"));
        assert_ne!(MessageId::new("abc:2,S"), MessageId::new("abc"));
        assert!(MessageId::new("a") < MessageId::new("b"));
    }

    #[test]
    fn an_id_printed_for_debugging_has_its_control_characters_escaped() {
        let shown = format!("{:?}", MessageId::new("a\x1b[2Jb"));
        assert!(!shown.contains('\x1b'), "{shown}");
    }

    #[test]
    fn timestamp_round_trips() {
        assert_eq!(Timestamp::from_unix_seconds(-5).unix_seconds(), -5);
    }

    #[test]
    fn timestamps_order_by_time() {
        assert!(Timestamp::from_unix_seconds(1) < Timestamp::from_unix_seconds(2));
    }
}

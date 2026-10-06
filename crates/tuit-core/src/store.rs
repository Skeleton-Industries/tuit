//! Somewhere messages are kept, and what you can ask of it.

use std::fmt;

use crate::message::MessageSummary;

/// Somewhere messages are kept.
pub trait MailStore {
    /// Every message in the store, in no particular order. A store should
    /// give each message a different id.
    ///
    /// # Errors
    ///
    /// When the store can't be read.
    fn list(&self) -> Result<Vec<MessageSummary>, StoreError>;
}

/// A store couldn't do what was asked.
#[derive(Debug)]
pub struct StoreError {
    message: String,
}

impl StoreError {
    /// Makes an error with a message for a person to read.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StoreError {}

/// A store's messages, newest first. Messages with no date come last.
/// Messages with the same date are ordered by id, so the order doesn't depend
/// on the store. That holds as long as no two messages share an id and a date.
///
/// # Errors
///
/// When the store's `list` fails.
pub fn list_newest_first(
    store: &(impl MailStore + ?Sized),
) -> Result<Vec<MessageSummary>, StoreError> {
    let mut messages = store.list()?;
    // `None` sorts before any `Some`, so comparing `b` to `a` puts the newest
    // first and the undated last.
    messages.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.id.cmp(&b.id)));
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{MessageId, Timestamp};

    struct Fake(Result<Vec<MessageSummary>, String>);

    impl MailStore for Fake {
        fn list(&self) -> Result<Vec<MessageSummary>, StoreError> {
            match &self.0 {
                Ok(messages) => Ok(messages.clone()),
                Err(message) => Err(StoreError::new(message.clone())),
            }
        }
    }

    fn msg(id: &str, date: Option<i64>) -> MessageSummary {
        MessageSummary {
            id: MessageId::new(id),
            from: String::new(),
            subject: String::new(),
            date: date.map(Timestamp::from_unix_seconds),
        }
    }

    fn ids(messages: &[MessageSummary]) -> Vec<MessageId> {
        messages.iter().map(|m| m.id.clone()).collect()
    }

    fn named<const N: usize>(names: [&str; N]) -> Vec<MessageId> {
        names.into_iter().map(MessageId::new).collect()
    }

    #[test]
    fn newest_first() {
        let store = Fake(Ok(vec![
            msg("a", Some(100)),
            msg("b", Some(300)),
            msg("c", Some(200)),
        ]));
        let sorted = list_newest_first(&store).unwrap();
        assert_eq!(ids(&sorted), named(["b", "c", "a"]));
    }

    #[test]
    fn undated_come_last() {
        let store = Fake(Ok(vec![
            msg("a", None),
            msg("b", Some(-50)),
            msg("c", Some(10)),
        ]));
        let sorted = list_newest_first(&store).unwrap();
        assert_eq!(ids(&sorted), named(["c", "b", "a"]));
    }

    #[test]
    fn same_date_is_ordered_by_id() {
        let store = Fake(Ok(vec![
            msg("z", Some(5)),
            msg("m", None),
            msg("a", Some(5)),
            msg("b", None),
        ]));
        let sorted = list_newest_first(&store).unwrap();
        assert_eq!(ids(&sorted), named(["a", "z", "b", "m"]));
    }

    #[test]
    fn a_store_behind_a_trait_object_can_be_listed() {
        let store: Box<dyn MailStore> = Box::new(Fake(Ok(vec![msg("a", Some(1))])));
        let sorted = list_newest_first(store.as_ref()).unwrap();
        assert_eq!(ids(&sorted), named(["a"]));
    }

    #[test]
    fn an_empty_store_is_an_empty_list() {
        let sorted = list_newest_first(&Fake(Ok(Vec::new()))).unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn a_store_error_is_passed_on() {
        let error = list_newest_first(&Fake(Err("no such place".into()))).unwrap_err();
        assert_eq!(error.to_string(), "no such place");
    }
}

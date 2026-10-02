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

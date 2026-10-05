//! Draws tuit's screens and handles key presses.
//! Of our crates, it may depend only on `tuit-core`. It must not read mail from disk or the
//! network itself: it draws what it is given.
//!
//! [`MessageList`] is the message list without a terminal: feed it events and draw it into any
//! ratatui buffer, such as the in-memory `TestBackend`.

mod list;
mod style;
mod text;

pub use list::{MIN_HEIGHT, MIN_WIDTH, MessageList, Update};

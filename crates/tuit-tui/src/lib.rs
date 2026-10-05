//! Draws tuit's screens and handles key presses.
//! Of our crates, it may depend only on `tuit-core`. It must not read mail from disk or the
//! network itself: it draws what it is given.
//!
//! [`run`] takes over the terminal and shows the message list. [`MessageList`] is the same list
//! without a terminal: feed it events and draw it into any ratatui buffer, such as the
//! in-memory `TestBackend`.

mod list;
mod style;
mod text;

use std::io::{self, IsTerminal};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use tuit_core::MessageSummary;

pub use list::{MIN_HEIGHT, MIN_WIDTH, MessageList, Update};

/// Takes over the terminal, shows `messages` in a list, and returns when the user quits. The
/// terminal is put back as it was, including if the program panics. It is not put back if the
/// program is killed by a signal.
///
/// Fails without touching the terminal if standard input or output is not a terminal.
pub fn run(messages: Vec<MessageSummary>) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other("needs a terminal to draw on"));
    }
    // Starting up is several steps. If a later one fails, undo the earlier ones.
    let mut terminal = ratatui::try_init().inspect_err(|_| ratatui::restore())?;
    let result = event_loop(&mut terminal, MessageList::new(messages));
    ratatui::restore();
    result
}

/// Waits for events, and draws only after one that changes the screen or resizes it.
fn event_loop(terminal: &mut DefaultTerminal, mut list: MessageList) -> io::Result<()> {
    loop {
        terminal.draw(|frame| list.draw(frame))?;
        loop {
            match list.handle_event(&event::read()?) {
                Update::Ignored => {}
                Update::Redraw => break,
                Update::Quit => return Ok(()),
            }
        }
    }
}

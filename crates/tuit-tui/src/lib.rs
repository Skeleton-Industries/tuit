//! Draws tuit's screens and handles key presses.
//! Of our crates, it may depend only on `tuit-core`. It must not read mail from disk or the
//! network itself: it draws what it is given.
//!
//! [`run`] takes over the terminal and shows the message list. [`MessageList`] is the same list
//! without a terminal: feed it events and draw it into any ratatui buffer, such as the
//! in-memory `TestBackend`.
//!
//! [`TerminalLine`] is the rule that makes text from outside safe for one line of a terminal:
//! `text.sanitize_with(&TerminalLine)`. It lives here because this is the crate that knows what
//! a terminal is; the command line uses it for its error lines.

mod list;
mod safe;
mod style;
mod text;

use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use jiff::tz::TimeZone;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use signal_hook::consts::SIGTERM;
use signal_hook::{flag, low_level};
use tuit_core::MessageSummary;

pub use list::{MIN_HEIGHT, MIN_WIDTH, MessageList, Update};
pub use safe::TerminalLine;

/// Takes over the terminal, shows `messages` in a list with their dates in `time_zone`, and
/// returns when the user quits. The terminal is put back as it was, including if the program
/// panics.
///
/// On SIGTERM the terminal is put back the same way, and then the program ends by SIGTERM, as it
/// would have had it not caught it: this function does not return. The signal is noticed within
/// a quarter of a second. Any other signal ends the program with the terminal as it is; SIGHUP
/// is left alone on purpose (design 0003). This is meant to be the last thing a program does:
/// the SIGTERM handler stays until the program ends.
///
/// Fails without touching the terminal if standard input or output is not a terminal.
pub fn run(messages: Vec<MessageSummary>, time_zone: TimeZone) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other("needs a terminal to draw on"));
    }
    // Registered before the terminal is taken over, so no SIGTERM can end the program with the
    // terminal raw. One that arrives early waits in the flag until the loop looks.
    let terminated = Arc::new(AtomicBool::new(false));
    flag::register(SIGTERM, Arc::clone(&terminated))?;
    // Starting up is several steps. If a later one fails, undo the earlier ones.
    let mut terminal = ratatui::try_init().inspect_err(|_| ratatui::restore())?;
    let result = event_loop(
        &mut terminal,
        MessageList::new(messages, time_zone),
        &terminated,
    );
    ratatui::restore();
    // Dropping the terminal shows the cursor again. It must happen before the program ends.
    drop(terminal);
    if terminated.load(Ordering::Relaxed) {
        let _ = low_level::emulate_default_handler(SIGTERM);
        // Not reached if that worked. If not, exit as a shell reports death by a signal.
        std::process::exit(128 + SIGTERM);
    }
    result
}

/// How long to wait for a key before looking for SIGTERM again.
const SIGNAL_CHECK: Duration = Duration::from_millis(250);

/// Waits for events, and draws only after one that changes the screen or resizes it. Returns
/// when the user quits or `terminated` is set.
fn event_loop(
    terminal: &mut DefaultTerminal,
    mut list: MessageList,
    terminated: &AtomicBool,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| list.draw(frame))?;
        loop {
            if terminated.load(Ordering::Relaxed) {
                return Ok(());
            }
            if !event::poll(SIGNAL_CHECK)? {
                continue;
            }
            match list.handle_event(&event::read()?) {
                Update::Ignored => {}
                Update::Redraw => break,
                Update::Quit => return Ok(()),
            }
        }
    }
}

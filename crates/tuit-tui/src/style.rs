//! Every style the screen uses, in one place. Changing the look is an edit to this file.
//!
//! Colours come only from the terminal's own palette: here that is just the default foreground
//! and background, one named ANSI colour, and modifiers. That way the screen follows whatever
//! theme the user's terminal has. "Quiet" is the dim modifier rather than the dark-grey colour,
//! because some popular themes draw dark grey almost invisibly on their background.

use ratatui::style::{Color, Modifier, Style};

/// The one accent colour. Used for the title and nowhere else.
const ACCENT: Color = Color::Cyan;

/// The frame's rounded border.
pub(crate) const BORDER: Style = Style::new().add_modifier(Modifier::DIM);

/// The word "Inbox" in the frame's top edge.
pub(crate) const TITLE: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);

/// "12 of 30" in the frame's top edge.
pub(crate) const POSITION: Style = Style::new().add_modifier(Modifier::DIM);

/// The key hint under the frame.
pub(crate) const HINT: Style = Style::new().add_modifier(Modifier::DIM);

/// The date column.
pub(crate) const DATE: Style = Style::new().add_modifier(Modifier::DIM);

/// "(no subject)" and "(no sender)": present, but visibly not real text.
pub(crate) const PLACEHOLDER: Style = Style::new()
    .add_modifier(Modifier::DIM)
    .add_modifier(Modifier::ITALIC);

/// The selected row's band. Reversed video swaps the terminal's own foreground and background,
/// so it is readable in every theme, light or dark.
pub(crate) const SELECTED: Style = Style::new().add_modifier(Modifier::REVERSED);

/// A placeholder inside the selected band: italic only, because dim would fade it into the band.
pub(crate) const SELECTED_PLACEHOLDER: Style = Style::new().add_modifier(Modifier::ITALIC);

/// "No messages" and "Terminal too small".
pub(crate) const NOTICE: Style = Style::new().add_modifier(Modifier::DIM);

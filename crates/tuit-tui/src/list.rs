//! The message list: what is loaded, which row is selected, how keys move it, and how it draws.
//! None of it needs a real terminal. Feed it events and draw it into any ratatui buffer.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Widget};
use tuit_core::{MessageSummary, Untrusted};

use crate::style;
use crate::text::{cells, fit, format_date, sanitize_and_fit};

/// Below this width (in terminal columns) the list shows "Terminal too small".
pub const MIN_WIDTH: u16 = 30;
/// Below this height (in terminal rows) the list shows "Terminal too small".
pub const MIN_HEIGHT: u16 = 5;

/// Space before the first column and after the last, inside the frame.
const PADDING: usize = 1;
/// Space between columns.
const GAP: usize = 2;
/// "30 Sep 2026", the longest date a year of four digits makes.
const DATE_WIDTH: usize = 11;
/// The sender column's usual width.
const SENDER_WIDTH: usize = 22;
/// The date column is dropped unless the subject can keep at least this much.
const SUBJECT_MIN_WIDTH: usize = 20;
/// With no date column, the sender shrinks before the subject falls below this.
const SUBJECT_MIN_WIDTH_NO_DATE: usize = 14;

const NO_SUBJECT: &str = "(no subject)";
const NO_SENDER: &str = "(no sender)";
const HINT: &str = "q quit · j/k move · g/G top/bottom · PgUp/PgDn page";

/// What the screen should do after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
    /// Nothing changed. Don't redraw.
    Ignored,
    /// Draw the screen again.
    Redraw,
    /// The user asked to quit.
    Quit,
}

/// A scrolling list of messages with one selected row.
#[derive(Debug, Clone)]
pub struct MessageList {
    messages: Vec<MessageSummary>,
    selected: usize,
    /// Index of the first visible row.
    offset: usize,
    /// How many rows the last draw had room for, used for paging. At least 1.
    page: usize,
}

impl MessageList {
    /// A list showing `messages` in the order given, with the first one selected.
    pub fn new(messages: Vec<MessageSummary>) -> Self {
        Self {
            messages,
            selected: 0,
            offset: 0,
            page: 1,
        }
    }

    /// How many messages are loaded.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether no messages are loaded.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// The selected row's index, or `None` if the list is empty.
    pub fn selected(&self) -> Option<usize> {
        (!self.messages.is_empty()).then_some(self.selected)
    }

    /// Applies a terminal event. Only key presses, repeats and resizes need a redraw.
    pub fn handle_event(&mut self, event: &Event) -> Update {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_key(*key),
            Event::Resize(..) => Update::Redraw,
            _ => Update::Ignored,
        }
    }

    /// Applies one key press. A movement key that leaves the selection where it was, such as
    /// down on the last row, is `Ignored`.
    pub fn handle_key(&mut self, key: KeyEvent) -> Update {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Update::Quit;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Update::Ignored;
        }
        let last = self.messages.len().saturating_sub(1);
        let before = self.selected;
        match key.code {
            KeyCode::Char('q') => return Update::Quit,
            KeyCode::Char('j') | KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Char('k') | KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = last,
            KeyCode::PageDown => self.selected = (self.selected + self.page).min(last),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(self.page),
            _ => return Update::Ignored,
        }
        if self.selected == before {
            Update::Ignored
        } else {
            Update::Redraw
        }
    }

    /// Draws the list over the whole frame.
    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.render(area, frame.buffer_mut());
    }

    /// Draws the list into `area` of `buf`. Builds only the rows that are visible.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            render_too_small(area, buf);
            return;
        }
        let [frame_area, hint_area] = [
            Rect::new(area.x, area.y, area.width, area.height - 1),
            Rect::new(area.x, area.bottom() - 1, area.width, 1),
        ];
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(style::BORDER)
            .title_top(Line::from(vec![
                Span::raw(" "),
                Span::styled("Inbox", style::TITLE),
                Span::raw(" "),
            ]));
        if let Some(selected) = self.selected() {
            let position = format!(" {} of {} ", selected + 1, self.len());
            block = block
                .title_top(Line::styled(position, style::POSITION).alignment(Alignment::Right));
        }
        let inner = block.inner(frame_area);
        block.render(frame_area, buf);

        put(
            buf,
            hint_area.x + PADDING as u16,
            hint_area.y,
            usize::from(hint_area.width).saturating_sub(PADDING),
            HINT,
            style::HINT,
        );

        if self.messages.is_empty() {
            render_centered(inner, buf, &["No messages"], style::NOTICE);
            return;
        }

        let rows = usize::from(inner.height);
        self.page = rows;
        self.scroll_into_view(rows);
        let columns = Columns::for_width(usize::from(inner.width));
        let end = (self.offset + rows).min(self.messages.len());
        for (row, index) in (self.offset..end).enumerate() {
            let y = inner.y + row as u16;
            render_row(
                &self.messages[index],
                index == self.selected,
                inner,
                y,
                &columns,
                buf,
            );
        }
    }

    /// Moves the window the minimum needed to show the selected row, and never leaves blank
    /// rows at the bottom when the list is long enough to fill them (after a resize, say).
    fn scroll_into_view(&mut self, rows: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + rows {
            self.offset = self.selected + 1 - rows;
        }
        self.offset = self.offset.min(self.messages.len().saturating_sub(rows));
    }
}

/// Draws one message on row `y` of `inner`.
fn render_row(
    message: &MessageSummary,
    selected: bool,
    inner: Rect,
    y: u16,
    columns: &Columns,
    buf: &mut Buffer,
) {
    let row = Rect::new(inner.x, y, inner.width, 1);
    if selected {
        buf.set_style(row, style::SELECTED);
    }
    let placeholder = if selected {
        style::SELECTED_PLACEHOLDER
    } else {
        style::PLACEHOLDER
    };
    // The placeholder goes in when nothing of the text would be seen: it is empty, or only
    // spaces (a tab and a line break show as one) or characters that take no cells. Any other
    // control character shows as a replacement character, so it counts as something to see.
    let mut field = |x: u16, width: usize, text: &Untrusted, none: &str| {
        let shown = sanitize_and_fit(text, width);
        if shown.trim().is_empty() {
            put(buf, x, y, width, none, placeholder);
        } else {
            buf.set_stringn(x, y, &shown, width, Style::new());
        }
    };
    let x = inner.x + PADDING as u16;
    field(x, columns.sender, &message.from, NO_SENDER);
    let subject_x = x + (columns.sender + GAP) as u16;
    field(subject_x, columns.subject, &message.subject, NO_SUBJECT);

    // A date too long for its column is left out: cut short, its year would read as another.
    let date = message.date.map(|date| format_date(date.utc_date()));
    if let (true, Some(date)) = (columns.date, date)
        && cells(&date) <= DATE_WIDTH
    {
        let date_x = inner.right() - PADDING as u16 - cells(&date) as u16;
        let date_style = if selected { Style::new() } else { style::DATE };
        buf.set_stringn(date_x, y, &date, DATE_WIDTH, date_style);
    }
}

/// Where the columns go for a given inner width.
struct Columns {
    sender: usize,
    subject: usize,
    /// Whether the date column is shown.
    date: bool,
}

impl Columns {
    /// The date column goes first when space runs out, then the sender shrinks.
    fn for_width(width: usize) -> Self {
        let edges = 2 * PADDING;
        let with_date = edges + SENDER_WIDTH + GAP + SUBJECT_MIN_WIDTH + GAP + DATE_WIDTH;
        if width >= with_date {
            return Self {
                sender: SENDER_WIDTH,
                subject: width - edges - SENDER_WIDTH - 2 * GAP - DATE_WIDTH,
                date: true,
            };
        }
        let sender = width
            .saturating_sub(edges + GAP + SUBJECT_MIN_WIDTH_NO_DATE)
            .min(SENDER_WIDTH);
        Self {
            sender,
            subject: width.saturating_sub(edges + GAP + sender),
            date: false,
        }
    }
}

/// Writes `text`, one of tuit's own labels, cut to `width` cells, at (`x`, `y`).
fn put(buf: &mut Buffer, x: u16, y: u16, width: usize, text: &str, style: Style) {
    let text = fit(text, width);
    buf.set_stringn(x, y, &text, width, style);
}

/// Centres lines of text in `area`, each cut to fit.
fn render_centered(area: Rect, buf: &mut Buffer, lines: &[&str], style: Style) {
    let top = area.y + (area.height.saturating_sub(lines.len() as u16)) / 2;
    for (i, line) in lines.iter().enumerate() {
        let y = top + i as u16;
        if y >= area.bottom() {
            break;
        }
        let text = fit(line, usize::from(area.width));
        let x = area.x + (area.width - cells(&text) as u16) / 2;
        buf.set_stringn(x, y, &text, usize::from(area.width), style);
    }
}

fn render_too_small(area: Rect, buf: &mut Buffer) {
    let needs = format!("needs {MIN_WIDTH} x {MIN_HEIGHT}");
    render_centered(area, buf, &["Terminal too small", &needs], style::NOTICE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_always_fit_and_keep_their_minimums() {
        // The narrowest list is `MIN_WIDTH` less its two border columns.
        let narrowest = usize::from(MIN_WIDTH) - 2;
        let at_narrowest = Columns::for_width(narrowest);
        assert_eq!((at_narrowest.sender, at_narrowest.subject), (10, 14));
        for width in narrowest..300 {
            let columns = Columns::for_width(width);
            let date = if columns.date { GAP + DATE_WIDTH } else { 0 };
            assert_eq!(
                2 * PADDING + columns.sender + GAP + columns.subject + date,
                width
            );
            assert!(columns.sender >= 10 && columns.sender <= SENDER_WIDTH);
            let subject_min = if columns.date {
                SUBJECT_MIN_WIDTH
            } else {
                SUBJECT_MIN_WIDTH_NO_DATE
            };
            assert!(columns.subject >= subject_min, "{width}");
        }
    }
}

//! The message list, driven with key events and drawn into ratatui's in-memory backend.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::Modifier;
use tuit_core::{MessageId, MessageSummary, Timestamp};
use tuit_tui::{MessageList, Update};

/// 2026-10-02 00:00:00 UTC
const OCT_2_2026: i64 = 1_790_899_200;

fn message(n: usize, from: &str, subject: &str, date: Option<i64>) -> MessageSummary {
    MessageSummary {
        id: MessageId::new(format!("m{n}")),
        from: from.to_string(),
        subject: subject.to_string(),
        date: date.map(Timestamp::from_unix_seconds),
    }
}

fn numbered(count: usize) -> Vec<MessageSummary> {
    (1..=count)
        .map(|n| {
            message(
                n,
                &format!("Sender {n}"),
                &format!("Subject {n}"),
                Some(OCT_2_2026 - n as i64 * 86_400),
            )
        })
        .collect()
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn press(list: &mut MessageList, codes: &[KeyCode]) {
    for code in codes {
        list.handle_event(&key(*code));
    }
}

fn draw(list: &mut MessageList, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| list.draw(frame)).unwrap();
    terminal
}

fn lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn screen(list: &mut MessageList, width: u16, height: u16) -> String {
    lines(&draw(list, width, height)).join("\n")
}

/// The row of the screen that is shown in reverse video, if exactly one is.
fn selected_row(buffer: &Buffer) -> Option<u16> {
    let rows: Vec<u16> = (0..buffer.area.height)
        .filter(|&y| {
            buffer[(buffer.area.width / 2, y)]
                .modifier
                .contains(Modifier::REVERSED)
        })
        .collect();
    (rows.len() == 1).then(|| rows[0])
}

#[test]
fn shows_three_columns_in_a_frame() {
    let mut list = MessageList::new(vec![
        message(1, "Ada Example", "Welcome aboard", Some(OCT_2_2026)),
        message(2, "Bo Sample", "Second one", Some(OCT_2_2026 - 86_400 * 40)),
    ]);
    let rows = lines(&draw(&mut list, 80, 10));
    assert!(rows[0].starts_with("╭ Inbox "), "{}", rows[0]);
    assert!(rows[0].ends_with(" 1 of 2 ╮"), "{}", rows[0]);
    assert_eq!(rows[1].chars().count(), 80);
    assert!(rows[1].starts_with("│ Ada Example "));
    assert!(rows[1].contains("  Welcome aboard "));
    assert!(rows[1].ends_with("  2 Oct 2026 │"), "{}", rows[1]);
    assert!(rows[2].ends_with("  23 Aug 2026 │"), "{}", rows[2]);
    assert!(rows[8].starts_with("╰"));
    assert!(rows[9].contains("q quit"));
    // The dates share a right edge and the subjects a left edge.
    assert_eq!(rows[1].find("Welcome"), rows[2].find("Second"));
}

#[test]
fn the_selected_row_is_a_band_across_the_whole_width() {
    let mut list = MessageList::new(numbered(5));
    press(&mut list, &[KeyCode::Down, KeyCode::Down]);
    let terminal = draw(&mut list, 80, 12);
    let buffer = terminal.backend().buffer();
    assert_eq!(selected_row(buffer), Some(3));
    for x in 1..79 {
        assert!(buffer[(x, 3)].modifier.contains(Modifier::REVERSED), "{x}");
    }
}

#[test]
fn missing_things_show_quiet_placeholders() {
    let mut list = MessageList::new(vec![
        message(1, "Real Sender", "Real subject", None),
        message(2, "", "", None),
    ]);
    let terminal = draw(&mut list, 80, 8);
    let rows = lines(&terminal);
    assert!(rows[2].contains("(no sender)"));
    assert!(rows[2].contains("(no subject)"));
    assert!(rows[1].contains("Real subject"));
    let buffer = terminal.backend().buffer();
    // The unselected placeholder is dim and italic; real text is not.
    let x = rows[2]
        .find("(no sender)")
        .map(|b| rows[2][..b].chars().count())
        .unwrap();
    let placeholder = buffer[(x as u16, 2)].modifier;
    assert!(placeholder.contains(Modifier::DIM | Modifier::ITALIC));
    assert_eq!(buffer[(3, 1)].modifier, Modifier::REVERSED);
    // On the selected band it stays visibly a placeholder: italic, and not faded.
    press(&mut list, &[KeyCode::Down]);
    let terminal = draw(&mut list, 80, 8);
    let selected = terminal.backend().buffer()[(x as u16, 2)].modifier;
    assert!(selected.contains(Modifier::ITALIC | Modifier::REVERSED));
    assert!(!selected.contains(Modifier::DIM));
}

#[test]
fn a_message_with_no_date_leaves_the_date_column_empty() {
    let mut list = MessageList::new(vec![message(1, "Ada", "No date here", None)]);
    let rows = lines(&draw(&mut list, 80, 8));
    assert!(
        rows[1]
            .trim_end_matches('│')
            .trim()
            .ends_with("No date here")
    );
}

#[test]
fn selection_moves_and_stops_at_both_ends() {
    let mut list = MessageList::new(numbered(3));
    draw(&mut list, 80, 12);
    assert_eq!(list.selected(), Some(0));
    press(&mut list, &[KeyCode::Up, KeyCode::Char('k')]);
    assert_eq!(list.selected(), Some(0));
    press(&mut list, &[KeyCode::Char('j'), KeyCode::Down]);
    assert_eq!(list.selected(), Some(2));
    press(
        &mut list,
        &[KeyCode::Down, KeyCode::Char('j'), KeyCode::PageDown],
    );
    assert_eq!(list.selected(), Some(2));
    press(&mut list, &[KeyCode::Char('g')]);
    assert_eq!(list.selected(), Some(0));
    press(&mut list, &[KeyCode::End]);
    assert_eq!(list.selected(), Some(2));
    press(&mut list, &[KeyCode::Home]);
    assert_eq!(list.selected(), Some(0));
    press(&mut list, &[KeyCode::Char('G')]);
    assert_eq!(list.selected(), Some(2));
    press(&mut list, &[KeyCode::PageUp]);
    assert_eq!(list.selected(), Some(0));
}

#[test]
fn scrolling_keeps_the_selection_in_view() {
    let mut list = MessageList::new(numbered(30));
    // 10 high: 1 hint row and 2 frame rows leave 7 list rows.
    let mut terminal = draw(&mut list, 80, 10);
    for _ in 0..20 {
        press(&mut list, &[KeyCode::Char('j')]);
        terminal.draw(|frame| list.draw(frame)).unwrap();
        let row = selected_row(terminal.backend().buffer()).expect("the selection is on screen");
        assert!((1..=7).contains(&row), "row {row}");
    }
    assert!(lines(&terminal)[row_of(&terminal, "Subject 21")].contains("Subject 21"));
    // Scrolling back up brings the first rows back.
    press(&mut list, &[KeyCode::Char('g')]);
    let rows = screen(&mut list, 80, 10);
    assert!(rows.contains("Subject 1 "));
}

fn row_of(terminal: &Terminal<TestBackend>, needle: &str) -> usize {
    lines(terminal)
        .iter()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("{needle} is not on screen"))
}

#[test]
fn a_page_moves_by_a_screenful() {
    let mut list = MessageList::new(numbered(30));
    draw(&mut list, 80, 10);
    press(&mut list, &[KeyCode::PageDown]);
    assert_eq!(list.selected(), Some(7));
    press(
        &mut list,
        &[KeyCode::PageDown, KeyCode::PageDown, KeyCode::PageDown],
    );
    assert_eq!(list.selected(), Some(28));
    press(&mut list, &[KeyCode::PageDown]);
    assert_eq!(list.selected(), Some(29));
    press(&mut list, &[KeyCode::PageUp]);
    assert_eq!(list.selected(), Some(22));
}

#[test]
fn a_resize_keeps_the_selection_in_view() {
    let mut list = MessageList::new(numbered(30));
    draw(&mut list, 80, 24);
    press(&mut list, &[KeyCode::Char('G')]);
    let mut terminal = draw(&mut list, 80, 24);
    assert!(selected_row(terminal.backend().buffer()).is_some());
    // Shrink: the selected last row must still be on screen.
    terminal.backend_mut().resize(80, 8);
    assert_eq!(list.handle_event(&Event::Resize(80, 8)), Update::Redraw);
    terminal.draw(|frame| list.draw(frame)).unwrap();
    let row = selected_row(terminal.backend().buffer()).expect("selection on screen");
    assert!((1..=5).contains(&row));
    assert!(lines(&terminal)[usize::from(row)].contains("Subject 30"));
    // Grow: no blank rows below the last message while there is more above.
    terminal.backend_mut().resize(80, 40);
    terminal.draw(|frame| list.draw(frame)).unwrap();
    let rows = lines(&terminal);
    assert!(rows[1].contains("Subject 1 "), "{}", rows[1]);
    assert!(rows[30].contains("Subject 30"));
}

#[test]
fn wide_text_is_cut_by_width_and_never_spills() {
    let mut list = MessageList::new(vec![
        message(
            1,
            "田中花子田中花子田中花子田中花子",
            "日本語の件名がとても長い場合でもカラムを壊してはいけません日本語の件名がとても長い",
            Some(OCT_2_2026),
        ),
        message(
            2,
            "Zoë 🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉",
            "ñ\u{303}\u{303}e\u{301} 🎉 emoji and marks in a long subject line that just keeps going on",
            Some(OCT_2_2026),
        ),
    ]);
    let rows = lines(&draw(&mut list, 80, 8));
    for (y, row) in rows.iter().enumerate().take(3).skip(1) {
        assert!(row.contains('…'), "row {y}: {row}");
        assert!(row.ends_with("2 Oct 2026 │"), "row {y}: {row}");
    }
    // Every row is exactly as wide as the screen, so nothing spilled or shifted.
    let terminal = draw(&mut list, 80, 8);
    let buffer = terminal.backend().buffer();
    for y in 1..3 {
        assert_eq!(buffer[(0, y)].symbol(), "│");
        assert_eq!(buffer[(79, y)].symbol(), "│");
    }
}

#[test]
fn an_empty_list_says_so() {
    let mut list = MessageList::new(Vec::new());
    assert_eq!(list.selected(), None);
    let rows = lines(&draw(&mut list, 60, 11));
    assert!(rows[0].starts_with("╭ Inbox"));
    assert!(!rows[0].contains(" of "));
    let middle = rows
        .iter()
        .find(|r| r.contains("No messages"))
        .expect("No messages");
    assert!(middle.contains("No messages"));
    let start = middle.chars().take_while(|c| *c != 'N').count();
    // Centred between the two borders, give or take one cell.
    assert!((start as i32 - (60 - 11) / 2).abs() <= 1, "{middle}");
    // Keys do nothing, and don't panic.
    press(
        &mut list,
        &[KeyCode::Down, KeyCode::PageDown, KeyCode::Char('G')],
    );
    assert_eq!(list.selected(), None);
}

#[test]
fn a_terminal_that_is_too_small_gets_a_message_and_not_a_broken_layout() {
    let mut list = MessageList::new(numbered(5));
    for (w, h) in [(29, 24), (80, 4), (10, 3), (1, 1)] {
        let rows = lines(&draw(&mut list, w, h));
        assert!(!rows.iter().any(|r| r.contains('╭')), "{w}x{h}");
    }
    let rows = lines(&draw(&mut list, 40, 4));
    assert!(rows.iter().any(|r| r.contains("Terminal too small")));
    assert!(rows.iter().any(|r| r.contains("needs 30 x 5")));
}

#[test]
fn the_smallest_supported_terminal_still_draws_a_list() {
    let mut list = MessageList::new(numbered(5));
    let rows = lines(&draw(&mut list, 30, 5));
    assert!(rows[0].starts_with("╭ Inbox"));
    assert!(rows[1].contains("Sender 1"));
    assert!(rows[1].contains("Subject 1"));
}

#[test]
fn narrow_terminals_drop_the_date_before_the_sender_shrinks() {
    let mut list = MessageList::new(vec![message(
        1,
        "A rather long sender name here",
        "A subject that is quite long indeed",
        Some(OCT_2_2026),
    )]);
    let wide = lines(&draw(&mut list, 61, 6));
    assert!(wide[1].contains("2 Oct 2026"));
    assert!(wide[1].contains("A rather long sender …"), "{}", wide[1]);
    let narrower = lines(&draw(&mut list, 60, 6));
    assert!(!narrower[1].contains("Oct"));
    assert!(
        narrower[1].contains("A rather long sender …"),
        "{}",
        narrower[1]
    );
    let narrowest = lines(&draw(&mut list, 40, 6));
    assert!(
        narrowest[1].contains("A rather long sende…"),
        "{}",
        narrowest[1]
    );
}

#[test]
fn quitting() {
    let mut list = MessageList::new(numbered(3));
    assert_eq!(list.handle_event(&key(KeyCode::Char('q'))), Update::Quit);
    let ctrl_c = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(list.handle_event(&ctrl_c), Update::Quit);
    // A plain c does nothing, and neither does a key with nothing to do.
    assert_eq!(list.handle_event(&key(KeyCode::Char('c'))), Update::Ignored);
    assert_eq!(list.handle_event(&key(KeyCode::Tab)), Update::Ignored);
    assert_eq!(list.handle_event(&Event::FocusGained), Update::Ignored);
}

#[test]
fn key_releases_do_nothing() {
    let mut list = MessageList::new(numbered(3));
    let mut release = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert_eq!(list.handle_event(&Event::Key(release)), Update::Ignored);
    assert_eq!(list.selected(), Some(0));
}

#[test]
fn mail_text_cannot_send_control_characters_to_the_terminal() {
    let mut list = MessageList::new(vec![
        message(
            1,
            "Eve\x1b[2J\x1b]0;owned\x07 \u{9b}31m",
            "Hello\x1b[31m red\nnewline\ttab\r\x08\x7f \u{85}end",
            Some(OCT_2_2026),
        ),
        message(2, "Normal", "Normal", Some(OCT_2_2026)),
    ]);
    // Both when the row is selected and when it is not.
    for selected_first in [true, false] {
        if !selected_first {
            press(&mut list, &[KeyCode::Down]);
        }
        let terminal = draw(&mut list, 100, 10);
        let buffer = terminal.backend().buffer();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                assert!(
                    !symbol.chars().any(char::is_control),
                    "control character at {x},{y}: {symbol:?}"
                );
            }
        }
        let row = &lines(&terminal)[1];
        assert!(row.contains("Hello [31m red newline tab"), "{row}");
    }
}

fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

#[test]
fn each_movement_key_stops_at_its_end() {
    let mut list = MessageList::new(numbered(3));
    draw(&mut list, 80, 12);
    let downward = [
        KeyCode::Down,
        KeyCode::Char('j'),
        KeyCode::PageDown,
        KeyCode::End,
        KeyCode::Char('G'),
    ];
    for code in downward {
        press(&mut list, &[KeyCode::Home]);
        press(&mut list, &[code; 5]);
        assert_eq!(list.selected(), Some(2), "{code:?}");
    }
    let upward = [
        KeyCode::Up,
        KeyCode::Char('k'),
        KeyCode::PageUp,
        KeyCode::Home,
        KeyCode::Char('g'),
    ];
    for code in upward {
        press(&mut list, &[KeyCode::End]);
        press(&mut list, &[code; 5]);
        assert_eq!(list.selected(), Some(0), "{code:?}");
    }
}

#[test]
fn the_arrows_and_j_and_k_move_one_row() {
    let mut list = MessageList::new(numbered(5));
    for (code, expected) in [
        (KeyCode::Down, 1),
        (KeyCode::Char('j'), 2),
        (KeyCode::Up, 1),
        (KeyCode::Char('k'), 0),
    ] {
        press(&mut list, &[code]);
        assert_eq!(list.selected(), Some(expected), "{code:?}");
    }
}

#[test]
fn a_key_that_moves_nothing_asks_for_no_redraw() {
    let mut list = MessageList::new(numbered(3));
    assert_eq!(list.handle_event(&key(KeyCode::Up)), Update::Ignored);
    assert_eq!(list.handle_event(&key(KeyCode::Down)), Update::Redraw);
    assert_eq!(list.handle_event(&key(KeyCode::End)), Update::Redraw);
    assert_eq!(list.handle_event(&key(KeyCode::Down)), Update::Ignored);
    assert_eq!(list.handle_event(&key(KeyCode::End)), Update::Ignored);
    let mut empty = MessageList::new(Vec::new());
    assert_eq!(empty.handle_event(&key(KeyCode::Down)), Update::Ignored);
    assert_eq!(empty.selected(), None);
}

#[test]
fn a_resize_asks_for_a_redraw() {
    let mut list = MessageList::new(numbered(3));
    assert_eq!(list.handle_event(&Event::Resize(40, 10)), Update::Redraw);
}

#[test]
fn a_held_key_keeps_moving() {
    let mut list = MessageList::new(numbered(3));
    let mut repeat = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    assert_eq!(list.handle_event(&Event::Key(repeat)), Update::Redraw);
    assert_eq!(list.selected(), Some(1));
}

#[test]
fn ctrl_and_alt_keys_do_nothing_except_ctrl_c() {
    let mut list = MessageList::new(numbered(3));
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        for code in [KeyCode::Char('j'), KeyCode::Char('q'), KeyCode::Down] {
            let event = with_modifiers(code, modifiers);
            assert_eq!(list.handle_event(&event), Update::Ignored, "{event:?}");
        }
    }
    assert_eq!(list.selected(), Some(0));
    let alt_c = with_modifiers(KeyCode::Char('c'), KeyModifiers::ALT);
    assert_eq!(list.handle_event(&alt_c), Update::Ignored);
    // Shift is not one of them: some terminals report it with a capital letter.
    let shift_g = with_modifiers(KeyCode::Char('G'), KeyModifiers::SHIFT);
    assert_eq!(list.handle_event(&shift_g), Update::Redraw);
    assert_eq!(list.selected(), Some(2));
}

#[test]
fn the_date_is_dim_except_on_the_selected_row() {
    let mut list = MessageList::new(numbered(2));
    let terminal = draw(&mut list, 80, 8);
    let rows = lines(&terminal);
    let buffer = terminal.backend().buffer();
    let date_x = |row: &str| row[..row.find("2026").unwrap()].chars().count() as u16;
    let selected = buffer[(date_x(&rows[1]), 1)].modifier;
    assert!(selected.contains(Modifier::REVERSED) && !selected.contains(Modifier::DIM));
    let other = buffer[(date_x(&rows[2]), 2)].modifier;
    assert!(other.contains(Modifier::DIM) && !other.contains(Modifier::REVERSED));
}

#[test]
fn the_hint_names_the_keys() {
    let mut list = MessageList::new(numbered(2));
    let rows = lines(&draw(&mut list, 80, 8));
    assert_eq!(
        rows[7],
        " q quit · j/k move · g/G top/bottom · PgUp/PgDn page"
    );
}

#[test]
fn the_narrowest_screen_still_says_how_to_quit() {
    let mut list = MessageList::new(numbered(2));
    let rows = lines(&draw(&mut list, tuit_tui::MIN_WIDTH, 8));
    assert!(rows[7].starts_with(" q quit · "), "{}", rows[7]);
    assert!(rows[7].ends_with('…'), "{}", rows[7]);
}

#[test]
fn text_that_would_show_nothing_gets_the_placeholder() {
    // Spaces, control characters, and characters that take no cells.
    for blank in ["   ", "\x1b\x07", "\u{7f}", "\u{200b}\u{200b}", "\u{202e}"] {
        let mut list = MessageList::new(vec![message(1, blank, blank, None)]);
        let rows = lines(&draw(&mut list, 80, 8));
        assert!(rows[1].contains("(no sender)"), "{blank:?}: {}", rows[1]);
        assert!(rows[1].contains("(no subject)"), "{blank:?}: {}", rows[1]);
    }
}

#[test]
fn a_date_too_long_for_its_column_is_left_out() {
    // The year 9999 fits; the far future doesn't, and isn't shown cut short.
    let year_9999 = 253_402_214_400;
    let mut list = MessageList::new(vec![
        message(1, "Ada", "Far future", Some(i64::MAX)),
        message(2, "Ada", "Year 9999", Some(year_9999)),
    ]);
    let rows = lines(&draw(&mut list, 80, 8));
    assert!(
        rows[1].trim_end_matches('│').trim().ends_with("Far future"),
        "{}",
        rows[1]
    );
    assert!(rows[2].contains("31 Dec 9999"), "{}", rows[2]);
}

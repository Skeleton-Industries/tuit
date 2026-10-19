//! Making mail text safe and the right width for a terminal.

use jiff::tz::TimeZone;
use ratatui::buffer::CellWidth;
use tuit_core::{Timestamp, Untrusted};
use unicode_segmentation::UnicodeSegmentation;

use crate::safe::TerminalLinePrefix;

const ELLIPSIS: &str = "…";

/// How much of a piece of text `sanitize_and_fit` ever looks at, in bytes. A mail header can be
/// very long, and a row shows a few hundred cells at most, so the cost of drawing a row doesn't
/// depend on how much text the mail carries.
const LOOK_BYTES: usize = 4096;

/// Mail text made safe and cut to at most `width` terminal cells, with `…` where it was cut.
///
/// Made safe by the same rule as [`crate::TerminalLine`], applied to a prefix; cut as [`fit`]
/// cuts. Only the first `LOOK_BYTES` of `text` are looked at. Longer text counts as cut, so
/// where there is room for a `…` it gets one, even if what was looked at fits.
pub(crate) fn sanitize_and_fit(text: &Untrusted, width: usize) -> String {
    let (text, cut) = text.sanitize_with(&TerminalLinePrefix {
        max_bytes: LOOK_BYTES,
    });
    if cut {
        cut_short(&text, width)
    } else {
        fit(&text, width)
    }
}

/// tuit's own text cut to at most `width` terminal cells, with `…` where it was cut. Mail text
/// goes through [`sanitize_and_fit`], which is the only way in for it.
///
/// Text that fits is returned uncut; text that doesn't keeps one cell for the `…`, and at a
/// `width` of 0 nothing is returned. Cuts fall between grapheme clusters, so a letter keeps its
/// combining marks and an emoji made of several characters stays whole. Widths are counted one
/// cluster at a time with ratatui's own `cell_width`, so the cut agrees with what ratatui draws;
/// a test holds that for a set of awkward text. A cluster that takes no cells by itself (a
/// zero-width space) is dropped, cut or not.
pub(crate) fn fit(text: &str, width: usize) -> String {
    let (start, all) = start_that_fits(text, width);
    if all { start } else { cut_short(text, width) }
}

/// As much of the start of `text` as leaves one cell free, then `…`: at most `width` cells in
/// all. For text there is more of than will be shown, so the `…` goes on even if `text` would
/// fit. Clusters are counted and dropped as in [`fit`]. If `width` is 0 there is no room for
/// the `…` either, and the result is empty.
fn cut_short(text: &str, width: usize) -> String {
    let Some(room) = width.checked_sub(1) else {
        return String::new();
    };
    let (mut start, _) = start_that_fits(text, room);
    start.push_str(ELLIPSIS);
    start
}

/// The clusters from the start of `text` that fit in `width` cells, leaving out any that take
/// no cells, and whether it got to the end of `text`. The loop behind [`fit`] and [`cut_short`].
fn start_that_fits(text: &str, width: usize) -> (String, bool) {
    let mut start = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let cells = usize::from(grapheme.cell_width());
        if cells == 0 {
            continue;
        }
        if used + cells > width {
            return (start, false);
        }
        start.push_str(grapheme);
        used += cells;
    }
    (start, true)
}

/// How many terminal cells `text` takes when drawn, counted one cluster at a time.
pub(crate) fn cells(text: &str) -> usize {
    text.graphemes(true)
        .map(|grapheme| usize::from(grapheme.cell_width()))
        .sum()
}

/// The date of a moment as "2026-10-02", by the calendar of `zone`. The one place a date is
/// turned into text. It never asks the system for a zone: it is told one.
///
/// `None` for a date before the year 0 in `zone`, which no real message has, or after
/// 9999-12-30 22:00:00 UTC, the last moment `jiff` can hold: a store could hand over any
/// number of seconds. Every date returned is 10 cells, and the list relies on that.
pub(crate) fn format_date(moment: Timestamp, zone: &TimeZone) -> Option<String> {
    let moment = jiff::Timestamp::from_second(moment.unix_seconds()).ok()?;
    let date = zone.to_datetime(moment).date();
    let year = u16::try_from(date.year()).ok()?;
    Some(format!("{year:04}-{:02}-{:02}", date.month(), date.day()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::{civil, tz};

    fn sanitize_and_fit(text: &str, width: usize) -> String {
        super::sanitize_and_fit(&Untrusted::new(text), width)
    }

    #[test]
    fn short_text_is_untouched() {
        assert_eq!(sanitize_and_fit("hello", 5), "hello");
    }

    #[test]
    fn long_text_gets_an_ellipsis_within_the_width() {
        assert_eq!(sanitize_and_fit("hello world", 6), "hello…");
    }

    #[test]
    fn wide_characters_are_counted_by_width() {
        // Each of these takes two cells: four cells hold one plus the ellipsis.
        assert_eq!(sanitize_and_fit("日本語の件名", 4), "日…");
        assert_eq!(sanitize_and_fit("日本語の件名", 5), "日本…");
        assert_eq!(sanitize_and_fit("日本語", 6), "日本語");
    }

    #[test]
    fn combining_marks_stay_with_their_letter() {
        let text = "e\u{301}e\u{301}e\u{301}e\u{301}";
        assert_eq!(sanitize_and_fit(text, 3), "e\u{301}e\u{301}…");
    }

    #[test]
    fn an_emoji_made_of_several_characters_is_never_split() {
        // Each family is one cluster of seven characters, two cells wide.
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
        let three = family.repeat(3);
        assert_eq!(sanitize_and_fit(&three, 6), three);
        assert_eq!(sanitize_and_fit(&three, 5), format!("{family}{family}…"));
        assert_eq!(sanitize_and_fit(&three, 4), format!("{family}…"));
    }

    #[test]
    fn text_is_measured_one_cluster_at_a_time() {
        // Arabic lam then alef: one cell when measured as a pair, two when measured apart, which
        // is how the screen draws them. Twelve pairs need 24 cells.
        let pairs = "\u{644}\u{627}".repeat(12);
        assert_eq!(sanitize_and_fit(&pairs, 24), pairs);
        assert_eq!(
            sanitize_and_fit(&pairs, 22),
            format!("{}…", "\u{644}\u{627}".repeat(10) + "\u{644}")
        );
    }

    #[test]
    fn clusters_that_take_no_cells_are_dropped() {
        // A zero-width space and a byte-order mark. The right-to-left override is already gone:
        // the rule for a terminal line removes it.
        assert_eq!(sanitize_and_fit("a\u{200b}b\u{202e}c\u{feff}", 10), "abc");
        assert_eq!(sanitize_and_fit("\u{200b}\u{200b}", 10), "");
    }

    #[test]
    fn very_long_text_is_cut_without_reading_all_of_it() {
        let long = "x".repeat(LOOK_BYTES * 3);
        assert_eq!(sanitize_and_fit(&long, 4), "xxx…");
        // Wider than the part that is looked at: what was looked at, marked as cut.
        let shown = sanitize_and_fit(&long, LOOK_BYTES * 2);
        assert_eq!(shown.chars().count(), LOOK_BYTES + 1);
        assert!(shown.ends_with(ELLIPSIS));
        // The limit never lands inside a character.
        let wide = "日".repeat(LOOK_BYTES);
        assert!(sanitize_and_fit(&wide, LOOK_BYTES * 2).ends_with("日…"));
    }

    #[test]
    fn what_fit_returns_is_drawn_in_full() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::style::Style;

        let awkward = [
            "plain text that is long enough to be cut",
            "日本語の件名です",
            // Halfwidth katakana with sound marks: ratatui gives each mark a cell of its own.
            "ｶﾞｷﾞｸﾞｹﾞｺﾞ ﾊﾟﾋﾟﾌﾟ",
            "\u{644}\u{627}\u{644}\u{627}\u{644}\u{627}",
            "e\u{301}e\u{301}e\u{301}e\u{301}",
            "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} and \u{1f1ec}\u{1f1e7}\u{1f1ef}\u{1f1f5}",
            "नमस्ते! कैसे हैं आप?",
            "a\u{200b}b\u{202e}c\td\x1b[31me",
        ];
        for text in awkward {
            for width in 0..=24 {
                let shown = sanitize_and_fit(text, width);
                assert!(cells(&shown) <= width, "{text:?} at {width}: {shown:?}");
                // Draw it with room to spare, then in exactly `width` cells: the same comes out.
                let drawn = |room: u16| {
                    let mut buf = Buffer::empty(Rect::new(0, 0, room, 1));
                    buf.set_stringn(0, 0, &shown, usize::from(room), Style::new());
                    let row: String = (0..room).map(|x| buf[(x, 0)].symbol()).collect();
                    row.trim_end().to_string()
                };
                assert_eq!(drawn(width as u16), drawn(60), "{text:?} at {width}");
            }
        }
    }

    #[test]
    fn zero_width_gives_nothing() {
        assert_eq!(sanitize_and_fit("abc", 0), "");
    }

    #[test]
    fn text_cut_before_it_arrived_gets_the_ellipsis_if_there_is_room() {
        // It fits with a cell to spare, it fits exactly, and it doesn't fit.
        assert_eq!(cut_short("abc", 5), "abc…");
        assert_eq!(cut_short("abc", 4), "abc…");
        assert_eq!(cut_short("abc", 3), "ab…");
        assert_eq!(cut_short("abc", 1), "…");
        // No room for the ellipsis either.
        assert_eq!(cut_short("abc", 0), "");
        // Nothing that takes a cell is still more text than was shown.
        assert_eq!(cut_short("\u{200b}", 5), "…");
    }

    #[test]
    fn text_longer_than_is_looked_at_gives_nothing_at_zero_width() {
        let long = "a".repeat(LOOK_BYTES * 2);
        assert_eq!(sanitize_and_fit(&long, 0), "");
    }

    #[test]
    fn mail_text_is_made_safe_before_it_is_fitted() {
        assert_eq!(
            sanitize_and_fit("a\x1b[2Jb\nc\td\u{9b}e", 20),
            "a\u{fffd}[2Jb c d\u{fffd}e"
        );
    }

    fn at(seconds: i64, zone: &TimeZone) -> Option<String> {
        format_date(Timestamp::from_unix_seconds(seconds), zone)
    }

    fn utc(seconds: i64) -> Option<String> {
        at(seconds, &TimeZone::UTC)
    }

    /// Seconds for a date and time in UTC.
    fn seconds(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> i64 {
        let moment = civil::date(year, month, day).at(hour, minute, 0, 0);
        TimeZone::UTC.to_timestamp(moment).unwrap().as_second()
    }

    #[test]
    fn dates_are_year_month_day_with_every_part_padded() {
        let shown = |year, month, day| utc(seconds(year, month, day, 9, 30));
        assert_eq!(shown(2026, 10, 2).as_deref(), Some("2026-10-02"));
        assert_eq!(shown(2026, 1, 31).as_deref(), Some("2026-01-31"));
        assert_eq!(shown(2024, 2, 29).as_deref(), Some("2024-02-29"));
        assert_eq!(shown(987, 3, 4).as_deref(), Some("0987-03-04"));
        assert_eq!(shown(0, 1, 1).as_deref(), Some("0000-01-01"));
        assert_eq!(shown(9999, 12, 30).as_deref(), Some("9999-12-30"));
    }

    #[test]
    fn the_day_changes_at_midnight_in_the_zone_given() {
        assert_eq!(utc(0).as_deref(), Some("1970-01-01"));
        assert_eq!(utc(-1).as_deref(), Some("1969-12-31"));
        let plus_two = TimeZone::fixed(tz::offset(2));
        let minus_five = TimeZone::fixed(tz::offset(-5));
        // 23:30 UTC is already the next day two hours east of Greenwich.
        let late = seconds(2026, 10, 2, 23, 30);
        assert_eq!(at(late, &TimeZone::UTC).as_deref(), Some("2026-10-02"));
        assert_eq!(at(late, &plus_two).as_deref(), Some("2026-10-03"));
        assert_eq!(at(late, &minus_five).as_deref(), Some("2026-10-02"));
        // 00:30 UTC is still the day before five hours west of it.
        let early = seconds(2026, 10, 2, 0, 30);
        assert_eq!(at(early, &TimeZone::UTC).as_deref(), Some("2026-10-02"));
        assert_eq!(at(early, &minus_five).as_deref(), Some("2026-10-01"));
        assert_eq!(at(early, &plus_two).as_deref(), Some("2026-10-02"));
    }

    #[test]
    fn a_year_before_zero_has_no_date() {
        assert_eq!(utc(seconds(-1, 1, 1, 12, 0)), None);
        // The first hours of year 0 are still year -1 west of UTC.
        let start_of_year_0 = seconds(0, 1, 1, 0, 0);
        assert_eq!(utc(start_of_year_0).as_deref(), Some("0000-01-01"));
        let minus_five = TimeZone::fixed(tz::offset(-5));
        assert_eq!(at(start_of_year_0, &minus_five), None);
    }

    #[test]
    fn a_moment_jiff_cannot_hold_has_no_date_and_does_not_panic() {
        // The first and last moments `jiff` can hold: -9999-01-02 01:59:59 UTC, which is before
        // the year 0 and so has no date, and 9999-12-30 22:00:00 UTC.
        let first = -377_705_023_201;
        let last = 253_402_207_200;
        assert_eq!(utc(first), None);
        assert_eq!(utc(last).as_deref(), Some("9999-12-30"));
        assert_eq!(utc(first - 1), None);
        assert_eq!(utc(last + 1), None);
        for extreme in [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX] {
            assert_eq!(utc(extreme), None);
            assert_eq!(at(extreme, &TimeZone::fixed(tz::offset(14))), None);
        }
    }
}

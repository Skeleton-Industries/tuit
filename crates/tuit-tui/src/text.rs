//! Making mail text safe and the right width for a terminal.

use ratatui::buffer::CellWidth;
use unicode_segmentation::UnicodeSegmentation;

const ELLIPSIS: &str = "…";

/// How much of a piece of text `sanitize_and_fit` ever looks at, in bytes. A mail header can be
/// very long, and a row shows a few hundred cells at most, so the cost of drawing a row doesn't
/// depend on how much text the mail carries.
const LOOK_BYTES: usize = 4096;

/// Replaces every control character (C0, DEL and C1: newline, tab, and the escape that starts a
/// terminal sequence) with a space. The rest of a sequence is left behind as plain, harmless
/// text.
pub(crate) fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `text` made safe and cut to at most `width` terminal cells, with `…` where it was cut.
///
/// Cuts fall between grapheme clusters, so a letter keeps its combining marks and an emoji made
/// of several characters stays whole. Widths are counted one cluster at a time with ratatui's own
/// `cell_width`, so the cut agrees with what ratatui draws; a test holds that for a set of
/// awkward text. A cluster that takes no cells by itself (a zero-width
/// space, a text-direction override) is dropped. Only the first `LOOK_BYTES` of `text` are
/// looked at; longer text counts as cut.
pub(crate) fn sanitize_and_fit(text: &str, width: usize) -> String {
    let end = text.floor_char_boundary(LOOK_BYTES);
    let mut cut = end < text.len();
    let text = sanitize(&text[..end]);
    let mut out = String::new();
    let mut used = 0;
    // How much of `out` to keep if the ellipsis has to go in after it.
    let mut keep = 0;
    for grapheme in text.graphemes(true) {
        let cells = usize::from(grapheme.cell_width());
        if cells == 0 {
            continue;
        }
        if used + cells > width {
            cut = true;
            break;
        }
        out.push_str(grapheme);
        used += cells;
        if used < width {
            keep = out.len();
        }
    }
    if !cut {
        return out;
    }
    if width == 0 {
        return String::new();
    }
    out.truncate(keep);
    out.push_str(ELLIPSIS);
    out
}

/// How many terminal cells `text` takes when drawn, counted one cluster at a time.
pub(crate) fn cells(text: &str) -> usize {
    text.graphemes(true)
        .map(|grapheme| usize::from(grapheme.cell_width()))
        .sum()
}

/// A short month name for 1 to 12.
fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES[(month as usize).saturating_sub(1).min(11)]
}

/// A date as "2 Oct 2026".
pub(crate) fn format_date((year, month, day): (i64, u32, u32)) -> String {
    format!("{day} {} {year}", month_name(month))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // A zero-width space, a right-to-left override and a byte-order mark.
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
    fn controls_become_spaces() {
        assert_eq!(sanitize("a\x1b[2Jb\nc\td\u{9b}e"), "a [2Jb c d e");
    }

    #[test]
    fn dates_are_day_month_year() {
        let names = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        for (month, name) in (1..).zip(names) {
            assert_eq!(format_date((2026, month, 2)), format!("2 {name} 2026"));
        }
        assert_eq!(format_date((2026, 1, 31)), "31 Jan 2026");
    }
}

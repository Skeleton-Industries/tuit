//! Made-up messages, which `tuit --sample` shows in place of a real Maildir. Everything here is
//! invented, and the dates are fixed so the screen looks the same on any day and in any zone.
//! The set is a test card for the look: long and empty fields, wide and combining characters, an
//! emoji, a missing date, and dates spread over more than a year. Newest first with the undated
//! one last, as the core lists them.

use jiff::civil;
use jiff::tz::TimeZone;
use tuit_core::{MessageId, MessageSummary, Timestamp, Untrusted};

/// The moment 09:30 on a calendar date in `zone`, so the date shown in that zone is the one
/// written. Panics if the date doesn't exist.
fn at(zone: &TimeZone, year: i16, month: i8, day: i8) -> Timestamp {
    let moment = zone
        .to_timestamp(civil::date(year, month, day).at(9, 30, 0, 0))
        .expect("the sample dates are real");
    Timestamp::from_unix_seconds(moment.as_second())
}

/// Sender, subject and a (year, month, day) if there is a date.
type Row = (&'static str, &'static str, Option<(i16, i8, i8)>);

/// About thirty made-up messages, newest first, dated for `zone`, the zone the list will show
/// them in.
pub fn messages(zone: &TimeZone) -> Vec<MessageSummary> {
    let rows: [Row; 30] = [
        ("Priya Raman", "Lunch on Thursday?", Some((2026, 10, 1))),
        (
            "Harbour Books",
            "Your order has shipped: three paperbacks and a very long note about delivery windows, parcel lockers and what to do if nobody is home",
            Some((2026, 9, 29)),
        ),
        (
            "Tomás Alvarez",
            "Re: Re: Fwd: Quarterly figures",
            Some((2026, 9, 27)),
        ),
        ("", "Is this thing on?", Some((2026, 9, 24))),
        (
            "The Committee for the Preservation of Unreasonably Long Display Names",
            "Minutes of the last meeting",
            Some((2026, 9, 19)),
        ),
        ("Mina Kowalczyk", "", Some((2026, 9, 12))),
        ("田中 花子", "来週の打ち合わせについて", Some((2026, 9, 8))),
        (
            "Riverside Library",
            "Two items are due back on Monday",
            Some((2026, 8, 30)),
        ),
        ("Анна Петрова", "Фотографии из поездки", Some((2026, 8, 21))),
        (
            "Sam Okafor",
            "Party planning 🎉 (and a cake question)",
            Some((2026, 8, 14)),
        ),
        (
            "noreply@example.org",
            "Your monthly statement is ready",
            Some((2026, 8, 3)),
        ),
        (
            "Jonas Berg",
            "Draft of the talk, with comments in the margins",
            Some((2026, 7, 22)),
        ),
        ("प्रिया शर्मा", "नमस्ते! कैसे हैं आप?", Some((2026, 7, 11))),
        (
            "Cleo Navarro",
            "Re: the thing we talked about",
            Some((2026, 6, 30)),
        ),
        (
            "Lakeside Cycling Club",
            "Saturday ride moved to Sunday",
            Some((2026, 6, 17)),
        ),
        (
            "Σοφία Παπαδοπούλου",
            "Καλή επιτυχία στη νέα δουλειά",
            Some((2026, 5, 29)),
        ),
        (
            "Wei Chen",
            "Notes from the design review",
            Some((2026, 5, 9)),
        ),
        ("Ola Nordmann", "Skål! Recipe inside", Some((2026, 4, 18))),
        (
            "billing@example.org",
            "Invoice 2026-0412",
            Some((2026, 4, 2)),
        ),
        (
            "Maren Holt",
            "Can you take a look at this before Friday",
            Some((2026, 3, 14)),
        ),
        ("Théo Lefèvre", "Réunion annulée", Some((2026, 2, 26))),
        ("Ikenna Eze", "Welcome to the team", Some((2026, 2, 5))),
        (
            "Alder Street Surgery",
            "Appointment reminder",
            Some((2026, 1, 20)),
        ),
        ("Hannah Brandt", "Happy new year", Some((2026, 1, 1))),
        ("김민준", "회의 자료 공유드립니다", Some((2025, 12, 12))),
        (
            "Gus Pemberton",
            "Photos from the weekend",
            Some((2025, 11, 2)),
        ),
        (
            "Northfield Allotments",
            "Plot 14 is yours from March",
            Some((2025, 9, 18)),
        ),
        ("Rosa Delgado", "Thanks again", Some((2025, 8, 7))),
        ("Archive Bot", "Welcome", Some((2025, 6, 24))),
        ("Dr. Evelyn Marchetti-Whitlock", "Referral letter", None),
    ];
    rows.into_iter()
        .enumerate()
        .map(|(n, (from, subject, date))| MessageSummary {
            id: MessageId::new(format!("sample-{n}")),
            from: Untrusted::new(from),
            subject: Untrusted::new(subject),
            date: date.map(|(y, m, d)| at(zone, y, m, d)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_come_out_as_written() {
        // The far west and the far east of the world's zones, and UTC between them.
        for hours in [-12, 0, 14] {
            let zone = TimeZone::fixed(jiff::tz::offset(hours));
            let date = |moment: Timestamp| {
                let moment = jiff::Timestamp::from_second(moment.unix_seconds()).unwrap();
                zone.to_datetime(moment)
            };
            let at = |year, month, day| date(at(&zone, year, month, day));
            assert_eq!(at(2026, 10, 2).date(), civil::date(2026, 10, 2), "{hours}");
            assert_eq!(at(2025, 6, 24).date(), civil::date(2025, 6, 24), "{hours}");
            assert_eq!(at(2024, 2, 29).date(), civil::date(2024, 2, 29), "{hours}");
            assert_eq!(at(2026, 1, 1).date(), civil::date(2026, 1, 1), "{hours}");
            assert_eq!(at(2026, 10, 2).time(), civil::time(9, 30, 0, 0), "{hours}");
        }
    }

    #[test]
    fn the_test_card_covers_what_it_should() {
        let all = messages(&TimeZone::UTC);
        assert_eq!(all.len(), 30);
        assert!(all.iter().any(|m| m.from.is_empty()));
        assert!(all.iter().any(|m| m.subject.is_empty()));
        assert!(all.iter().any(|m| m.date.is_none()));
        assert!(all.iter().any(|m| {
            m.from
                .sanitize_with(&tuit_tui::TerminalLine)
                .chars()
                .count()
                > 60
        }));
        assert!(all.iter().any(|m| {
            m.subject
                .sanitize_with(&tuit_tui::TerminalLine)
                .chars()
                .count()
                > 100
        }));
        let text = |m: &MessageSummary| {
            format!(
                "{}{}",
                m.from.sanitize_with(&tuit_tui::TerminalLine),
                m.subject.sanitize_with(&tuit_tui::TerminalLine)
            )
        };
        assert!(all.iter().any(|m| text(m).contains('田')), "wide");
        assert!(all.iter().any(|m| text(m).contains('\u{93f}')), "combining");
        assert!(
            all.iter()
                .any(|m| text(m).chars().any(|c| c >= '\u{1f300}')),
            "emoji"
        );
        // `None` compares below any date, so this also says the undated one is last.
        assert!(
            all.windows(2).all(|w| w[0].date >= w[1].date),
            "newest first"
        );
        let dated: Vec<_> = all.iter().filter_map(|m| m.date).collect();
        let span = dated[0].unix_seconds() - dated[dated.len() - 1].unix_seconds();
        assert!(span > 365 * 86_400);
    }
}

//! What a message is: its name in a store, a moment in time, and what a message
//! list shows.

use crate::untrusted::Untrusted;

/// A store's own name for one message. Only the store that issued it can interpret it.
///
/// It can come from a file name, so it can contain anything. It is a key, not something to
/// show: it can be compared, ordered and hashed, and has no `Display` and no method that reads it
/// as text. `Debug` shows it with control characters escaped. A store that has to find a message
/// again will need to read it; that method is added here then, on purpose.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(String);

impl MessageId {
    /// Wraps a store's name for a message.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// A moment in time: whole seconds since 1970-01-01 00:00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Makes a timestamp from seconds since 1970-01-01 00:00:00 UTC. Negative
    /// values are before 1970.
    pub fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Seconds since 1970-01-01 00:00:00 UTC.
    pub fn unix_seconds(self) -> i64 {
        self.0
    }

    /// The calendar date in UTC as (year, month 1-12, day 1-31). The calendar
    /// is today's, run backwards without limit, so the year can be zero or
    /// negative.
    pub fn utc_date(self) -> (i64, u32, u32) {
        // Howard Hinnant's "civil from days" algorithm, on a calendar that
        // starts on 1 March so the leap day is the last day of the year.
        let days = self.0.div_euclid(86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 {
            month_from_march + 3
        } else {
            month_from_march - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        (year, month as u32, day as u32)
    }
}

/// What a message list shows for one message.
///
/// `from` and `subject` are whatever the message said. They can contain
/// anything, including the control characters that drive a terminal, so they
/// are [`Untrusted`]: they can only be shown once made safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSummary {
    /// The store's name for the message.
    pub id: MessageId,
    /// The sender's name if the message gives one, otherwise their address. Empty if neither.
    pub from: Untrusted,
    /// Empty if the message has no subject.
    pub subject: Untrusted,
    /// `None` if the message has no date or the date can't be read.
    pub date: Option<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(seconds: i64) -> (i64, u32, u32) {
        Timestamp::from_unix_seconds(seconds).utc_date()
    }

    #[test]
    fn ids_compare_and_order_by_their_text() {
        assert_eq!(MessageId::new("abc:2,S"), MessageId::new("abc:2,S"));
        assert_ne!(MessageId::new("abc:2,S"), MessageId::new("abc"));
        assert!(MessageId::new("a") < MessageId::new("b"));
    }

    #[test]
    fn an_id_printed_for_debugging_has_its_control_characters_escaped() {
        let shown = format!("{:?}", MessageId::new("a\x1b[2Jb"));
        assert!(!shown.contains('\x1b'), "{shown}");
    }

    #[test]
    fn timestamp_round_trips() {
        assert_eq!(Timestamp::from_unix_seconds(-5).unix_seconds(), -5);
    }

    #[test]
    fn the_epoch() {
        assert_eq!(date(0), (1970, 1, 1));
    }

    #[test]
    fn the_last_second_before_the_epoch() {
        assert_eq!(date(-1), (1969, 12, 31));
    }

    #[test]
    fn the_last_second_of_a_day() {
        assert_eq!(date(86_399), (1970, 1, 1));
        assert_eq!(date(86_400), (1970, 1, 2));
    }

    #[test]
    fn a_leap_day() {
        // 2000-02-29 00:00:00 UTC
        assert_eq!(date(951_782_400), (2000, 2, 29));
        assert_eq!(date(951_782_400 + 86_399), (2000, 2, 29));
        assert_eq!(date(951_782_400 + 86_400), (2000, 3, 1));
    }

    #[test]
    fn a_century_that_is_not_a_leap_year() {
        // 1900-03-01 00:00:00 UTC follows 1900-02-28.
        assert_eq!(date(-2_203_891_200), (1900, 3, 1));
        assert_eq!(date(-2_203_891_201), (1900, 2, 28));
    }

    #[test]
    fn a_recent_date() {
        // 2026-10-01 00:00:00 UTC
        assert_eq!(date(1_790_812_800), (2026, 10, 1));
    }

    #[test]
    fn a_date_long_before_1970() {
        // 1600-01-01 00:00:00 UTC
        assert_eq!(date(-11_676_096_000), (1600, 1, 1));
    }

    #[test]
    fn timestamps_order_by_time() {
        assert!(Timestamp::from_unix_seconds(1) < Timestamp::from_unix_seconds(2));
    }
}

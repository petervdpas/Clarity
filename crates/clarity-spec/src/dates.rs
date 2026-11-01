//! A date or a date and time written out, as a field's default is written
//! and as the backends send them: `2026-10-20`, `2026-10-20T14:05:00`
//! (also with a space, or without seconds). Every generator writes a
//! default from these parts, so a default means the same on every stack.

/// The parts of a date, and of its time of day (zero for a date).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// What a date default must look like, for the checker's message.
pub const DATE_SHAPE: &str = "2026-10-17";
/// What a date and time default must look like.
pub const DATE_TIME_SHAPE: &str = "2026-10-17T09:30:00";

fn number(text: &str, digits: usize) -> Option<u32> {
    (text.len() == digits && text.bytes().all(|b| b.is_ascii_digit())).then(|| text.parse().ok()).flatten()
}

fn days_in(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `2026-10-20`: a day of the calendar; none for anything else.
pub fn date(text: &str) -> Option<Moment> {
    let mut parts = text.split('-');
    let (year, month, day) = (number(parts.next()?, 4)?, number(parts.next()?, 2)?, number(parts.next()?, 2)?);
    if parts.next().is_some() || !(1..=12).contains(&month) || day == 0 || day > days_in(year as i32, month) {
        return None;
    }
    Some(Moment { year: year as i32, month, day, hour: 0, minute: 0, second: 0 })
}

/// `2026-10-20T14:05:00`, with a space for the T or without seconds; none for anything else.
pub fn date_time(text: &str) -> Option<Moment> {
    let (day, time) = text.split_once(['T', ' '])?;
    let day = date(day)?;
    let mut parts = time.split(':');
    let (hour, minute) = (number(parts.next()?, 2)?, number(parts.next()?, 2)?);
    let second = match parts.next() {
        Some(s) => number(s, 2)?,
        None => 0,
    };
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(Moment { hour, minute, second, ..day })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_read_as_the_backends_send_it() {
        assert_eq!(date("2026-10-20"), Some(Moment { year: 2026, month: 10, day: 20, hour: 0, minute: 0, second: 0 }));
        assert_eq!(date("2028-02-29").map(|m| m.day), Some(29));
        for wrong in ["", "2026-10-2", "20-10-2026", "2026-13-01", "2026-02-29", "2026-04-31", "2026-10-20T10:00", "today"] {
            assert_eq!(date(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn a_date_and_time_is_read_with_or_without_seconds() {
        assert_eq!(date_time("2026-10-20T14:05:00"), Some(Moment { year: 2026, month: 10, day: 20, hour: 14, minute: 5, second: 0 }));
        assert_eq!(date_time("2026-10-20 14:05").map(|m| (m.hour, m.minute, m.second)), Some((14, 5, 0)));
        for wrong in ["2026-10-20", "2026-10-20T24:00", "2026-10-20T10:60", "2026-10-20T10:00:00:00", "2026-10-20T9:30"] {
            assert_eq!(date_time(wrong), None, "{wrong}");
        }
    }
}

//! Numbers, dates and times as every format reads and writes them, exactly:
//! a number is turned from the text it was written in, never through a
//! floating point number, so a decimal keeps every digit and every stack
//! gives the same value.

/// A JSON number as plain decimal text, without an exponent: `1e3` is
/// `1000`, `1.5e-2` is `0.015`, `12.50` stays `12.50`. None for what is no number.
pub fn plain(number: &str) -> Option<String> {
    let (negative, rest) = match number.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, number.strip_prefix('+').unwrap_or(number)),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i64>().ok()?),
        None => (rest, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) || (mantissa.contains('.') && fraction.is_empty()) {
        return None;
    }
    if exponent.unsigned_abs() > 4096 {
        // A number past any field's reach, written to make the text huge.
        return None;
    }
    let digits = format!("{whole}{fraction}");
    let point = whole.len() as i64 + exponent;
    let (whole, fraction) = if point <= 0 {
        ("0".to_string(), format!("{}{digits}", "0".repeat((-point) as usize)))
    } else if point as usize >= digits.len() {
        (format!("{digits}{}", "0".repeat(point as usize - digits.len())), String::new())
    } else {
        (digits[..point as usize].to_string(), digits[point as usize..].to_string())
    };
    // An exponent moved the point: the zeros it leaves at the ends are not written.
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = if exponent != 0 { fraction.trim_end_matches('0').to_string() } else { fraction };
    let sign = if negative { "-" } else { "" };
    Some(if fraction.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") })
}

/// A decimal in its shortest exact form, as every stack writes it whatever
/// its decimal type keeps: no zeros at the end of the fraction (`12.50` is
/// `12.5`, `2.00` is `2`), and `-0` is `0`. None for what is no decimal.
pub fn shortest(decimal: &str) -> Option<String> {
    let plain = plain(decimal.trim())?;
    let text = match plain.split_once('.') {
        Some((whole, fraction)) => {
            let fraction = fraction.trim_end_matches('0');
            if fraction.is_empty() { whole.to_string() } else { format!("{whole}.{fraction}") }
        }
        None => plain,
    };
    Some(if text == "-0" { "0".to_string() } else { text })
}

/// A JSON number as a whole number in the 64-bit range, as text; none when
/// it has a fraction (other than zeros) or is past the range.
pub fn integer(number: &str) -> Option<String> {
    let plain = plain(number)?;
    let whole = match plain.split_once('.') {
        Some((whole, fraction)) if fraction.bytes().all(|b| b == b'0') => whole.to_string(),
        Some(_) => return None,
        None => plain,
    };
    integer_text(&whole)
}

/// A string of a whole number in the 64-bit range, as text (`+7` is `7`, `-0` is `0`).
pub fn integer_text(text: &str) -> Option<String> {
    text.parse::<i64>().ok().map(|n| n.to_string())
}

/// Whether a text is a decimal as Clarity writes one: `-12.50`, `3`.
pub fn is_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    !whole.is_empty() && whole.bytes().all(|b| b.is_ascii_digit()) && (fraction.is_empty() && !digits.ends_with('.') || fraction.bytes().all(|b| b.is_ascii_digit()) && !fraction.is_empty())
}

/// Whether a text is a date, `YYYY-MM-DD`.
pub fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && [0, 1, 2, 3, 5, 6, 8, 9].iter().all(|&i| b[i].is_ascii_digit())
}

/// A date and time as Clarity writes it, `YYYY-MM-DDTHH:MM:SS`, from the
/// same with or without seconds, a fraction of a second and a `Z`.
pub fn date_time(text: &str) -> Option<String> {
    let text = text.strip_suffix('Z').unwrap_or(text);
    let (date, time) = text.split_once(['T', ' '])?;
    if !is_date(date) {
        return None;
    }
    let time = time.split('.').next()?;
    let parts: Vec<&str> = time.split(':').collect();
    let two = |p: &str| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [h, m] if two(h) && two(m) => Some(format!("{date}T{h}:{m}:00")),
        [h, m, s] if two(h) && two(m) && two(s) => Some(format!("{date}T{h}:{m}:{s}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_becomes_plain_decimal_text_exactly() {
        for (given, wanted) in [("12.50", "12.50"), ("1e3", "1000"), ("1.5e-2", "0.015"), ("-2.5E+1", "-25"), ("0.1", "0.1"), ("123456789012345678901234.5", "123456789012345678901234.5"), ("7", "7"), ("1.20e1", "12")] {
            assert_eq!(plain(given).as_deref(), Some(wanted), "{given}");
        }
        assert_eq!(plain("1e99999"), None);
    }

    #[test]
    fn a_decimal_is_written_in_its_shortest_exact_form() {
        for (given, wanted) in [("12.50", "12.5"), ("2.00", "2"), ("-0.0", "0"), ("1e3", "1000"), ("0.10", "0.1"), ("7", "7")] {
            assert_eq!(shortest(given).as_deref(), Some(wanted), "{given}");
        }
    }

    #[test]
    fn a_whole_number_is_in_the_64_bit_range_without_a_fraction() {
        assert_eq!(integer("3.0").as_deref(), Some("3"));
        assert_eq!(integer("1e3").as_deref(), Some("1000"));
        assert_eq!(integer("9223372036854775807").as_deref(), Some("9223372036854775807"));
        assert_eq!(integer("9223372036854775808"), None);
        assert_eq!(integer("2.5"), None);
        assert_eq!(integer_text("+7").as_deref(), Some("7"));
        assert_eq!(integer_text("-0").as_deref(), Some("0"));
    }
}

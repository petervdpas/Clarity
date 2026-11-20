//! Values shown with a format written out, as a report's `format` says it
//! (docs/ledger/reports-pdf.md), in the language's separators and month names.

use clarity_runtime::shown::{FormatKind, Formats};
use serde_json::json;

#[test]
fn a_format_written_out_shows_numbers_and_dates_in_the_language() {
    let nl = Formats::new(Some("nl-NL"), Some("EUR"));
    let en = Formats::new(Some("en-US"), Some("USD"));
    assert_eq!(nl.show_as(FormatKind::Money, &json!("1234.5"), "0.00"), "1234,50");
    assert_eq!(nl.show_as(FormatKind::Money, &json!("1234.5"), "#,##0.00"), "1.234,50");
    assert_eq!(en.show_as(FormatKind::Decimal, &json!("1234.567"), "#,##0.0"), "1,234.6");
    assert_eq!(en.show_as(FormatKind::Decimal, &json!("0.255"), "0%"), "26%");
    assert_eq!(nl.show_as(FormatKind::Date, &json!("2026-11-19"), "dd-MM-yyyy"), "19-11-2026");
    assert_eq!(nl.show_as(FormatKind::Date, &json!("2026-11-19"), "d MMMM yyyy"), "19 november 2026");
    assert_eq!(en.show_as(FormatKind::DateTime, &json!("2026-11-19T14:05:00"), "yyyy-MM-dd HH:mm"), "2026-11-19 14:05");
    assert_eq!(en.show_as(FormatKind::Date, &json!("not a date"), "dd-MM-yyyy"), "not a date");
}

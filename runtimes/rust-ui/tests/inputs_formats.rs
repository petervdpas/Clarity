//! Showing and reading values in the person's locale: every case of
//! conformance/formats.json that shows a value or reads typed text.

use clarity_ui::parts::inputs::formats::{FieldFormat, Formats};
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/formats.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/formats.json")).expect("it is JSON")
}

fn formats_of(case: &Value) -> Formats {
    Formats::new(case["locale"].as_str(), case["currency"].as_str())
}

#[test]
fn every_value_shows_as_on_every_renderer() {
    let cases = cases();
    let mut wrong = Vec::new();
    for case in cases["shows"].as_array().unwrap() {
        let field = FieldFormat::of_json(&case["field"]).expect("a field format");
        let shown = formats_of(case).show(&field, &case["value"]);
        if shown != case["shows"].as_str().unwrap() {
            wrong.push(format!("{} {}: {shown:?}, not {:?}", case["locale"], case["what"], case["shows"]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_typed_text_reads_as_on_every_renderer() {
    let cases = cases();
    let mut wrong = Vec::new();
    for case in cases["reads"].as_array().unwrap() {
        let field = FieldFormat::of_json(&case["field"]).expect("a field format");
        let read = formats_of(case).read(&field, case["typed"].as_str().unwrap()).unwrap_or(Value::Null);
        if read != case["value"] {
            wrong.push(format!("{} {} {:?}: {read}, not {}", case["locale"], case["what"], case["typed"], case["value"]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn money_in_a_number_input_shows_without_its_sign() {
    let formats = Formats::new(Some("nl-NL"), Some("EUR"));
    let money = FieldFormat::parse("money").unwrap();
    assert_eq!("1.250,50", formats.edit_text(&money, &Value::String("1250.5".into())));
}

#[test]
fn a_field_format_reads_from_its_written_words() {
    let field = FieldFormat::parse("decimal 2 percent").unwrap();
    assert_eq!(Some(2), field.scale);
    assert_eq!(Some("percent"), field.format.as_deref());
    assert!(FieldFormat::parse("text").is_none());
}

#[test]
fn a_typed_day_reads_in_the_locale_s_order() {
    assert_eq!(Some((2026, 10, 17)), Formats::new(Some("nl-NL"), None).read_date("17-10-2026"));
    assert_eq!(Some((2026, 10, 17)), Formats::new(Some("en-US"), None).read_date("10/17/2026"));
    assert_eq!(Some((2026, 10, 17)), Formats::new(Some("de-DE"), None).read_date("2026-10-17"));
    assert_eq!(None, Formats::new(Some("nl-NL"), None).read_date("31-02-2026"));
}

#[test]
fn a_calendar_has_the_locale_s_days_and_titles() {
    let us = Formats::new(Some("en-US"), None);
    assert_eq!(0, us.first_weekday());
    assert_eq!("Sun", us.weekdays()[0]);
    assert_eq!("October 2026", us.month_title(2026, 10));
    let nl = Formats::new(Some("nl-NL"), None);
    assert_eq!(1, nl.first_weekday());
    assert_eq!(["ma", "di", "wo", "do", "vr", "za", "zo"], nl.weekdays());
    assert_eq!("oktober 2026", nl.month_title(2026, 10));
    assert_eq!("2026\u{5e74}2\u{6708}", Formats::new(Some("ja-JP"), None).month_title(2026, 2));
}

#[test]
fn a_long_date_reads_too() {
    assert_eq!(Some((2026, 10, 21)), Formats::new(Some("en-US"), None).read_date("October 21, 2026"));
    assert_eq!(Some((2026, 10, 21)), Formats::new(Some("nl-NL"), None).read_date("21 oktober 2026"));
    assert_eq!(Some((2026, 3, 2)), Formats::new(Some("de-DE"), None).read_date("2. M\u{e4}rz 2026"));
}

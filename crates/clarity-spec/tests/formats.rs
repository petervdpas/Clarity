//! Named formats (I1a): `format` names how a field is shown, from the
//! formats its type has.

use clarity_spec::dictionary::{FieldSpec, validate_field};
use clarity_spec::model::Modifiers;
use clarity_spec::{Preview, Severity, load_sources};

fn errors(source: &str) -> Vec<String> {
    let text = format!("module shop\n\nentity Thing\n{source}");
    load_sources([("shop.clarity", text.as_str())])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_field_names_a_format_of_its_type() {
    let ok = "    since: date, format long\n    at: datetime, format iso\n    rate: decimal(6, 4), format percent\n    year: integer, format plain\n    price: money, format plain\n";
    assert!(errors(ok).is_empty(), "{:#?}", errors(ok));
    assert_eq!(
        errors("    since: date, format percent\n"),
        ["unknown format 'percent' for field 'since'; it can be short, long or iso"]
    );
    assert_eq!(errors("    year: integer, format long\n"), ["unknown format 'long' for field 'year'; it can be plain"]);
    assert_eq!(errors("    name: text, format short\n"), ["field 'name' is shown one way only; it takes no format"]);
    // A format is a name, not text.
    assert_eq!(errors("    since: date, format \"dd-MM\"\n"), ["expected a format name, such as short or plain"]);
}

#[test]
fn the_dictionary_writes_a_format_as_a_name() {
    let spec = FieldSpec {
        name: "since".into(),
        type_text: "date".into(),
        modifiers: Modifiers {
            format: Some("long".into()),
            ..Modifiers::default()
        },
    };
    assert_eq!(validate_field(&spec).unwrap(), "since: date, format long");
}

#[test]
fn the_designer_gets_the_formats_of_each_type() {
    let preview = Preview::new(load_sources([("shop.clarity", "module shop\n")]));
    let names = |kind: &str| preview.formats[kind].iter().map(|f| f.name).collect::<Vec<_>>();
    assert_eq!(names("date"), ["short", "long", "iso"]);
    assert_eq!(names("decimal"), ["plain", "percent"]);
    assert!(!preview.formats.contains_key("text"));
}

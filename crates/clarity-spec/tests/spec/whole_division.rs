//! Whole-number division: `a div b` is the quotient of two whole numbers,
//! rounded toward zero, and `a mod b` what is left, with the sign of `a`;
//! both are whole numbers, as `/` always gives a decimal.

use clarity_spec::{Severity, load_sources};

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn code(body: &str) -> String {
    format!("module m\n\nfunction half(width: integer, places: integer) -> integer\n{body}\n")
}

#[test]
fn div_and_mod_give_whole_numbers() {
    let ok = code("    let middle = width div 2\n    let left = width mod places\n    let mod = 3\n    return middle + left * 2 div 3 - mod");
    assert!(errors(&ok).is_empty(), "{:#?}", errors(&ok));
    // A decimal is no whole number: `/` divides those.
    let decimal = errors(&code("    return width div 2.5"));
    assert!(decimal.iter().any(|e| e.contains("div") && e.contains("whole")), "{decimal:#?}");
    let modded = errors(&code("    return 2.5 mod places"));
    assert!(modded.iter().any(|e| e.contains("mod") && e.contains("whole")), "{modded:#?}");
    // `/` stays a decimal, so it is not an integer result.
    let slash = errors(&code("    return width / 2"));
    assert!(!slash.is_empty(), "width / 2 is a decimal");
}

#[test]
fn div_and_mod_bind_as_times_does_and_print_back() {
    let project = load_sources([("m.clarity", code("    return 1 + width div 2 * 3").as_str())]);
    let f = &project.files[0].functions[0];
    let clarity_spec::code::StmtKind::Return { value: Some(value) } = &f.body[0].kind else { panic!("{:?}", f.body[0]) };
    assert_eq!("1 + width div 2 * 3", clarity_spec::code::print::expr(value));
    let clarity_spec::code::ExprKind::Binary { op, .. } = &value.kind else { panic!() };
    assert_eq!(clarity_spec::code::BinaryOp::Add, *op, "div binds as tight as times");
}

//! A number box shows its value in the person's format and reads typed text
//! the same way (conformance/formats.json): every reading case of the shared
//! file, typed into a real number box; a value without a format keeps its
//! text; what code sets shows and is no change. A spin box against the cases
//! every stack shares (conformance/ranges.json): its keys move it a step, or
//! ten, within its range; a typed number is kept within it; only a value
//! that differs is a change; the arrows spin it.

use std::rc::Rc;

use clarity_runtime::ranges::RangeKey;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::formats::{Exact, FieldFormat, FormatKind};
use clarity_ui::parts::inputs::number::NumberPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::Screen;
use serde_json::{json, Value};

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    screen
}

fn number<'a>(screen: &'a Screen, name: &str) -> &'a NumberPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<NumberPart>()).expect("a number part")
}

/// Draws the screen again, as after a change: the parts show what the values are now.
fn redraw(screen: &mut Screen) {
    Shot::of(screen, 800, 400, 1.0);
}

/// Types `text` into the part in place of what it shows, and leaves it.
fn type_in(screen: &mut Screen, name: &str, text: &str) {
    let id = part_named(screen, name);
    screen.focus(Some(id));
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().editor_mut().select_all());
    if text.is_empty() {
        screen.key_pressed(KeyInput::plain(Key::Backspace));
    } else {
        screen.text_typed(text);
    }
    screen.focus(None);
}

fn changes(host: &InputsHost) -> Vec<String> {
    host.logged().into_iter().filter(|l| l.starts_with("run ")).collect()
}

fn facts(format: &str) -> FieldFacts {
    FieldFacts { label: "Amount".into(), format: Some(format.into()), ..FieldFacts::default() }
}

fn number_of(format: &str, value: Value, locale: &str, currency: Option<&str>) -> (Screen, Rc<InputsHost>) {
    let host = InputsHost::new();
    host.use_formats(locale, currency);
    host.facts("amount", facts(format));
    host.set("amount", value);
    let screen = window(r#"<Number at="1,1" span="4" name="amount" bind="amount" onchange="changed" />"#, host.clone());
    (screen, host)
}

fn cases(file: &str) -> Value {
    let path = format!("{}/../../conformance/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file")).expect("it is JSON")
}

/// A field format as `FieldFacts::format` writes it, from formats.json's.
fn written(field: &Value) -> String {
    let mut words = vec![field["kind"].as_str().unwrap().to_string()];
    if let Some(scale) = field["scale"].as_u64() {
        words.push(scale.to_string());
    }
    if let Some(format) = field["format"].as_str() {
        words.push(format.to_string());
    }
    words.join(" ")
}

fn number_value(value: &Option<Value>) -> Option<Exact> {
    value.as_ref().and_then(Exact::of).map(|e| e.rounded(e.digits()))
}

#[test]
fn typed_text_is_read_in_the_persons_format_as_every_stack_reads_it() {
    let all = cases("formats.json");
    let mut wrong = Vec::new();
    let mut run = 0;
    for case in all["reads"].as_array().unwrap() {
        let field = FieldFormat::of_json(&case["field"]).unwrap();
        if matches!(field.kind, FormatKind::Date | FormatKind::DateTime) {
            continue;
        }
        run += 1;
        let start = if matches!(field.kind, FormatKind::Integer | FormatKind::Duration) { json!(987654) } else { json!("987654") };
        let (mut screen, host) = number_of(&written(&case["field"]), start.clone(), case["locale"].as_str().unwrap(), case["currency"].as_str());
        type_in(&mut screen, "amount", case["typed"].as_str().unwrap());
        let value = host.read("amount");
        let part = number(&screen, "amount");
        let ok = if case["value"].is_null() {
            part.bad().is_some() && value == Some(start)
        } else {
            part.bad().is_none() && number_value(&value) == number_value(&Some(case["value"].clone()))
        };
        if !ok {
            wrong.push(format!("{} {:?}: {value:?} (bad {:?})", case["what"], case["typed"], part.bad()));
        }
    }
    assert!(run > 5);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_value_shows_in_the_persons_format_without_a_currency_sign() {
    let (screen, _) = number_of("money", json!("1250.5"), "nl-NL", Some("EUR"));
    assert_eq!("1.250,50", number(&screen, "amount").editor().text());
    let (screen, _) = number_of("integer", json!(1234567), "en-US", None);
    assert_eq!("1,234,567", number(&screen, "amount").editor().text());
}

#[test]
fn a_whole_number_field_stores_a_whole_number_and_a_cleared_one_is_zero() {
    let (mut screen, host) = number_of("integer", json!(5), "en-US", None);
    type_in(&mut screen, "amount", "1,200");
    assert_eq!(Some(json!(1200)), host.read("amount"));
    type_in(&mut screen, "amount", "");
    assert_eq!(Some(json!(0)), host.read("amount"));
    assert_eq!(vec!["run amount change changed", "run amount change changed"], changes(&host));
}

#[test]
fn the_same_number_typed_otherwise_is_no_change_and_shows_as_the_value() {
    let (mut screen, host) = number_of("decimal 2", json!("2.5"), "en-US", None);
    assert_eq!("2.50", number(&screen, "amount").editor().text());
    type_in(&mut screen, "amount", "2.5");
    assert!(changes(&host).is_empty());
    assert_eq!("2.50", number(&screen, "amount").editor().text());
}

#[test]
fn what_code_sets_shows_unmarks_and_is_no_change() {
    let (mut screen, host) = number_of("decimal 1", json!("1.5"), "en-US", None);
    type_in(&mut screen, "amount", "lots");
    assert_eq!(Some("lots"), number(&screen, "amount").bad());
    host.set("amount", json!("4"));
    redraw(&mut screen);
    assert_eq!(None, number(&screen, "amount").bad());
    assert_eq!("4.0", number(&screen, "amount").editor().text());
    assert!(changes(&host).is_empty());
}

#[test]
fn without_a_format_it_keeps_the_text_and_unbound_in_its_state() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Number at="1,1" span="3" name="count" onchange="changed" />"#, host.clone());
    assert!(number(&screen, "count").format().is_none());
    type_in(&mut screen, "count", "12");
    assert_eq!(Some("12".to_string()), host.state(Some("count")).unwrap().text);
    assert_eq!(vec!["run count change changed"], changes(&host));
}

#[test]
fn a_field_of_a_number_type_reads_by_its_type() {
    let host = InputsHost::new();
    host.set("count", json!(3));
    host.types.borrow_mut().insert("count".into(), "integer".into());
    let mut screen = window(r#"<Number at="1,1" span="3" name="count" bind="count" />"#, host.clone());
    assert_eq!(Some(FormatKind::Integer), number(&screen, "count").format().map(|f| f.kind));
    type_in(&mut screen, "count", "4");
    assert_eq!(Some(json!(4)), host.read("count"));
}

// -- The spin box ------------------------------------------------------------------

fn text(case: &Value, name: &str) -> Option<String> {
    case[name].as_str().map(str::to_string)
}

fn attr(name: &str, value: Option<String>) -> String {
    value.map(|v| format!(" {name}=\"{v}\"")).unwrap_or_default()
}

fn spin_of(case: &Value, step: Option<String>, field_range: bool) -> (Screen, Rc<InputsHost>) {
    let whole = case["whole"].as_bool().unwrap();
    let host = InputsHost::new();
    let mut facts = facts(if whole { "integer" } else { "decimal 3" });
    if field_range {
        facts.min = text(case, "min");
        facts.max = text(case, "max");
    }
    host.facts("level", facts);
    let value = match text(case, "value") {
        Some(v) if whole => json!(v.parse::<i64>().unwrap()),
        Some(v) => json!(v),
        None => Value::Null,
    };
    host.set("level", value);
    let markup = if field_range {
        format!(r#"<Spinbox at="1,1" span="3" name="level" bind="level"{} onchange="changed" />"#, attr("step", step))
    } else {
        format!(r#"<Spinbox at="1,1" span="3" name="level" bind="level"{}{}{} onchange="changed" />"#, attr("min", text(case, "min")), attr("max", text(case, "max")), attr("step", step))
    };
    let screen = window(&markup, host.clone());
    (screen, host)
}

#[test]
fn a_key_moves_a_spin_box_as_on_every_renderer() {
    let all = cases("ranges.json");
    for case in all["keys"].as_array().unwrap() {
        let whole = case["whole"].as_bool().unwrap();
        let key = case["key"].as_str().unwrap();
        let (mut screen, host) = spin_of(case, text(case, "step"), false);
        let id = part_named(&screen, "level");
        let pressed = match key {
            "up" => Some(Key::Up),
            "down" => Some(Key::Down),
            "pageUp" => Some(Key::PageUp),
            "pageDown" => Some(Key::PageDown),
            _ => None,
        };
        match pressed {
            Some(k) => {
                screen.focus(Some(id));
                screen.key_pressed(KeyInput::plain(k));
            }
            None => {
                let to = if key == "home" { RangeKey::Home } else { RangeKey::End };
                screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(to));
            }
        }
        let value = host.read("level");
        assert_eq!(number_value(&Some(json!(text(case, "after").unwrap()))), number_value(&value), "{key} from {:?}", case["value"]);
        assert_eq!(whole, matches!(value, Some(Value::Number(_)) | None), "{key} from {:?}: {value:?}", case["value"]);
    }
}

#[test]
fn a_typed_number_stays_within_the_range_as_on_every_renderer() {
    let all = cases("ranges.json");
    for case in all["typed"].as_array().unwrap() {
        let (mut screen, host) = spin_of(case, Some("1".into()), false);
        host.set("level", Value::Null);
        redraw(&mut screen);
        type_in(&mut screen, "level", &text(case, "value").unwrap());
        assert_eq!(number_value(&Some(json!(text(case, "after").unwrap()))), number_value(&host.read("level")), "{:?}", case["value"]);
    }
}

#[test]
fn without_its_own_range_it_takes_the_fields() {
    let all = cases("ranges.json");
    let case = &all["typed"][1];
    let (mut screen, host) = spin_of(case, Some("1".into()), true);
    type_in(&mut screen, "level", "500");
    assert_eq!(Some(json!(99)), host.read("level"));
}

/// A range code set (`levelBox.max = 3`) goes before the one the line writes.
#[test]
fn a_range_code_set_goes_before_the_written_one() {
    let host = InputsHost::new();
    host.set("level", json!(3));
    host.types.borrow_mut().insert("level".into(), "integer".into());
    host.set_state("level", |s| s.max = Some(3));
    let mut screen = window(r#"<Spinbox at="1,1" span="3" name="level" bind="level" min="1" max="99" onchange="changed" />"#, host.clone());
    let id = part_named(&screen, "level");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(RangeKey::Up));
    assert_eq!(Some(json!(3)), host.read("level"));
    type_in(&mut screen, "level", "7");
    assert_eq!(Some(json!(3)), host.read("level"));
}

#[test]
fn only_a_value_that_differs_is_a_change_and_the_arrows_spin() {
    let host = InputsHost::new();
    host.set("level", json!(3));
    host.types.borrow_mut().insert("level".into(), "integer".into());
    let mut screen = window(r#"<Spinbox at="1,1" span="3" name="level" bind="level" min="1" max="3" onchange="changed" />"#, host.clone());
    let id = part_named(&screen, "level");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(RangeKey::Up));
    assert!(changes(&host).is_empty());
    let down = number(&screen, "level").down().unwrap().bounds;
    screen.pointer_pressed(down.x + 3.0, down.y + 3.0, PointerButton::Left, 1);
    screen.pointer_released(down.x + 3.0, down.y + 3.0, PointerButton::Left);
    assert_eq!(Some(json!(2)), host.read("level"));
    assert_eq!(vec!["run level change changed"], changes(&host));
    assert_eq!("2", number(&screen, "level").editor().text());
    // Left and Right still move the caret.
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::plain(Key::Left));
    assert_eq!(0, number(&screen, "level").editor().caret());
    assert_eq!(Some(json!(2)), host.read("level"));
}

#[test]
fn a_number_box_has_no_arrows_and_a_read_only_spin_box_does_not_move() {
    let host = InputsHost::new();
    host.set("level", json!(3));
    let mut screen = window(
        r#"<Number at="1,1" span="3" name="count" bind="level" />
           <Spinbox at="5,1" span="3" name="level" bind="level" />"#,
        host.clone(),
    );
    assert!(number(&screen, "count").up().is_none());
    assert!(number(&screen, "count").range().is_none());
    host.set_state("level", |s| s.readonly = Some(true));
    let id = part_named(&screen, "level");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(RangeKey::Up));
    assert_eq!(Some(json!(3)), host.read("level"));
}

#[test]
fn an_unbound_spin_box_keeps_its_number_as_text() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Spinbox at="1,1" span="3" name="copies" min="1" max="9" />"#, host.clone());
    let id = part_named(&screen, "copies");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(RangeKey::Up));
    assert_eq!(Some("2".to_string()), host.state(Some("copies")).unwrap().text);
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<NumberPart>().unwrap().spin(RangeKey::End));
    assert_eq!(Some("9".to_string()), host.state(Some("copies")).unwrap().text);
}

#[test]
fn a_number_box_and_a_spin_box_draw() {
    let host = InputsHost::new();
    host.use_formats("nl-NL", Some("EUR"));
    host.facts("price", facts("money"));
    host.set("price", json!("1250.5"));
    host.set("level", json!(3));
    let mut screen = window(
        r#"<Number at="1,1" span="4" name="price" bind="price" />
           <Spinbox at="1,2" span="4" name="level" bind="level" min="1" max="9" />
           <Number at="1,3" span="4" name="wrong" />"#,
        host,
    );
    type_in(&mut screen, "wrong", "");
    let mut shot = Shot::of(&mut screen, 400, 160, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-number.png")).expect("the picture is written");
    }
    let up = number(&screen, "level").up().unwrap().bounds;
    assert!(up.width > 0.0 && up.height > 0.0);
}

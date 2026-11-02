//! The checkbox and the switch: the person's tick is a change, bound or kept
//! in the part's state; what code sets is not; a read-only or disabled one
//! cannot be ticked; Space ticks the focused one; a ticked box is drawn in
//! the accent color. With the cases of conformance/forms.json that tick.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::check_box::CheckPart;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::{json, Value};

fn window(markup: &str, host: Rc<TestHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn check<'a>(screen: &'a Screen, name: &str) -> &'a CheckPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<CheckPart>()).expect("a check part")
}

fn click(screen: &mut Screen, name: &str) {
    let b = check(screen, name).node().bounds;
    let (x, y) = (b.x + 8.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn ticking_a_bound_checkbox_changes_it() {
    let host = TestHost::new();
    host.set("task.done", json!(false));
    let mut screen = window(r#"<Checkbox at="1,1" span="3" name="done" bind="task.done" caption="Done" onchange="doneChanged" />"#, host.clone());
    click(&mut screen, "done");
    assert!(check(&screen, "done").is_checked());
    assert_eq!(vec!["write done task.done true", "run done change doneChanged"], host.logged());
}

#[test]
fn a_switch_flips_like_a_checkbox() {
    let host = TestHost::new();
    host.set("task.urgent", json!(false));
    let mut screen = window(r#"<Switch at="1,1" span="3" name="urgent" bind="task.urgent" onchange="urgentChanged" />"#, host.clone());
    assert!(check(&screen, "urgent").is_switch());
    click(&mut screen, "urgent");
    click(&mut screen, "urgent");
    assert!(!check(&screen, "urgent").is_checked());
    assert_eq!(2, host.logged().iter().filter(|l| l.starts_with("run urgent change")).count());
}

#[test]
fn what_code_sets_is_no_change() {
    let host = TestHost::new();
    host.set("task.done", json!(false));
    let screen = window(r#"<Checkbox at="1,1" span="3" name="done" bind="task.done" onchange="doneChanged" />"#, host.clone());
    host.set("task.done", json!(true));
    assert!(check(&screen, "done").is_checked());
    assert!(host.logged().is_empty());
}

#[test]
fn an_unbound_checkbox_keeps_its_value_in_its_state() {
    let host = TestHost::new();
    let mut screen = window(r#"<Checkbox at="1,1" span="3" name="remember" onchange="remembered" />"#, host.clone());
    click(&mut screen, "remember");
    assert_eq!(Some(true), host.state(Some("remember")).unwrap().checked);
    assert_eq!(vec!["remember".to_string()], host.edits.borrow().clone());
    assert_eq!(vec!["run remember change remembered"], host.logged());
}

#[test]
fn a_read_only_or_disabled_checkbox_cannot_be_ticked() {
    let host = TestHost::new();
    host.set("a", json!(false));
    host.set("b", json!(false));
    let mut screen = window(
        r#"<Checkbox at="1,1" span="3" name="first" bind="a" readonly="true" />
           <Checkbox at="1,2" span="3" name="second" bind="b" />"#,
        host.clone(),
    );
    host.set_state("second", |s| s.enabled = Some(false));
    click(&mut screen, "first");
    click(&mut screen, "second");
    assert!(host.logged().is_empty());
    assert!(!check(&screen, "second").focusable());
}

#[test]
fn space_ticks_the_focused_checkbox() {
    let host = TestHost::new();
    host.set("a", json!(false));
    let mut screen = window(r#"<Checkbox at="1,1" span="3" name="first" bind="a" />"#, host.clone());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    screen.key_pressed(KeyInput::plain(Key::Space));
    assert_eq!(Some(json!(true)), host.read("a"));
}

#[test]
fn a_ticked_box_is_drawn_in_the_accent_color() {
    let host = TestHost::new();
    host.set("a", json!(false));
    let mut screen = window(r#"<Checkbox at="1,1" span="3" name="first" bind="a" caption="Done" />"#, host.clone());
    let (input, accent) = (screen.look().color("input"), screen.look().color("accent"));
    let mut off = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(input, off.pixel(9, 12), 10));
    host.set("a", json!(true));
    let mut on = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(accent, on.pixel(9, 12), 10));
}

#[test]
fn a_switch_and_a_box_draw_with_their_captions() {
    let host = TestHost::new();
    host.set("a", json!(true));
    host.set("b", json!(false));
    host.set("c", json!(true));
    let mut screen = window(
        r#"<Checkbox at="1,1" span="3" name="first" bind="a" caption="Ticked" />
           <Checkbox at="1,2" span="3" name="second" bind="b" caption="Not ticked" />
           <Switch at="1,3" span="3" name="third" bind="c" caption="On" />
           <Switch at="1,4" span="3" name="fourth" caption="Off" />"#,
        host,
    );
    screen.key_pressed(KeyInput::plain(Key::Tab));
    let accent = screen.look().color("accent");
    let mut shot = Shot::of(&mut screen, 400, 200, 1.0);
    keep(&mut shot, "check_box.png");
    let switch = check(&screen, "third").node().bounds;
    assert!(near(accent, shot.pixel((switch.x + 4.0) as i32, (switch.y + switch.height / 2.0) as i32), 10));
}

// -- The form cases that tick ---------------------------------------------------

const MINE: [&str; 2] = ["done", "urgent"];

const FORM: &str = r#"
    <Checkbox at="1,1" span="3" name="done" bind="done" onchange="changed" />
    <Switch at="1,2" span="3" name="urgent" bind="urgent" onchange="changed" />
"#;

fn forms() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/forms.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/forms.json")).expect("it is JSON")
}

fn load(host: &TestHost, record: &Value) {
    for field in MINE {
        host.values.borrow_mut().insert(field.to_string(), record[field].clone());
    }
}

/// Whether a case only concerns the fields these parts show (its other values are left out).
fn concerns(case: &Value) -> bool {
    let steps = case["steps"].as_array().unwrap();
    let ticks = steps.iter().any(|s| s[0] == "tick");
    let only_mine = steps.iter().all(|s| s[0] == "reload" || s[0] == "tick" || (s[0] == "handler" && MINE.contains(&s[1].as_str().unwrap_or(""))));
    ticks && only_mine
}

#[test]
fn every_form_case_that_ticks_behaves_as_on_every_renderer() {
    let forms = forms();
    let mut run = 0;
    for case in forms["cases"].as_array().unwrap() {
        if !concerns(case) {
            continue;
        }
        run += 1;
        let name = case["name"].as_str().unwrap();
        let host = TestHost::new();
        load(&host, &forms["record"]);
        let mut screen = window(FORM, host.clone());
        for step in case["steps"].as_array().unwrap() {
            match step[0].as_str().unwrap() {
                "tick" => click(&mut screen, step[1].as_str().unwrap()),
                "handler" => host.set(step[1].as_str().unwrap(), step[2].clone()),
                "reload" => {
                    load(&host, &forms["record"]);
                    host.change();
                }
                other => panic!("{name}: no step {other}"),
            }
        }
        let events: Vec<String> = host
            .logged()
            .iter()
            .filter_map(|l| l.strip_prefix("run ").map(|r| r.split(' ').take(2).collect::<Vec<_>>()))
            .map(|w| format!("{} {}", w[1], w[0]))
            .collect();
        let wanted: Vec<String> = case["expect"]["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).filter(|e| MINE.contains(&e.rsplit(' ').next().unwrap())).collect();
        assert_eq!(wanted, events, "{name}: events");
        for v in case["expect"]["values"].as_array().into_iter().flatten() {
            let field = v[0].as_str().unwrap();
            if MINE.contains(&field) {
                assert_eq!(Some(v[1].clone()), host.read(field), "{name}: {field}");
                assert_eq!(v[1].as_bool().unwrap(), check(&screen, field).is_checked(), "{name}: {field} shows");
            }
        }
    }
    assert!(run >= 2, "only {run} cases ran");
}

#[test]
fn what_a_handler_sets_on_a_box_and_a_switch_is_no_change() {
    let forms = forms();
    let host = TestHost::new();
    load(&host, &forms["record"]);
    let screen = window(FORM, host.clone());
    host.set("done", json!(true));
    host.set("urgent", json!(true));
    assert!(check(&screen, "done").is_checked());
    assert!(check(&screen, "urgent").is_checked());
    assert!(host.logged().is_empty());
}

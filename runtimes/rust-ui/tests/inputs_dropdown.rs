//! The plain dropdown: a closed box showing its choice's label, which opens
//! its list over the window below it (above when there is no room below).
//! Picking another choice is a change; the one it has, closing the list and
//! what code sets are not. Up, Down, Enter and Escape work the list. With
//! the cases of conformance/forms.json that pick.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::choice_list::{ChoiceList, LINE};
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use serde_json::{json, Value};

const OWNER: &str = r#"<Dropdown at="1,1" span="4" name="owner" bind="task.owner" onchange="ownerChanged" />"#;

fn owners() -> Rc<InputsHost> {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("task.owner".into(), json!(1));
    host.offer("task.owner", &[(Value::Null, ""), (json!(1), "Ada"), (json!(2), "Grace")]);
    host
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn dropdown<'a>(screen: &'a Screen, name: &str) -> &'a DropdownPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<DropdownPart>()).expect("a dropdown")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn it_shows_its_choices_label_and_what_code_sets_is_no_change() {
    let host = owners();
    let screen = window(OWNER, host.clone());
    assert_eq!("Ada", dropdown(&screen, "owner").shown());
    host.set("task.owner", json!(2));
    assert_eq!("Grace", dropdown(&screen, "owner").shown());
    assert!(host.logged().is_empty());
}

#[test]
fn a_click_opens_its_list_below_it_and_a_pick_changes_it() {
    let host = owners();
    let mut screen = window(OWNER, host.clone());
    click(&mut screen, 20.0, 18.0);
    assert_eq!(1, screen.overlays().len());
    let place = screen.overlays()[0].place;
    let part = dropdown(&screen, "owner");
    assert!(place.y >= part.node().bounds.bottom());
    assert_eq!(Some(1), part.highlighted());
    click(&mut screen, place.x + 10.0, place.y + 1.0 + LINE * 2.5);
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["write owner task.owner 2", "run owner change ownerChanged"], host.logged());
    assert_eq!(Some(part_named(&screen, "owner")), screen.focused());
    assert_eq!("Grace", dropdown(&screen, "owner").shown());
}

#[test]
fn picking_the_choice_it_has_or_a_press_outside_is_no_change() {
    let host = owners();
    let mut screen = window(OWNER, host.clone());
    click(&mut screen, 20.0, 18.0);
    let place = screen.overlays()[0].place;
    click(&mut screen, place.x + 10.0, place.y + 1.0 + LINE * 1.5);
    click(&mut screen, 20.0, 18.0);
    assert_eq!(1, screen.overlays().len());
    click(&mut screen, 700.0, 300.0);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
}

#[test]
fn a_click_on_its_box_closes_its_open_list() {
    let host = owners();
    let mut screen = window(OWNER, host.clone());
    click(&mut screen, 20.0, 18.0);
    click(&mut screen, 20.0, 18.0);
    assert!(screen.overlays().is_empty());
    assert!(dropdown(&screen, "owner").list().is_none());
}

#[test]
fn up_down_enter_and_escape_work_its_list() {
    let host = owners();
    let mut screen = window(OWNER, host.clone());
    press(&mut screen, Key::Tab);
    press(&mut screen, Key::Down);
    assert!(dropdown(&screen, "owner").list().is_some());
    press(&mut screen, Key::Down);
    assert_eq!(Some(2), dropdown(&screen, "owner").highlighted());
    press(&mut screen, Key::Escape);
    assert!(dropdown(&screen, "owner").list().is_none());
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
    press(&mut screen, Key::Enter);
    press(&mut screen, Key::Up);
    assert_eq!(Some(0), dropdown(&screen, "owner").highlighted());
    press(&mut screen, Key::Enter);
    assert!(dropdown(&screen, "owner").list().is_none());
    assert_eq!(Some(Value::Null), host.read("task.owner"));
    assert_eq!("", dropdown(&screen, "owner").shown());
    assert_eq!(vec!["write owner task.owner null", "run owner change ownerChanged"], host.logged());
}

#[test]
fn tab_away_closes_its_list() {
    let host = owners();
    let mut screen = window(
        r#"<Dropdown at="1,1" span="4" name="owner" bind="task.owner" />
           <Dropdown at="1,2" span="4" name="second" bind="task.owner" />"#,
        host.clone(),
    );
    press(&mut screen, Key::Tab);
    press(&mut screen, Key::Down);
    assert_eq!(1, screen.overlays().len());
    press(&mut screen, Key::Tab);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(part_named(&screen, "second")), screen.focused());
}

#[test]
fn its_list_opens_above_it_when_there_is_no_room_below() {
    let host = owners();
    let mut screen = window_of(r#"<Dropdown at="1,8" span="4" name="owner" bind="task.owner" />"#, host);
    screen.resize(800.0, 360.0, 1.0);
    let b = dropdown(&screen, "owner").node().bounds;
    click(&mut screen, 20.0, b.y + 18.0);
    assert!(screen.overlays()[0].place.bottom() <= b.y);
}

#[test]
fn a_disabled_or_read_only_dropdown_does_not_open() {
    let host = owners();
    let mut screen = window(
        r#"<Dropdown at="1,1" span="4" name="first" bind="task.owner" readonly="true" />
           <Dropdown at="1,2" span="4" name="second" bind="task.owner" />"#,
        host.clone(),
    );
    host.set_state("second", |s| s.enabled = Some(false));
    let (first, second) = (dropdown(&screen, "first").node().bounds, dropdown(&screen, "second").node().bounds);
    click(&mut screen, 20.0, first.y + 18.0);
    click(&mut screen, 20.0, second.y + 18.0);
    assert!(screen.overlays().is_empty());
}

#[test]
fn the_wheel_scrolls_a_long_list() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("n".into(), json!(0));
    let many: Vec<(Value, String)> = (0..20).map(|i| (json!(i), format!("Choice {i}"))).collect();
    let refs: Vec<(Value, &str)> = many.iter().map(|(v, l)| (v.clone(), l.as_str())).collect();
    host.offer("n", &refs);
    let mut screen = window(r#"<Dropdown at="1,1" span="4" name="n" bind="n" />"#, host.clone());
    click(&mut screen, 20.0, 18.0);
    let place = screen.overlays()[0].place;
    assert!(place.height < 20.0 * LINE);
    screen.wheel(place.x + 10.0, place.y + 10.0, 0.0, LINE * 3.0, clarity_ui::input::Modifiers::NONE);
    click(&mut screen, place.x + 10.0, place.y + 1.0 + LINE * 0.5);
    assert_eq!(Some(json!(3)), host.read("n"));
}

#[test]
fn its_box_and_open_list_draw_in_the_theme() {
    let host = owners();
    let mut screen = window(OWNER, host.clone());
    let input = screen.look().color("input");
    let mut closed = Shot::of(&mut screen, 800, 400, 1.0);
    let b = dropdown(&screen, "owner").node().bounds;
    assert!(near(input, closed.pixel((b.x + b.width / 2.0) as i32, (b.y + 4.0) as i32), 10));
    click(&mut screen, 20.0, 18.0);
    let place = screen.overlays()[0].place;
    let selection = screen.look().part("list", Some("hover")).background.unwrap_or_else(|| screen.look().color("selection"));
    let mut open = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut open, "inputs-dropdown.png");
    // The line of its value is marked.
    assert!(near(selection, open.pixel((place.right() - 6.0) as i32, (place.y + 1.0 + LINE * 1.5) as i32), 10));
    assert!(screen.find::<ChoiceList>().is_some());
}

// -- The form cases that pick ------------------------------------------------------

/// The dropdowns of the form (conformance/forms.json, kind `choice`).
const MINE: [&str; 2] = ["priority", "owner"];

const FORM: &str = r#"
    <Dropdown at="1,1" span="6" name="priority" bind="priority" onchange="changed" />
    <Dropdown at="1,2" span="6" name="owner" bind="owner" onchange="changed" />
"#;

fn forms() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/forms.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/forms.json")).expect("it is JSON")
}

fn load(host: &InputsHost, forms: &Value) {
    for field in MINE {
        host.values.borrow_mut().insert(field.to_string(), forms["record"][field].clone());
    }
}

fn form(forms: &Value) -> (Rc<InputsHost>, Screen) {
    let host = InputsHost::new();
    load(&host, forms);
    for field in forms["fields"].as_array().unwrap() {
        let name = field["field"].as_str().unwrap();
        if MINE.contains(&name) {
            let choices: Vec<(Value, String)> = field["choices"].as_array().unwrap().iter().map(|c| (c[0].clone(), c[1].as_str().unwrap().to_string())).collect();
            let refs: Vec<(Value, &str)> = choices.iter().map(|(v, l)| (v.clone(), l.as_str())).collect();
            host.offer(name, &refs);
        }
    }
    let screen = window(FORM, host.clone());
    (host, screen)
}

fn pick(screen: &mut Screen, name: &str, value: &Value) {
    let at = dropdown(screen, name).choices().iter().position(|c| &c.value == value).expect("a choice of that value");
    let id = part_named(screen, name);
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<DropdownPart>().expect("a dropdown").open(cx));
    let place = screen.overlays().last().expect("its list").place;
    click(screen, place.x + 10.0, place.y + 1.0 + (at as f64 + 0.5) * LINE);
}

/// Whether a case only concerns the dropdowns (its other fields left out).
fn concerns(case: &Value) -> bool {
    let steps = case["steps"].as_array().unwrap();
    let mine = |s: &Value| MINE.contains(&s[1].as_str().unwrap_or(""));
    let picks = steps.iter().any(|s| s[0] == "pick" && mine(s));
    let shows = case["expect"]["shown"].as_array().is_some_and(|s| s.iter().any(|v| MINE.contains(&v[0].as_str().unwrap_or("")))) || case["expect"].get("choices").is_some();
    (picks || shows) && steps.iter().all(|s| s[0] == "reload" || s[0] == "pick" || s[0] == "handler")
}

#[test]
fn every_form_case_that_picks_behaves_as_on_every_renderer() {
    let forms = forms();
    let mut run = 0;
    for case in forms["cases"].as_array().unwrap() {
        if !concerns(case) {
            continue;
        }
        run += 1;
        let name = case["name"].as_str().unwrap();
        let (host, mut screen) = form(&forms);
        for step in case["steps"].as_array().unwrap() {
            let field = step[1].as_str().unwrap_or("");
            match step[0].as_str().unwrap() {
                "pick" if MINE.contains(&field) => pick(&mut screen, field, &step[2]),
                "handler" if MINE.contains(&field) => host.set(field, step[2].clone()),
                "reload" => {
                    load(&host, &forms);
                    host.change();
                }
                _ => {}
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
            }
        }
        for s in case["expect"]["shown"].as_array().into_iter().flatten() {
            let field = s[0].as_str().unwrap();
            if MINE.contains(&field) {
                assert_eq!(s[1].as_str().unwrap(), dropdown(&screen, field).shown(), "{name}: {field} shows");
            }
        }
        if let Some(choices) = case["expect"]["choices"].as_object() {
            for (field, labels) in choices {
                if MINE.contains(&field.as_str()) {
                    let offered: Vec<String> = dropdown(&screen, field).choices().into_iter().map(|c| c.label).collect();
                    let wanted: Vec<String> = labels.as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
                    assert_eq!(wanted, offered, "{name}: {field} offers");
                }
            }
        }
    }
    assert!(run >= 7, "only {run} cases ran");
}

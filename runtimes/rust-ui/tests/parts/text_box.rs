//! A text box, a memo and a password against the cases every stack shares
//! (conformance/forms.json): typing goes through the screen, Enter through
//! the keys, leaving through the focus, inside a form whose submit is wired.
//! A password's eye and copy button, and an unbound input that keeps its
//! text in its state.

use std::rc::Rc;

use clarity_runtime::ui::Text;
use clarity_ui::element::{find, take_typed_from, Cx, Element, GridPanel, Node};
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{element_node, locale, Painter, Screen, ThemeLook};
use serde_json::{json, Value};
use skia_safe::Color;

/// A form around a grid of parts: Enter in its one-line inputs takes what is typed and submits it.
struct TestForm {
    node: Node,
    host: Rc<TestHost>,
}

impl Element for TestForm {
    element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        clarity_ui::element::draw_children(&self.node, p);
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Enter {
            return false;
        }
        let focused = cx.focused();
        if focused.and_then(|f| find(self, f)).is_none_or(|e| e.multiline()) {
            return false;
        }
        take_typed_from(self, focused, cx);
        self.host.log.borrow_mut().push("submit".to_string());
        true
    }
}

/// The parts of `markup` in a form, on a screen of 800 by 400.
fn in_form(markup: &str, host: Rc<TestHost>) -> Screen {
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{markup}</Window>")).expect("the markup reads");
    let mut grid = GridPanel::new(12);
    for part in spec.parts {
        let made = clarity_ui::parts::make(part, host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let mut form = TestForm { node: Node::new(), host };
    form.node.add(Box::new(grid));
    let mut screen = Screen::new(Box::new(form), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn window(markup: &str, host: Rc<TestHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn text_box<'a>(screen: &'a Screen, name: &str) -> &'a TextBoxPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<TextBoxPart>()).expect("a text box")
}

fn text_box_mut<'a>(screen: &'a mut Screen, name: &str) -> &'a mut TextBoxPart {
    let id = part_named(screen, name);
    screen.element_mut(id).and_then(|e| e.as_any_mut().downcast_mut::<TextBoxPart>()).expect("a text box")
}

/// Puts `text` in an input as the person types it, without taking it.
fn type_in(screen: &mut Screen, name: &str, text: &str) {
    let id = part_named(screen, name);
    screen.focus(Some(id));
    text_box_mut(screen, name).editor_mut().select_all();
    screen.key_pressed(KeyInput::plain(Key::Backspace));
    if !text.is_empty() {
        screen.text_typed(text);
    }
}

/// The handlers that ran, as `event part`, and the forms submitted, in order.
fn events(host: &TestHost) -> Vec<String> {
    host.logged()
        .iter()
        .filter_map(|line| {
            if line == "submit" {
                return Some(line.clone());
            }
            let mut words = line.strip_prefix("run ")?.split(' ');
            let (part, event) = (words.next()?, words.next()?);
            Some(format!("{event} {part}"))
        })
        .collect()
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn inked(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 5)))
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

// -- The form cases every stack shares ------------------------------------------

/// The fields of the cases these parts show.
const MINE: [&str; 2] = ["name", "notes"];

const FORM: &str = r#"
    <Textbox at="1,1" span="6" name="name" bind="name" onchange="changed" />
    <Memo at="1,2" span="6" rows="3" name="notes" bind="notes" onchange="changed" />
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

/// Whether a case only concerns the fields these parts show.
fn concerns(case: &Value) -> bool {
    let mine = |v: &Value| v.as_str().is_some_and(|s| MINE.contains(&s));
    for step in case["steps"].as_array().unwrap() {
        let step = step.as_array().unwrap();
        if matches!(step[0].as_str(), Some("tick" | "pick" | "pickDay")) {
            return false;
        }
        if step.len() > 1 && !mine(&step[1]) {
            return false;
        }
    }
    let expect = &case["expect"];
    if let Some(values) = expect["values"].as_array() {
        if values.iter().any(|v| !mine(&v[0])) {
            return false;
        }
    }
    expect["events"].as_array().unwrap().iter().all(|e| {
        let e = e.as_str().unwrap();
        e == "submit" || MINE.contains(&e.rsplit(' ').next().unwrap())
    })
}

#[test]
fn every_form_case_of_text_and_memo_behaves_as_on_every_renderer() {
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
        let mut screen = in_form(FORM, host.clone());
        for step in case["steps"].as_array().unwrap() {
            let field = step.get(1).and_then(Value::as_str).unwrap_or("");
            match step[0].as_str().unwrap() {
                "focus" => {
                    let id = part_named(&screen, field);
                    screen.focus(Some(id));
                }
                "type" => type_in(&mut screen, field, step[2].as_str().unwrap()),
                "enter" => {
                    screen.key_pressed(KeyInput::plain(Key::Enter));
                }
                "leave" => screen.focus(None),
                "handler" => host.set(field, step[2].clone()),
                "reload" => {
                    load(&host, &forms["record"]);
                    host.change();
                }
                other => panic!("{name}: no step {other}"),
            }
        }
        Shot::of(&mut screen, 800, 400, 1.0);
        let expect = &case["expect"];
        let wanted: Vec<String> = expect["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(wanted, events(&host), "{name}: events");
        for v in expect["values"].as_array().into_iter().flatten() {
            let field = v[0].as_str().unwrap();
            assert_eq!(Some(&v[1]), host.values.borrow().get(field), "{name}: {field}");
        }
        for s in expect["shown"].as_array().into_iter().flatten() {
            let field = s[0].as_str().unwrap();
            if MINE.contains(&field) {
                assert_eq!(s[1].as_str().unwrap(), text_box(&screen, field).editor().text(), "{name}: {field} shows");
            }
        }
    }
    assert!(run >= 8, "only {run} cases ran");
}

// -- What the parts do -------------------------------------------------------------

#[test]
fn entering_and_leaving_an_input_run_its_handlers() {
    let host = TestHost::new();
    let mut screen = in_form(r#"<Textbox at="1,1" span="4" name="note" onenter="came" onleave="went" />"#, host.clone());
    let id = part_named(&screen, "note");
    screen.focus(Some(id));
    screen.focus(None);
    assert_eq!(vec!["enter note", "leave note"], events(&host));
}

#[test]
fn an_unbound_named_input_keeps_its_text_in_its_state() {
    let host = TestHost::new();
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="search" onchange="look" />"#, host.clone());
    type_in(&mut screen, "search", "milk");
    screen.focus(None);
    assert_eq!(Some("milk".to_string()), host.state(Some("search")).unwrap().text);
    assert_eq!(vec!["search".to_string()], host.edits.borrow().clone());
    assert_eq!(vec!["change search"], events(&host));
    // What code sets shows, and is no change.
    host.set_state("search", |s| s.text = Some("bread".to_string()));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("bread", text_box(&screen, "search").editor().text());
    assert_eq!(1, events(&host).len());
}

#[test]
fn a_bound_value_set_by_code_shows_and_is_no_change() {
    let host = TestHost::new();
    host.set("name", json!("Milk"));
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="name" bind="name" onchange="changed" />"#, host.clone());
    assert_eq!("Milk", text_box(&screen, "name").editor().text());
    host.set("name", json!("Bread"));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("Bread", text_box(&screen, "name").editor().text());
    screen.focus(Some(part_named(&screen, "name")));
    screen.focus(None);
    assert!(host.logged().is_empty());
}

#[test]
fn enter_outside_a_form_takes_the_text_and_is_not_taken() {
    let host = TestHost::new();
    host.set("name", json!("a"));
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="name" bind="name" />"#, host.clone());
    type_in(&mut screen, "name", "b");
    let id = part_named(&screen, "name");
    let taken = screen.act(id, |e, cx| e.on_key(KeyInput::plain(Key::Enter), cx));
    assert_eq!(Some(false), taken);
    assert_eq!(Some(json!("b")), host.read("name"));
}

#[test]
fn a_key_that_runs_a_command_takes_what_is_typed_first() {
    let host = TestHost::new();
    host.set("name", json!("a"));
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="name" bind="name" />"#, host.clone());
    type_in(&mut screen, "name", "typed");
    let id = part_named(&screen, "name");
    assert_eq!(Some(true), screen.act(id, |e, cx| e.take_typed(cx)));
    assert_eq!(Some(json!("typed")), host.read("name"));
}

#[test]
fn a_placeholder_is_translated_and_a_read_only_input_takes_no_typing() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("searchHint".to_string(), "Search".to_string());
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="q" placeholder="@searchHint" />"#, host.clone());
    assert_eq!(Some("Search"), text_box(&screen, "q").editor().placeholder.as_deref());
    host.set_state("q", |s| s.readonly = Some(true));
    type_in(&mut screen, "q", "x");
    assert_eq!("", text_box(&screen, "q").editor().text());
    host.set_state("q", |s| s.enabled = Some(false));
    assert!(!text_box(&screen, "q").focusable());
}

#[test]
fn a_memo_wraps_and_enter_is_a_new_line() {
    let host = TestHost::new();
    let mut screen = window(r#"<Memo at="1,1" span="4" rows="3" name="notes" />"#, host.clone());
    assert!(text_box(&screen, "notes").editor().is_multiline());
    assert!(text_box(&screen, "notes").multiline());
    assert!(text_box(&screen, "notes").typing());
    let id = part_named(&screen, "notes");
    screen.focus(Some(id));
    screen.text_typed("one");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    screen.text_typed("two");
    assert_eq!("one\ntwo", text_box(&screen, "notes").editor().text());
    assert!(host.logged().is_empty());
}

// -- A password -----------------------------------------------------------------

#[test]
fn the_eye_and_copy_say_what_they_do_when_the_pointer_rests() {
    let host = TestHost::new();
    let screen = window(r#"<Password at="1,1" span="4" name="secret" />"#, host);
    let password = text_box(&screen, "secret");
    let (eye, copy) = (password.eye().unwrap().bounds, password.copy().unwrap().bounds);
    assert_eq!(Some(locale::translate("showSecret", "Show")), password.tooltip_at(eye.x + 5.0, eye.y + 5.0));
    assert_eq!(Some(locale::translate("copySecret", "Copy")), password.tooltip_at(copy.x + 5.0, copy.y + 5.0));
}

#[test]
fn the_eye_shows_what_is_typed_and_hides_it_again() {
    let host = TestHost::new();
    host.set("token", json!("s3cr3t"));
    let mut screen = window(r#"<Password at="1,1" span="4" name="token" bind="token" />"#, host);
    assert!(text_box(&screen, "token").editor().password());
    assert_eq!("Show", text_box(&screen, "token").eye().unwrap().name);
    let eye = text_box(&screen, "token").eye().unwrap().bounds;
    click(&mut screen, eye.x + 5.0, eye.y + 5.0);
    assert!(!text_box(&screen, "token").editor().password());
    // Pressed, it says what it does next.
    assert_eq!("Hide", text_box(&screen, "token").eye().unwrap().name);
    click(&mut screen, eye.x + 5.0, eye.y + 5.0);
    assert!(text_box(&screen, "token").editor().password());
}

#[test]
fn copy_puts_what_is_in_the_input_on_the_clipboard_typed_or_not_yet_taken() {
    let host = TestHost::new();
    host.set("token", json!("s3cr3t"));
    let mut screen = window(r#"<Password at="1,1" span="4" name="token" bind="token" />"#, host);
    text_box_mut(&mut screen, "token").editor_mut().set_text("typed");
    assert_eq!("Copy", text_box(&screen, "token").copy().unwrap().name);
    let copy = text_box(&screen, "token").copy().unwrap().bounds;
    click(&mut screen, copy.x + 5.0, copy.y + 5.0);
    assert_eq!(Some("typed".to_string()), screen.clipboard_text());
}

#[test]
fn a_text_input_has_neither() {
    let screen = window(r#"<Textbox at="1,1" span="4" name="x" />"#, TestHost::new());
    let text = text_box(&screen, "x");
    assert!(text.eye().is_none());
    assert!(text.copy().is_none());
}

#[test]
fn an_error_marks_the_input_in_the_error_color() {
    let host = TestHost::new();
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="email" />"#, host.clone());
    host.set_state("email", |s| s.error = Some(Text::plain("Not an address")));
    let error = screen.look().color("error");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let bounds = text_box(&screen, "email").node().bounds;
    assert!(inked(&mut shot, bounds, error));
}

#[test]
fn the_inputs_draw_in_the_themes_input_look() {
    let host = TestHost::new();
    host.set("name", json!("Oat milk"));
    host.set("token", json!("s3cr3t"));
    let mut screen = window(
        r#"<Textbox at="1,1" span="4" name="name" bind="name" />
           <Textbox at="1,2" span="4" name="empty" placeholder="Type a name" />
           <Password at="1,3" span="4" name="token" bind="token" />
           <Memo at="1,4" span="4" rows="3" name="notes" />"#,
        host,
    );
    let id = part_named(&screen, "notes");
    screen.focus(Some(id));
    screen.text_typed("A memo of more words than fit on one line, so it wraps.");
    let input = screen.look().color("input");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "text_box_inputs.png");
    let name = text_box(&screen, "name").node().bounds;
    assert!(near(input, shot.pixel((name.x + name.width - 6.0) as i32, (name.y + name.height / 2.0) as i32), 3));
}

/// C# Part.Enabled is EnabledInTree: a text box inside a disabled panel
/// draws as disabled (dimmed), as the button beside it does (note S14).
#[test]
fn a_text_box_in_a_disabled_panel_draws_disabled() {
    let host = TestHost::new();
    host.set("note", json!("Typed"));
    let mut screen = window(r#"<Panel name="box" at="1,1" span="12" rows="3"><Textbox name="note" at="1,1" span="6" bind="note" /></Panel>"#, host.clone());
    Shot::of(&mut screen, 800, 400, 1.0);
    assert!(text_box(&screen, "note").editor().enabled);
    host.set_state("box", |s| s.enabled = Some(false));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert!(!text_box(&screen, "note").editor().enabled);
}

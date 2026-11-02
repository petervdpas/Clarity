//! The typed and the search dropdown against the cases every renderer
//! shares (conformance/dropdown-options.json): text typed into their box,
//! an offered row picked from the list below it. A typed dropdown takes any
//! text; a search dropdown a record picked, and empty text is none. Only a
//! value that differs is a change. A key that runs a command takes what is
//! typed first.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::choice_list::{ChoiceList, LINE};
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::inputs::typed_dropdown::{line_of, TypeAheadPart, Variant};
use clarity_ui::parts::{Choice, FieldFacts, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::Screen;
use serde_json::{json, Value};

const CITY: &str = r#"<Dropdown at="1,1" span="4" name="city" bind="person.city" typed="true" onchange="cityChanged" />"#;
const OWNER: &str = r#"<Dropdown at="1,1" span="4" name="owner" bind="task.owner" search="true" onchange="ownerChanged" />"#;

fn options() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/dropdown-options.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/dropdown-options.json")).expect("it is JSON")
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn part<'a>(screen: &'a Screen, name: &str) -> &'a TypeAheadPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<TypeAheadPart>()).expect("a type-ahead dropdown")
}

fn select_all(screen: &mut Screen, name: &str) {
    let id = part_named(screen, name);
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TypeAheadPart>().unwrap().editor_mut().select_all());
}

fn set_text(screen: &mut Screen, name: &str, text: &str) {
    let id = part_named(screen, name);
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TypeAheadPart>().unwrap().editor_mut().set_text(text));
}

fn take(screen: &mut Screen, name: &str) {
    let id = part_named(screen, name);
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<TypeAheadPart>().unwrap().take(cx));
}

fn focus(screen: &mut Screen, name: &str) {
    let id = part_named(screen, name);
    screen.focus(Some(id));
}

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

fn cities(values: &[&str], value: Option<&str>) -> (Rc<InputsHost>, Screen) {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("person.city".into(), value.map(Value::from).unwrap_or(Value::Null));
    let offered: Vec<(Value, &str)> = values.iter().map(|v| (json!(v), *v)).collect();
    host.offer("person.city", &offered);
    let screen = window(CITY, host.clone());
    (host, screen)
}

fn labels(choices: Vec<Choice>) -> Vec<String> {
    choices.into_iter().map(|c| c.label).collect()
}

#[test]
fn a_typed_dropdown_offers_the_values_its_field_has() {
    for case in options()["suggestions"].as_array().unwrap() {
        let values: Vec<&str> = case["values"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let gives: Vec<String> = case["gives"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        let (_, mut screen) = cities(&values, None);
        focus(&mut screen, "city");
        screen.text_typed(case["typed"].as_str().unwrap());
        let city = part(&screen, "city");
        assert_eq!(Variant::Typed, city.variant());
        assert_eq!(gives, labels(city.offered()), "typed {}", case["typed"]);
        assert_eq!(!gives.is_empty(), city.list().is_some(), "typed {}", case["typed"]);
    }
}

#[test]
fn typed_text_is_the_value_and_a_change_only_when_it_differs() {
    let (host, mut screen) = cities(&["Utrecht", "Gent"], Some("Gent"));
    assert_eq!("Gent", part(&screen, "city").editor().text());
    focus(&mut screen, "city");
    select_all(&mut screen, "city");
    screen.text_typed("Rome");
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("Rome")), host.read("person.city"));
    assert_eq!(vec!["write city person.city \"Rome\"", "run city change cityChanged"], host.logged());
    select_all(&mut screen, "city");
    screen.text_typed("utr");
    assert_eq!(1, screen.overlays().len());
    let place = screen.overlays()[0].place;
    let (x, y) = (place.x + 10.0, place.y + 1.0 + LINE / 2.0);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    assert_eq!(Some(json!("Utrecht")), host.read("person.city"));
    assert_eq!("Utrecht", part(&screen, "city").editor().text());
    assert!(screen.overlays().is_empty());
    assert_eq!(4, host.logged().len());
    assert_eq!(Some(part_named(&screen, "city")), screen.focused());
    take(&mut screen, "city");
    assert_eq!(4, host.logged().len());
    // The press in its list was no leaving and no new entering.
    assert!(!host.logged().iter().any(|l| l.contains(" leave ") || l.contains(" enter ")));
}

#[test]
fn down_and_enter_pick_and_escape_puts_the_text_back() {
    let (host, mut screen) = cities(&["Utrecht", "amsterdam", "Gent"], Some("Gent"));
    focus(&mut screen, "city");
    assert_eq!(1, screen.overlays().len());
    select_all(&mut screen, "city");
    screen.text_typed("e");
    assert_eq!(vec!["amsterdam", "Gent", "Utrecht"], labels(part(&screen, "city").offered()));
    press(&mut screen, Key::Escape);
    assert!(screen.overlays().is_empty());
    assert_eq!("Gent", part(&screen, "city").editor().text());
    press(&mut screen, Key::Down);
    press(&mut screen, Key::Down);
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("Gent")), host.read("person.city"));
    assert!(host.logged().is_empty());
}

#[test]
fn what_code_sets_shows_and_is_no_change() {
    let (host, mut screen) = cities(&["Utrecht", "Gent"], Some("Gent"));
    host.set("person.city", json!("Utrecht"));
    screen.resize(800.0, 400.0, 1.0);
    assert_eq!("Utrecht", part(&screen, "city").editor().text());
    assert!(host.logged().is_empty());
}

#[test]
fn a_read_only_one_offers_nothing_and_it_draws() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("person.city".into(), json!("Gent"));
    host.offer("person.city", &[(json!("Utrecht"), "Utrecht")]);
    let mut screen = window(r#"<Dropdown at="1,1" span="4" name="city" bind="person.city" typed="true" readonly="true" />"#, host.clone());
    focus(&mut screen, "city");
    assert!(screen.overlays().is_empty());
    set_text(&mut screen, "city", "Rome");
    take(&mut screen, "city");
    assert!(host.logged().is_empty());
    Shot::of(&mut screen, 800, 400, 1.0);
}

// -- Search ---------------------------------------------------------------------

fn records() -> Vec<Choice> {
    let mut all = vec![Choice { value: Value::Null, label: String::new(), cells: None }];
    for o in options()["search"]["options"].as_array().unwrap() {
        all.push(Choice {
            value: o["id"].clone(),
            label: o["text"].as_str().unwrap().to_string(),
            cells: Some(o["fields"].as_object().unwrap().values().map(|v| v.as_str().unwrap().to_string()).collect()),
        });
    }
    all
}

fn owners(value: Value) -> (Rc<InputsHost>, Screen) {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("task.owner".into(), value);
    host.choices.borrow_mut().insert("task.owner".into(), records());
    host.facts("task.owner", FieldFacts { label: "Owner".into(), refers: Some("Person".into()), ..FieldFacts::default() });
    let screen = window(OWNER, host.clone());
    (host, screen)
}

#[test]
fn a_search_dropdown_offers_the_records_that_hold_every_typed_word() {
    for case in options()["search"]["cases"].as_array().unwrap() {
        let gives: Vec<Value> = case["gives"].as_array().unwrap().clone();
        let (_, mut screen) = owners(Value::Null);
        focus(&mut screen, "owner");
        screen.text_typed(case["typed"].as_str().unwrap());
        let owner = part(&screen, "owner");
        assert_eq!(Variant::Search, owner.variant());
        let ids: Vec<Value> = owner.offered().into_iter().map(|c| c.value).collect();
        assert_eq!(gives, ids, "typed {}", case["typed"]);
    }
}

#[test]
fn a_records_fields_show_as_columns_in_the_list() {
    let (_, mut screen) = owners(Value::Null);
    Shot::of(&mut screen, 800, 400, 1.0);
    focus(&mut screen, "owner");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "inputs-search-dropdown.png");
    let list = screen.find::<ChoiceList>().expect("its list");
    let first = list.cell_boxes(0, screen.fonts(), screen.look());
    assert_eq!(2, first.len());
    // Each field starts where its column starts, in every row.
    assert!(first[1].x > first[0].right());
    assert_eq!(first[1].x, list.cell_boxes(1, screen.fonts(), screen.look())[1].x);
}

#[test]
fn a_picked_record_is_the_value_and_empty_text_is_none() {
    let (host, mut screen) = owners(json!(1));
    assert_eq!("Jansen", part(&screen, "owner").editor().text());
    focus(&mut screen, "owner");
    select_all(&mut screen, "owner");
    screen.text_typed("vries");
    let offered = part(&screen, "owner").offered();
    assert_eq!(1, offered.len());
    assert_eq!("de Vries, Utrecht", line_of(&offered[0]));
    press(&mut screen, Key::Down);
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!(3)), host.read("task.owner"));
    assert_eq!("de Vries", part(&screen, "owner").editor().text());
    select_all(&mut screen, "owner");
    screen.text_typed("Jan");
    take(&mut screen, "owner");
    assert_eq!(Some(json!(3)), host.read("task.owner"));
    assert_eq!("de Vries", part(&screen, "owner").editor().text());
    set_text(&mut screen, "owner", "");
    take(&mut screen, "owner");
    assert_eq!(Some(Value::Null), host.read("task.owner"));
    assert_eq!(vec!["write owner task.owner 3", "run owner change ownerChanged", "write owner task.owner null", "run owner change ownerChanged"], host.logged());
}

#[test]
fn only_a_reference_searches_and_a_plain_dropdown_stays_plain() {
    let host = InputsHost::new();
    host.offer("task.kind", &[(json!("a"), "A")]);
    let screen = window(
        r#"<Dropdown at="1,1" span="4" name="kind" bind="task.kind" search="true" />
           <Dropdown at="1,2" span="4" name="city" bind="city" typed="true" />
           <Dropdown at="1,3" span="4" name="plain" bind="city" />"#,
        host,
    );
    assert!(screen.element(part_named(&screen, "kind")).unwrap().as_any().is::<DropdownPart>());
    assert!(screen.element(part_named(&screen, "city")).unwrap().as_any().is::<TypeAheadPart>());
    assert!(screen.element(part_named(&screen, "plain")).unwrap().as_any().is::<DropdownPart>());
}


#[test]
fn a_window_makes_the_variant_that_fits() {
    let host = InputsHost::new();
    let screen = window(
        r#"<Dropdown at="1,1" span="4" name="typed" bind="city" typed="true" />
           <Image at="1,2" span="4" name="photo" bind="person.photo" />
           <Dropdown at="1,3" span="4" name="plain" bind="city" />"#,
        host,
    );
    assert!(screen.element(part_named(&screen, "typed")).unwrap().as_any().is::<TypeAheadPart>());
    assert!(screen.element(part_named(&screen, "photo")).unwrap().as_any().is::<clarity_ui::parts::inputs::picture_field::PictureFieldPart>());
    assert!(screen.element(part_named(&screen, "plain")).unwrap().as_any().is::<DropdownPart>());
}

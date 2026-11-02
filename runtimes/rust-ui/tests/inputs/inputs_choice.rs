//! The checkbox, switch, dropdown and radio group of a form against the
//! cases every renderer shares (conformance/forms.json): ticks and picks are
//! the person's and change the record; what a handler sets is not. The
//! cases that type, enter or leave are the text inputs'.

use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_ui::input::PointerButton;
use clarity_ui::parts::inputs::choice_list::LINE;
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::radio::RadioPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::Screen;
use serde_json::Value;

const STEPS: [&str; 4] = ["tick", "pick", "handler", "reload"];

fn forms() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/forms.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/forms.json")).expect("it is JSON")
}

fn tag(kind: &str) -> Option<&'static str> {
    match kind {
        "check" => Some("Checkbox"),
        "switch" => Some("Switch"),
        "choice" => Some("Dropdown"),
        "radio" => Some("Radio"),
        _ => None,
    }
}

/// The form: each choice part of the fields, bound to `task.<field>`, with its kind by its name.
fn make(file: &Value) -> (Rc<InputsHost>, Screen, BTreeMap<String, String>) {
    let host = InputsHost::new();
    let mut markup = String::new();
    let mut parts = BTreeMap::new();
    for (row, field) in file["fields"].as_array().unwrap().iter().filter(|f| tag(f["kind"].as_str().unwrap()).is_some()).enumerate() {
        let name = field["field"].as_str().unwrap();
        let tag = tag(field["kind"].as_str().unwrap()).unwrap();
        let path = format!("task.{name}");
        host.set(&path, file["record"][name].clone());
        if let Some(choices) = field["choices"].as_array() {
            let made: Vec<(Value, String)> = choices.iter().map(|c| (c[0].clone(), c[1].as_str().unwrap().to_string())).collect();
            let refs: Vec<(Value, &str)> = made.iter().map(|(v, l)| (v.clone(), l.as_str())).collect();
            host.offer(&path, &refs);
        }
        markup.push_str(&format!(r#"<{tag} at="1,{}" span="6" name="{name}" bind="{path}" onchange="changed" />"#, row + 1));
        parts.insert(name.to_string(), tag.to_string());
    }
    let mut screen = window_of(&markup, host.clone());
    screen.resize(800.0, 600.0, 1.0);
    Shot::of(&mut screen, 800, 600, 1.0);
    (host, screen, parts)
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn run(host: &InputsHost, screen: &mut Screen, parts: &BTreeMap<String, String>, step: &Value) {
    let field = step[1].as_str().unwrap_or("");
    match step[0].as_str().unwrap() {
        "tick" => {
            let b = screen.element(part_named(screen, field)).unwrap().node().bounds;
            click(screen, b.x + 8.0, b.y + b.height / 2.0);
        }
        "pick" if parts[field] == "Dropdown" => {
            let id = part_named(screen, field);
            let at = screen.element(id).and_then(|e| e.as_any().downcast_ref::<DropdownPart>()).unwrap().choices().iter().position(|c| c.value == step[2]).unwrap();
            screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<DropdownPart>().unwrap().open(cx));
            let list = screen.overlays().last().expect("its list").place;
            click(screen, list.x + 10.0, list.y + 1.0 + (at as f64 + 0.5) * LINE);
        }
        "pick" => {
            let radio = screen.element(part_named(screen, field)).and_then(|e| e.as_any().downcast_ref::<RadioPart>()).unwrap();
            let at = radio.choices().iter().position(|c| c.value == step[2]).unwrap();
            let place = radio.places()[at];
            click(screen, place.x + 4.0, place.y + 12.0);
        }
        "handler" => host.set(&format!("task.{field}"), step[2].clone()),
        _ => host.change(),
    }
}

#[test]
fn every_case_of_the_choices_behaves_as_on_every_renderer() {
    let file = forms();
    let mut ran = 0;
    for case in file["cases"].as_array().unwrap() {
        let steps = case["steps"].as_array().unwrap();
        if !steps.iter().all(|s| STEPS.contains(&s[0].as_str().unwrap())) {
            continue;
        }
        ran += 1;
        let name = case["name"].as_str().unwrap();
        let (host, mut screen, parts) = make(&file);
        for step in steps {
            run(&host, &mut screen, &parts, step);
        }
        let expect = &case["expect"];
        let events: Vec<String> = host.logged().iter().filter_map(|l| l.strip_prefix("run ")).map(|r| format!("change {}", r.split(' ').next().unwrap())).collect();
        let wanted: Vec<String> = expect["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(wanted, events, "{name}: events");
        for v in expect["values"].as_array().into_iter().flatten() {
            let field = v[0].as_str().unwrap();
            if parts.contains_key(field) {
                assert_eq!(Some(v[1].clone()), host.read(&format!("task.{field}")), "{name}: {field}");
            }
        }
        for s in expect["shown"].as_array().into_iter().flatten() {
            let field = s[0].as_str().unwrap();
            if parts.get(field).map(String::as_str) == Some("Dropdown") {
                let shown = screen.element(part_named(&screen, field)).and_then(|e| e.as_any().downcast_ref::<DropdownPart>()).unwrap().shown();
                assert_eq!(s[1].as_str().unwrap(), shown, "{name}: {field} shows");
            }
        }
        for (field, labels) in expect["choices"].as_object().into_iter().flatten() {
            let e = screen.element(part_named(&screen, field)).unwrap();
            let offered: Vec<String> = match (e.as_any().downcast_ref::<DropdownPart>(), e.as_any().downcast_ref::<RadioPart>()) {
                (Some(d), _) => d.choices().into_iter().map(|c| c.label).collect(),
                (_, Some(r)) => r.choices().into_iter().map(|c| c.label).collect(),
                _ => Vec::new(),
            };
            let wanted: Vec<String> = labels.as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
            assert_eq!(wanted, offered, "{name}: {field} offers");
        }
    }
    assert!(ran >= 8, "only {ran} cases ran");
}

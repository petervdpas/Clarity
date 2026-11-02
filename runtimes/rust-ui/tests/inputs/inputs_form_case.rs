//! Every case of conformance/forms.json run on one whole form, as the C#
//! FormCaseTests do: the text inputs, the checkbox and switch, the
//! dropdowns, the radio group, the date and the moment side by side, so the
//! cases that mix them (a record read again, then ticked and typed) and the
//! date and time cases run from the shared file too, not restated by hand.

use std::rc::Rc;

use clarity_ui::element::{draw_children, find, in_order, take_typed_from, Cx, Element, ElementId, GridPanel, Node};
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::painter::Painter;
use clarity_ui::parts::inputs::calendar::Day;
use clarity_ui::parts::inputs::choice_list::LINE;
use clarity_ui::parts::inputs::date::DatePart;
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::number::NumberPart;
use clarity_ui::parts::inputs::radio::RadioPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::{part_named, Shot};
use clarity_ui::{Rect, Screen, ThemeLook};
use serde_json::Value;

fn forms() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/forms.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/forms.json")).expect("it is JSON")
}

/// The markup tag of each kind of field.
fn tag_of(kind: &str) -> &'static str {
    match kind {
        "text" => "Textbox",
        "memo" => "Memo",
        "number" => "Number",
        "check" => "Checkbox",
        "switch" => "Switch",
        "choice" => "Dropdown",
        "radio" => "Radio",
        _ => "Date",
    }
}

/// A value of the file as the record holds it: a number as a number, else as it is.
fn stored(kind: &str, value: &Value) -> Value {
    match (kind, value) {
        ("number", Value::String(s)) => s.parse::<f64>().map(Value::from).unwrap_or(Value::Null),
        _ => value.clone(),
    }
}

/// A value as one text, the same for the file's and the record's: a number by its amount.
fn written(kind: &str, value: &Value) -> String {
    match (kind, value) {
        ("number", Value::String(s)) => s.parse::<f64>().map(|n| n.to_string()).unwrap_or_else(|_| s.clone()),
        ("number", Value::Number(n)) => n.as_f64().unwrap_or_default().to_string(),
        (_, Value::String(s)) => s.clone(),
        (_, other) => other.to_string(),
    }
}

/// A form around a grid of parts: Enter in its one-line inputs takes what is typed and submits it.
struct TestForm {
    node: Node,
    host: Rc<InputsHost>,
}

impl Element for TestForm {
    clarity_ui::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        draw_children(&self.node, p);
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

struct Form {
    host: Rc<InputsHost>,
    screen: Screen,
    kinds: Vec<(String, String)>,
}

impl Form {
    fn kind(&self, field: &str) -> &str {
        self.kinds.iter().find(|(f, _)| f == field).map(|(_, k)| k.as_str()).unwrap_or("")
    }

    fn load(&self, forms: &Value) {
        for (field, value) in forms["record"].as_object().unwrap() {
            let kind = self.kind(field).to_string();
            self.host.values.borrow_mut().insert(field.clone(), stored(&kind, value));
        }
    }

    fn shot(&mut self) {
        Shot::of(&mut self.screen, 800, 900, 1.0);
    }

    fn part<T: 'static>(&self, field: &str) -> &T {
        self.screen.element(part_named(&self.screen, field)).and_then(|e| e.as_any().downcast_ref::<T>()).unwrap_or_else(|| panic!("{field} is no such part"))
    }

    /// The input a field's text is typed in: a moment's is its time.
    fn input(&self, field: &str) -> ElementId {
        let part = self.screen.element(part_named(&self.screen, field)).unwrap();
        let inputs: Vec<ElementId> = in_order(part).into_iter().filter(|e| e.typing() && e.focusable()).map(|e| e.node().id()).collect();
        if self.kind(field) == "moment" { inputs[1] } else { inputs[0] }
    }

    fn text(&self, field: &str) -> String {
        match self.kind(field) {
            "choice" => self.part::<DropdownPart>(field).shown(),
            "number" => self.part::<NumberPart>(field).editor().text(),
            "moment" => self.part::<DatePart>(field).time().unwrap().text(),
            "date" => self.part::<DatePart>(field).editor().text(),
            _ => self.part::<TextBoxPart>(field).editor().text(),
        }
    }

    fn marked(&self, field: &str) -> bool {
        match self.kind(field) {
            "number" => self.part::<NumberPart>(field).bad().is_some(),
            "date" | "moment" => {
                let date = self.part::<DatePart>(field);
                date.bad().is_some() || date.bad_time().is_some()
            }
            _ => false,
        }
    }

    fn bounds(&self, field: &str) -> Rect {
        self.screen.element(part_named(&self.screen, field)).unwrap().node().bounds
    }

    fn click(&mut self, x: f64, y: f64) {
        self.screen.pointer_pressed(x, y, PointerButton::Left, 1);
        self.screen.pointer_released(x, y, PointerButton::Left);
    }

    fn run(&mut self, forms: &Value, step: &Value) {
        let field = step.get(1).and_then(Value::as_str).unwrap_or("").to_string();
        let kind = self.kind(&field).to_string();
        match step[0].as_str().unwrap() {
            "focus" => {
                let input = self.input(&field);
                self.screen.focus(Some(input));
            }
            "type" => {
                let input = self.input(&field);
                self.screen.focus(Some(input));
                self.screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
                let text = step[2].as_str().unwrap();
                if text.is_empty() {
                    self.screen.key_pressed(KeyInput::plain(Key::Backspace));
                } else {
                    self.screen.text_typed(text);
                }
            }
            "enter" => {
                self.screen.key_pressed(KeyInput::plain(Key::Enter));
            }
            "leave" => self.screen.focus(None),
            "tick" => {
                let b = self.bounds(&field);
                self.click(b.x + 8.0, b.y + b.height / 2.0);
            }
            "pick" if kind == "choice" => {
                let want = written(&kind, &step[2]);
                let at = self.part::<DropdownPart>(&field).choices().iter().position(|c| written(&kind, &c.value) == want).expect("the choice is offered");
                let b = self.bounds(&field);
                self.click(b.x + 20.0, b.y + b.height / 2.0);
                let list = self.screen.overlays().last().expect("the list is open").place;
                self.click(list.x + 10.0, list.y + 1.0 + (at as f64 + 0.5) * LINE);
            }
            "pick" => {
                let want = written(&kind, &step[2]);
                let radio = self.part::<RadioPart>(&field);
                let at = radio.choices().iter().position(|c| written(&kind, &c.value) == want).expect("the choice is offered");
                let place = radio.places()[at];
                self.click(place.x + 4.0, place.y + place.height / 2.0);
            }
            "pickDay" => {
                let button = self.part::<DatePart>(&field).button();
                self.click(button.x + button.width / 2.0, button.y + button.height / 2.0);
                let calendar = self.part::<DatePart>(&field).calendar().expect("the calendar is open");
                let cell = calendar.cell_of(Day::parse(step[2].as_str().unwrap()).unwrap()).expect("the day shows");
                self.click(cell.x + cell.width / 2.0, cell.y + cell.height / 2.0);
            }
            "handler" => self.host.set(&field, stored(&kind, &step[2])),
            "reload" => {
                self.load(forms);
                self.host.change();
            }
            other => panic!("no step {other}"),
        }
        self.shot();
    }
}

fn make(forms: &Value) -> Form {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.facts("hours", FieldFacts { label: "Hours".into(), format: Some("decimal 1".into()), ..Default::default() });
    host.facts("day", FieldFacts { label: "Day".into(), format: Some("date".into()), ..Default::default() });
    host.facts("due", FieldFacts { label: "Due".into(), format: Some("dateTime".into()), ..Default::default() });
    let mut markup = String::new();
    let mut kinds = Vec::new();
    let mut row = 1;
    for field in forms["fields"].as_array().unwrap() {
        let name = field["field"].as_str().unwrap();
        let kind = field["kind"].as_str().unwrap();
        kinds.push((name.to_string(), kind.to_string()));
        if let Some(choices) = field["choices"].as_array() {
            let offered: Vec<(Value, &str)> = choices.iter().map(|c| (c[0].clone(), c[1].as_str().unwrap())).collect();
            host.offer(name, &offered);
        }
        let rows = if kind == "memo" { r#" rows="2""# } else { "" };
        let label = field["label"].as_str().unwrap();
        markup.push_str(&format!(r#"<{} at="1,{row}" span="6"{rows} name="{name}" bind="{name}" caption="{label}" onchange="changed" />"#, tag_of(kind)));
        row += if kind == "memo" { 2 } else { 1 };
    }
    let spec = clarity_ui::markup::read(&format!(r#"<Window name="Test">{markup}</Window>"#)).expect("the markup reads");
    let mut grid = GridPanel::new(12);
    for p in spec.parts {
        let made = clarity_ui::parts::make(p, host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let mut form_element = TestForm { node: Node::new(), host: host.clone() };
    form_element.node.add(Box::new(grid));
    let mut screen = Screen::new(Box::new(form_element), ThemeLook::clarity(false));
    screen.resize(800.0, 900.0, 1.0);
    let mut form = Form { host, screen, kinds };
    form.load(forms);
    form.host.change();
    form.shot();
    form
}

#[test]
fn every_case_behaves_as_on_every_renderer() {
    let forms = forms();
    let cases = forms["cases"].as_array().unwrap();
    let mut wrong = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut form = make(&forms);
        for step in case["steps"].as_array().unwrap() {
            form.run(&forms, step);
        }
        let expect = &case["expect"];
        let events: Vec<String> = form
            .host
            .logged()
            .iter()
            .filter_map(|l| if l == "submit" { Some(l.clone()) } else { l.strip_prefix("run ").and_then(|r| r.split_once(' ')).filter(|(_, rest)| rest.starts_with("change ")).map(|(part, _)| format!("change {part}")) })
            .collect();
        let wanted: Vec<String> = expect["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        if wanted != events {
            wrong.push(format!("{name}: events {events:?}"));
        }
        for v in expect["values"].as_array().into_iter().flatten() {
            let field = v[0].as_str().unwrap();
            let kind = form.kind(field).to_string();
            let now = form.host.read(field).unwrap_or(Value::Null);
            if written(&kind, &v[1]) != written(&kind, &now) {
                wrong.push(format!("{name}: {field} is {now}"));
            }
        }
        let marked: Vec<&str> = expect["marked"].as_array().into_iter().flatten().map(|m| m.as_str().unwrap()).collect();
        for (field, _) in form.kinds.clone() {
            if marked.contains(&field.as_str()) != form.marked(&field) {
                wrong.push(format!("{name}: {field} marked {}", form.marked(&field)));
            }
        }
        for s in expect["shown"].as_array().into_iter().flatten() {
            let field = s[0].as_str().unwrap();
            let text = form.text(field);
            if s[1].as_str().unwrap() != text {
                wrong.push(format!("{name}: {field} shows {text:?}"));
            }
        }
        for (field, labels) in expect["choices"].as_object().into_iter().flatten() {
            let offered: Vec<String> = match form.kind(field) {
                "choice" => form.part::<DropdownPart>(field).choices().into_iter().map(|c| c.label).collect(),
                _ => form.part::<RadioPart>(field).choices().into_iter().map(|c| c.label).collect(),
            };
            let wanted: Vec<String> = labels.as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
            if wanted != offered {
                wrong.push(format!("{name}: {field} offers {offered:?}"));
            }
        }
    }
    assert_eq!(25, cases.len(), "every case of the file runs");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

//! The parts of a sign-in view (docs/APP-CAPABILITIES-PLAN.md, "The
//! sign-in window is designed, and guided"): a field for an input of a
//! strategy (a label over a text box, a password box for `password`), a
//! sign-in button for a strategy, "keep me signed in", the language
//! dropdown, an "other way" link that shows another sign-in view, and the
//! refusal, where a refused try says why. Each is drawn with the parts
//! every window has, in Clarity's own words unless its caption says others;
//! what it signs in with is the project's `signin` block. The window host
//! signs in with the fields of the button's view (`ProgramHost::sign_in_view`).

use std::rc::Rc;

use clarity_runtime::ui::{Level, PartState, Text};
use serde_json::{Map, Value};

use crate::element::{self, Cx, Element, GridPanel};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::markup::{HandlerCall, PartSpec};
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::container;
use crate::parts::{Choice, Formats, Part, PartBase, PartHost};

/// The path the language dropdown is bound to: no data of the window, the language shown.
pub const LANGUAGE_PATH: &str = "@language";

/// The handler of an "other way" link: `signinShow <view>`.
pub const SHOW: &str = "signinShow";

/// The roles of what a sign-in part holds, the last word of its name: a field, a button, a check box.
pub const FIELD: &str = "field";
pub const BUTTON: &str = "button";
pub const BOX: &str = "box";

/// The words of an input, as a caption: Clarity's key, `@` first.
pub fn input_caption(input: &str) -> String {
    match input {
        "name" => "@signinName".to_string(),
        "password" => "@signinPassword".to_string(),
        "token" => "@signinToken".to_string(),
        "code" => "@signinCode".to_string(),
        other => other.to_string(),
    }
}

/// The words of a strategy's button.
pub fn button_caption(strategy: &str) -> &'static str {
    match strategy {
        "account" => "@signinAccountButton",
        "github" => "@signinGithub",
        "microsoft" => "@signinMicrosoft",
        "openid" => "@signinOpenid",
        _ => "@signinButton",
    }
}

/// The words of an "other way" to the view of `strategy`.
pub fn other_caption(strategy: Option<&str>) -> &'static str {
    match strategy {
        Some("recovery") => "@signinUseCode",
        Some("token") => "@signinUseToken",
        _ => "@signinUsePassword",
    }
}

/// The window's host as the parts inside a sign-in part see it: Clarity's
/// own words for their captions, and the language shown for the dropdown
/// bound to `LANGUAGE_PATH`.
struct OwnWords {
    window: Rc<dyn PartHost>,
}

impl PartHost for OwnWords {
    fn read(&self, path: &str) -> Option<Value> {
        if path == LANGUAGE_PATH {
            return Some(Value::from(crate::choices::language()));
        }
        self.window.read(path)
    }

    /// Picking a language shows every text in it, as the frame's picker does.
    fn write(&self, part: &str, path: &str, value: Value) {
        if path == LANGUAGE_PATH {
            if let Some(language) = value.as_str() {
                crate::choices::choose_language(language);
            }
            return;
        }
        self.window.write(part, path, value);
    }

    fn choices(&self, path: &str) -> Vec<Choice> {
        if path == LANGUAGE_PATH {
            return crate::choices::languages().into_iter().map(|o| Choice { value: Value::from(o.name), label: o.label, cells: None }).collect();
        }
        self.window.choices(path)
    }

    fn offered(&self, part: &PartSpec, path: &str) -> Vec<Choice> {
        if path == LANGUAGE_PATH {
            return self.choices(path);
        }
        self.window.offered(part, path)
    }

    fn edited(&self, part: &str) {
        self.window.edited(part);
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.window.state(name)
    }

    fn set_state(&self, name: &str, state: PartState) {
        self.window.set_state(name, state);
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.window.run(part, call)
    }

    fn can_run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.window.can_run(part, call)
    }

    fn text(&self, written: Option<&str>) -> String {
        match written.and_then(|w| w.strip_prefix('@')).and_then(crate::parts::signin::word) {
            Some(own) => own,
            None => self.window.text(written),
        }
    }

    fn formats(&self) -> Formats {
        self.window.formats()
    }

    fn icon_path(&self, name: &str) -> Option<String> {
        self.window.icon_path(name)
    }

    fn changes(&self) -> u64 {
        self.window.changes()
    }

    fn say(&self, level: Level, text: Text) {
        self.window.say(level, text);
    }

    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        self.window.keys_of(command, designed)
    }

    fn window_name(&self) -> Option<String> {
        self.window.window_name()
    }

    fn signin(&self) -> Option<crate::facts::SigninFacts> {
        self.window.signin()
    }

    fn sign_in(&self, strategy: &str, inputs: Map<String, Value>) -> Result<(), Text> {
        self.window.sign_in(strategy, inputs)
    }

    fn signin_refusal(&self) -> Option<Text> {
        self.window.signin_refusal()
    }

    fn signin_way(&self, view: &str) -> Option<String> {
        self.window.signin_way(view)
    }
}

/// A part spec of `kind` named `name` in row `row` of a one-column grid, with Clarity's defaults of the kind.
fn spec_of(kind: &str, name: &str, row: i64) -> PartSpec {
    let mut props = crate::markup::defaults_of(kind).cloned().unwrap_or_default();
    props.insert("kind".into(), Value::from(kind));
    props.insert("name".into(), Value::from(name));
    for (field, value) in [("col", 1), ("row", row), ("width", 1), ("height", 1)] {
        props.insert(field.into(), Value::from(value));
    }
    PartSpec::new(kind, props)
}

/// The caption a part shows: its own, else Clarity's words for it.
fn caption_or(spec: &PartSpec, own: &str) -> Value {
    Value::from(spec.text("caption").filter(|c| !c.is_empty()).unwrap_or(own).to_string())
}

/// A sign-in part drawn as other parts in a grid of one column: a label
/// over a field, or one button, check box or link filling it.
pub struct SigninViewPart {
    base: PartBase,
}

impl SigninViewPart {
    fn new(spec: PartSpec, host: Rc<dyn PartHost>, inside: Vec<PartSpec>) -> SigninViewPart {
        let own: Rc<dyn PartHost> = Rc::new(OwnWords { window: host.clone() });
        let mut base = PartBase::new(spec, host);
        let grid = container::grid_of(&inside, 1, GAP, &own);
        base.node.add(Box::new(grid));
        SigninViewPart { base }
    }

    fn grid(&self) -> &GridPanel {
        self.base.node.children()[0].as_any().downcast_ref::<GridPanel>().expect("a sign-in part holds its grid")
    }
}

impl Element for SigninViewPart {
    part_element!();

    fn around(&self) -> f64 {
        container::around_for(&self.base.spec, 0.0, container::least_of(self.grid()))
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        element::arrange_safely(self.base.node.children_mut()[0].as_mut(), area, cx);
    }

    fn draw(&self, p: &Painter) {
        element::draw_children(&self.base.node, p);
    }
}

impl Part for SigninViewPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// The name of the field, button, box or link inside the part named `part`: `passwordField.field`.
pub fn inside(part: &str, role: &str) -> String {
    format!("{part}.{role}")
}

fn own_name(spec: &PartSpec) -> String {
    spec.name().unwrap_or_default().to_string()
}

/// `signinField input password`: its label, then a text box (a password box for `password`).
pub fn make_field(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let name = own_name(&spec);
    let input = spec.text("input").unwrap_or_default().to_string();
    let words = caption_or(&spec, &input_caption(&input));
    let field_name = inside(&name, FIELD);
    let mut label = spec_of("label", &inside(&name, "label"), 1);
    label.props.insert("caption".into(), words.clone());
    label.props.insert("for".into(), Value::from(field_name.as_str()));
    let kind = if input == "password" { "password" } else { "textbox" };
    let mut field = spec_of(kind, &field_name, 2);
    field.props.insert("caption".into(), words);
    field.props.insert("noUndo".into(), Value::Bool(true));
    Box::new(SigninViewPart::new(spec, host, vec![label, field]))
}

/// `signinButton strategy password`: a button that signs in with the strategy and the fields of its view.
pub fn make_button(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let name = own_name(&spec);
    let strategy = spec.text("strategy").unwrap_or_default().to_string();
    let mut button = spec_of("button", &inside(&name, BUTTON), 1);
    button.props.insert("caption".into(), caption_or(&spec, button_caption(&strategy)));
    button.handlers.push(HandlerCall::new("click", "signin", vec![strategy]));
    Box::new(SigninViewPart::new(spec, host, vec![button]))
}

/// "Keep me signed in": a check box that goes with the sign-in of its view as the input `remember`.
pub fn make_remember(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let mut check = spec_of("checkbox", &inside(&own_name(&spec), BOX), 1);
    check.props.insert("caption".into(), caption_or(&spec, "@signinRemember"));
    Box::new(SigninViewPart::new(spec, host, vec![check]))
}

/// The language dropdown: its label, then the program's languages, each by its own name.
pub fn make_language(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let name = own_name(&spec);
    let dropdown_name = inside(&name, "dropdown");
    let mut label = spec_of("label", &inside(&name, "label"), 1);
    label.props.insert("caption".into(), caption_or(&spec, "@languagePicker"));
    label.props.insert("for".into(), Value::from(dropdown_name.as_str()));
    let mut dropdown = spec_of("dropdown", &dropdown_name, 2);
    dropdown.props.insert("bind".into(), Value::from(LANGUAGE_PATH));
    Box::new(SigninViewPart::new(spec, host, vec![label, dropdown]))
}

/// "Other way": a link that shows another sign-in view (`shows CodeView`), named after the way it shows.
pub fn make_other(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let view = spec.text("shows").unwrap_or_default().to_string();
    let way = host.signin_way(&view);
    let mut link = spec_of("link", &inside(&own_name(&spec), "link"), 1);
    link.props.insert("caption".into(), caption_or(&spec, other_caption(way.as_deref())));
    link.handlers.push(HandlerCall::new("click", SHOW, vec![view]));
    Box::new(SigninViewPart::new(spec, host, vec![link]))
}

/// Where a refused sign-in says why: the last refusal of the window, in the error color; nothing before one.
pub struct RefusalPart {
    base: PartBase,
}

pub fn make_refusal(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(RefusalPart { base: PartBase::new(spec, host) })
}

impl RefusalPart {
    /// What it says now: the last refusal, as it shows.
    pub fn text(&self) -> Option<String> {
        self.base.host.signin_refusal().map(|t| t.show()).filter(|t| !t.is_empty())
    }
}

impl Element for RefusalPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let Some(says) = self.text() else { return };
        let bounds = self.base.node.bounds;
        let size = p.look.size("small");
        p.text(&p.fit(&says, bounds.width, size, "text", false), bounds, p.look.color("error"), size, "text", false, TextAlign::Left);
    }
}

impl Part for RefusalPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// The kinds of a sign-in view, each with what makes it.
pub const KINDS: [(&str, crate::parts::MakePart); 6] = [
    ("signinField", make_field),
    ("signinButton", make_button),
    ("signinRemember", make_remember),
    ("signinLanguage", make_language),
    ("signinOther", make_other),
    ("signinRefusal", make_refusal),
];

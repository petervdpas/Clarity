//! The sign-in part (docs/APP-CAPABILITIES-PLAN.md step 4, "The sign-in
//! part"): on the sign-in window or the app's splash, it draws itself from
//! the project's `signin` block, never from strategies written here. Per
//! strategy it offers (the block's `shows`, else every strategy of the
//! block that works on this target) a labelled field per input of the strategy
//! (a password field for `password`, a text box for the others) and the
//! strategy's button; `account` is a button alone, and `recovery`, beside
//! other strategies, waits behind "Use a recovery code", which swaps to its
//! name and code and a Back. When the block's `remember` asks, a check box
//! "Keep me signed in" goes with every sign-in as the input `remember`.
//! When the block turns on its `language picker`, a dropdown of the
//! program's languages, each by its own name, stands above the fields;
//! picking one shows every text in it, as the frame's language picker does.
//! The words are Clarity's own, translated.
//!
//! The fields and buttons are the text boxes and buttons every window has,
//! in a grid of one column; what is typed stays in the part (it never goes
//! into the window's values, nor to the program, until a sign-in). Pressing
//! a strategy's button, or Enter in one of its fields, signs in through the
//! host the same way a `signin <strategy>` button does; a refusal shows on
//! a line under the fields.
//!
//! In the IDE's designer (its markup carries `designItem`) the part is a
//! placeholder: a dashed frame naming the strategies it offers, no fields.
//! What it offers is the project's sign-in setup, not the window's.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use clarity_runtime::ui::{Level, PartState, Text};
use serde_json::{Map, Value};
use skia_safe::{Paint, PaintStyle, PathEffect};

use crate::element::{self, Cx, Element, GridPanel, Node};
use crate::facts::{SigninFacts, SigninStrategyFacts};
use crate::geometry::Rect;
use crate::grid::{row_height, GAP};
use crate::locale;
use crate::markup::{HandlerCall, PartSpec};
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::container;
use crate::parts::{Formats, Part, PartBase, PartHost};

/// The strategy that waits behind "Use a recovery code" when others are offered too.
const RECOVERY: &str = "recovery";

/// The handler a strategy's button runs: `signin(<strategy>)`, as a hand-built button writes it.
const SIGN_IN: &str = "signin";
/// The handlers of "Use a recovery code" and Back.
const USE_CODE: &str = "signinUseCode";
const BACK: &str = "signinBack";

/// Clarity's own words of the part, by their keys, with the English they fall back to.
const WORDS: [(&str, &str); 16] = [
    ("signinName", "Name"),
    ("signinPassword", "Password"),
    ("signinToken", "Token"),
    ("signinCode", "Recovery code"),
    ("signinButton", "Sign in"),
    ("signinAccountButton", "Use this computer's account"),
    ("signinUseCode", "Use a recovery code"),
    ("signinUsePassword", "Sign in with a password"),
    ("signinUseToken", "Use a token"),
    ("signinBack", "Back"),
    ("signinRemember", "Keep me signed in"),
    ("signinNothing", "No way to sign in here"),
    ("languagePicker", "Language"),
    ("signinGithub", "Sign in with GitHub"),
    ("signinMicrosoft", "Sign in with Microsoft"),
    ("signinOpenid", "Sign in with your account"),
];

/// The words of an input as written in a part's caption: Clarity's key, `@` first.
fn input_caption(input: &str) -> String {
    match input {
        "name" => "@signinName".to_string(),
        "password" => "@signinPassword".to_string(),
        "token" => "@signinToken".to_string(),
        "code" => "@signinCode".to_string(),
        // An input the catalog has and the part has no word for shows by its name.
        other => other.to_string(),
    }
}

/// The words of a strategy's button.
fn button_caption(strategy: &str) -> &'static str {
    match strategy {
        "account" => "@signinAccountButton",
        _ => "@signinButton",
    }
}

/// One of the part's words in the language shown.
pub fn word(key: &str) -> Option<String> {
    WORDS.iter().find(|(k, _)| *k == key).map(|(k, english)| locale::translate(k, english))
}

/// The target this screen runs on: `web` in the browser, else `desktop` (headless as the desktop).
pub fn this_target() -> &'static str {
    if cfg!(target_os = "emscripten") { "web" } else { "desktop" }
}

/// The strategies a sign-in part offers on `target`: those the block's
/// `shows` names, in that order, else every one of the block in its order;
/// each only where it works on `target`.
pub fn offered(block: &SigninFacts, target: &str) -> Vec<SigninStrategyFacts> {
    let works = |s: &&SigninStrategyFacts| s.works_on.iter().any(|t| t == target);
    if block.shows.is_empty() {
        return block.strategies.iter().filter(works).cloned().collect();
    }
    block.shows.iter().filter_map(|n| block.strategies.iter().find(|s| &s.name == n)).filter(works).cloned().collect()
}

/// What the part's fields and buttons stand on: their own states (what was
/// typed), the refusal to show, whether the recovery code shows, and the
/// window's host for everything else.
pub struct SigninHost {
    window: Rc<dyn PartHost>,
    /// The part's name, the first word of every name inside it.
    prefix: String,
    states: RefCell<HashMap<String, PartState>>,
    changed: Cell<u64>,
    refusal: RefCell<Option<Text>>,
    recovering: Cell<bool>,
    strategies: Vec<SigninStrategyFacts>,
    /// Whether the part asks to keep the person signed in.
    remember_asks: bool,
    /// Whether the part lets people pick their language.
    language_picker: bool,
}

impl SigninHost {
    /// The name of the check box "Keep me signed in": `signin.remember`.
    pub fn remember_name(&self) -> String {
        [self.prefix.as_str(), "remember"].join(".")
    }

    /// The name of the language dropdown: `signin.language`.
    pub fn language_name(&self) -> String {
        [self.prefix.as_str(), "language"].join(".")
    }

    /// The name of the field of `input` of `strategy`: `signin.password.name`.
    pub fn field_name(&self, strategy: &str, input: &str) -> String {
        [self.prefix.as_str(), strategy, input].join(".")
    }

    /// The name of a button of the part: `signin.password.button`.
    fn button_name(&self, strategy: &str) -> String {
        [self.prefix.as_str(), strategy, "button"].join(".")
    }

    fn mine(&self, name: &str) -> bool {
        name.strip_prefix(self.prefix.as_str()).is_some_and(|rest| rest.starts_with('.'))
    }

    fn bump(&self) {
        self.changed.set(self.changed.get() + 1);
    }

    /// What was typed in a field of the part.
    fn typed(&self, name: &str) -> String {
        self.states.borrow().get(name).and_then(|s| s.text.clone()).unwrap_or_default()
    }

    /// The refusal of the last try, as it shows; none after a sign-in that worked or before any.
    pub fn refusal(&self) -> Option<String> {
        self.refusal.borrow().as_ref().map(Text::show).filter(|t| !t.is_empty())
    }

    /// Whether the recovery code shows in place of the other strategies.
    pub fn recovering(&self) -> bool {
        self.recovering.get()
    }

    /// Signs in with `strategy` and what was typed in its fields, through
    /// the window's host as a `signin <strategy>` button does. True when it worked.
    pub fn sign_in(&self, strategy: &str) -> bool {
        let Some(offered) = self.strategies.iter().find(|s| s.name == strategy) else { return false };
        let mut inputs = Map::new();
        for input in &offered.inputs {
            inputs.insert(input.clone(), Value::String(self.typed(&self.field_name(strategy, input))));
        }
        if self.remember_asks {
            let ticked = self.states.borrow().get(&self.remember_name()).and_then(|s| s.checked).unwrap_or(false);
            inputs.insert("remember".into(), Value::String(ticked.to_string()));
        }
        let answer = self.window.sign_in(strategy, inputs);
        let worked = answer.is_ok();
        *self.refusal.borrow_mut() = answer.err();
        self.bump();
        worked
    }

    /// Shows the recovery code (with the name typed so far), or the other strategies again.
    pub fn recover(&self, on: bool) {
        if on {
            let typed = self.strategies.iter().filter(|s| s.name != RECOVERY && s.inputs.iter().any(|i| i == "name")).map(|s| self.typed(&self.field_name(&s.name, "name"))).find(|t| !t.is_empty());
            let name = self.field_name(RECOVERY, "name");
            if let Some(typed) = typed.filter(|_| self.typed(&name).is_empty()) {
                self.states.borrow_mut().entry(name).or_default().text = Some(typed);
            }
        }
        self.recovering.set(on);
        *self.refusal.borrow_mut() = None;
        self.bump();
    }
}

impl PartHost for SigninHost {
    fn read(&self, path: &str) -> Option<Value> {
        if self.language_picker && path == self.language_name() {
            return Some(Value::from(crate::choices::language()));
        }
        self.window.read(path)
    }

    /// Picking a language shows every text in it, as the frame's picker does.
    fn write(&self, part: &str, path: &str, value: Value) {
        if self.language_picker && path == self.language_name() {
            if let Some(language) = value.as_str() {
                crate::choices::choose_language(language);
            }
            self.bump();
            return;
        }
        self.window.write(part, path, value);
    }

    fn choices(&self, path: &str) -> Vec<crate::parts::Choice> {
        if self.language_picker && path == self.language_name() {
            return crate::choices::languages().into_iter().map(|o| crate::parts::Choice { value: Value::from(o.name), label: o.label, cells: None }).collect();
        }
        self.window.choices(path)
    }

    /// What is typed in the part is no edit of the window: it stays here.
    fn edited(&self, part: &str) {
        if !self.mine(part) {
            self.window.edited(part);
        }
        self.bump();
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        let name = name?;
        if !self.mine(name) {
            return self.window.state(Some(name));
        }
        Some(self.states.borrow().get(name).cloned().unwrap_or_default())
    }

    fn set_state(&self, name: &str, state: PartState) {
        if !self.mine(name) {
            return self.window.set_state(name, state);
        }
        self.states.borrow_mut().insert(name.to_string(), state);
        self.bump();
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        // While a long call runs, nothing else does, as for every button.
        if !self.window.can_run(part, call) {
            return false;
        }
        match (call.handler.as_str(), call.args.as_slice()) {
            (SIGN_IN, [strategy]) => self.sign_in(strategy),
            (USE_CODE, []) => {
                self.recover(true);
                true
            }
            (BACK, []) => {
                self.recover(false);
                true
            }
            _ => false,
        }
    }

    fn can_run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.window.can_run(part, call)
    }

    /// The part's own words are Clarity's; anything else as the window says it.
    fn text(&self, written: Option<&str>) -> String {
        match written.and_then(|w| w.strip_prefix('@')).and_then(word) {
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
        self.window.changes().wrapping_add(self.changed.get())
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

    fn signin(&self) -> Option<SigninFacts> {
        self.window.signin()
    }
}

/// The fields and buttons of one side of the part: the strategies, or the recovery code.
struct Side {
    node: Node,
    own: Rc<SigninHost>,
    recovery: bool,
}

impl Side {
    fn grid(&self) -> &GridPanel {
        self.node.children()[0].as_any().downcast_ref::<GridPanel>().expect("a side holds its grid")
    }
}

impl Element for Side {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.own.recovering() == self.recovery
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        element::arrange_safely(self.node.children_mut()[0].as_mut(), area, cx);
    }

    fn draw(&self, p: &Painter) {
        if self.is_shown() {
            element::draw_children(&self.node, p);
        }
    }
}

/// A part spec of `kind` in row `row` of the part's one column, with Clarity's defaults of the kind.
fn spec_of(kind: &str, name: &str, row: i64) -> PartSpec {
    let mut props = crate::markup::defaults_of(kind).cloned().unwrap_or_default();
    props.insert("kind".into(), Value::from(kind));
    props.insert("name".into(), Value::from(name));
    for (field, value) in [("col", 1), ("row", row), ("width", 1), ("height", 1)] {
        props.insert(field.into(), Value::from(value));
    }
    PartSpec::new(kind, props)
}

/// The rows of `strategies` on one side: a label and a field per input, then the strategy's button.
fn rows_of(own: &SigninHost, strategies: &[&SigninStrategyFacts], default: Option<&str>, remember: bool, row: &mut i64, specs: &mut Vec<PartSpec>) {
    let mut remember = remember;
    for strategy in strategies {
        for input in &strategy.inputs {
            let field = own.field_name(&strategy.name, input);
            let mut label = spec_of("label", &[field.as_str(), "label"].join("."), *row);
            label.props.insert("caption".into(), Value::from(input_caption(input)));
            label.props.insert("for".into(), Value::from(field.as_str()));
            specs.push(label);
            *row += 1;
            let kind = if input == "password" { "password" } else { "textbox" };
            let mut spec = spec_of(kind, &field, *row);
            spec.props.insert("caption".into(), Value::from(input_caption(input)));
            spec.props.insert("noUndo".into(), Value::Bool(true));
            specs.push(spec);
            *row += 1;
        }
        // "Keep me signed in" comes once, above the first button.
        if std::mem::take(&mut remember) {
            let mut check = spec_of("checkbox", &own.remember_name(), *row);
            check.props.insert("caption".into(), Value::from("@signinRemember"));
            specs.push(check);
            *row += 1;
        }
        let mut button = spec_of("button", &own.button_name(&strategy.name), *row);
        button.props.insert("caption".into(), Value::from(button_caption(&strategy.name)));
        button.props.insert("default".into(), Value::Bool(default == Some(strategy.name.as_str())));
        button.handlers.push(HandlerCall::new("click", SIGN_IN, vec![strategy.name.clone()]));
        specs.push(button);
        *row += 1;
    }
}

/// The language dropdown with its label, from row `row`.
fn language_rows(own: &SigninHost, row: &mut i64, specs: &mut Vec<PartSpec>) {
    let name = own.language_name();
    let mut label = spec_of("label", &[name.as_str(), "label"].join("."), *row);
    label.props.insert("caption".into(), Value::from("@languagePicker"));
    label.props.insert("for".into(), Value::from(name.as_str()));
    specs.push(label);
    *row += 1;
    let mut dropdown = spec_of("dropdown", &name, *row);
    dropdown.props.insert("bind".into(), Value::from(name.as_str()));
    specs.push(dropdown);
    *row += 1;
}

/// A button of the part's own (Use a recovery code, Back) in row `row`.
fn own_button(own: &SigninHost, which: &str, handler: &str, caption: &str, row: i64) -> PartSpec {
    let mut button = spec_of("button", &own.button_name(which), row);
    button.props.insert("caption".into(), Value::from(caption));
    button.handlers.push(HandlerCall::new("click", handler, Vec::new()));
    button
}

pub struct SigninPart {
    base: PartBase,
    own: Rc<SigninHost>,
    /// Whether the recovery code showed when it was last laid out: the focus follows a swap.
    laid_recovering: Cell<bool>,
    /// Drawn by the IDE's designer: a placeholder without fields.
    designed: bool,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(SigninPart::new(spec, host))
}

impl SigninPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> SigninPart {
        let block = host.signin();
        let strategies = block.as_ref().map(|block| offered(block, this_target())).unwrap_or_default();
        let remember_asks = block.as_ref().is_some_and(|b| b.remember_asks);
        let language_picker = block.as_ref().is_some_and(|b| b.language_picker);
        let prefix = spec.name().filter(|n| !n.is_empty()).unwrap_or("signin").to_string();
        let only_recovery = strategies.iter().all(|s| s.name == RECOVERY);
        let own = Rc::new(SigninHost { window: host.clone(), prefix, states: RefCell::default(), changed: Cell::new(0), refusal: RefCell::default(), recovering: Cell::new(false), strategies, remember_asks, language_picker });
        let designed = spec.props.contains_key(crate::markup::DESIGN_ITEM);
        let mut base = PartBase::new(spec, host);
        if designed {
            return SigninPart { base, own, laid_recovering: Cell::new(false), designed };
        }
        let (front, recovery): (Vec<&SigninStrategyFacts>, Vec<&SigninStrategyFacts>) = own.strategies.iter().partition(|s| s.name != RECOVERY || only_recovery);
        // The first strategy shown is the default: Enter anywhere in the window presses its button.
        let default = front.first().map(|s| s.name.clone());
        let mut specs = Vec::new();
        let mut row = 1;
        if language_picker {
            language_rows(&own, &mut row, &mut specs);
        }
        rows_of(&own, &front, default.as_deref(), remember_asks, &mut row, &mut specs);
        if !recovery.is_empty() {
            specs.push(own_button(&own, "useCode", USE_CODE, "@signinUseCode", row));
        }
        let shared: Rc<dyn PartHost> = own.clone();
        base.node.add(Box::new(Side { node: Self::holding(container::grid_of(&specs, 1, GAP, &shared)), own: own.clone(), recovery: false }));
        if !recovery.is_empty() {
            let (mut specs, mut row) = (Vec::new(), 1);
            rows_of(&own, &recovery, Some(RECOVERY), false, &mut row, &mut specs);
            specs.push(own_button(&own, "back", BACK, "@signinBack", row));
            base.node.add(Box::new(Side { node: Self::holding(container::grid_of(&specs, 1, GAP, &shared)), own: own.clone(), recovery: true }));
        }
        SigninPart { base, own, laid_recovering: Cell::new(false), designed }
    }

    /// What the designer's placeholder says: `Sign in: password, account`; none outside the designer.
    pub fn placeholder(&self) -> Option<String> {
        if !self.designed {
            return None;
        }
        let names: Vec<&str> = self.own.strategies.iter().map(|s| s.name.as_str()).collect();
        let offers = if names.is_empty() { word("signinNothing").unwrap_or_default() } else { names.join(", ") };
        let picker = if self.own.language_picker { format!("; {}", word("languagePicker").unwrap_or_default()) } else { String::new() };
        Some(format!("{}: {offers}{picker}", word("signinButton").unwrap_or_default()))
    }

    fn holding(grid: GridPanel) -> Node {
        let mut node = Node::new();
        node.add(Box::new(grid));
        node
    }

    /// What its fields and buttons stand on: what was typed, the refusal shown.
    pub fn own(&self) -> &Rc<SigninHost> {
        &self.own
    }

    /// The names of the strategies it offers, in order.
    pub fn strategies(&self) -> Vec<String> {
        self.own.strategies.iter().map(|s| s.name.clone()).collect()
    }

    fn side(&self) -> Option<&Side> {
        self.base.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<Side>()).find(|s| s.is_shown())
    }

    /// How high the line under the fields is, where a refusal shows.
    fn message_height(&self) -> f64 {
        row_height()
    }

    /// Enter in its field `field`: the strategy of that field signs in. False for a name that is none of its fields.
    pub fn enter(&self, field: Option<&str>) -> bool {
        let Some(field) = field else { return false };
        let strategy = self.own.strategies.iter().find(|s| s.inputs.iter().any(|i| self.own.field_name(&s.name, i) == field));
        match strategy {
            Some(strategy) => {
                self.own.sign_in(&strategy.name);
                true
            }
            None => false,
        }
    }
}

impl Element for SigninPart {
    part_element!();

    /// The room it needs beyond its rows: its fields and buttons, and the line under them.
    fn around(&self) -> f64 {
        if self.designed {
            return 0.0;
        }
        let needed = self.side().map(|s| container::least_of(s.grid())).unwrap_or(0.0);
        container::around_for(&self.base.spec, GAP + self.message_height(), needed)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let fields = Rect { height: (area.height - GAP - self.message_height()).max(0.0), ..area };
        for side in self.base.node.children_mut() {
            element::arrange_safely(side.as_mut(), fields, cx);
        }
        // After a swap the first field of what shows now has the focus.
        let recovering = self.own.recovering();
        if self.laid_recovering.replace(recovering) != recovering {
            if let Some(first) = self.side().and_then(|s| element::focus_takers(s).into_iter().next()) {
                cx.focus(Some(first));
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        if let Some(says) = self.placeholder() {
            let look = p.look;
            let mut dashed = Paint::default();
            dashed.set_color(look.color("strong_border"));
            dashed.set_anti_alias(true);
            dashed.set_style(PaintStyle::Stroke);
            dashed.set_stroke_width(look.border() as f32);
            dashed.set_path_effect(PathEffect::dash(&[4.0, 4.0], 0.0));
            let radius = look.radius("panel") as f32;
            p.canvas.draw_round_rect(bounds.inset(look.border() / 2.0).sk(), radius, radius, &dashed);
            let words = bounds.inset(2.0 * look.padding());
            let size = look.size("small");
            p.text(&p.fit(&says, words.width, size, "text", false), words, look.color("muted"), size, "text", false, TextAlign::Center);
            return;
        }
        if self.own.strategies.is_empty() {
            let row = Rect { height: bounds.height.min(row_height()), ..bounds };
            p.text(&word("signinNothing").unwrap_or_default(), row.inset(6.0), p.look.color("muted"), p.look.size("small"), "text", false, TextAlign::Left);
            return;
        }
        element::draw_children(&self.base.node, p);
        if let Some(refusal) = self.own.refusal() {
            let line = Rect::new(bounds.x, bounds.bottom() - self.message_height(), bounds.width, self.message_height());
            let size = p.look.size("small");
            p.text(&p.fit(&refusal, line.width, size, "text", false), line, p.look.color("error"), size, "text", false, TextAlign::Left);
        }
    }
}

impl Part for SigninPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// Enter in an input inside the sign-in part `part`: the strategy of the input signs in.
pub fn enter(part: &dyn Part, field: Option<&str>) -> bool {
    part.as_any().downcast_ref::<SigninPart>().is_some_and(|p| p.enter(field))
}

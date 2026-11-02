//! A question a handler asks: `ask ... then handler(values)` (DESIGN 5.14),
//! in Clarity's own dialog over the window, never a dialog of the system:
//! the question, the input its form needs (a text box, a number box in the
//! value's format, or a dropdown of its choices) and OK and Cancel. OK or
//! Enter answers, and the window runs the handler the answer goes to;
//! Cancel or Escape runs nothing. A question for a dialog window of the
//! application is no question here: the window stack opens that window.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Ask, AskForm, PartState};
use serde_json::Value;

use crate::dialogs::{backdrop, DialogButton};
use crate::element::{Cx, Element, ElementId, Node};
use crate::facts::ModelFacts;
use crate::geometry::Rect;
use crate::grid::ROW_HEIGHT;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::markup::{self, HandlerCall};
use crate::painter::{Painter, TextAlign};
use crate::parts::{self, Choice, FieldFacts, Formats, PartHost};
use crate::window::messages::wrap;

const WIDTH: f64 = 440.0;
const LINE: f64 = 20.0;
const BUTTON_WIDTH: f64 = 104.0;

/// The path the answer's input is bound to.
pub const PATH: &str = "answer";

/// What the person did with the question: the values the handler runs
/// with (those written for it, then the answer for a value), or none
/// while it waits or when it was cancelled.
pub type Answered = Rc<RefCell<Option<Vec<Value>>>>;

/// The question's dialog: its text, its input for a value, OK and Cancel.
pub struct AskDialog {
    node: Node,
    pub ask: Ask,
    host: Rc<AskHost>,
    answered: Answered,
    has_input: bool,
    panel: Rect,
}

/// The kinds of value typed in a number box.
fn is_number(ty: &str) -> bool {
    matches!(ty, "integer" | "decimal" | "money" | "duration")
}

/// The choices of a question for an enum's value or a record, narrowed to
/// what it allows; none for a value that is typed.
pub fn choices_of(form: &AskForm, facts: &ModelFacts, program: &dyn Program) -> Option<Vec<Choice>> {
    let AskForm::Value { ty, of, only, .. } = form else { return None };
    let all = match ty.as_str() {
        "enum" => facts.enum_choices(of.as_deref().unwrap_or_default()),
        "record" => program
            .ref_options(of.as_deref().unwrap_or_default())
            .map(|options| options.into_iter().map(|o| Choice { value: Value::from(o.id), label: o.text, cells: None }).collect())
            .unwrap_or_default(),
        _ => return None,
    };
    Some(match only {
        Some(only) => all.into_iter().filter(|c| only.contains(&c.value)).collect(),
        None => all,
    })
}

/// The values a question's answer sends: those written for its handler, then the answer (none after a yes).
pub fn answer_values(ask: &Ask, answer: Option<Value>) -> Vec<Value> {
    let mut values = ask.values.clone();
    if !matches!(ask.form, AskForm::Confirm { .. }) {
        values.push(answer.unwrap_or(Value::Null));
    }
    values
}

/// Whether `value` answers the question: a choice or a number needs one; text may be empty; a yes needs nothing.
pub fn answerable(form: &AskForm, value: Option<&Value>) -> bool {
    match form {
        AskForm::Confirm { .. } | AskForm::Window { .. } => true,
        AskForm::Value { ty, .. } if ty == "text" => true,
        AskForm::Value { .. } => !matches!(value, None | Some(Value::Null)) && value != Some(&Value::from("")),
    }
}

impl AskDialog {
    /// The dialog of `ask`; `choices` are what a question for an enum's value or a record offers.
    pub fn new(ask: Ask, choices: Option<Vec<Choice>>) -> AskDialog {
        let mut node = Node::new();
        let dialog = node.id();
        let (ok, cancel) = match &ask.form {
            AskForm::Confirm { yes, no } => (
                yes.as_ref().map(|t| t.show()).unwrap_or_else(|| locale::translate("ok", "OK")),
                no.as_ref().map(|t| t.show()).unwrap_or_else(|| locale::translate("cancel", "Cancel")),
            ),
            _ => (locale::translate("ok", "OK"), locale::translate("cancel", "Cancel")),
        };
        let (ty, default) = match &ask.form {
            AskForm::Value { ty, default, .. } => (Some(ty.clone()), default.clone()),
            _ => (None, None),
        };
        let start = default.or_else(|| choices.as_ref().and_then(|c| c.first()).map(|c| c.value.clone()));
        let host = Rc::new(AskHost { ty: ty.clone(), choices: choices.clone().unwrap_or_default(), value: RefCell::new(start), changes: Cell::new(0) });
        let mut has_input = false;
        if let Some(ty) = &ty {
            let kind = if choices.is_some() {
                "Dropdown"
            } else if is_number(ty) {
                "Number"
            } else {
                "Textbox"
            };
            let markup = format!(r#"<Window><{kind} name="answer" bind="{PATH}" at="1,1" span="12" /></Window>"#);
            if let Some(spec) = markup::read(&markup).ok().and_then(|w| w.parts.into_iter().next()) {
                let as_host: Rc<dyn PartHost> = host.clone();
                node.add(parts::make(spec, as_host));
                has_input = true;
            }
        }
        let answered: Answered = Rc::new(RefCell::new(None));
        let ok_press = {
            let (host, answered, ask) = (host.clone(), answered.clone(), ask.clone());
            Rc::new(move |cx: &mut Cx| {
                let value = host.value.borrow().clone();
                if answerable(&ask.form, value.as_ref()) {
                    *answered.borrow_mut() = Some(answer_values(&ask, value));
                    cx.close(dialog);
                }
            })
        };
        node.add(Box::new(DialogButton::new(cancel, false, Rc::new(move |cx: &mut Cx| cx.close(dialog)))));
        node.add(Box::new(DialogButton::new(ok, true, ok_press)));
        AskDialog { node, ask, host, answered, has_input, panel: Rect::default() }
    }

    /// What the person answered, shared with whoever opened it: read it once it closed.
    pub fn answered(&self) -> Answered {
        self.answered.clone()
    }

    /// The input of a question for a value; none for yes or no.
    pub fn input(&self) -> Option<&dyn Element> {
        self.has_input.then(|| self.node.children()[0].as_ref())
    }

    fn button(&self, last: usize) -> &DialogButton {
        let children = self.node.children();
        children[children.len() - 1 - last].as_any().downcast_ref::<DialogButton>().expect("the dialog holds its buttons")
    }

    /// OK, in the accent's color.
    pub fn ok(&self) -> &DialogButton {
        self.button(0)
    }

    /// Cancel.
    pub fn cancel(&self) -> &DialogButton {
        self.button(1)
    }

    /// What the person focuses first: the input, else OK.
    pub fn first(&self) -> ElementId {
        match self.input() {
            Some(input) => crate::element::focus_takers(input).first().copied().unwrap_or(input.node().id()),
            None => self.ok().node().id(),
        }
    }

    /// What the input holds now; none for a yes.
    pub fn value(&self) -> Option<Value> {
        self.host.value.borrow().clone()
    }

    /// Where the dialog itself is, in the middle of the window.
    pub fn panel(&self) -> Rect {
        self.panel
    }

    /// Answers with what the input holds, what is typed in it taken first; nothing when it does not answer the question.
    fn answer_now(&mut self, cx: &mut Cx) {
        if self.has_input {
            let input = &mut self.node.children_mut()[0];
            input.take_typed(cx);
        }
        let last = self.node.children().len() - 1;
        if let Some(ok) = self.node.children_mut()[last].as_any_mut().downcast_mut::<DialogButton>() {
            ok.press(cx);
        }
    }

    fn lines(&self, width: f64, cx_fonts: &crate::fonts::Fonts, size: f64) -> Vec<String> {
        wrap(&self.ask.question.show(), width, cx_fonts, size)
    }
}

impl Element for AskDialog {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let width = WIDTH.min((area.width - 32.0).max(200.0));
        let inner = width - 40.0;
        let lines = self.lines(inner, cx.fonts(), cx.look().size("text")).len() as f64;
        let input_room = if self.has_input { ROW_HEIGHT + 16.0 } else { 0.0 };
        let height = 20.0 + lines * LINE + 16.0 + input_room + ROW_HEIGHT + 20.0;
        self.panel = Rect::new(area.x + (area.width - width) / 2.0, area.y + ((area.height - height) / 2.0).max(16.0), width, height);
        let mut y = self.panel.y + 20.0 + lines * LINE + 16.0;
        let panel = self.panel;
        let children = self.node.children_mut();
        let count = children.len();
        if self.has_input {
            children[0].arrange(Rect::new(panel.x + 20.0, y, inner, ROW_HEIGHT), cx);
            y += ROW_HEIGHT + 16.0;
        }
        let ok = Rect::new(panel.right() - 20.0 - BUTTON_WIDTH, y, BUTTON_WIDTH, ROW_HEIGHT);
        children[count - 1].arrange(ok, cx);
        children[count - 2].arrange(Rect::new(ok.x - 8.0 - BUTTON_WIDTH, y, BUTTON_WIDTH, ROW_HEIGHT), cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        backdrop(p, self.node.bounds, self.panel);
        let size = look.size("text");
        let inner = self.panel.width - 40.0;
        for (i, line) in self.lines(inner, p.fonts, size).iter().enumerate() {
            p.text(line, Rect::new(self.panel.x + 20.0, self.panel.y + 20.0 + i as f64 * LINE, inner, LINE), look.color("text"), size, "text", false, TextAlign::Left);
        }
        crate::element::draw_children(&self.node, p);
    }

    /// Enter answers (a dropdown's open list has it first), Escape cancels.
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE {
            return false;
        }
        match key.key {
            Key::Enter => {
                self.answer_now(cx);
                true
            }
            Key::Escape => {
                cx.close(self.node.id());
                true
            }
            _ => false,
        }
    }
}

/// The answer's one value, which the input reads and writes; its format and choices are the question's.
pub struct AskHost {
    ty: Option<String>,
    choices: Vec<Choice>,
    value: RefCell<Option<Value>>,
    changes: Cell<u64>,
}

impl PartHost for AskHost {
    fn read(&self, path: &str) -> Option<Value> {
        if path == PATH { self.value.borrow().clone() } else { None }
    }

    fn write(&self, _part: &str, path: &str, value: Value) {
        if path == PATH {
            *self.value.borrow_mut() = Some(value);
            self.changes.set(self.changes.get() + 1);
        }
    }

    fn edited(&self, _part: &str) {
        self.changes.set(self.changes.get() + 1);
    }

    fn state(&self, _name: Option<&str>) -> Option<PartState> {
        None
    }

    fn change_state(&self, _name: &str, _change: &mut dyn FnMut(&mut PartState)) {}

    fn run(&self, _part: Option<&str>, _call: &HandlerCall) -> bool {
        true
    }

    fn text(&self, written: Option<&str>) -> String {
        written.unwrap_or_default().to_string()
    }

    fn changes(&self) -> u64 {
        self.changes.get()
    }

    fn type_of(&self, path: &str) -> Option<String> {
        if path == PATH { self.ty.clone().filter(|t| is_number(t) || matches!(t.as_str(), "date" | "datetime" | "text")) } else { None }
    }

    fn field(&self, path: &str) -> Option<FieldFacts> {
        let ty = self.ty.as_deref().filter(|t| path == PATH && is_number(t))?;
        Some(FieldFacts { format: Some(ty.to_string()), ..FieldFacts::default() })
    }

    fn choices(&self, path: &str) -> Vec<Choice> {
        if path == PATH { self.choices.clone() } else { Vec::new() }
    }

    fn formats(&self) -> Formats {
        Formats { language: Some(crate::choices::language()), currency: None }
    }
}

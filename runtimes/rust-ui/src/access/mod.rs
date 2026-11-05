//! Screen readers (DESIGN 7.9, A1): the accessibility tree of a screen, made
//! from its elements. Every part, the window's frame, dialogs and what lies
//! over the window become nodes with a role, a name (a part's caption or the
//! text of its label, translated as drawn), a value, states, a place and the
//! actions a screen reader may ask for.
//!
//! The tree is plain data (`AccessTree`), made by a plain function from the
//! element trees (`tree_of`), so it is tested without a system. The desktop
//! hands it to the system's screen reader through AccessKit (`kit`), the
//! driven API answers it as JSON (`AccessTree::json`, the `access` step),
//! and what a screen reader asks for runs as the person's input (`act`).

pub mod act;
pub mod kit;
pub mod mirror;

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::element::{Element, ElementId};
use crate::geometry::Rect;
use crate::parts::Part;
use crate::screen::Screen;

/// The id of the window's own node, above every element.
pub const ROOT: u64 = 1 << 62;

/// Where the ids of nodes that are no element start (a table's rows, a menu's
/// items): `VIRTUAL | element << 20 | index`.
const VIRTUAL: u64 = 1 << 63;

/// The id of the `index`th node an element shows that is no element of its own.
pub fn virtual_id(owner: ElementId, index: usize) -> u64 {
    VIRTUAL | (owner << 20) | (index as u64 & 0xF_FFFF)
}

/// The element a node belongs to: itself, or the element that shows it.
pub fn owner_of(id: u64) -> Option<ElementId> {
    if id == ROOT {
        None
    } else if id & VIRTUAL != 0 {
        Some((id & !VIRTUAL) >> 20)
    } else {
        Some(id)
    }
}

/// What a node is, by the names ARIA gives roles where it has one (the
/// browser mirrors the tree with them); `window`, `label`, `textarea`,
/// `password`, `date` and `color` are Clarity's own (DESIGN 7.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Window,
    Dialog,
    AlertDialog,
    Group,
    Form,
    Heading,
    Label,
    Document,
    Button,
    Link,
    CheckBox,
    Switch,
    RadioGroup,
    Radio,
    TextBox,
    TextArea,
    Password,
    SpinButton,
    Slider,
    ComboBox,
    ListBox,
    Option,
    Date,
    Color,
    Image,
    Canvas,
    ProgressBar,
    Table,
    TreeGrid,
    Row,
    Cell,
    ColumnHeader,
    TabList,
    Tab,
    MenuBar,
    Menu,
    MenuItem,
    MenuItemCheckBox,
    Toolbar,
    Complementary,
    ContentInfo,
    Status,
    Log,
    Separator,
}

impl Role {
    /// Its name in the tree's JSON.
    pub fn word(self) -> &'static str {
        match self {
            Role::Window => "window",
            Role::Dialog => "dialog",
            Role::AlertDialog => "alertdialog",
            Role::Group => "group",
            Role::Form => "form",
            Role::Heading => "heading",
            Role::Label => "label",
            Role::Document => "document",
            Role::Button => "button",
            Role::Link => "link",
            Role::CheckBox => "checkbox",
            Role::Switch => "switch",
            Role::RadioGroup => "radiogroup",
            Role::Radio => "radio",
            Role::TextBox => "textbox",
            Role::TextArea => "textarea",
            Role::Password => "password",
            Role::SpinButton => "spinbutton",
            Role::Slider => "slider",
            Role::ComboBox => "combobox",
            Role::ListBox => "listbox",
            Role::Option => "option",
            Role::Date => "date",
            Role::Color => "color",
            Role::Image => "img",
            Role::Canvas => "canvas",
            Role::ProgressBar => "progressbar",
            Role::Table => "table",
            Role::TreeGrid => "treegrid",
            Role::Row => "row",
            Role::Cell => "cell",
            Role::ColumnHeader => "columnheader",
            Role::TabList => "tablist",
            Role::Tab => "tab",
            Role::MenuBar => "menubar",
            Role::Menu => "menu",
            Role::MenuItem => "menuitem",
            Role::MenuItemCheckBox => "menuitemcheckbox",
            Role::Toolbar => "toolbar",
            Role::Complementary => "complementary",
            Role::ContentInfo => "contentinfo",
            Role::Status => "status",
            Role::Log => "log",
            Role::Separator => "separator",
        }
    }
}

/// What a screen reader may ask a node to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessAction {
    Focus,
    /// Its default action: a press of a button, a tick, choosing a row.
    Click,
    SetValue,
    Increment,
    Decrement,
    ScrollIntoView,
    Expand,
    Collapse,
}

impl AccessAction {
    pub fn word(self) -> &'static str {
        match self {
            AccessAction::Focus => "focus",
            AccessAction::Click => "click",
            AccessAction::SetValue => "setValue",
            AccessAction::Increment => "increment",
            AccessAction::Decrement => "decrement",
            AccessAction::ScrollIntoView => "scrollIntoView",
            AccessAction::Expand => "expand",
            AccessAction::Collapse => "collapse",
        }
    }

    pub fn from_word(word: &str) -> Option<AccessAction> {
        ALL_ACTIONS.into_iter().find(|a| a.word() == word)
    }
}

const ALL_ACTIONS: [AccessAction; 8] = [
    AccessAction::Focus,
    AccessAction::Click,
    AccessAction::SetValue,
    AccessAction::Increment,
    AccessAction::Decrement,
    AccessAction::ScrollIntoView,
    AccessAction::Expand,
    AccessAction::Collapse,
];

/// A node's states; what does not apply is false or none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct States {
    pub focused: bool,
    pub disabled: bool,
    pub readonly: bool,
    /// A check box, switch, radio button or toggle: whether it is ticked.
    pub checked: Option<bool>,
    /// A row, tab or option: whether it is the chosen one.
    pub selected: Option<bool>,
    /// What opens and closes (a tree row, an accordion's section, a dropdown).
    pub expanded: Option<bool>,
    pub required: bool,
    /// Why its value is not accepted, as the window says it; none while it is.
    pub invalid: Option<String>,
    /// A dialog, which takes the person's input alone.
    pub modal: bool,
}

/// A number a slider or spin box holds, with its range.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Numeric {
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

/// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessNode {
    pub id: u64,
    pub role: Role,
    pub name: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub states: States,
    /// Where it is, in the screen's units.
    pub bounds: Rect,
    pub actions: Vec<AccessAction>,
    pub children: Vec<u64>,
    pub numeric: Option<Numeric>,
    /// A heading's level, a tree row's depth from 1.
    pub level: Option<usize>,
    /// The name of the part it is (or is in, for a row or cell); none for the frame's own.
    pub part: Option<String>,
    /// A row's place in its data.
    pub place: Option<usize>,
    /// Where a tree row's toggle is, which Expand and Collapse press.
    pub toggle: Option<Rect>,
}

impl AccessNode {
    pub fn new(id: u64, role: Role, name: impl Into<String>, bounds: Rect) -> AccessNode {
        AccessNode {
            id,
            role,
            name: name.into(),
            value: None,
            description: None,
            states: States::default(),
            bounds,
            actions: Vec::new(),
            children: Vec::new(),
            numeric: None,
            level: None,
            part: None,
            place: None,
            toggle: None,
        }
    }

    fn with_actions(mut self, actions: &[AccessAction]) -> AccessNode {
        self.actions.extend_from_slice(actions);
        self
    }

    /// The node as JSON: role, name, value, description, states (only those
    /// that apply), place `at`, actions, children, and what applies of
    /// `numeric`, `level`, `part` and `place`.
    pub fn json(&self) -> Value {
        let mut out = Map::new();
        // Ids as text: a row's or cell's id is above 2^53, which a JavaScript number does not hold.
        out.insert("id".into(), json!(self.id.to_string()));
        out.insert("role".into(), json!(self.role.word()));
        out.insert("name".into(), json!(self.name));
        if let Some(value) = &self.value {
            out.insert("value".into(), json!(value));
        }
        if let Some(description) = &self.description {
            out.insert("description".into(), json!(description));
        }
        let s = &self.states;
        let mut states = Map::new();
        for (word, on) in [("focused", s.focused), ("disabled", s.disabled), ("readonly", s.readonly), ("required", s.required), ("modal", s.modal)] {
            if on {
                states.insert(word.into(), json!(true));
            }
        }
        for (word, state) in [("checked", s.checked), ("selected", s.selected), ("expanded", s.expanded)] {
            if let Some(on) = state {
                states.insert(word.into(), json!(on));
            }
        }
        if let Some(why) = &s.invalid {
            states.insert("invalid".into(), json!(why));
        }
        out.insert("states".into(), Value::Object(states));
        let b = self.bounds;
        out.insert("at".into(), json!([b.x, b.y, b.width, b.height]));
        out.insert("actions".into(), json!(self.actions.iter().map(|a| a.word()).collect::<Vec<_>>()));
        out.insert("children".into(), json!(self.children.iter().map(u64::to_string).collect::<Vec<_>>()));
        if let Some(n) = &self.numeric {
            out.insert("numeric".into(), json!({ "value": n.value, "min": n.min, "max": n.max, "step": n.step }));
        }
        if let Some(level) = self.level {
            out.insert("level".into(), json!(level));
        }
        if let Some(part) = &self.part {
            out.insert("part".into(), json!(part));
        }
        if let Some(place) = self.place {
            out.insert("place".into(), json!(place));
        }
        Value::Object(out)
    }
}

/// The accessibility tree of a screen: its nodes, the window's first and
/// every node before its children, and the node with the focus.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessTree {
    pub nodes: Vec<AccessNode>,
    pub focus: u64,
}

impl AccessTree {
    pub fn root(&self) -> &AccessNode {
        &self.nodes[0]
    }

    pub fn node(&self, id: u64) -> Option<&AccessNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The first node of a part by its name.
    pub fn part(&self, name: &str) -> Option<&AccessNode> {
        self.nodes.iter().find(|n| n.part.as_deref() == Some(name) && owner_of(n.id).is_some() && n.id & VIRTUAL == 0)
    }

    /// The first node with this role and name.
    pub fn named(&self, role: Role, name: &str) -> Option<&AccessNode> {
        self.nodes.iter().find(|n| n.role == role && n.name == name)
    }

    /// The tree as JSON (DESIGN 7.9): `root`, `focus` and `nodes` in tree order, every id as text.
    pub fn json(&self) -> Value {
        json!({ "root": ROOT.to_string(), "focus": self.focus.to_string(), "nodes": self.nodes.iter().map(AccessNode::json).collect::<Vec<_>>() })
    }
}

/// The tree of a screen: its window and what lies over it.
pub fn tree(screen: &Screen) -> AccessTree {
    let mut roots: Vec<(&dyn Element, bool)> = vec![(screen.root(), false)];
    roots.extend(screen.overlays().iter().map(|o| (o.content.as_ref(), o.modal)));
    tree_of(&roots, screen.focused(), screen.whole())
}

/// The tree of element trees (the window's first, then each overlay with
/// whether it is modal), with the element that has the focus, over `whole`.
pub fn tree_of(roots: &[(&dyn Element, bool)], focused: Option<ElementId>, whole: Rect) -> AccessTree {
    let title = roots.first().and_then(|(root, _)| root.as_any().downcast_ref::<crate::window::WindowStack>()).map(|s| s.title()).unwrap_or_default();
    let mut builder = Builder { nodes: vec![AccessNode::new(ROOT, Role::Window, title, whole)], labels: HashMap::new(), helps: HashMap::new(), given: HashMap::new(), under: HashMap::new(), focused, focus: None, active: None };
    for (root, _) in roots {
        builder.gather(*root);
    }
    for (root, modal) in roots {
        let before = builder.nodes[0].children.len();
        builder.walk(*root, 0);
        if *modal {
            for i in before..builder.nodes[0].children.len() {
                let id = builder.nodes[0].children[i];
                if let Some(n) = builder.nodes.iter_mut().find(|n| n.id == id) {
                    n.states.modal = true;
                }
            }
        }
    }
    let focus = builder.active.or(builder.focus).unwrap_or(ROOT);
    if let Some(n) = builder.nodes.iter_mut().find(|n| n.id == focus) {
        n.states.focused = true;
    }
    AccessTree { nodes: builder.nodes, focus }
}

/// What a parent says of an element under it: a radio button's choice, a tab's state.
#[derive(Clone)]
struct Given {
    role: Role,
    name: String,
    checked: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
}

struct Builder {
    nodes: Vec<AccessNode>,
    /// The text of each label by the part its `for` names.
    labels: HashMap<String, String>,
    /// The text of each help part by the part its `for` names.
    helps: HashMap<String, String>,
    given: HashMap<ElementId, Given>,
    /// The nodes a parent gives an element under it to show (a ribbon's tabs, its strip's).
    under: HashMap<ElementId, Vec<(AccessNode, Vec<AccessNode>)>>,
    focused: Option<ElementId>,
    /// The node of the element with the focus, or of the nearest it is in.
    focus: Option<u64>,
    /// What inside the focused node is active: a table's chosen row, a menu's marked item.
    active: Option<u64>,
}

impl Builder {
    /// Collects the labels and help texts by the parts they are for.
    fn gather(&mut self, e: &dyn Element) {
        if let Some(part) = e.as_part() {
            let spec = &part.base().spec;
            if let Some(target) = spec.text("for").filter(|t| !t.is_empty()) {
                let any = e.as_any();
                if let Some(label) = any.downcast_ref::<crate::parts::label::LabelPart>() {
                    self.labels.entry(target.to_string()).or_insert_with(|| label.text());
                } else if let Some(help) = any.downcast_ref::<crate::parts::containers::small::HelpPart>() {
                    self.helps.entry(target.to_string()).or_insert_with(|| help.text());
                }
            }
        }
        for child in e.node().children() {
            self.gather(child.as_ref());
        }
    }

    fn push(&mut self, parent: usize, node: AccessNode) -> usize {
        let id = node.id;
        self.nodes[parent].children.push(id);
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Adds the node of `e` under the node at `parent`, then its children
    /// under it; an element that is no node (a grid, a plate) gives its
    /// children to `parent`.
    fn walk(&mut self, e: &dyn Element, parent: usize) {
        if !e.is_shown() {
            return;
        }
        let id = e.node().id();
        let b = e.node().bounds;
        let sized = b.width > 0.0 && b.height > 0.0;
        let made = if sized { crate::element::safely_read(e, None, |e| self.describe(e)) } else { None };
        let at = match made {
            Some((mut node, extra)) => {
                if !e.is_enabled() || !crate::element::enabled_in_tree(e) {
                    node.states.disabled = true;
                }
                let at = self.push(parent, node);
                for (child, under) in extra {
                    self.add_virtual(at, child, under);
                }
                at
            }
            None => parent,
        };
        if self.focused.is_some_and(|f| f == id) {
            self.focus = Some(self.nodes[at].id);
        }
        for child in e.node().children() {
            self.walk(child.as_ref(), at);
        }
    }

    /// Adds a node that is no element (a row, a cell) and the nodes under it.
    fn add_virtual(&mut self, parent: usize, node: AccessNode, under: Vec<AccessNode>) {
        let at = self.push(parent, node);
        for child in under {
            self.push(at, child);
        }
    }

    /// The node of an element and the nodes it shows that are no element
    /// (each with those under it); none for an element that is no node.
    #[allow(clippy::type_complexity)]
    fn describe(&mut self, e: &dyn Element) -> Option<(AccessNode, Vec<(AccessNode, Vec<AccessNode>)>)> {
        let id = e.node().id();
        let b = e.node().bounds;
        let any = e.as_any();
        if let Some(given) = self.given.get(&id).cloned() {
            let mut node = AccessNode::new(id, given.role, given.name, b).with_actions(&[AccessAction::Focus, AccessAction::Click, AccessAction::ScrollIntoView]);
            node.states.checked = given.checked;
            node.states.selected = given.selected;
            node.states.expanded = given.expanded;
            if given.role == Role::TabList {
                node.actions.retain(|a| *a != AccessAction::Click);
            }
            return Some((node, self.under.remove(&id).unwrap_or_default()));
        }
        if let Some(part) = e.as_part() {
            return self.part(e, part);
        }
        frame_node(self, e, id, b, any)
    }

    /// The node of a part, by its kind.
    #[allow(clippy::type_complexity)]
    fn part(&mut self, e: &dyn Element, part: &dyn Part) -> Option<(AccessNode, Vec<(AccessNode, Vec<AccessNode>)>)> {
        use crate::parts::{containers::*, data::*, inputs::*, *};
        use AccessAction::*;
        let base = part.base();
        let spec = &base.spec;
        let host = &base.host;
        let id = e.node().id();
        let b = e.node().bounds;
        let any = e.as_any();
        let name_of = |own: Option<String>| -> String {
            let labelled = spec.name().and_then(|n| self.labels.get(n).cloned()).filter(|l| !l.is_empty());
            let field = || base.bound().and_then(|p| host.field(p)).map(|f| host.text(Some(&f.label))).filter(|l| !l.is_empty());
            labelled.or(own.filter(|c| !c.is_empty())).or_else(field).unwrap_or_default()
        };
        let caption = base.caption();
        let mut extra = Vec::new();
        let interactive = [Focus, Click, ScrollIntoView];
        let input = [Focus, SetValue, ScrollIntoView];
        let mut node = match spec.kind.as_str() {
            "label" => AccessNode::new(id, Role::Label, any.downcast_ref::<label::LabelPart>().map(|l| l.text()).unwrap_or_default(), b),
            "heading" => {
                let mut n = AccessNode::new(id, Role::Heading, any.downcast_ref::<heading::HeadingPart>().map(|h| h.text()).unwrap_or_default(), b);
                n.level = Some(spec.text("level").and_then(|l| l.parse().ok()).unwrap_or(1));
                n
            }
            "text" => AccessNode::new(id, Role::Label, any.downcast_ref::<text::TextPart>().map(|t| t.text()).unwrap_or_default(), b),
            "display" => {
                let mut n = AccessNode::new(id, Role::Label, name_of(caption), b);
                n.value = any.downcast_ref::<display::DisplayPart>().map(|d| d.text());
                n
            }
            "badge" => AccessNode::new(id, Role::Label, any.downcast_ref::<small::BadgePart>().map(|t| t.text()).unwrap_or_default(), b),
            "help" => AccessNode::new(id, Role::Label, any.downcast_ref::<small::HelpPart>().map(|t| t.text()).unwrap_or_default(), b),
            "busy" => AccessNode::new(id, Role::ProgressBar, any.downcast_ref::<small::BusyPart>().map(|t| t.text()).unwrap_or_default(), b),
            "progress" => {
                let mut n = AccessNode::new(id, Role::ProgressBar, name_of(caption), b);
                let full = any.downcast_ref::<small::ProgressPart>().map(|p| p.shown()).unwrap_or(0.0);
                n.numeric = Some(Numeric { value: Some((full * 100.0).round()), min: Some(0.0), max: Some(100.0), step: None });
                n.value = Some(format!("{}%", (full * 100.0).round()));
                n
            }
            "markdown" => {
                let mut n = AccessNode::new(id, Role::Document, name_of(caption), b);
                n.value = any.downcast_ref::<markdown::MarkdownPart>().map(|m| m.markdown());
                n
            }
            "image" => match any.downcast_ref::<picture_field::PictureFieldPart>() {
                Some(field) => {
                    let mut n = AccessNode::new(id, Role::Image, name_of(caption), b);
                    n.value = (!field.is_empty()).then(|| base.bound().and_then(|p| host.read(p)).map(|v| short(&v)).unwrap_or_default());
                    n
                }
                None => AccessNode::new(id, Role::Image, name_of(caption.or_else(|| any.downcast_ref::<image::ImagePart>().map(|i| i.says()))), b),
            },
            "icon" => {
                let named = name_of(caption.or_else(|| spec.text("tooltip").map(|t| host.text(Some(t)))));
                if named.is_empty() {
                    return None;
                }
                AccessNode::new(id, Role::Image, named, b)
            }
            "chart" | "trend" => AccessNode::new(id, Role::Image, name_of(caption), b),
            "canvas" => AccessNode::new(id, Role::Canvas, name_of(caption), b).with_actions(&interactive),
            "button" => {
                let text = any.downcast_ref::<button::ButtonPart>().map(|b| b.caption()).or(caption);
                AccessNode::new(id, Role::Button, name_of(text), b).with_actions(&interactive)
            }
            "link" => AccessNode::new(id, Role::Link, name_of(any.downcast_ref::<link::LinkPart>().map(|l| l.shown()).or(caption)), b).with_actions(&interactive),
            "checkbox" | "switch" => {
                let check = any.downcast_ref::<check_box::CheckPart>();
                let role = if spec.kind == "switch" { Role::Switch } else { Role::CheckBox };
                let mut n = AccessNode::new(id, role, name_of(check.map(|c| c.caption()).or(caption)), b).with_actions(&interactive);
                n.states.checked = Some(check.is_some_and(|c| c.is_checked()));
                n
            }
            "toggle" => {
                let toggle = any.downcast_ref::<toggle::TogglePart>();
                let mut n = AccessNode::new(id, Role::Button, name_of(toggle.map(|t| t.caption()).or(caption)), b).with_actions(&interactive);
                n.states.checked = Some(toggle.is_some_and(|t| t.is_on()));
                n
            }
            "textbox" | "memo" | "password" => {
                let role = match spec.kind.as_str() {
                    "memo" => Role::TextArea,
                    "password" => Role::Password,
                    _ => Role::TextBox,
                };
                let text_box = any.downcast_ref::<text_box::TextBoxPart>();
                let mut n = AccessNode::new(id, role, name_of(caption.or_else(|| text_box.and_then(|t| t.editor().placeholder.clone()))), b).with_actions(&input);
                // A password's text is never read out; the others' as shown, also while typed and not yet taken.
                if role != Role::Password {
                    n.value = text_box.map(|t| t.editor().text());
                }
                n
            }
            "number" | "spinbox" => {
                let number = any.downcast_ref::<number::NumberPart>();
                let mut n = AccessNode::new(id, Role::SpinButton, name_of(caption), b).with_actions(&[Focus, SetValue, Increment, Decrement, ScrollIntoView]);
                let value = number.and_then(|p| p.value());
                n.value = value.as_ref().map(short);
                let range = number.and_then(|p| p.range());
                n.numeric = Some(Numeric {
                    value: value.as_ref().and_then(number_of),
                    min: range.as_ref().and_then(|r| r.min.as_deref()).and_then(|m| m.parse().ok()),
                    max: range.as_ref().and_then(|r| r.max.as_deref()).and_then(|m| m.parse().ok()),
                    step: range.as_ref().and_then(|r| r.step.parse().ok()),
                });
                n
            }
            "slider" => {
                let slider = any.downcast_ref::<slider::SliderPart>();
                let mut n = AccessNode::new(id, Role::Slider, name_of(caption), b).with_actions(&[Focus, Increment, Decrement, ScrollIntoView]);
                if let Some(slider) = slider {
                    let range = slider.range();
                    n.value = Some(slider.value().unwrap_or_else(|| slider.shown().to_string()));
                    n.numeric = Some(Numeric {
                        value: Some(slider.shown()),
                        min: range.min.as_deref().and_then(|m| m.parse().ok()),
                        max: range.max.as_deref().and_then(|m| m.parse().ok()),
                        step: range.step.parse().ok(),
                    });
                }
                n
            }
            "dropdown" => {
                let mut n = AccessNode::new(id, Role::ComboBox, name_of(caption), b).with_actions(&interactive);
                if let Some(drop) = any.downcast_ref::<dropdown::DropdownPart>() {
                    n.value = Some(drop.shown());
                    n.states.expanded = Some(drop.list().is_some());
                } else if let Some(typed) = any.downcast_ref::<typed_dropdown::TypeAheadPart>() {
                    n.value = Some(typed.editor().text());
                    n.states.expanded = Some(typed.list().is_some());
                    n.actions.push(SetValue);
                }
                n
            }
            "radio" => {
                let mut n = AccessNode::new(id, Role::RadioGroup, name_of(caption), b);
                if let Some(radio) = any.downcast_ref::<radio::RadioPart>() {
                    let (choices, chosen) = (radio.choices(), radio.chosen());
                    for (i, button) in radio.buttons().into_iter().enumerate() {
                        let name = choices.get(i).map(|c| c.label.clone()).unwrap_or_default();
                        self.given.insert(button, Given { role: Role::Radio, name, checked: Some(chosen == Some(i)), selected: None, expanded: None });
                    }
                }
                n.value = None;
                n
            }
            "color" => {
                let mut n = AccessNode::new(id, Role::Color, name_of(caption), b).with_actions(&input);
                n.value = any.downcast_ref::<color::ColorPart>().map(|c| c.value());
                n
            }
            "date" => {
                let mut n = AccessNode::new(id, Role::Date, name_of(caption), b).with_actions(&[ScrollIntoView]);
                if let Some(date) = any.downcast_ref::<date::DatePart>() {
                    let day = date.editor().text();
                    let time = date.time().map(|t| t.text());
                    n.value = Some(match &time {
                        Some(time) if !time.is_empty() => format!("{day} {time}"),
                        _ => day.clone(),
                    });
                    let name = n.name.clone();
                    if let Some(input) = date.input(date::Which::Day) {
                        self.given.insert(input, Given { role: Role::TextBox, name: name.clone(), checked: None, selected: None, expanded: None });
                    }
                    if let Some(input) = date.input(date::Which::Time) {
                        self.given.insert(input, Given { role: Role::TextBox, name, checked: None, selected: None, expanded: None });
                    }
                }
                n
            }
            "file" => {
                let mut n = AccessNode::new(id, Role::Button, name_of(caption), b).with_actions(&interactive);
                n.value = any.downcast_ref::<file::FilePart>().map(|f| f.shown());
                n
            }
            "folder" => {
                let mut n = AccessNode::new(id, Role::TextBox, name_of(caption), b).with_actions(&input);
                n.value = any.downcast_ref::<folder::FolderPart>().map(|f| f.value());
                n
            }
            "tags" => {
                let mut n = AccessNode::new(id, Role::Group, name_of(caption), b);
                n.value = any.downcast_ref::<tags::TagsPart>().map(|t| t.items().iter().map(short).collect::<Vec<_>>().join(", "));
                n
            }
            "list" => {
                let n = AccessNode::new(id, Role::ListBox, name_of(caption), b).with_actions(&[Focus, ScrollIntoView]);
                if let Some(list) = any.downcast_ref::<list::ListPart>() {
                    let chosen = list.chosen();
                    for (place, record) in list.records().iter().enumerate() {
                        let Some(row) = list.row_box(place) else { continue };
                        let mut item = AccessNode::new(virtual_id(id, place), Role::Option, list.text_of(record), row).with_actions(&[Click, ScrollIntoView]);
                        item.states.selected = Some(chosen == Some(place));
                        item.place = Some(place);
                        item.part = spec.name().map(str::to_string);
                        if chosen == Some(place) && self.focused == Some(id) {
                            self.active = Some(item.id);
                        }
                        extra.push((item, Vec::new()));
                    }
                }
                n
            }
            "table" | "tree" | "treeTable" => {
                let table = any.downcast_ref::<table::TablePart>();
                let tree = table.is_some_and(|t| t.is_tree());
                let n = AccessNode::new(id, if tree { Role::TreeGrid } else { Role::Table }, name_of(caption), b).with_actions(&[Focus, ScrollIntoView]);
                if let Some(table) = table {
                    extra = self.table_rows(table, id, spec.name());
                }
                n
            }
            "tabs" => {
                let n = AccessNode::new(id, Role::TabList, name_of(caption), b);
                if let Some(tabs) = any.downcast_ref::<tabs::TabsPart>() {
                    let open = tabs.open();
                    for (i, tab) in tabs.tabs().into_iter().enumerate() {
                        let title = tab.as_any().downcast_ref::<tabs::TabButton>().map(|t| t.title()).unwrap_or_default();
                        self.given.insert(tab.node().id(), Given { role: Role::Tab, name: title, checked: None, selected: Some(open == Some(i as i64 + 1)), expanded: None });
                    }
                }
                n
            }
            "accordion" => {
                let n = AccessNode::new(id, Role::Group, name_of(caption), b);
                if let Some(accordion) = any.downcast_ref::<accordion::AccordionPart>() {
                    let open = accordion.open();
                    for (i, header) in accordion.headers().into_iter().enumerate() {
                        self.given.insert(header.node().id(), Given { role: Role::Button, name: header.text(), checked: None, selected: None, expanded: Some(open == i as i64 + 1) });
                    }
                }
                n
            }
            "steps" => {
                let mut n = AccessNode::new(id, Role::Group, name_of(caption), b);
                if let Some(steps) = any.downcast_ref::<steps::StepsPart>() {
                    let shown = steps.shown();
                    n.value = spec.pages.get((shown - 1).max(0) as usize).and_then(|p| p.title.as_deref()).map(|t| host.text(Some(t)));
                }
                n
            }
            "popover" => {
                let mut n = AccessNode::new(id, Role::Button, name_of(caption), b).with_actions(&interactive);
                n.states.expanded = any.downcast_ref::<popover::PopoverPart>().map(|p| p.is_open());
                n
            }
            "form" => AccessNode::new(id, Role::Form, name_of(caption), b),
            "panel" | "scroll" | "buttons" | "splitter" => {
                let named = name_of(caption);
                if named.is_empty() {
                    return None;
                }
                AccessNode::new(id, Role::Group, named, b)
            }
            "custom" => {
                let kind = spec.text("part").unwrap_or_default().to_string();
                let custom = any.downcast_ref::<custom::CustomPlace>();
                let own = caption.or_else(|| Some(kind.clone()));
                let mut n = AccessNode::new(id, Role::Group, name_of(own), b).with_actions(&[Focus, Click, ScrollIntoView]);
                n.description = spec.text("description").filter(|d| !d.is_empty()).map(|d| host.text(Some(d)));
                if let Some(why) = custom.and_then(|c| c.why.clone()) {
                    n.description = Some(why);
                }
                n
            }
            "separator" | "sound" | "timer" => return None,
            _ => match any.downcast_ref::<MissingPart>() {
                Some(missing) => AccessNode::new(id, Role::Label, missing.says(), b),
                None => AccessNode::new(id, Role::Group, name_of(caption), b),
            },
        };
        node.part = spec.name().map(str::to_string);
        if node.description.is_none() {
            let help = spec.name().and_then(|n| self.helps.get(n).cloned()).filter(|h| !h.is_empty());
            let tip = spec.text("tooltip").filter(|t| !t.is_empty()).map(|t| host.text(Some(t)));
            node.description = help.or(tip).filter(|d| *d != node.name);
        }
        if base.read_only() {
            node.states.readonly = true;
            node.actions.retain(|a| *a != SetValue);
        }
        if let Some(path) = base.bound() {
            node.states.required = host.field(path).is_some_and(|f| f.required);
        }
        if let Some(error) = base.state().and_then(|s| s.error.clone()).map(|e| e.show()).filter(|e| !e.is_empty()) {
            node.states.invalid = Some(error);
        }
        Some((node, extra))
    }

    /// The header row and the rows of a table or tree that show, each with its cells.
    fn table_rows(&mut self, table: &crate::parts::data::table::TablePart, id: ElementId, part: Option<&str>) -> Vec<(AccessNode, Vec<AccessNode>)> {
        let host = &table.base().host;
        let columns = table.columns();
        let mut out = Vec::new();
        let mut index = 0usize;
        let mut next = || {
            index += 1;
            virtual_id(id, index)
        };
        let headers: Vec<AccessNode> = columns
            .iter()
            .filter_map(|c| table.header_box(&c.field).map(|b| AccessNode::new(0, Role::ColumnHeader, host.text(Some(&c.label)), b)))
            .collect();
        if !headers.is_empty() {
            let first = headers[0].bounds;
            let last = headers[headers.len() - 1].bounds;
            let row = AccessNode::new(next(), Role::Row, "", Rect::new(first.x, first.y, last.right() - first.x, first.height));
            let cells = headers.into_iter().map(|mut h| {
                h.id = next();
                h
            });
            let cells: Vec<AccessNode> = cells.collect();
            out.push((row, cells));
        }
        let chosen = table.chosen();
        for place in table.shown_places() {
            let Some(b) = table.row_box(place) else { continue };
            let texts: Vec<String> = columns.iter().map(|c| table.shown(place, &c.field)).collect();
            let mut row = AccessNode::new(next(), Role::Row, texts.iter().filter(|t| !t.is_empty()).cloned().collect::<Vec<_>>().join(", "), b).with_actions(&[AccessAction::Click, AccessAction::ScrollIntoView]);
            row.states.selected = Some(chosen == Some(place));
            row.place = Some(place);
            row.part = part.map(str::to_string);
            if let Some(shown) = table.tree_place(place) {
                row.level = Some(shown.1 + 1);
                if shown.2 {
                    let open = table.is_open(place);
                    row.states.expanded = Some(open);
                    row.toggle = table.toggle_box(place);
                    row.actions.push(if open { AccessAction::Collapse } else { AccessAction::Expand });
                }
            }
            if chosen == Some(place) && self.focused == Some(id) {
                self.active = Some(row.id);
            }
            let cells: Vec<AccessNode> = columns
                .iter()
                .zip(texts)
                .filter_map(|(c, text)| {
                    let mut cell = AccessNode::new(0, Role::Cell, text, table.cell_box(place, &c.field)?);
                    cell.place = Some(place);
                    Some(cell)
                })
                .map(|mut cell| {
                    cell.id = next();
                    cell
                })
                .collect();
            out.push((row, cells));
        }
        out
    }
}

/// The node of an element that is no part: the window's frame, dialogs, menus and lists over the window.
#[allow(clippy::type_complexity)]
fn frame_node(builder: &mut Builder, e: &dyn Element, id: ElementId, b: Rect, any: &dyn std::any::Any) -> Option<(AccessNode, Vec<(AccessNode, Vec<AccessNode>)>)> {
    use crate::dialogs::{ask::AskDialog, keys::KeysWindow, long_call::LongCallBar, over::DialogOverlay, DialogButton};
    use crate::frame::{buttons::FrameButton, footer::FrameFooter, menu_bar::FrameMenuBar, menus::FrameMenuList, ribbon::FrameRibbon, sidebar::FrameSidebar, toolbar::FrameToolbar};
    use crate::parts::containers::container::ContainerButton;
    use crate::parts::inputs::choice_list::{ChoiceList, LINE};
    use crate::window::messages::{NoteList, SaidDialog};
    use AccessAction::*;
    let button = |name: String| AccessNode::new(id, Role::Button, name, b).with_actions(&[Focus, Click, ScrollIntoView]);
    let mut extra = Vec::new();
    let node = if let Some(d) = any.downcast_ref::<DialogButton>() {
        button(d.text.clone())
    } else if any.is::<crate::window::messages::OkButton>() {
        button(crate::locale::translate("ok", "OK"))
    } else if let Some(c) = any.downcast_ref::<ContainerButton>() {
        button(c.text())
    } else if let Some(f) = any.downcast_ref::<FrameButton>() {
        let caption = f.caption();
        if caption.is_empty() && crate::frame::rules::is_separator(f.item()) {
            return None;
        }
        let mut n = button(caption);
        n.part = f.item().text("name").map(str::to_string);
        n
    } else if any.is::<FrameToolbar>() {
        AccessNode::new(id, Role::Toolbar, "", b)
    } else if let Some(ribbon) = any.downcast_ref::<FrameRibbon>() {
        let host = ribbon.host();
        let shown = ribbon.shown_tab();
        for (i, (tab, place)) in ribbon.tabs().into_iter().enumerate() {
            let title = ribbon.ribbon().tabs.get(tab.saturating_sub(1)).map(|t| host.text(Some(&t.title))).unwrap_or_default();
            let mut n = AccessNode::new(virtual_id(id, i), Role::Tab, title, place).with_actions(&[Click]);
            n.states.selected = Some(shown == Some(tab));
            extra.push((n, Vec::new()));
        }
        // The tabs are the strip's, which takes the focus: a tab list.
        let strip = ribbon.tab_strip().node().id();
        builder.given.insert(strip, Given { role: Role::TabList, name: String::new(), checked: None, selected: None, expanded: None });
        builder.under.insert(strip, std::mem::take(&mut extra));
        AccessNode::new(id, Role::Toolbar, "", b)
    } else if let Some(bar) = any.downcast_ref::<FrameMenuBar>() {
        let host = bar.host();
        for (i, (item, place)) in bar.placed().into_iter().enumerate() {
            let mut n = AccessNode::new(virtual_id(id, i), Role::MenuItem, host.text(item.text("caption")), place).with_actions(&[Click]);
            n.part = item.text("name").map(str::to_string);
            n.states.disabled = !bar.enabled() || !crate::frame::rules::takes(host.as_ref(), item.text("name"));
            extra.push((n, Vec::new()));
        }
        AccessNode::new(id, Role::MenuBar, "", b)
    } else if let Some(menu) = any.downcast_ref::<FrameMenuList>() {
        for (i, entry) in menu.entries().iter().enumerate().filter(|(_, e)| !e.separator) {
            let role = if entry.checked { Role::MenuItemCheckBox } else { Role::MenuItem };
            let mut n = AccessNode::new(virtual_id(id, i), role, entry.caption.clone(), menu.row_of(i)).with_actions(&[Click]);
            n.states.disabled = !entry.enabled;
            if entry.checked {
                n.states.checked = Some(true);
            }
            if !entry.items.is_empty() {
                n.states.expanded = Some(false);
            }
            if menu.marked() == Some(i) {
                builder.active = Some(n.id);
            }
            extra.push((n, Vec::new()));
        }
        AccessNode::new(id, Role::Menu, "", b)
    } else if let Some(list) = any.downcast_ref::<ChoiceList>() {
        let state = list.state.borrow();
        for (i, label) in state.labels.iter().enumerate() {
            let line = Rect::new(b.x + 1.0, b.y + 1.0 + i as f64 * LINE - state.top, b.width - 2.0, LINE);
            if line.y < b.y - 1.0 || line.bottom() > b.bottom() + 1.0 {
                continue;
            }
            let mut n = AccessNode::new(virtual_id(id, i), Role::Option, label.clone(), line).with_actions(&[Click]);
            n.states.selected = Some(state.highlighted == Some(i));
            if state.highlighted == Some(i) {
                builder.active = Some(n.id);
            }
            extra.push((n, Vec::new()));
        }
        AccessNode::new(id, Role::ListBox, "", b)
    } else if let Some(side) = any.downcast_ref::<FrameSidebar>() {
        let mut n = AccessNode::new(id, Role::Complementary, "", b);
        n.part = side.sidebar().name.clone();
        n.states.expanded = side.sidebar().collapsible.then_some(!side.collapsed());
        n
    } else if let Some(footer) = any.downcast_ref::<FrameFooter>() {
        if let Some(status) = footer.status().filter(|s| !s.is_empty()) {
            extra.push((AccessNode::new(virtual_id(id, 0), Role::Status, status, footer.status_box()), Vec::new()));
        }
        AccessNode::new(id, Role::ContentInfo, "", b)
    } else if let Some(notes) = any.downcast_ref::<NoteList>() {
        for (i, note) in notes.notes().iter().enumerate() {
            extra.push((AccessNode::new(virtual_id(id, i), Role::Status, note.text.show(), b), Vec::new()));
        }
        if extra.is_empty() {
            return None;
        }
        AccessNode::new(id, Role::Log, "", b)
    } else if let Some(said) = any.downcast_ref::<SaidDialog>() {
        AccessNode::new(id, Role::AlertDialog, said.message.text.show(), said.panel())
    } else if let Some(ask) = any.downcast_ref::<AskDialog>() {
        AccessNode::new(id, Role::Dialog, ask.ask.question.show(), ask.panel())
    } else if let Some(dialog) = any.downcast_ref::<DialogOverlay>() {
        AccessNode::new(id, Role::Dialog, dialog.title.clone(), dialog.panel())
    } else if let Some(slideout) = any.downcast_ref::<crate::parts::containers::slideout::SlideoutFrame>() {
        // A slide-out is a region like a dialog, named by its caption; it holds the focus while it is open.
        AccessNode::new(id, Role::Dialog, slideout.caption(), b)
    } else if let Some(handle) = any.downcast_ref::<crate::parts::containers::slideout::SlideoutHandle>() {
        // Its handle on the window's edge: a button named by its caption that says whether it is out.
        let mut n = AccessNode::new(id, Role::Button, handle.slide().caption(), b).with_actions(&[AccessAction::Click, AccessAction::Focus, AccessAction::ScrollIntoView]);
        n.states.expanded = Some(handle.slide().is_open());
        n
    } else if any.is::<KeysWindow>() {
        AccessNode::new(id, Role::Dialog, crate::dialogs::keys::title(), b)
    } else if let Some(bar) = any.downcast_ref::<LongCallBar>() {
        let mut n = AccessNode::new(id, Role::ProgressBar, bar.said_text(), b);
        let full = bar.fraction();
        n.numeric = Some(Numeric { value: Some((full * 100.0).round()), min: Some(0.0), max: Some(100.0), step: None });
        n
    } else if e.focusable() {
        // A small button inside a part (a calendar's, a tag's remove) or of the frame (a sidebar's fold): it says what it does.
        let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
        let said = e.tooltip_at(x, y).unwrap_or_default();
        if let Some(editor) = any.downcast_ref::<crate::parts::text_editor::TextEditor>() {
            let mut n = AccessNode::new(id, if editor.is_multiline() { Role::TextArea } else { Role::TextBox }, said, b).with_actions(&[Focus, SetValue, ScrollIntoView]);
            n.value = Some(editor.text());
            n
        } else {
            button(said)
        }
    } else {
        return None;
    };
    Some((node, extra))
}

/// A value as a short text: a text as it is, anything else as JSON writes it.
fn short(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

//! The parts of a window (DESIGN 7.9): elements made from their spec in the
//! window's markup, which read and change the window's values through their
//! host. Each kind of part is a module of its own; `make` finds it by its
//! kind's name, and a kind not drawn yet shows that in its place.

pub mod button;
pub mod custom;
pub mod containers;
pub mod data;
pub mod inputs;
pub mod check_box;
pub mod heading;
pub mod label;
pub mod sound;
pub mod text_box;
pub mod text_editor;
pub mod timer;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clarity_runtime::ui::{Level, PartState, Text};
use serde_json::{Map, Value};

use crate::element::{Element, Node};
use crate::grid::{row_height, GridItem};
use crate::locale;
use crate::markup::{CommandSpec, HandlerCall, PartSpec};
use crate::painter::{Painter, TextAlign};

/// What a dropdown or a radio group offers: a value and its label, and for a search dropdown the texts of the fields it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub value: Value,
    pub label: String,
    pub cells: Option<Vec<String>>,
}

/// What the application knows of a field: its label, format, whether it is required, its range, what it refers to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldFacts {
    pub label: String,
    pub format: Option<String>,
    pub required: bool,
    pub min: Option<String>,
    pub max: Option<String>,
    pub refers: Option<String>,
    pub parent: Option<String>,
    pub color: bool,
    /// The help it declares (`help @customerVatNumberHelp`), a key or the text itself; none for none.
    pub help: Option<String>,
}

/// The person's formats for numbers and dates: their language and currency.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Formats {
    pub language: Option<String>,
    pub currency: Option<String>,
}

/// What a part reads and changes: the window's values, as the host makes
/// them reachable. A value is named by its path in the window's data, as the
/// markup binds it: `order.status`, `tasks[1].done`.
pub trait PartHost {
    /// The value at `path`; none when there is none.
    fn read(&self, path: &str) -> Option<Value>;

    /// The records of a list at `path`: a table's, a list's, a chart's rows.
    fn rows(&self, path: &str) -> Vec<Value> {
        match self.read(path) {
            Some(Value::Array(rows)) => rows,
            _ => Vec::new(),
        }
    }

    /// The person changed the value at `path` in `part`: one edit, which Ctrl+Z takes back.
    fn write(&self, part: &str, path: &str, value: Value);

    /// The person changed what an unbound part keeps in its own state (its
    /// text, its tick): one edit of `part`, which Ctrl+Z takes back.
    fn edited(&self, part: &str);

    /// The state of the part named `name`, which code sets; none for a part without a name.
    fn state(&self, name: Option<&str>) -> Option<PartState>;

    /// The part named `name` has this state now (an unbound part's own text
    /// or tick, an accordion's open section): plain data, as every language's
    /// host gives it. A part changes one with `parts::change_state`.
    fn set_state(&self, name: &str, state: PartState);

    /// Runs a handler of `part`, or opens the window a click targets; true
    /// when it ran and did not refuse.
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool;

    /// Whether a handler of `part` can run now (C# IPartHost.CanRun, a command's
    /// CanExecute): a timer leaves a tick out when it cannot.
    fn can_run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        let _ = (part, call);
        true
    }

    /// A text as written in the project, in the language shown: `@saveCaption` by its key, else as it is.
    fn text(&self, written: Option<&str>) -> String;

    /// The type of the value at `path`, by its name: an enum offers its values, a record its fields.
    fn type_of(&self, path: &str) -> Option<String> {
        let _ = path;
        None
    }

    /// What a dropdown or radio group bound to `path` offers.
    fn choices(&self, path: &str) -> Vec<Choice> {
        let _ = path;
        Vec::new()
    }

    /// What the dropdown `part` bound to `path` offers: a typed one its field's values, a search one its records.
    fn offered(&self, part: &PartSpec, path: &str) -> Vec<Choice> {
        let _ = part;
        self.choices(path)
    }

    /// What the application knows of the field at `path`; none for a value that is no entity's field.
    fn field(&self, path: &str) -> Option<FieldFacts> {
        let _ = path;
        None
    }

    /// The person's formats for numbers and dates.
    fn formats(&self) -> Formats {
        Formats::default()
    }

    /// Opens an address in the system's program for it: a web page in the browser.
    fn open_address(&self, address: &str) {
        let _ = address;
    }

    /// A file of the application's resources by its name (`logo.png`); none when it carries none.
    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        let _ = name;
        None
    }

    /// The SVG path of one of the icons the application carries, by its name; none when it carries none.
    fn icon_path(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }

    /// How often the window's values changed: parts that keep something of them read them again when it moved.
    fn changes(&self) -> u64;

    /// The stored file of the record's field `bind`, or of the field's store at `store_path`; none when there is none.
    fn fetch(&self, bind: &str, store_path: Option<&str>) -> Option<Vec<u8>> {
        let _ = (bind, store_path);
        None
    }

    /// Says something to the person among the window's messages.
    fn say(&self, level: Level, text: Text) {
        let _ = (level, text);
    }

    /// Runs the handler of `event` of canvas `part` with where it was; none when the window has no command for it.
    fn pointed(&self, part: &str, event: &str, at: &Map<String, Value>) -> Option<bool> {
        let _ = (part, event, at);
        None
    }

    /// Applies what the person did on a canvas with a mode as one edit; none when the window has no command for it.
    fn acted(&self, part: &str, action: &Map<String, Value>) -> Option<bool> {
        let _ = (part, action);
        None
    }

    /// A canvas's engine or a custom part told the window `tell`: the handler wired to it runs with the values.
    fn told(&self, part: &str, tell: &str, values: &[Value]) {
        let _ = (part, tell, values);
    }

    /// The path the part named `part` of this window binds; none when it binds none.
    fn bound_of(&self, part: &str) -> Option<String> {
        let _ = part;
        None
    }

    /// The keys command `command` answers: the person's, where they set their own, else `designed`.
    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        let _ = command;
        designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default()
    }

    /// What code told the custom part `part` since the last time (`arena.restart(2)`), in order; taken, so each is heard once.
    fn hears(&self, part: &str) -> Vec<clarity_runtime::ui::Hear> {
        let _ = part;
        Vec::new()
    }

    /// The chosen row of the list, table or tree `part`: its place in the data; none when none is chosen.
    fn selected(&self, part: &str) -> Option<usize> {
        let _ = part;
        None
    }

    /// The person chose row `place` of `part` (none: no row); handlers read it, it is no edit.
    fn select(&self, part: &str, place: Option<usize>) {
        let _ = (part, place);
    }

    /// Starts a new instance of the engine beside the screen `name` for a
    /// canvas that `runs` it: its number, or why it could not start; none
    /// when the host has no engines (a test). The host keeps the instance
    /// (`crate::engines::Engines`) until `engine_stop`.
    fn engine_start(&self, name: &str) -> Option<Result<u64, String>> {
        let _ = name;
        None
    }

    /// The pointer and keys engine instance `id` takes.
    fn engine_hears(&self, id: u64) -> crate::parts::data::canvas::engine::Hears {
        let _ = id;
        crate::parts::data::canvas::engine::Hears::default()
    }

    /// Starts the next frame of engine instance `id`: the input since the
    /// last, then `ms`, the milliseconds since the last frame. What it draws
    /// comes through `engine_drawn`, at once or on a later turn.
    fn engine_frame(&self, id: u64, ms: f64, input: Vec<crate::parts::data::canvas::engine::ScreenInput>) {
        let _ = (id, ms, input);
    }

    /// The frame of engine instance `id` that finished since the last ask:
    /// its shapes and tells, or why the engine stopped; none while it runs.
    fn engine_drawn(&self, id: u64) -> Option<Result<crate::parts::data::canvas::engine::Drawn, String>> {
        let _ = id;
        None
    }

    /// Ends engine instance `id`: its canvas left the screen or went.
    fn engine_stop(&self, id: u64) {
        let _ = id;
    }

    /// The name of the window the parts are in (`Main`); none when the host has no window (a test).
    fn window_name(&self) -> Option<String> {
        None
    }

}

/// What every part has: its node, its spec and its host.
pub struct PartBase {
    pub node: Node,
    pub spec: PartSpec,
    pub host: Rc<dyn PartHost>,
    /// Its state as last read, with the window's change count then: read once
    /// per change, not copied on every look (a canvas's state holds its whole drawing).
    kept: RefCell<Option<(u64, Rc<PartState>)>>,
}

impl PartBase {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> PartBase {
        PartBase { node: Node::new(), spec, host, kept: RefCell::new(None) }
    }

    pub fn name(&self) -> Option<&str> {
        self.spec.name()
    }

    /// Its state, which code sets by its name.
    /// Its state as code set it, read again only when the window changed; a
    /// host that counts no changes (0) is asked every time.
    pub fn state(&self) -> Option<Rc<PartState>> {
        let changes = self.host.changes();
        if changes != 0 {
            if let Some((at, kept)) = self.kept.borrow().as_ref() {
                if *at == changes {
                    return Some(kept.clone());
                }
            }
        }
        let read = Rc::new(self.host.state(self.spec.name())?);
        if changes != 0 {
            *self.kept.borrow_mut() = Some((changes, read.clone()));
        }
        Some(read)
    }

    /// Whether it shows: code may hide it.
    pub fn shown(&self) -> bool {
        self.state().and_then(|s| s.visible).unwrap_or(true)
    }

    /// Whether the person may only look: marked read-only in its markup, or made read-only by code.
    pub fn read_only(&self) -> bool {
        self.spec.flag("readonly") || self.state().and_then(|s| s.readonly).unwrap_or(false)
    }

    /// Its own state's enabled; `enabled` also asks the parts it is in.
    pub fn own_enabled(&self) -> bool {
        self.state().and_then(|s| s.enabled).unwrap_or(true)
    }

    /// Whether the person may use it: it and every part it is in are
    /// enabled (a disabled form disables its buttons), as the screen last
    /// saw its tree.
    pub fn enabled(&self) -> bool {
        self.own_enabled() && !self.node.within_disabled()
    }

    /// The path in the window's data it binds: `order.status`; none unbound.
    pub fn bound(&self) -> Option<&str> {
        self.spec.text("bind").filter(|b| !b.is_empty())
    }

    /// Its caption as shown: as code set it in its state, else as written, translated; none when it has none.
    pub fn caption(&self) -> Option<String> {
        if let Some(caption) = self.state().and_then(|s| s.caption.clone()) {
            if caption.key.is_some() || !caption.text.is_empty() {
                return Some(caption.show());
            }
        }
        self.spec.text("caption").map(|written| self.host.text(Some(written)))
    }

    /// Runs its handlers of `event`; a click also opens the window its `opens` names. False when one refused.
    pub fn fire(&self, event: &str) -> bool {
        let mut ran = true;
        for call in self.spec.on(event) {
            ran &= self.host.run(self.spec.name(), call);
        }
        if event == "click" {
            if let Some(target) = self.spec.text("onClick").filter(|t| !t.is_empty()) {
                ran &= self.host.run(self.spec.name(), &HandlerCall::new(route_event(self.spec.flag("newWindow")), target, Vec::new()));
            }
        }
        ran
    }

    /// The person picked an item of its menu: the item's handler runs, or the window it opens opens.
    pub fn pick(&self, item: &CommandSpec) {
        pick(self.host.as_ref(), self.spec.name(), item);
    }
}

/// Changes the state of the part named `name` over its host: reads it,
/// changes it and gives it back with `PartHost::set_state`.
pub fn change_state(host: &dyn PartHost, name: &str, change: impl FnOnce(&mut PartState)) {
    let mut state = host.state(Some(name)).unwrap_or_default();
    change(&mut state);
    host.set_state(name, state);
}

/// A menu's item picked: its handler runs, or the window its `opens` names opens.
pub fn pick(host: &dyn PartHost, part: Option<&str>, item: &CommandSpec) {
    if let Some(handler) = item.text("handler").filter(|h| !h.is_empty()) {
        host.run(part, &crate::markup::handler_of("click", handler));
    }
    if let Some(target) = item.text("onClick").filter(|t| !t.is_empty()) {
        host.run(part, &HandlerCall::new(route_event(item.flag("newWindow")), target, Vec::new()));
    }
}

/// The event a click route runs with: a route marked `in new window` opens
/// its window in a desktop window of its own (where the host gives them),
/// any other in place, whatever other routes to the same window say.
pub const NEW_WINDOW_CLICK: &str = "clickNewWindow";

fn route_event(new_window: bool) -> &'static str {
    if new_window { NEW_WINDOW_CLICK } else { "click" }
}

/// A part: an element with its spec and host.
pub trait Part: Element {
    fn base(&self) -> &PartBase;

    /// Whether its rows share the window's spare height (a table, a list).
    fn grows(&self) -> bool {
        false
    }

    /// Clicked by a key: a button's shortcut, Enter for the default button. A part pressed by a key as a click on it does.
    fn press(&mut self, cx: &mut crate::element::Cx) {
        self.on_click(cx);
    }

    /// Which of its corners are round: in a button group only the outer ones. A part with no round corners lets it go.
    fn set_corners(&mut self, corners: crate::painter::Corners) {
        let _ = corners;
    }

    /// Whether it draws its `style` itself (a button, a badge); any other part has it from `style_of`.
    fn draws_own_style(&self) -> bool {
        false
    }
}

/// The items of a part's menu as they show over `host`: hidden ones left
/// out, disabled ones greyed, sub menus under their item; picking one runs
/// its handler or opens its window.
pub fn menu_entries(host: &Rc<dyn PartHost>, part: Option<&str>, items: &[CommandSpec]) -> Vec<crate::frame::menus::MenuEntry> {
    use crate::frame::{menus::MenuEntry, rules};
    let mut entries = Vec::new();
    for item in items {
        let name = item.text("name");
        if !rules::shows(host.as_ref(), name) {
            continue;
        }
        if rules::is_separator(item) {
            entries.push(MenuEntry::line());
            continue;
        }
        let sub = menu_entries(host, part, &item.items);
        let run: Option<Rc<dyn Fn()>> = if sub.is_empty() {
            let (host, part, item) = (host.clone(), part.map(str::to_string), item.clone());
            Some(Rc::new(move || pick(host.as_ref(), part.as_deref(), &item)))
        } else {
            None
        };
        entries.push(MenuEntry {
            caption: host.text(item.text("caption")),
            icon: item.text("icon").filter(|i| !i.is_empty()).and_then(|i| host.icon_path(i)),
            key: item.text("shortcut").filter(|k| !k.is_empty()).map(str::to_string),
            checked: rules::checked(host.as_ref(), name),
            enabled: rules::takes(host.as_ref(), name),
            separator: false,
            items: sub,
            run,
            ..MenuEntry::default()
        });
    }
    entries
}

/// A part's `menu`, as a context menu at the pointer: hidden items left
/// out, disabled ones greyed, as the menu's and each item's state says. A
/// split button's menu is its dropdown, not a context menu.
pub fn designed_menu(part: &dyn Part) -> Option<Vec<crate::frame::menus::MenuEntry>> {
    let base = part.base();
    let menu = base.spec.menu.as_ref().filter(|m| !m.items.is_empty())?;
    if base.spec.flag("split") {
        return None;
    }
    let host = base.host.as_ref();
    let named = menu.name.as_deref();
    if !crate::frame::rules::shows(host, named) || !crate::frame::rules::takes(host, named) {
        return None;
    }
    let entries = menu_entries(&base.host, base.spec.name(), &menu.items);
    (!entries.iter().all(|e| e.separator)).then_some(entries)
}

thread_local! {
    /// The looks of the styles drawn, by the look they lie over and the style's name.
    static STYLED: RefCell<HashMap<(u64, String), (std::sync::Arc<crate::theme::ThemeLook>, Rc<crate::fonts::Fonts>)>> = RefCell::new(HashMap::new());
}

/// A part's `style` (a style of the theme): the style's fill and border
/// under it, drawn here, and its text color, size, font, weight and align
/// as the look of everything it draws. None for a part without one.
pub fn style_of(base: &PartBase, p: &Painter) -> Option<crate::painter::StyledLook> {
    let name = base.spec.text("style").filter(|s| !s.is_empty())?;
    let style = p.look.style(name);
    let bounds = base.node.bounds;
    let radius = style.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("input"));
    if let Some(fill) = style.background {
        p.fill(bounds, fill, radius);
    }
    if let Some(border) = style.border {
        p.stroke(bounds, border, p.look.border(), radius);
    }
    // The style's weight and align reach every text the part draws.
    let align = match style.align.as_deref() {
        Some("center") => Some(TextAlign::Center),
        Some("right" | "end") => Some(TextAlign::Right),
        Some("left" | "start") => Some(TextAlign::Left),
        _ => None,
    };
    let bold = style.bold();
    if style.text.is_none() && style.size.is_none() && style.font.is_none() {
        return (bold || align.is_some()).then_some(crate::painter::StyledLook { look: None, bold, align });
    }
    let key = (p.look.serial(), name.to_string());
    let look = STYLED.with(|styled| {
        if let Some(kept) = styled.borrow().get(&key) {
            return kept.clone();
        }
        let hex = |c: skia_safe::Color| format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a());
        let mut own = Map::new();
        if let Some(text) = style.text {
            // The style's foreground is every text of the part, a label's muted one too.
            own.insert("colors".into(), serde_json::json!({ "text": hex(text), "muted": hex(text) }));
        }
        if let Some(size) = style.size {
            own.insert("sizes".into(), serde_json::json!({ "text": size, "heading": size }));
        }
        if let Some(font) = &style.font {
            own.insert("fonts".into(), serde_json::json!({ "text": font }));
        }
        let look = p.look.over_this(&Value::Object(own).to_string());
        let fonts = Rc::new(crate::fonts::Fonts::new(look.clone()));
        styled.borrow_mut().insert(key, (look.clone(), fonts.clone()));
        (look, fonts)
    });
    Some(crate::painter::StyledLook { look: Some(look), bold, align })
}

/// Writes the methods every part's element has the same way, for a struct with a field `base`: its node, and that code may hide it.
#[macro_export]
macro_rules! part_element {
    () => {
        fn node(&self) -> &$crate::element::Node {
            &self.base.node
        }
        fn node_mut(&mut self) -> &mut $crate::element::Node {
            &mut self.base.node
        }
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
        fn is_shown(&self) -> bool {
            self.base.shown()
        }
        fn as_part(&self) -> Option<&dyn $crate::parts::Part> {
            Some(self)
        }
        fn as_part_mut(&mut self) -> Option<&mut dyn $crate::parts::Part> {
            Some(self)
        }
    };
}

/// Makes the part of a kind from its spec and host.
pub type MakePart = fn(PartSpec, Rc<dyn PartHost>) -> Box<dyn Part>;

/// The kinds Clarity.UI draws, each with the module that makes it.
const DRAWN: [(&str, MakePart); 11] = [
    ("label", label::make),
    ("heading", heading::make),
    ("button", button::make),
    ("textbox", text_box::make),
    ("memo", text_box::make),
    ("password", text_box::make),
    ("checkbox", check_box::make),
    ("switch", check_box::make),
    ("custom", custom::make),
    ("sound", sound::make),
    ("timer", timer::make),
];

thread_local! {
    static ADDED: RefCell<HashMap<String, MakePart>> = RefCell::new(HashMap::new());
}

/// Adds a part of a kind of its own (a program's custom part), or one drawn in place of Clarity.UI's.
pub fn register(kind: &str, make: MakePart) {
    ADDED.with(|added| added.borrow_mut().insert(kind.to_string(), make));
}

/// The kinds Clarity.UI draws now, its own and those added.
pub fn drawn() -> Vec<String> {
    let mut kinds: Vec<String> = own_kinds().map(|(k, _)| k.to_string()).collect();
    ADDED.with(|added| kinds.extend(added.borrow().keys().cloned()));
    kinds.sort();
    kinds.dedup();
    kinds
}

/// Every kind Clarity.UI draws itself: the first parts and each group's.
fn own_kinds() -> impl Iterator<Item = &'static (&'static str, MakePart)> {
    DRAWN.iter().chain(inputs::KINDS).chain(containers::KINDS).chain(data::KINDS)
}

fn maker(kind: &str) -> Option<MakePart> {
    ADDED.with(|added| added.borrow().get(kind).copied()).or_else(|| own_kinds().find(|(k, _)| *k == kind).map(|(_, m)| *m))
}

/// The part of `spec`: its kind's own part; a kind not drawn yet, or one that
/// fails as it is made, shows that in its place.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let Some(make) = maker(&spec.kind) else {
        return Box::new(MissingPart::new(spec, host, None));
    };
    let (kept_spec, kept_host) = (spec.clone(), host.clone());
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| make(spec, host))) {
        Ok(part) => part,
        Err(panic) => {
            let why = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            Box::new(MissingPart::new(kept_spec, kept_host, Some(why)))
        }
    }
}

/// An element as a part of a window, when it is one.
pub fn as_part(element: &dyn Element) -> Option<&dyn Part> {
    element.as_part()
}

/// The place in a grid of a part made from `spec`.
pub fn place_of(part: &dyn Part) -> GridItem {
    part.base().spec.place(part.grows())
}

/// A kind of part Clarity.UI does not draw yet, or one that failed: which kind, and why, in its place.
pub struct MissingPart {
    base: PartBase,
    /// Why it failed; none for a kind not drawn yet.
    pub why: Option<String>,
}

impl MissingPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>, why: Option<String>) -> MissingPart {
        MissingPart { base: PartBase::new(spec, host), why }
    }

    /// What it says in its place.
    pub fn says(&self) -> String {
        let kind = &self.base.spec.kind;
        match &self.why {
            None => locale::fill(&locale::translate("partNotDrawn", "{kind}: not drawn yet"), "kind", kind),
            Some(why) => locale::fill(&locale::fill(&locale::translate("partFailed", "{kind}: {why}"), "kind", kind), "why", why),
        }
    }
}

impl Element for MissingPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let line = p.look.color(if self.why.is_none() { "border" } else { "error" });
        p.stroke(bounds, line, 1.0, p.look.radius("input"));
        let row = crate::geometry::Rect { height: bounds.height.min(row_height()), ..bounds };
        p.text(&self.says(), row.inset(6.0), p.look.color(if self.why.is_none() { "muted" } else { "error" }), p.look.size("small"), "text", false, TextAlign::Left);
    }
}

impl Part for MissingPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

//! Clarity's keys window on Clarity.UI (DESIGN 5.28), where the project file
//! says `keys editable`: every command per place, Everywhere (the app's
//! frame) first, then each window's own, with its keys. Change, then press
//! the new key; a key another command has in the same place offers a swap;
//! Reset puts the design's key back, Reset all every one; a key the inputs
//! need is refused, saying why. In Clarity's words, as on the web.
//!
//! It works in two places: as the root of a screen of its own (a desktop
//! window of its own) and as the content of a modal overlay over the window
//! the person works in. It scrolls with the wheel when its commands are
//! taller than it is. The commands come from the model (`keys`), the
//! person's keys are kept with the settings by whoever opens it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::keys::{Command, KeyRules, PersonKeys};
use serde_json::Value;

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::window::messages::wrap;

const CONTENT_WIDTH: f64 = 460.0;
const PAD: f64 = 16.0;
const GAP: f64 = 8.0;
const HEADING_HEIGHT: f64 = 28.0;
const MESSAGE_HEIGHT: f64 = 76.0;
const LINE: f64 = 20.0;
const TITLE_HEIGHT: f64 = 22.0;
const ROW_HEIGHT: f64 = 28.0;
const ROW_GAP: f64 = 4.0;
const GROUP_PAD: f64 = 8.0;
const KEYS_WIDTH: f64 = 120.0;
const COLUMN_GAP: f64 = 6.0;
const MOST_HIGH: f64 = 640.0;

/// A command a key can run: its id among the person's keys, its caption as written, and the keys its design gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyCommandInfo {
    pub id: String,
    pub caption: String,
    pub keys: Vec<String>,
}

/// The commands of one place: the app's frame (`id` empty, no title) or a window.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyLayerInfo {
    pub id: String,
    pub title: Option<String>,
    pub commands: Vec<KeyCommandInfo>,
}

/// A text as written in the model: a key (`@newOrder`) in the language shown, else as written.
fn shown(written: &str) -> String {
    match written.strip_prefix('@') {
        Some(key) => clarity_runtime::i18n::value(key).unwrap_or_else(|| key.to_string()),
        None => written.to_string(),
    }
}

/// The commands of the keys window, as the model carries them (`keys`); none where the project does not let people set keys.
pub fn catalog(model: &str) -> Vec<KeyLayerInfo> {
    let model: Value = serde_json::from_str(model).unwrap_or_default();
    let strings = |v: &Value| v.as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()).unwrap_or_default();
    model["keys"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|layer| KeyLayerInfo {
            id: layer["id"].as_str().unwrap_or_default().to_string(),
            title: layer["title"].as_str().map(str::to_string),
            commands: layer["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    let name = c["name"].as_str().unwrap_or_default();
                    KeyCommandInfo { id: c["id"].as_str().unwrap_or_default().to_string(), caption: c["caption"].as_str().unwrap_or(name).to_string(), keys: strings(&c["keys"]) }
                })
                .collect(),
        })
        .collect()
}

/// Keeps the person's keys, as they changed.
pub type KeepKeys = Rc<dyn Fn(&PersonKeys)>;

/// What a button of the keys window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Change,
    Reset,
    Swap,
    Cancel,
    ResetAll,
    Close,
}

/// Why a key cannot be a command's, as Clarity's words say it.
fn refusal(why: &str, key: &str) -> String {
    let text = match why {
        "typing" => locale::translate("keysTyping", "{key} cannot be used: a key without Ctrl or Alt is typing"),
        "tab" => locale::translate("keysTab", "{key} cannot be used: Tab moves between fields"),
        "enter" => locale::translate("keysEnter", "{key} cannot be used: Enter confirms"),
        _ => locale::translate("keysEscape", "{key} cannot be used: Escape cancels"),
    };
    locale::fill(&text, "key", key)
}

/// A key pressed as Clarity writes keys (`Ctrl+Shift+N`); none for a key Clarity.UI does not know.
pub fn written(key: KeyInput) -> Option<String> {
    let name = match key.key {
        Key::Other | Key::Shift | Key::Control | Key::Alt => return None,
        k if Key::LETTERS.contains(&k) => ((b'A' + Key::LETTERS.iter().position(|l| *l == k)? as u8) as char).to_string(),
        k if Key::DIGITS.contains(&k) => ((b'0' + Key::DIGITS.iter().position(|d| *d == k)? as u8) as char).to_string(),
        k if Key::FUNCTIONS.contains(&k) => format!("F{}", Key::FUNCTIONS.iter().position(|f| *f == k)? + 1),
        Key::Plus => "+".into(),
        Key::Minus => "-".into(),
        Key::Multiply => "*".into(),
        Key::Slash => "/".into(),
        Key::Period => ".".into(),
        Key::Comma => ",".into(),
        Key::Equal => "=".into(),
        Key::Semicolon => ";".into(),
        Key::Quote => "'".into(),
        Key::Backslash => "\\".into(),
        Key::BracketLeft => "[".into(),
        Key::BracketRight => "]".into(),
        Key::Backquote => "`".into(),
        other => format!("{other:?}"),
    };
    let mut held = Vec::new();
    if key.control() {
        held.push("Ctrl".to_string());
    }
    if key.alt() {
        held.push("Alt".to_string());
    }
    if key.shift() {
        held.push("Shift".to_string());
    }
    if key.meta() {
        held.push("Meta".to_string());
    }
    held.push(name);
    Some(held.join("+"))
}

/// What the keys window shows and does, shared with its buttons.
pub struct KeysState {
    layers: Vec<KeyLayerInfo>,
    commands: Vec<Command>,
    captions: BTreeMap<String, String>,
    person: PersonKeys,
    keep: KeepKeys,
    capturing: Option<String>,
    note: Option<String>,
    swap: Option<(String, String, String)>,
    /// The person closed it: Close, or Escape while it waits for no key.
    pub done: bool,
}

impl KeysState {
    fn name_of(&self, id: &str) -> String {
        self.captions.get(id).map(|c| shown(c)).unwrap_or_else(|| id.to_string())
    }

    fn keys_of(&self, c: &KeyCommandInfo) -> Vec<String> {
        self.person.choices.get(&c.id).cloned().unwrap_or_else(|| c.keys.clone())
    }

    fn keys_text(&self, c: &KeyCommandInfo) -> String {
        let keys = self.keys_of(c);
        if keys.is_empty() { locale::translate("keysNone", "None") } else { keys.join(", ") }
    }

    fn place_name(layer: &KeyLayerInfo) -> String {
        match &layer.title {
            Some(title) => shown(title),
            None if layer.id.is_empty() => locale::translate("keysEverywhere", "Everywhere"),
            None => layer.id.clone(),
        }
    }

    fn message(&self) -> Option<String> {
        if let Some((_, key, other)) = &self.swap {
            let text = locale::fill(&locale::translate("keysTaken", "{key} is the key of {command}. Swap them?"), "key", key);
            return Some(locale::fill(&text, "command", &self.name_of(other)));
        }
        self.note.clone().or_else(|| self.capturing.as_ref().map(|_| locale::translate("keysPress", "Press the new key; Escape stops")))
    }

    fn set(&mut self, choices: clarity_runtime::keys::Choices) {
        self.person = PersonKeys { choices };
        (self.keep)(&self.person);
    }

    fn take(&mut self, id: &str, key: &str) {
        let also = KeyRules::also_has(&self.commands, &self.person.choices, id, key);
        self.note = if also.is_empty() {
            None
        } else {
            let names: Vec<String> = also.iter().map(|a| self.name_of(a)).collect();
            let text = locale::fill(&locale::translate("keysAlso", "{key} is also the key of {command}, elsewhere; here {mine} goes first"), "key", key);
            let text = locale::fill(&text, "command", &names.join(", "));
            Some(locale::fill(&text, "mine", &self.name_of(id)))
        };
        self.swap = None;
        let (next, _) = KeyRules::assign(&self.commands, &self.person.choices, id, key);
        self.set(next);
    }

    /// A button pressed.
    pub fn press(&mut self, role: Role, command: Option<&str>) {
        match role {
            Role::Change => {
                self.capturing = command.map(str::to_string);
                self.swap = None;
                self.note = None;
            }
            Role::Reset => {
                if let Some(id) = command.filter(|id| self.person.choices.contains_key(*id)) {
                    self.note = None;
                    let next = KeyRules::reset(&self.commands, &self.person.choices, id);
                    self.set(next);
                }
            }
            Role::Swap => {
                if let Some((id, key, _)) = self.swap.clone() {
                    self.take(&id, &key);
                }
            }
            Role::Cancel => self.swap = None,
            Role::ResetAll => {
                if !self.person.choices.is_empty() {
                    self.note = None;
                    self.swap = None;
                    self.set(Default::default());
                }
            }
            Role::Close => {
                self.capturing = None;
                self.swap = None;
                self.done = true;
            }
        }
    }

    /// A key for the window: while it waits for a command's new key, the key
    /// becomes that (or is refused, saying why); Escape stops waiting, or
    /// else closes it. True when it took the key.
    pub fn key_pressed(&mut self, key: KeyInput) -> bool {
        let escape = key.key == Key::Escape && key.modifiers == Modifiers::NONE;
        let Some(id) = self.capturing.clone() else {
            if !escape {
                return false;
            }
            self.press(Role::Close, None);
            return true;
        };
        if escape {
            self.capturing = None;
            return true;
        }
        let Some(written) = written(key) else { return true };
        if let Some(why) = KeyRules::refused(&written) {
            self.note = Some(refusal(why, &written));
            return true;
        }
        self.capturing = None;
        let (_, other) = KeyRules::assign(&self.commands, &self.person.choices, &id, &written);
        match other {
            Some(other) => self.swap = Some((id, written, other)),
            None => self.take(&id, &written),
        }
        true
    }

    fn enabled(&self, role: Role, command: Option<&str>) -> bool {
        match role {
            Role::Reset => command.is_some_and(|c| self.person.choices.contains_key(c)),
            Role::ResetAll => !self.person.choices.is_empty(),
            _ => true,
        }
    }

    fn caption(role: Role) -> String {
        match role {
            Role::Change => locale::translate("keysChange", "Change"),
            Role::Reset => locale::translate("keysReset", "Reset"),
            Role::Swap => locale::translate("keysSwap", "Swap"),
            Role::Cancel => locale::translate("cancel", "Cancel"),
            Role::ResetAll => locale::translate("keysResetAll", "Reset all"),
            Role::Close => locale::translate("keysClose", "Close"),
        }
    }
}

/// The keys window: its places, its commands and their keys, and the buttons that change them.
pub struct KeysWindow {
    node: Node,
    state: Rc<RefCell<KeysState>>,
    root: bool,
    panel: Rect,
    scroll: f64,
    focused_once: bool,
}

/// Its title, for its window's title bar too.
pub fn title() -> String {
    locale::translate("keysTitle", "Keys")
}

impl KeysWindow {
    /// The keys window of `layers` with the person's keys `person`, which
    /// `keep` keeps as they change; `root` when it is a screen's root.
    pub fn new(layers: Vec<KeyLayerInfo>, person: PersonKeys, keep: KeepKeys, root: bool) -> KeysWindow {
        let commands = layers.iter().flat_map(|l| l.commands.iter().map(move |c| Command { id: c.id.clone(), layers: vec![l.id.clone()], keys: c.keys.clone() })).collect();
        let mut captions = BTreeMap::new();
        for c in layers.iter().flat_map(|l| &l.commands) {
            captions.entry(c.id.clone()).or_insert_with(|| c.caption.clone());
        }
        let state = Rc::new(RefCell::new(KeysState { layers: layers.clone(), commands, captions, person, keep, capturing: None, note: None, swap: None, done: false }));
        let mut node = Node::new();
        let window = node.id();
        for c in layers.iter().flat_map(|l| &l.commands) {
            node.add(Box::new(Push::new(state.clone(), window, root, Role::Change, Some(c.id.clone()))));
            node.add(Box::new(Push::new(state.clone(), window, root, Role::Reset, Some(c.id.clone()))));
        }
        for role in [Role::Swap, Role::Cancel, Role::ResetAll, Role::Close] {
            node.add(Box::new(Push::new(state.clone(), window, root, role, None)));
        }
        KeysWindow { node, state, root, panel: Rect::default(), scroll: 0.0, focused_once: false }
    }

    /// Whether the person closed it.
    pub fn done(&self) -> bool {
        self.state.borrow().done
    }

    /// Whether it waits for the new key of a command.
    pub fn capturing(&self) -> bool {
        self.state.borrow().capturing.is_some()
    }

    /// The person's keys as it has them now.
    pub fn person(&self) -> PersonKeys {
        self.state.borrow().person.clone()
    }

    /// A button: its role and, for a row's, its command.
    pub fn button(&self, role: Role, command: Option<&str>) -> &Push {
        self.node
            .children()
            .iter()
            .filter_map(|c| c.as_any().downcast_ref::<Push>())
            .find(|p| p.role == role && p.command.as_deref() == command)
            .expect("the keys window has that button")
    }

    /// Every text it shows now, in order: for the tests, and for a reader.
    pub fn says(&self) -> Vec<String> {
        let state = self.state.borrow();
        let mut said = vec![title()];
        if let Some(message) = state.message() {
            said.push(message);
        }
        for layer in &state.layers {
            said.push(KeysState::place_name(layer));
            for c in &layer.commands {
                said.push(state.name_of(&c.id));
                said.push(state.keys_text(c));
            }
        }
        said
    }

    /// The size it wants for a window of its own: its content's width, and its height up to 640.
    pub fn wanted(&self) -> (f64, f64) {
        (CONTENT_WIDTH + 2.0 * PAD, MOST_HIGH.min(self.content_height() + 2.0 * PAD))
    }

    /// Where its panel is: the whole place as a window of its own, a centered panel as an overlay.
    pub fn panel(&self) -> Rect {
        self.panel
    }

    fn group_height(layer: &KeyLayerInfo) -> f64 {
        let n = layer.commands.len() as f64;
        2.0 * GROUP_PAD + n * ROW_HEIGHT + (n - 1.0).max(0.0) * ROW_GAP
    }

    fn content_height(&self) -> f64 {
        let state = self.state.borrow();
        HEADING_HEIGHT + GAP + MESSAGE_HEIGHT + GAP + state.layers.iter().map(|l| TITLE_HEIGHT + ROW_GAP + Self::group_height(l) + GAP).sum::<f64>() + ROW_HEIGHT
    }

    fn view(&self) -> Rect {
        self.panel.inset(PAD)
    }

    fn max_scroll(&self) -> f64 {
        (self.content_height() - self.view().height).max(0.0)
    }

    /// A key for the window, as `KeysState::key_pressed`.
    pub fn key_pressed(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let took = self.state.borrow_mut().key_pressed(key);
        self.changed(cx);
        took
    }

    /// What it shows changed: when it is done it closes (as an overlay), and it draws again.
    fn changed(&mut self, cx: &mut Cx) {
        if self.done() && !self.root {
            cx.close(self.node.id());
        }
        cx.invalidate();
    }

    fn button_width(&self, role: Role, cx: &Cx) -> f64 {
        72.0f64.max(cx.fonts().measure(&KeysState::caption(role), "text", cx.look().size("text"), false) + 24.0)
    }
}

impl Element for KeysWindow {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn wants_tab(&self) -> bool {
        self.capturing()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.panel = if self.root {
            area
        } else {
            let width = (CONTENT_WIDTH + 2.0 * PAD).min((area.width - 32.0).max(0.0));
            let height = (self.content_height() + 2.0 * PAD).min((area.height - 32.0).max(0.0));
            Rect::new(area.x + (area.width - width) / 2.0, area.y + (area.height - height) / 2.0, width, height)
        };
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
        let view = self.view();
        let x = view.x;
        let width = CONTENT_WIDTH.min(view.width);
        let message_bottom = view.y - self.scroll + HEADING_HEIGHT + GAP + MESSAGE_HEIGHT;
        let swap_width = self.button_width(Role::Swap, cx);
        let cancel_width = self.button_width(Role::Cancel, cx);
        let change_width = self.button_width(Role::Change, cx);
        let reset_width = self.button_width(Role::Reset, cx);
        let close_width = self.button_width(Role::Close, cx);
        let all_width = self.button_width(Role::ResetAll, cx);
        let mut places: Vec<Rect> = Vec::new();
        let mut y = message_bottom + GAP;
        let layers = self.state.borrow().layers.clone();
        for layer in &layers {
            y += TITLE_HEIGHT + ROW_GAP + GROUP_PAD;
            for _ in &layer.commands {
                let right = x + width - GROUP_PAD;
                places.push(Rect::new(right - reset_width - COLUMN_GAP - change_width, y, change_width, ROW_HEIGHT));
                places.push(Rect::new(right - reset_width, y, reset_width, ROW_HEIGHT));
                y += ROW_HEIGHT + ROW_GAP;
            }
            if !layer.commands.is_empty() {
                y -= ROW_GAP;
            }
            y += GROUP_PAD + GAP;
        }
        places.push(Rect::new(x, message_bottom - ROW_HEIGHT, swap_width, ROW_HEIGHT));
        places.push(Rect::new(x + swap_width + COLUMN_GAP, message_bottom - ROW_HEIGHT, cancel_width, ROW_HEIGHT));
        places.push(Rect::new(x + width - close_width - COLUMN_GAP - all_width, y, all_width, ROW_HEIGHT));
        places.push(Rect::new(x + width - close_width, y, close_width, ROW_HEIGHT));
        let inside = if self.root { self.panel } else { self.panel.inset(1.0) };
        for (child, place) in self.node.children_mut().iter_mut().zip(places) {
            if let Some(push) = child.as_any_mut().downcast_mut::<Push>() {
                push.in_view = place.y >= inside.y && place.bottom() <= inside.bottom();
            }
            child.arrange(place, cx);
        }
        if !self.focused_once {
            self.focused_once = true;
            let inside_focus = cx.focused().is_some_and(|f| crate::element::find(self, f).is_some());
            if !inside_focus {
                cx.focus(Some(self.node.id()));
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let radius = look.radius("panel");
        if !self.root {
            // The window under it is dimmed: it takes nothing while the keys window is open.
            crate::dialogs::backdrop(p, self.node.bounds, self.panel);
        }
        let state = self.state.borrow();
        let view = self.view();
        let x = view.x;
        let width = CONTENT_WIDTH.min(view.width);
        let size = look.size("text");
        let _clip = p.clip(if self.root { self.panel } else { self.panel.inset(look.border()) });
        let mut y = view.y - self.scroll;
        p.text(&title(), Rect::new(x, y, width, HEADING_HEIGHT), look.color("text"), look.size("heading"), "heading", true, TextAlign::Left);
        y += HEADING_HEIGHT + GAP;
        if let Some(message) = state.message() {
            let room = if state.swap.is_none() { MESSAGE_HEIGHT } else { MESSAGE_HEIGHT - ROW_HEIGHT - 4.0 };
            for (i, line) in wrap(&message, width, p.fonts, size).iter().enumerate() {
                if (i as f64 + 1.0) * LINE > room {
                    break;
                }
                p.text(line, Rect::new(x, y + i as f64 * LINE, width, LINE), look.color("text"), size, "text", false, TextAlign::Left);
            }
        }
        y += MESSAGE_HEIGHT + GAP;
        let mut index = 0;
        for layer in &state.layers {
            p.text(&KeysState::place_name(layer), Rect::new(x, y, width, TITLE_HEIGHT), look.color("text"), size, "text", true, TextAlign::Left);
            y += TITLE_HEIGHT + ROW_GAP;
            let group = Rect::new(x, y, width, Self::group_height(layer));
            p.stroke(group, look.color("border"), look.border(), radius);
            let mut row_y = y + GROUP_PAD;
            for c in &layer.commands {
                let change = self.node.children()[index * 2].node().bounds;
                index += 1;
                let keys_x = change.x - COLUMN_GAP - KEYS_WIDTH;
                let name_x = x + GROUP_PAD;
                let capturing = state.capturing.as_deref() == Some(c.id.as_str());
                p.text(&state.name_of(&c.id), Rect::new(name_x, row_y, (keys_x - COLUMN_GAP - name_x).max(0.0), ROW_HEIGHT), look.color("text"), size, "text", capturing, TextAlign::Left);
                let tone = if state.keys_of(c).is_empty() { "muted" } else { "text" };
                p.text(&state.keys_text(c), Rect::new(keys_x, row_y, KEYS_WIDTH, ROW_HEIGHT), look.color(tone), size, "text", false, TextAlign::Left);
                row_y += ROW_HEIGHT + ROW_GAP;
            }
            y += group.height + GAP;
        }
        drop(state);
        crate::element::draw_children(&self.node, p);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.key_pressed(key, cx)
    }

    /// As the content of a dialog on top: a key it waits for comes before the button with the focus.
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.capturing() || (key.key == Key::Escape && key.modifiers == Modifiers::NONE) {
            return self.key_pressed(key, cx);
        }
        false
    }

    /// As a screen's root, the same.
    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.dialog_key(key, cx)
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let most = self.max_scroll();
        if most <= 0.0 {
            return false;
        }
        self.scroll = (self.scroll + dy).clamp(0.0, most);
        cx.invalidate();
        true
    }
}

/// A button of the keys window: Enter or Space presses it; any other key goes to the window.
pub struct Push {
    node: Node,
    state: Rc<RefCell<KeysState>>,
    window: ElementId,
    root: bool,
    /// What it does.
    pub role: Role,
    /// The command of its row; none for the window's own buttons.
    pub command: Option<String>,
    in_view: bool,
}

impl Push {
    fn new(state: Rc<RefCell<KeysState>>, window: ElementId, root: bool, role: Role, command: Option<String>) -> Push {
        Push { node: Node::new(), state, window, root, role, command, in_view: true }
    }

    pub fn caption(&self) -> String {
        KeysState::caption(self.role)
    }

    pub fn enabled(&self) -> bool {
        self.state.borrow().enabled(self.role, self.command.as_deref())
    }

    fn after(&self, cx: &mut Cx) {
        if self.state.borrow().done && !self.root {
            cx.close(self.window);
        }
        // A button that no longer shows gives the focus back to the window.
        if matches!(self.role, Role::Swap | Role::Cancel) || !self.enabled() {
            cx.focus(Some(self.window));
        }
        cx.invalidate();
    }
}

impl Element for Push {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        let swapping = self.state.borrow().swap.is_some();
        let offered = !matches!(self.role, Role::Swap | Role::Cancel) || swapping;
        offered && self.in_view
    }

    fn focusable(&self) -> bool {
        self.enabled()
    }

    fn wants_tab(&self) -> bool {
        self.state.borrow().capturing.is_some()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.enabled() {
            self.state.borrow_mut().press(self.role, self.command.as_deref());
            self.after(cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let capturing = self.state.borrow().capturing.is_some();
        if !capturing && key.modifiers == Modifiers::NONE && matches!(key.key, Key::Enter | Key::Space) {
            self.on_click(cx);
            return true;
        }
        let took = self.state.borrow_mut().key_pressed(key);
        self.after(cx);
        took
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let enabled = self.enabled();
        let _dimmed = p.dimmed(enabled);
        let state = if enabled && self.node.pressed && self.node.hovered {
            Some("pressed")
        } else if enabled && self.node.hovered {
            Some("hover")
        } else {
            None
        };
        let values = look.part("button", state);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let bounds = self.node.bounds;
        p.fill(bounds, values.background.unwrap_or_else(|| look.color("raised")), radius);
        p.stroke(bounds, values.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
        if self.node.focused {
            p.focus_ring(bounds.inset(-2.0), radius + 2.0);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        p.text(&self.caption(), bounds.inset(look.padding()), values.text.unwrap_or_else(|| look.color("text")), size, "text", values.bold(), TextAlign::Center);
    }
}

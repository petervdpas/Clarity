//! The tags part, the same as on every stack (conformance/list-fields.json):
//! a list field's items as chips, each with a button that removes it, and a
//! place to add one after them: a box to type text, a number or a day in
//! (taken on Enter or when the box is left), or for a list of an enum a box
//! that opens the values the list has not. A typed item is trimmed; empty
//! text and an item the list has add nothing, and text that is no value of
//! the list's kind stays as typed and is marked (the runtime's rule,
//! `clarity_runtime::list_fields::added`). Every change writes the record a
//! new list.
//!
//! The kind of the items comes from the type of the list (`list<integer>`,
//! `list<Genre>`) as the host gives it. A day is typed as `yyyy-mm-dd`.
//!
//! The part is one element: it draws its chips and its place to add, holds
//! the box's text editor and hands it what the person does there.

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::list_fields::{self, Added, ItemKind};
use serde_json::Value;

use crate::element::{Cx, Element, ElementId};
use crate::geometry::Rect;
use crate::grid::ROW_HEIGHT;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::choice_list::{self, text_of, ListState, SharedList};
use crate::parts::inputs::choices;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Choice, Part, PartBase, PartHost};

const CHIP_HEIGHT: f64 = 24.0;
const SPACE: f64 = 4.0;
const EDITOR_WIDTH: f64 = 120.0;
const CHIP_PAD: f64 = 8.0;
const BUTTON_SIDE: f64 = 18.0;

/// An item of a tags part: its label in a round plate, with a button that removes it unless the list is read only.
#[derive(Debug, Clone, PartialEq)]
pub struct TagChip {
    pub label: String,
    /// Where it is, on the screen.
    pub bounds: Rect,
    /// Where its button that removes the item is; none when the list is read only.
    pub remove: Option<Rect>,
    /// What that button does, in the person's language.
    pub remove_name: Option<String>,
}

/// What a press began on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    Remove(usize),
    Adder,
    Editor,
}

pub struct TagsPart {
    base: PartBase,
    chips: Vec<TagChip>,
    /// The box a text, a number or a day is typed in; none for a list of an enum, or one that is read only.
    editor: Option<TextEditor>,
    /// Where the box that opens an enum's values is; none for other lists.
    adder: Option<Rect>,
    bad: bool,
    /// The enum's values the list has not, while they are open: the overlay's id, its state and the values.
    list: Option<(ElementId, SharedList, Vec<Choice>)>,
    pressing: Option<Target>,
    pointer: Option<(f64, f64)>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TagsPart::new(spec, host))
}

/// The kind of a list's items by the name of its type: `list<integer>` or `integer`; none for a name that is no plain kind.
fn kind_named(ty: &str) -> Option<ItemKind> {
    let item = ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')).unwrap_or(ty);
    match item {
        "text" | "string" => Some(ItemKind::Text),
        "integer" | "int" | "long" => Some(ItemKind::Integer),
        "decimal" | "money" | "number" => Some(ItemKind::Decimal),
        "date" => Some(ItemKind::Date),
        "" => None,
        _ => Some(ItemKind::Enum),
    }
}

impl TagsPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> TagsPart {
        let mut part = TagsPart { base: PartBase::new(spec, host), chips: Vec::new(), editor: None, adder: None, bad: false, list: None, pressing: None, pointer: None };
        part.sync();
        part
    }

    /// The chips, in the order of the items.
    pub fn chips(&self) -> &[TagChip] {
        &self.chips
    }

    pub fn editor(&self) -> Option<&TextEditor> {
        self.editor.as_ref()
    }

    pub fn editor_mut(&mut self) -> Option<&mut TextEditor> {
        self.editor.as_mut()
    }

    /// Where the box that opens an enum's values the list has not is; none for other lists.
    pub fn adder(&self) -> Option<Rect> {
        self.adder
    }

    /// Whether what is typed is no value of the list's kind: the box is marked, the list stays.
    pub fn bad(&self) -> bool {
        self.bad
    }

    /// The record's items.
    pub fn items(&self) -> Vec<Value> {
        match self.base.bound().and_then(|p| self.base.host.read(p)) {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        }
    }

    /// The kind of the list's items: by its type, else an enum when its values are offered, else by its items.
    pub fn item_kind(&self) -> ItemKind {
        if let Some(kind) = self.base.bound().and_then(|p| self.base.host.type_of(p)).and_then(|t| kind_named(&t)) {
            return kind;
        }
        if !self.all_choices().is_empty() {
            return ItemKind::Enum;
        }
        if self.items().iter().any(Value::is_number) {
            return ItemKind::Integer;
        }
        ItemKind::Text
    }

    /// An enum's values with their labels, as the host offers them.
    fn all_choices(&self) -> Vec<Choice> {
        choices::offered_for(self.base.host.as_ref(), self.base.bound())
    }

    /// The values of an enum the list has not, to add.
    pub fn offered(&self) -> Vec<Choice> {
        let items = self.items();
        self.all_choices().into_iter().filter(|c| !items.iter().any(|i| choices::same(i, &c.value))).collect()
    }

    fn usable(&self) -> bool {
        self.base.enabled() && !self.base.read_only() && self.base.bound().is_some()
    }

    /// An item as people read it: an enum's value by its label, the rest as written.
    fn label(&self, all: &[Choice], item: &Value) -> String {
        all.iter().find(|c| choices::same(&c.value, item)).map(|c| c.label.clone()).unwrap_or_else(|| text_of(item))
    }

    /// Its chips and its place to add follow the list and whether it may change.
    pub fn sync(&mut self) {
        let usable = self.usable();
        let all = self.all_choices();
        let remove_name = usable.then(|| locale::translate("removeTag", "Remove"));
        let labels: Vec<String> = self.items().iter().map(|i| self.label(&all, i)).collect();
        let same_labels = labels.len() == self.chips.len() && labels.iter().zip(&self.chips).all(|(l, c)| *l == c.label && c.remove_name == remove_name);
        if !same_labels {
            self.chips = labels.into_iter().map(|label| TagChip { label, bounds: Rect::default(), remove: None, remove_name: remove_name.clone() }).collect();
        }
        let enumerated = self.item_kind() == ItemKind::Enum;
        if !usable || enumerated {
            self.editor = None;
            self.bad = false;
        } else if self.editor.is_none() {
            let mut editor = TextEditor::new(false);
            editor.placeholder = Some(locale::translate("addTag", "Add"));
            self.editor = Some(editor);
        }
        if usable && enumerated {
            self.adder.get_or_insert_with(Rect::default);
        } else {
            self.adder = None;
        }
        if let Some(editor) = self.editor.as_mut() {
            editor.enabled = self.base.enabled();
            editor.marked = self.bad;
        }
    }

    /// Places its chips in lines, each as high as an input box, then the place to add.
    fn lay_out(&mut self, cx: &mut Cx) {
        let area = self.base.node.bounds;
        let size = cx.look().size("text");
        let line = CHIP_HEIGHT.max(area.height.min(ROW_HEIGHT));
        let chip_top = (line - CHIP_HEIGHT) / 2.0;
        let (mut x, mut y) = (area.x, area.y);
        for chip in self.chips.iter_mut() {
            let width = CHIP_PAD + cx.fonts().measure(&chip.label, "text", size, false) + if chip.remove_name.is_none() { CHIP_PAD } else { 2.0 + BUTTON_SIDE + 3.0 };
            if x > area.x && x + width > area.right() {
                x = area.x;
                y += line + SPACE;
            }
            chip.bounds = Rect::new(x, y + chip_top, width, CHIP_HEIGHT);
            chip.remove = chip.remove_name.as_ref().map(|_| Rect::new(chip.bounds.right() - 3.0 - BUTTON_SIDE, chip.bounds.y + (CHIP_HEIGHT - BUTTON_SIDE) / 2.0, BUTTON_SIDE, BUTTON_SIDE));
            x += width + SPACE;
        }
        if self.editor.is_some() || self.adder.is_some() {
            if x > area.x && x + EDITOR_WIDTH > area.right() {
                x = area.x;
                y += line + SPACE;
            }
            let place = Rect::new(x, y, EDITOR_WIDTH.max(area.right() - x), line);
            if let Some(editor) = self.editor.as_mut() {
                editor.place(place, cx);
            }
            if let Some(adder) = self.adder.as_mut() {
                *adder = place;
            }
        }
    }

    /// Follows the list now, and takes its places again.
    fn refresh(&mut self, cx: &mut Cx) {
        self.sync();
        if self.base.node.bounds.width > 0.0 {
            self.lay_out(cx);
        }
        cx.invalidate();
    }

    /// Takes what is typed: an item the list has not, of its kind.
    pub fn take(&mut self, cx: &mut Cx) {
        if !self.usable() {
            return;
        }
        let Some(typed) = self.editor.as_ref().map(TextEditor::text) else { return };
        let items = self.items();
        match list_fields::added(self.item_kind(), &[], &items, &typed) {
            Added::Refused => self.bad = true,
            Added::Items(list) => {
                self.bad = false;
                if let Some(editor) = self.editor.as_mut() {
                    editor.set_text("");
                }
                if list != items {
                    self.write(list);
                }
            }
        }
        self.refresh(cx);
    }

    /// Adds `item` when the list has it not: a change.
    pub fn add(&mut self, item: Value, cx: &mut Cx) {
        let mut items = self.items();
        if !self.usable() || items.iter().any(|i| choices::same(i, &item)) {
            return;
        }
        items.push(item);
        self.write(items);
        self.refresh(cx);
    }

    /// Removes the item at `at`: a change.
    pub fn remove_at(&mut self, at: usize, cx: &mut Cx) {
        let mut items = self.items();
        if !self.usable() || at >= items.len() {
            return;
        }
        items.remove(at);
        self.write(items);
        self.refresh(cx);
    }

    fn write(&mut self, items: Vec<Value>) {
        let Some(path) = self.base.bound().map(str::to_string) else { return };
        self.base.host.write(self.base.name().unwrap_or(""), &path, Value::Array(items));
        self.base.fire("change");
    }

    /// Opens the enum's values the list has not below the add box, or closes them.
    pub fn toggle_offered(&mut self, cx: &mut Cx) {
        if self.list.is_some() {
            self.close(cx);
            return;
        }
        let Some(adder) = self.adder else { return };
        let offered = self.offered();
        if !self.usable() || offered.is_empty() {
            return;
        }
        let labels = offered.iter().map(|c| c.label.clone()).collect();
        let width = adder.width.max(160.0);
        let place = choice_list::below(cx.whole(), adder, width, offered.len());
        let state: SharedList = Rc::new(RefCell::new(ListState::new(labels, Some(0))));
        let id = choice_list::open(cx, state.clone(), place, self.base.node.id());
        self.list = Some((id, state, offered));
    }

    fn close(&mut self, cx: &mut Cx) {
        if let Some((id, _, _)) = self.list.take() {
            cx.close(id);
        }
        cx.invalidate();
    }

    /// What a point on the screen is.
    fn target_at(&self, x: f64, y: f64) -> Option<Target> {
        if let Some(at) = self.chips.iter().position(|c| c.remove.is_some_and(|r| r.contains(x, y))) {
            return Some(Target::Remove(at));
        }
        if self.adder.is_some_and(|a| a.contains(x, y)) {
            return Some(Target::Adder);
        }
        if self.editor.as_ref().is_some_and(|e| e.node().bounds.contains(x, y)) {
            return Some(Target::Editor);
        }
        None
    }

    /// The editor mirrors the part's focus, and the pointer over the box.
    fn ready(&mut self) {
        let node = &self.base.node;
        let over = node.hovered && self.pointer.is_some_and(|(x, y)| self.editor.as_ref().is_some_and(|e| e.node().bounds.contains(x, y)));
        let (focused, pressed) = (node.focused, node.pressed && self.pressing == Some(Target::Editor));
        if let Some(editor) = self.editor.as_mut() {
            editor.node_mut().focused = focused;
            editor.node_mut().hovered = over;
            editor.node_mut().pressed = pressed;
            editor.marked = self.bad;
        }
    }

    /// A pointer input as the editor takes it: from its own top left.
    fn for_editor(&self, at: PointerInput) -> Option<PointerInput> {
        let b = self.editor.as_ref()?.node().bounds;
        Some(PointerInput { x: at.screen_x - b.x, y: at.screen_y - b.y, ..at })
    }

    fn draw_chip(&self, p: &Painter, chip: &TagChip) {
        let look = p.look;
        let values = look.part("tags", None);
        let b = chip.bounds;
        p.fill(b, values.background.unwrap_or_else(|| look.color("raised")), b.height / 2.0);
        p.stroke(b, values.border.unwrap_or_else(|| look.color("border")), look.border(), b.height / 2.0);
        let room = if chip.remove.is_none() { CHIP_PAD } else { BUTTON_SIDE + 5.0 };
        let text = Rect { x: b.x + CHIP_PAD, width: (b.width - CHIP_PAD - room).max(0.0), ..b };
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        p.text(&chip.label, text, values.text.unwrap_or_else(|| look.color("text")), size, &font, values.bold(), TextAlign::Left);
        if let Some(button) = chip.remove {
            let over = self.base.node.hovered && self.pointer.is_some_and(|(x, y)| button.contains(x, y));
            let pressing = self.base.node.pressed && over && matches!(self.pressing, Some(Target::Remove(_)));
            if over || pressing {
                p.fill(button, look.color(if pressing { "accent_soft" } else { "border" }), button.height / 2.0);
            }
            let (cx, cy) = (button.x + button.width / 2.0, button.y + button.height / 2.0);
            let muted = look.color("muted");
            p.line((cx - 3.5, cy - 3.5), (cx + 3.5, cy + 3.5), muted, 1.5);
            p.line((cx + 3.5, cy - 3.5), (cx - 3.5, cy + 3.5), muted, 1.5);
        }
    }

    fn draw_adder(&self, p: &Painter, b: Rect) {
        let look = p.look;
        let node = &self.base.node;
        let values = look.part("input", if node.focused { Some("focus") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        let over = node.hovered && self.pointer.is_some_and(|(x, y)| b.contains(x, y));
        p.fill(b, values.background.unwrap_or_else(|| look.color("input")), radius);
        let line = if over { values.border.unwrap_or_else(|| look.color("strong_border")) } else { values.border.unwrap_or_else(|| look.color("border")) };
        p.stroke(b, line, look.border(), radius);
        if node.focused {
            p.focus_ring(b.inset(-2.0), radius + 2.0);
        }
        let pad = look.padding() * 2.0;
        let text = Rect { x: b.x + pad, width: (b.width - 2.0 * pad - 18.0).max(0.0), ..b };
        p.text(&locale::translate("addTag", "Add"), text, look.color("muted"), look.size("text"), "text", false, TextAlign::Left);
        p.down_arrow(Rect::new(b.right() - pad - 18.0, b.y, 18.0, b.height), values.text.unwrap_or_else(|| look.color("text")));
    }
}

impl Element for TagsPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled() && (self.editor.is_some() || self.adder.is_some())
    }

    fn typing(&self) -> bool {
        self.editor.is_some()
    }

    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        if self.editor.is_none() {
            return false;
        }
        self.take(cx);
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.sync();
        self.lay_out(cx);
    }

    fn draw(&self, p: &Painter) {
        let _dimmed = p.dimmed(self.base.enabled());
        let _clip = p.clip(self.base.node.bounds);
        for chip in &self.chips {
            self.draw_chip(p, chip);
        }
        if let Some(editor) = self.editor.as_ref() {
            editor.paint(p);
        }
        if let Some(adder) = self.adder {
            self.draw_adder(p, adder);
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pressing = self.target_at(at.screen_x, at.screen_y);
        self.ready();
        if self.pressing == Some(Target::Editor) {
            if let Some(input) = self.for_editor(at) {
                if let Some(editor) = self.editor.as_mut() {
                    editor.pressed(input, cx);
                }
            }
        }
        cx.invalidate();
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let was = self.pointer.and_then(|(x, y)| self.target_at(x, y));
        self.pointer = Some((at.screen_x, at.screen_y));
        if was != self.target_at(at.screen_x, at.screen_y) {
            cx.invalidate();
        }
        if self.pressing == Some(Target::Editor) {
            self.ready();
            if let Some(input) = self.for_editor(at) {
                if let Some(editor) = self.editor.as_mut() {
                    editor.dragged(input, cx);
                }
            }
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(editor) = self.editor.as_mut() {
            editor.released();
        }
        let Some(target) = self.pressing.take() else { return };
        if !self.base.enabled() || self.target_at(at.screen_x, at.screen_y) != Some(target) {
            return;
        }
        match target {
            Target::Remove(at) => self.remove_at(at, cx),
            Target::Adder => self.toggle_offered(cx),
            Target::Editor => {}
        }
        cx.invalidate();
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.editor.as_mut() {
            editor.focus_moved(focused, cx);
        }
        if focused {
            return;
        }
        // A press in its own list takes the focus for a moment and gives it back.
        if self.list.is_some() && cx.focused().is_none() {
            return;
        }
        self.close(cx);
        if self.editor.is_some() {
            self.take(cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        if self.adder.is_some() {
            let plain = key.modifiers.is_empty();
            if let Some((_, state, _)) = self.list.as_ref() {
                match key.key {
                    Key::Down | Key::Up => {
                        state.borrow_mut().move_by(if key.key == Key::Down { 1 } else { -1 });
                        cx.invalidate();
                        return true;
                    }
                    Key::Enter | Key::Space => {
                        let at = state.borrow().highlighted;
                        let picked = at.and_then(|at| self.list.as_ref().and_then(|(_, _, offered)| offered.get(at).cloned()));
                        self.close(cx);
                        if let Some(choice) = picked {
                            self.add(choice.value, cx);
                        }
                        return true;
                    }
                    Key::Escape => {
                        self.close(cx);
                        return true;
                    }
                    _ => return false,
                }
            }
            if plain && matches!(key.key, Key::Enter | Key::Space | Key::Down) {
                self.toggle_offered(cx);
                return true;
            }
            return false;
        }
        if self.editor.is_none() {
            return false;
        }
        if key.key == Key::Enter && key.modifiers.is_empty() {
            self.take(cx);
            return true;
        }
        self.editor.as_mut().is_some_and(|e| e.key(key, cx))
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.editor.as_mut() {
            editor.typed(text, cx);
        }
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        false
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let Some((id, state, offered)) = self.list.take() else { return };
        if id != overlay {
            self.list = Some((id, state, offered));
            return;
        }
        let picked = state.borrow().picked;
        if let Some(choice) = picked.and_then(|at| offered.get(at).cloned()) {
            self.add(choice.value, cx);
        }
        cx.invalidate();
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        self.close(cx);
    }
}

impl Part for TagsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

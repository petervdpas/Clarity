//! The tags part, the same as on every stack (conformance/list-fields.json):
//! a list field's items as chips, each with a button that removes it, and a
//! place to add one after them: a box to type text, a number or a day in
//! (taken on Enter or when the box is left), or for a list of an enum a box
//! that opens the values the list has not. A typed item is trimmed; empty
//! text and an item the list has add nothing, and text that is no value of
//! the list's kind stays as typed and is marked (the runtime's rule,
//! `clarity_runtime::list_fields::added`). Every change writes the record a
//! new list. Enter in the box takes the item, and the form it is in submits.
//! A list of days has a calendar button inside the box's right edge (Down
//! in the box opens the calendar too). Each chip's remove button says what
//! it does, is a Tab stop and is pressed by Enter or Space; when it goes,
//! the focus goes on to the next chip's button, else to the place to add.
//! A tags part bound to no field keeps its own list: a named one in its part
//! state, so the list stays when the window is made again.
//!
//! The kind of the items comes from the type of the list (`list<integer>`,
//! `list<Genre>`) as the host gives it. A day is typed as `yyyy-mm-dd`.
//!
//! The part draws its chips; its remove buttons, the box, the calendar
//! button and the place to pick an enum's value are elements of their own
//! that share what the part knows (`Core`), so each takes the focus and its
//! keys by itself.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use clarity_runtime::list_fields::{self, Added, ItemKind};
use serde_json::{Map, Value};

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::grid::row_height;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::calendar::{Calendar, DayCalendar, Keyed};
use crate::parts::inputs::choice_list::{self, text_of, ListState, SharedList};
use crate::parts::inputs::choices;
use crate::parts::inputs::date::draw_calendar_sheet;
use crate::parts::inputs::formats::formats_of;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Choice, Part, PartBase, PartHost};

const CHIP_HEIGHT: f64 = 24.0;
const SPACE: f64 = 4.0;
const EDITOR_WIDTH: f64 = 120.0;
const CHIP_PAD: f64 = 8.0;
const BUTTON_SIDE: f64 = 18.0;
const CALENDAR_WIDTH: f64 = 26.0;

/// The name a part's own list goes by in its part state.
const OWN_LIST: &str = "tags";

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

/// What the part knows, shared by the part and its elements.
struct Core {
    host: Rc<dyn PartHost>,
    spec: PartSpec,
    part: ElementId,
    box_id: ElementId,
    adder_id: ElementId,
    chips: Vec<TagChip>,
    /// The box a text, a number or a day is typed in; none for a list of an enum, or one that is read only.
    editor: Option<TextEditor>,
    /// Where the box or the place to pick an enum's value is.
    place: Rect,
    /// Where the box that opens an enum's values is; none for other lists.
    adder: Option<Rect>,
    /// Where the calendar button of a list of days is; none for other lists.
    calendar_button: Option<Rect>,
    bad: bool,
    /// The enum's values the list has not, while they are open: the overlay's id, its state and the values.
    list: Option<(ElementId, SharedList, Vec<Choice>)>,
    /// The calendar of a list of days, while it is open.
    calendar: Option<(ElementId, Calendar)>,
    /// The list of a part bound to no field and named nothing.
    own_here: Vec<Value>,
    /// Where the part is.
    area: Rect,
}

impl Core {
    fn bound(&self) -> Option<&str> {
        self.spec.text("bind").filter(|b| !b.is_empty())
    }

    fn state(&self) -> Option<clarity_runtime::ui::PartState> {
        self.host.state(self.spec.name())
    }

    fn enabled(&self) -> bool {
        self.state().and_then(|s| s.enabled).unwrap_or(true)
    }

    fn read_only(&self) -> bool {
        self.spec.flag("readonly") || self.state().and_then(|s| s.readonly).unwrap_or(false)
    }

    fn usable(&self) -> bool {
        self.enabled() && !self.read_only()
    }

    fn fire(&self, event: &str) {
        for call in self.spec.on(event) {
            self.host.run(self.spec.name(), call);
        }
    }

    /// The record's items, or its own when it is bound to no field.
    fn items(&self) -> Vec<Value> {
        let held = match self.bound() {
            Some(path) => self.host.read(path),
            None if self.spec.name().is_some() => self.state().and_then(|s| s.values).and_then(|v| v.get(OWN_LIST).cloned()),
            None => return self.own_here.clone(),
        };
        match held {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        }
    }

    /// An enum's values with their labels, as the host offers them.
    fn all_choices(&self) -> Vec<Choice> {
        choices::offered_for(self.host.as_ref(), self.bound())
    }

    /// The kind of the list's items: by its type, else an enum when its values are offered, else by its items.
    fn item_kind(&self) -> ItemKind {
        if let Some(kind) = self.bound().and_then(|p| self.host.type_of(p)).and_then(|t| kind_named(&t)) {
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

    fn offered(&self) -> Vec<Choice> {
        let items = self.items();
        self.all_choices().into_iter().filter(|c| !items.iter().any(|i| choices::same(i, &c.value))).collect()
    }

    /// Its chips and its place to add follow the list and whether it may change.
    fn sync(&mut self) {
        let usable = self.usable();
        let all = self.all_choices();
        let remove_name = usable.then(|| locale::translate("removeTag", "Remove"));
        let label = |item: &Value| all.iter().find(|c| choices::same(&c.value, item)).map(|c| c.label.clone()).unwrap_or_else(|| text_of(item));
        let labels: Vec<String> = self.items().iter().map(label).collect();
        let same_labels = labels.len() == self.chips.len() && labels.iter().zip(&self.chips).all(|(l, c)| *l == c.label && c.remove_name == remove_name);
        if !same_labels {
            self.chips = labels.into_iter().map(|label| TagChip { label, bounds: Rect::default(), remove: None, remove_name: remove_name.clone() }).collect();
        }
        let kind = self.item_kind();
        let enumerated = kind == ItemKind::Enum;
        if !usable || enumerated {
            self.editor = None;
            self.bad = false;
        } else if self.editor.is_none() {
            let mut editor = TextEditor::new(false);
            // Its own placeholder, else Clarity's word for adding one.
            let own = self.spec.text("placeholder").map(|p| self.host.text(Some(p))).filter(|p| !p.is_empty());
            editor.placeholder = Some(own.unwrap_or_else(|| locale::translate("addTag", "Add")));
            self.editor = Some(editor);
        }
        let days = self.editor.is_some() && kind == ItemKind::Date;
        let (enabled, bad) = (self.enabled(), self.bad);
        if let Some(editor) = self.editor.as_mut() {
            editor.enabled = enabled;
            editor.marked = bad;
            editor.right_inset = if days { CALENDAR_WIDTH + 2.0 } else { 0.0 };
        }
        if usable && enumerated {
            self.adder.get_or_insert_with(Rect::default);
        } else {
            self.adder = None;
        }
        if days {
            self.calendar_button.get_or_insert_with(Rect::default);
        } else {
            self.calendar_button = None;
        }
    }

    /// Places its chips in lines, each as high as an input box, then the place to add.
    fn lay_out(&mut self, cx: &mut Cx) {
        let area = self.area;
        let size = cx.look().size("text");
        let line = CHIP_HEIGHT.max(area.height.min(row_height()));
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
            self.place = place;
            if let Some(editor) = self.editor.as_mut() {
                editor.place(place, cx);
            }
            if let Some(adder) = self.adder.as_mut() {
                *adder = place;
            }
            if let Some(button) = self.calendar_button.as_mut() {
                *button = Rect::new(place.right() - 2.0 - CALENDAR_WIDTH, place.y + 2.0, CALENDAR_WIDTH, (place.height - 4.0).max(0.0));
            }
        }
    }

    /// Follows the list now, and takes its places again.
    fn refresh(&mut self, cx: &mut Cx) {
        self.sync();
        if self.area.width > 0.0 {
            self.lay_out(cx);
        }
        cx.invalidate();
    }

    /// Takes what is typed: an item the list has not, of its kind.
    fn take(&mut self, cx: &mut Cx) {
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
    fn add(&mut self, item: Value, cx: &mut Cx) {
        let mut items = self.items();
        if !self.usable() || items.iter().any(|i| choices::same(i, &item)) {
            return;
        }
        items.push(item);
        self.write(items);
        self.refresh(cx);
    }

    /// Removes the item at `at`: a change.
    fn remove_at(&mut self, at: usize, cx: &mut Cx) {
        let mut items = self.items();
        if !self.usable() || at >= items.len() {
            return;
        }
        items.remove(at);
        self.write(items);
        self.refresh(cx);
    }

    /// The remove button of the chip at `at` (with the id `button`) was
    /// pressed. When it had the focus, the focus goes on to the button of
    /// the chip now in its place (the same button), else to the place to add.
    fn removed(&mut self, at: usize, button: ElementId, cx: &mut Cx) {
        let had_focus = cx.focused() == Some(button);
        self.remove_at(at, cx);
        if had_focus && at >= self.chips.len() {
            cx.focus(Some(if self.editor.is_some() { self.box_id } else { self.adder_id }));
        }
    }

    fn write(&mut self, items: Vec<Value>) {
        let name = self.spec.name().map(str::to_string);
        match (self.bound().map(str::to_string), name) {
            (Some(path), name) => self.host.write(name.as_deref().unwrap_or(""), &path, Value::Array(items)),
            (None, Some(name)) => {
                let list = Value::Array(items);
                crate::parts::change_state(&*self.host, &name, |s| {
                    s.values.get_or_insert_with(Map::new).insert(OWN_LIST.to_string(), list.clone());
                });
            }
            (None, None) => self.own_here = items,
        }
        self.fire("change");
    }

    /// Opens the enum's values the list has not below the add box, or closes them.
    fn toggle_offered(&mut self, cx: &mut Cx) {
        if self.list.is_some() {
            self.close_list(cx);
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
        let id = choice_list::open(cx, state.clone(), place, self.adder_id);
        self.list = Some((id, state, offered));
    }

    fn close_list(&mut self, cx: &mut Cx) {
        if let Some((id, _, _)) = self.list.take() {
            cx.close(id);
        }
        cx.invalidate();
    }

    /// For a list of days: opens the calendar under the box, which keeps the focus; the day picked is added.
    fn open_calendar(&mut self, cx: &mut Cx) {
        if self.calendar_button.is_none() || !self.usable() || self.calendar.is_some() {
            return;
        }
        let formats = formats_of(self.host.as_ref());
        let opened = DayCalendar::open(cx, self.place, None, &formats, Some(self.box_id), Some(self.part), Some(self.box_id));
        self.calendar = Some(opened);
    }

    fn close_calendar(&mut self, cx: &mut Cx) {
        if let Some((id, _)) = self.calendar.as_ref() {
            cx.close(*id);
        }
    }

    /// The calendar button: opens the calendar, or closes it when it is open.
    fn toggle_calendar(&mut self, cx: &mut Cx) {
        if self.calendar.is_some() {
            self.close_calendar(cx);
        } else {
            self.open_calendar(cx);
        }
    }

    /// Keys in the box of a list of days: while the calendar is open they work it; Down opens it.
    fn day_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if let Some((_, calendar)) = self.calendar.clone() {
            match calendar.keyed(key) {
                Keyed::Not => {}
                Keyed::Moved => {
                    cx.invalidate();
                    return true;
                }
                Keyed::Picked(_) | Keyed::Closed => {
                    self.close_calendar(cx);
                    return true;
                }
            }
        }
        if key.key == Key::Down && key.modifiers.is_empty() && self.calendar_button.is_some() {
            self.open_calendar(cx);
            return self.calendar.is_some();
        }
        false
    }

    /// An overlay that `opener` opened closed: the value or day picked in it, if any, is added.
    fn overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        if self.calendar.as_ref().is_some_and(|(id, _)| *id == overlay) {
            let (_, calendar) = self.calendar.take().unwrap();
            if let Some(day) = calendar.take_picked() {
                self.add(Value::String(day.iso()), cx);
            }
            cx.invalidate();
            return;
        }
        if self.list.as_ref().is_some_and(|(id, _, _)| *id == overlay) {
            let (_, state, offered) = self.list.take().unwrap();
            let picked = state.borrow().picked;
            if let Some(choice) = picked.and_then(|at| offered.get(at).cloned()) {
                self.add(choice.value, cx);
            }
            cx.invalidate();
        }
    }
}

pub struct TagsPart {
    base: PartBase,
    core: Rc<RefCell<Core>>,
    /// Where a press on the part itself began: the remove button of a chip.
    pressing: Option<usize>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TagsPart::new(spec, host))
}

impl TagsPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> TagsPart {
        let mut base = PartBase::new(spec, host);
        let (box_node, button_node, adder_node) = (Node::new(), Node::new(), Node::new());
        let core = Rc::new(RefCell::new(Core {
            host: base.host.clone(),
            spec: base.spec.clone(),
            part: base.node.id(),
            box_id: box_node.id(),
            adder_id: adder_node.id(),
            chips: Vec::new(),
            editor: None,
            place: Rect::default(),
            adder: None,
            calendar_button: None,
            bad: false,
            list: None,
            calendar: None,
            own_here: Vec::new(),
            area: Rect::default(),
        }));
        // The box stays after the chips' buttons, so Tab reaches it last.
        base.node.add(Box::new(TagBox { node: box_node, core: core.clone() }));
        base.node.add(Box::new(TagCalendarButton { node: button_node, core: core.clone() }));
        base.node.add(Box::new(TagAdder { node: adder_node, core: core.clone() }));
        core.borrow_mut().sync();
        TagsPart { base, core, pressing: None }
    }

    /// The chips, in the order of the items.
    pub fn chips(&self) -> Vec<TagChip> {
        self.core.borrow().chips.clone()
    }

    /// The box a text, a number or a day is typed in; none for a list of an enum, or one that is read only.
    pub fn editor(&self) -> Option<Ref<'_, TextEditor>> {
        Ref::filter_map(self.core.borrow(), |c| c.editor.as_ref()).ok()
    }

    pub fn editor_mut(&self) -> Option<RefMut<'_, TextEditor>> {
        RefMut::filter_map(self.core.borrow_mut(), |c| c.editor.as_mut()).ok()
    }

    /// The id of the box, to give it the focus; none when there is no box.
    pub fn box_id(&self) -> Option<ElementId> {
        let core = self.core.borrow();
        core.editor.is_some().then_some(core.box_id)
    }

    /// The id of the place that opens an enum's values; none for other lists.
    pub fn adder_id(&self) -> Option<ElementId> {
        let core = self.core.borrow();
        core.adder.is_some().then_some(core.adder_id)
    }

    /// Where the box that opens an enum's values the list has not is; none for other lists.
    pub fn adder(&self) -> Option<Rect> {
        self.core.borrow().adder
    }

    /// The word of the place to add, in the person's language.
    pub fn adder_name() -> String {
        locale::translate("addTag", "Add")
    }

    /// The id of the remove button of the chip at `at`, once it is placed; none when the chip has none.
    pub fn remove_id(&self, at: usize) -> Option<ElementId> {
        let core = self.core.borrow();
        if core.chips.get(at).is_none_or(|c| c.remove.is_none()) {
            return None;
        }
        self.base.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<TagRemove>()).find(|r| r.index == at).map(|r| r.node.id())
    }

    /// Where the calendar button of a list of days is; none for other lists.
    pub fn calendar_button(&self) -> Option<Rect> {
        self.core.borrow().calendar_button
    }

    /// The calendar of a list of days, while it is open.
    pub fn calendar(&self) -> Option<Calendar> {
        self.core.borrow().calendar.as_ref().map(|(_, c)| c.clone())
    }

    /// Whether what is typed is no value of the list's kind: the box is marked, the list stays.
    pub fn bad(&self) -> bool {
        self.core.borrow().bad
    }

    /// The record's items, or its own list when it is bound to no field.
    pub fn items(&self) -> Vec<Value> {
        self.core.borrow().items()
    }

    /// The kind of the list's items: by its type, else an enum when its values are offered, else by its items.
    pub fn item_kind(&self) -> ItemKind {
        self.core.borrow().item_kind()
    }

    /// The values of an enum the list has not, to add.
    pub fn offered(&self) -> Vec<Choice> {
        self.core.borrow().offered()
    }

    /// Its chips and its place to add follow the list and whether it may change.
    pub fn sync(&mut self) {
        self.core.borrow_mut().sync();
    }

    /// Takes what is typed: an item the list has not, of its kind.
    pub fn take(&mut self, cx: &mut Cx) {
        self.core.borrow_mut().take(cx);
    }

    /// Adds `item` when the list has it not: a change.
    pub fn add(&mut self, item: Value, cx: &mut Cx) {
        self.core.borrow_mut().add(item, cx);
    }

    /// Removes the item at `at`: a change.
    pub fn remove_at(&mut self, at: usize, cx: &mut Cx) {
        self.core.borrow_mut().remove_at(at, cx);
    }

    /// Opens the enum's values the list has not below the add box, or closes them.
    pub fn toggle_offered(&mut self, cx: &mut Cx) {
        self.core.borrow_mut().toggle_offered(cx);
    }

    fn draw_chip(p: &Painter, chip: &TagChip) {
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
    }
}

impl Element for TagsPart {
    part_element!();

    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        if self.core.borrow().editor.is_none() {
            return false;
        }
        self.take(cx);
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let count = {
            let mut core = self.core.borrow_mut();
            core.area = area;
            core.sync();
            core.lay_out(cx);
            core.chips.len()
        };
        // A remove button for each chip, before the box; those past the last chip hide.
        let have = self.base.node.children().iter().filter(|c| c.as_any().is::<TagRemove>()).count();
        for index in have..count {
            let button = TagRemove { node: Node::new(), core: self.core.clone(), index };
            self.base.node.children_mut().insert(index, Box::new(button));
        }
        for child in self.base.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        let _dimmed = p.dimmed(self.base.enabled());
        let _clip = p.clip(self.base.node.bounds);
        for chip in &self.core.borrow().chips {
            Self::draw_chip(p, chip);
        }
        crate::element::draw_children(&self.base.node, p);
    }

    /// A press on a chip's remove button before the button took its place.
    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let core = self.core.borrow();
        self.pressing = core.chips.iter().position(|c| c.remove.is_some_and(|r| r.contains(at.screen_x, at.screen_y)));
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(index) = self.pressing.take() else { return };
        let still = self.core.borrow().chips.get(index).and_then(|c| c.remove).is_some_and(|r| r.contains(at.screen_x, at.screen_y));
        if still && self.base.enabled() {
            self.remove_at(index, cx);
        }
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        false
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        self.core.borrow_mut().overlay_closed(overlay, cx);
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        let mut core = self.core.borrow_mut();
        core.close_list(cx);
        core.close_calendar(cx);
    }
}

impl Part for TagsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// A chip's small round button with a cross, which removes its item: a Tab stop that Enter and Space press, and its name as its tooltip.
pub struct TagRemove {
    node: Node,
    core: Rc<RefCell<Core>>,
    /// The chip it removes, by its place.
    index: usize,
}

impl TagRemove {
    fn area(&self) -> Option<Rect> {
        self.core.borrow().chips.get(self.index).and_then(|c| c.remove)
    }

    fn press(&mut self, cx: &mut Cx) {
        let id = self.node.id();
        let mut core = self.core.borrow_mut();
        if core.usable() {
            core.removed(self.index, id, cx);
        }
    }
}

impl Element for TagRemove {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.area().is_some()
    }

    fn focusable(&self) -> bool {
        self.area().is_some() && self.core.borrow().usable()
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.area().is_some_and(|a| a.contains(x, y))
    }

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.area().unwrap_or_default();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) || key.control() {
            return false;
        }
        self.press(cx);
        true
    }

    fn on_focus_changed(&mut self, _focused: bool, cx: &mut Cx) {
        cx.invalidate();
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        self.core.borrow().chips.get(self.index).and_then(|c| c.remove_name.clone()).filter(|n| !n.is_empty())
    }

    fn draw(&self, p: &Painter) {
        let Some(b) = self.area() else { return };
        let look = p.look;
        if self.node.hovered || self.node.pressed {
            p.fill(b, look.color(if self.node.pressed { "accent_soft" } else { "border" }), b.height / 2.0);
        }
        if self.node.focused {
            p.focus_ring(b, b.height / 2.0);
        }
        let (cx, cy) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
        let muted = look.color("muted");
        p.line((cx - 3.5, cy - 3.5), (cx + 3.5, cy + 3.5), muted, 1.5);
        p.line((cx + 3.5, cy - 3.5), (cx - 3.5, cy + 3.5), muted, 1.5);
    }
}

/// The box a tags part's items are typed in.
struct TagBox {
    node: Node,
    core: Rc<RefCell<Core>>,
}

impl TagBox {
    /// The editor mirrors the box's hover, press and focus.
    fn ready(&mut self) {
        let mut core = self.core.borrow_mut();
        let bad = core.bad;
        if let Some(editor) = core.editor.as_mut() {
            editor.mirror(&self.node);
            editor.marked = bad;
        }
    }

    /// A pointer input as the editor takes it: from its own top left.
    fn for_editor(&self, at: PointerInput) -> PointerInput {
        let b = self.core.borrow().place;
        PointerInput { x: at.screen_x - b.x, y: at.screen_y - b.y, ..at }
    }
}

impl Element for TagBox {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.core.borrow().editor.is_some()
    }

    fn focusable(&self) -> bool {
        let core = self.core.borrow();
        core.editor.is_some() && core.enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.core.borrow().place.contains(x, y)
    }

    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        self.core.borrow_mut().take(cx);
        true
    }

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.core.borrow().place;
        self.ready();
    }

    fn draw(&self, p: &Painter) {
        if let Some(editor) = self.core.borrow().editor.as_ref() {
            editor.paint(p);
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        let input = self.for_editor(at);
        if let Some(editor) = self.core.borrow_mut().editor.as_mut() {
            editor.pressed(input, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let input = self.for_editor(at);
        if let Some(editor) = self.core.borrow_mut().editor.as_mut() {
            editor.dragged(input, cx);
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        if let Some(editor) = self.core.borrow_mut().editor.as_mut() {
            editor.released();
        }
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.core.borrow_mut().editor.as_mut() {
            editor.focus_moved(focused, cx);
        }
        if focused {
            return;
        }
        let mut core = self.core.borrow_mut();
        // A press in its own calendar takes the focus for a moment and gives it back.
        if cx.focused().is_some() {
            core.close_calendar(cx);
        }
        core.take(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        let mut core = self.core.borrow_mut();
        if core.day_key(key, cx) {
            return true;
        }
        // Enter takes the item; the form it is in, or the window's default button, does the rest.
        if key.key == Key::Enter && key.modifiers.is_empty() {
            core.take(cx);
            return false;
        }
        core.editor.as_mut().is_some_and(|e| e.key(key, cx))
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.core.borrow_mut().editor.as_mut() {
            editor.typed(text, cx);
        }
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        self.core.borrow_mut().overlay_closed(overlay, cx);
    }
}

/// The calendar button inside the box of a list of days.
struct TagCalendarButton {
    node: Node,
    core: Rc<RefCell<Core>>,
}

impl Element for TagCalendarButton {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.core.borrow().calendar_button.is_some()
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.core.borrow().calendar_button.is_some_and(|b| b.contains(x, y))
    }

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.core.borrow().calendar_button.unwrap_or_default();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        let mut core = self.core.borrow_mut();
        if core.usable() {
            core.toggle_calendar(cx);
        }
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        Some(locale::translate("openCalendar", "Calendar"))
    }

    fn draw(&self, p: &Painter) {
        let Some(b) = self.core.borrow().calendar_button else { return };
        let usable = self.core.borrow().usable();
        draw_calendar_sheet(p, b, usable, self.node.hovered, self.node.pressed);
    }
}

/// A tags part's place to add one of an enum's values: a box with its word and an arrow, which opens the values the list has not.
struct TagAdder {
    node: Node,
    core: Rc<RefCell<Core>>,
}

impl Element for TagAdder {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.core.borrow().adder.is_some()
    }

    fn focusable(&self) -> bool {
        let core = self.core.borrow();
        core.adder.is_some() && core.enabled()
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.core.borrow().adder.is_some_and(|a| a.contains(x, y))
    }

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.core.borrow().adder.unwrap_or_default();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.core.borrow_mut().toggle_offered(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let mut core = self.core.borrow_mut();
        if let Some((_, state, _)) = core.list.clone() {
            match key.key {
                Key::Down | Key::Up => {
                    state.borrow_mut().move_by(if key.key == Key::Down { 1 } else { -1 });
                    cx.invalidate();
                }
                Key::Enter | Key::Space => {
                    let at = state.borrow().highlighted;
                    let picked = at.and_then(|at| core.list.as_ref().and_then(|(_, _, offered)| offered.get(at).cloned()));
                    core.close_list(cx);
                    if let Some(choice) = picked {
                        core.add(choice.value, cx);
                    }
                }
                Key::Escape => core.close_list(cx),
                _ => return false,
            }
            return true;
        }
        if key.modifiers.is_empty() && matches!(key.key, Key::Enter | Key::Space | Key::Down) {
            core.toggle_offered(cx);
            return true;
        }
        false
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        // Tab moved the focus on; a press in its own list takes the focus for a moment and gives it back.
        if !focused && cx.focused().is_some() {
            self.core.borrow_mut().close_list(cx);
        }
        cx.invalidate();
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        self.core.borrow_mut().overlay_closed(overlay, cx);
    }

    fn draw(&self, p: &Painter) {
        let Some(b) = self.core.borrow().adder else { return };
        let look = p.look;
        let values = look.part("input", if self.node.focused { Some("focus") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        p.fill(b, values.background.unwrap_or_else(|| look.color("input")), radius);
        let line = if self.node.hovered { values.border.unwrap_or_else(|| look.color("strong_border")) } else { values.border.unwrap_or_else(|| look.color("border")) };
        p.stroke(b, line, look.border(), radius);
        if self.node.focused {
            p.focus_ring(b.inset(-2.0), radius + 2.0);
        }
        let pad = look.padding() * 2.0;
        let text = Rect { x: b.x + pad, width: (b.width - 2.0 * pad - 18.0).max(0.0), ..b };
        p.text(&TagsPart::adder_name(), text, look.color("muted"), look.size("text"), "text", false, TextAlign::Left);
        p.down_arrow(Rect::new(b.right() - pad - 18.0, b.y, 18.0, b.height), values.text.unwrap_or_else(|| look.color("text")));
    }
}

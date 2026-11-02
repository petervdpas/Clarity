//! A list of records (U1c), as C# ListPart.cs: each row shows the fields of
//! `show`, in the theme's list look (hover, selected); or with `cards` each
//! row is a card in the card look, the first field its title and the others
//! lines below it in the card_line look. The person chooses a row with the
//! pointer or Up and Down; a double click or Enter opens it. Its rows scroll
//! in their own place, and it moves rows by hand with `reorder by` (DESIGN
//! 5.30). A text longer than its row ends in dots, and the rows leave room
//! for the bar while they scroll.

use std::collections::HashMap;
use std::rc::Rc;

use serde_json::Value;

use super::rows::{self, Measures, Rows, Scroller};
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::formats::{format_of, formats_of};
use crate::parts::{Choice, Part, PartBase, PartHost};

pub struct ListPart {
    base: PartBase,
    rows: Rows,
    choices: HashMap<String, Vec<Choice>>,
    pressed: Option<usize>,
    /// The sizes of the look it was last laid out in.
    look: Measures,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let mut part = ListPart { base: PartBase::new(spec, host), rows: Rows::default(), choices: HashMap::new(), pressed: None, look: Measures::default() };
    part.sync();
    Box::new(part)
}

impl ListPart {
    fn cards(&self) -> bool {
        self.base.spec.flag("cards")
    }

    fn show(&self) -> Vec<String> {
        self.base.spec.names("show")
    }

    /// The records it shows, in the order of the data.
    pub fn records(&self) -> &[Value] {
        &self.rows.records
    }

    /// The place of the chosen record in the data; none for none.
    pub fn chosen(&self) -> Option<usize> {
        self.rows.chosen(&self.base)
    }

    /// Chooses a row as code does: it shows, and no `select` runs.
    pub fn set_chosen(&mut self, place: Option<usize>) {
        self.rows.set_chosen(&self.base, place);
        if let Some(place) = self.rows.chosen(&self.base) {
            self.show_row(place);
        }
    }

    /// Where a dragged row lands now.
    pub fn drop(&self) -> Option<(usize, clarity_runtime::reorder::DropAt)> {
        self.rows.drop
    }

    /// Where its rows scroll.
    pub fn scroller(&self) -> &Scroller {
        &self.rows.scroller
    }

    /// Whether it draws its own border.
    pub fn framed(&self) -> bool {
        rows::framed(&self.base)
    }

    /// Reads the records again when the window's values changed.
    fn sync(&mut self) {
        if self.rows.read(&self.base) {
            let mut choices = HashMap::new();
            for field in self.show() {
                let offered = Rows::choices_of(&self.base, &field);
                choices.insert(field, offered);
            }
            self.choices = choices;
        }
        if let Some(place) = self.rows.chosen_by_code(&self.base) {
            self.show_row(place);
        }
    }

    /// The fields of `show` of a record as the person reads them: a choice by its label, a number in their format.
    fn lines(&self, record: &Value) -> Vec<String> {
        let host = self.base.host.as_ref();
        let formats = formats_of(host);
        self.show()
            .iter()
            .map(|f| {
                let value = rows::value_of(record, f);
                match self.choices.get(f).filter(|c| !c.is_empty()) {
                    Some(choices) => rows::shown_of(&value, choices),
                    None => {
                        let format = if value.is_number() { self.base.bound().and_then(|b| format_of(host, &rows::column_path(b, f))) } else { None };
                        rows::shown_formatted(&value, format.as_ref(), &formats)
                    }
                }
            })
            .collect()
    }

    /// What a row shows: the fields of `show`, between bars.
    pub fn text_of(&self, record: &Value) -> String {
        self.lines(record).join(" | ")
    }

    fn look(&self) -> Measures {
        self.look
    }

    fn line_height(size: f64) -> f64 {
        (size * 1.3).ceil()
    }

    /// A row's height, and the space below it.
    fn row_size(&self, look: &Measures) -> (f64, f64) {
        let pad = look.padding;
        if !self.cards() {
            return (Self::line_height(look.text) + 2.0 * (pad - 1.0) + look.border, 0.0);
        }
        let lines = self.show().len().saturating_sub(1) as f64;
        (4.0 * pad + Self::line_height(look.text) + lines * Self::line_height(look.small) + 2.0 * look.border, (pad * 1.5).round())
    }

    /// Where the rows show: inside its border and padding; a list of cards has neither.
    fn view(&self, look: &Measures) -> Rect {
        let b = self.base.node.bounds;
        if self.cards() { b } else { b.inset(if self.framed() { look.border } else { 0.0 } + look.padding) }
    }

    fn place_rows(&mut self) {
        let look = self.look();
        let (height, gap) = self.row_size(&look);
        let view = self.view(&look);
        self.rows.scroller.place(view, self.rows.records.len() as f64 * (height + gap));
    }

    /// A row narrowed by the bar while the rows scroll, so the bar covers none of it.
    fn beside(&self, row: Rect) -> Rect {
        if self.rows.scroller.max() > 0.0 { Rect { width: (row.width - Scroller::BAR).max(0.0), ..row } } else { row }
    }

    /// The box of the row of the record at `place` on the screen; none while it is scrolled out of sight.
    pub fn row_box(&self, place: usize) -> Option<Rect> {
        let (height, gap) = self.row_size(&self.look());
        if place >= self.rows.records.len() {
            return None;
        }
        self.rows.scroller.row_in_sight(place, height + gap, height).map(|r| self.beside(r))
    }

    fn row_at(&self, y: f64) -> Option<(usize, Rect)> {
        let (height, gap) = self.row_size(&self.look());
        let place = self.rows.scroller.index_at(y, height + gap, self.rows.records.len())?;
        self.row_box(place).map(|b| (place, b))
    }

    fn show_row(&mut self, place: usize) {
        self.place_rows();
        let (height, gap) = self.row_size(&self.look());
        self.rows.scroller.scroll_into(place as f64 * (height + gap), height);
    }

    fn choose(&mut self, place: usize, cx: &mut Cx) {
        if self.rows.choose(&self.base, place) {
            self.show_row(place);
        }
        cx.invalidate();
    }

    fn apply(&mut self, mv: clarity_runtime::reorder::Move, cx: &mut Cx) {
        if let Some(now) = self.rows.apply(&self.base, mv, None) {
            self.show_row(now);
        }
        cx.invalidate();
    }

    fn draw_row(&self, p: &Painter, record: &Value, row: Rect, state: Option<&str>) {
        let look = p.look;
        let shown = rows::draw_row_back(p, row, state);
        let pad = look.padding();
        let text = Rect { x: row.x + pad * 1.5, width: (row.width - pad * 3.0).max(0.0), height: row.height - look.border(), ..row };
        let size = shown.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = shown.font.clone().unwrap_or_else(|| "text".to_string());
        let fitted = p.fit(&self.text_of(record), text.width, size, &font, shown.bold());
        p.text(&fitted, text, shown.text.unwrap_or_else(|| look.color("text")), size, &font, shown.bold(), TextAlign::Left);
    }

    fn draw_card(&self, p: &Painter, record: &Value, card: Rect, state: Option<&str>) {
        let look = p.look;
        let shown = look.part("card", state);
        let radius = shown.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("panel"));
        p.fill(card, shown.background.unwrap_or_else(|| look.color("raised")), radius);
        if let Some(border) = shown.border {
            p.stroke(card, border, look.border(), radius);
        }
        let line = look.part("card_line", state);
        let pad = look.padding();
        let x = card.x + pad * 2.5;
        let width = (card.width - pad * 5.0).max(0.0);
        let mut y = card.y + 2.0 * pad + look.border();
        for (at, text) in self.lines(record).iter().enumerate() {
            if at == 0 {
                let height = Self::line_height(look.size("text"));
                let size = shown.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
                let fitted = p.fit(text, width, size, "text", shown.bold());
                p.text(&fitted, Rect::new(x, y, width, height), shown.text.unwrap_or_else(|| look.color("text")), size, "text", shown.bold(), TextAlign::Left);
                y += height;
            } else {
                let height = Self::line_height(look.size("small"));
                let size = look.size("small");
                let fitted = p.fit(text, width, size, "text", line.bold());
                let color = line.text.or(shown.text).unwrap_or_else(|| look.color("text"));
                p.text(&fitted, Rect::new(x, y, width, height), color, size, "text", line.bold(), TextAlign::Left);
                y += height;
            }
        }
    }
}

impl Element for ListPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.look = Measures::of(cx.look());
        self.base.node.bounds = area;
        self.sync();
        self.place_rows();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
        let list = look.part("list", None);
        let radius = list.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("panel"));
        let cards = self.cards();
        if !cards {
            p.fill(b, list.background.unwrap_or_else(|| look.color("input")), radius);
        }
        {
            let view = self.rows.scroller.bounds;
            let _clip = p.clip(view);
            let hover = match self.rows.pointer {
                Some((_, y)) if self.base.node.hovered && !self.rows.dragging() => self.row_at(y).map(|r| r.0),
                _ => None,
            };
            let chosen = self.rows.chosen(&self.base);
            let (height, gap) = self.row_size(&self.look);
            let (first, end) = self.rows.scroller.in_sight(height + gap, self.rows.records.len());
            for place in first..end {
                let row = self.beside(self.rows.scroller.row_on_screen(place, height + gap, height));
                let state = if Some(place) == chosen {
                    Some("selected")
                } else if Some(place) == hover {
                    Some("hover")
                } else {
                    None
                };
                let record = &self.rows.records[place];
                if cards {
                    self.draw_card(p, record, row, state);
                } else {
                    self.draw_row(p, record, row, state);
                }
                if let Some((target, at)) = self.rows.drop {
                    if target == place {
                        rows::draw_drop(p, row, at);
                    }
                }
            }
            self.rows.scroller.draw(p, self.base.node.hovered);
        }
        if self.framed() && !cards {
            p.stroke(b, list.border.unwrap_or_else(|| look.color("strong_border")), look.border(), radius);
        }
        if self.base.node.focused && !cards {
            p.stroke(b, look.color("focus"), 2.0, radius);
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.sync();
        self.pressed = None;
        if !self.base.enabled() {
            return;
        }
        if self.rows.scroller.press(at.screen_x, at.screen_y) {
            cx.invalidate();
            return;
        }
        let Some((place, _)) = self.row_at(at.screen_y) else { return };
        self.pressed = Some(place);
        self.rows.press_row(&self.base, place, at);
        if at.clicks == 2 {
            self.choose(place, cx);
            self.base.fire("open");
            self.pressed = None;
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.rows.pointer = Some((at.screen_x, at.screen_y));
        if self.rows.scroller.drag(at.screen_y) {
            cx.invalidate();
            return;
        }
        if self.base.node.pressed {
            let over = self.row_at(at.screen_y);
            self.rows.drag_to(at, over, false);
        }
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.rows.scroller.release() {
            cx.invalidate();
            return;
        }
        let was = self.pressed.take();
        let (dragged, mv) = self.rows.drop_row();
        if dragged {
            if let Some(mv) = mv {
                self.apply(mv, cx);
            }
            cx.invalidate();
            return;
        }
        let Some(press) = was else { return };
        if let Some((place, _)) = self.row_at(at.screen_y) {
            if place == press {
                self.choose(place, cx);
            }
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let moved = self.rows.scroller.wheel(dy);
        if moved {
            cx.invalidate();
        }
        moved
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.sync();
        let (was_move, mv) = self.rows.key_move(&self.base, key);
        if was_move {
            if let Some(mv) = mv {
                self.apply(mv, cx);
            }
            return true;
        }
        let count = self.rows.records.len();
        if !self.base.enabled() || count == 0 || !rows::plain(key) {
            return false;
        }
        let look = self.look();
        let (height, gap) = self.row_size(&look);
        let page = ((self.view(&look).height / (height + gap)) as usize).max(1);
        let at = self.rows.chosen(&self.base);
        if key.key == Key::Enter && at.is_some() {
            self.base.fire("open");
            return true;
        }
        let Some(place) = rows::moved_to(key.key, at, count, page) else { return false };
        self.choose(place, cx);
        self.show_row(place);
        cx.invalidate();
        true
    }
}

impl Part for ListPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}

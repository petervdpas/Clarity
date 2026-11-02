//! A dropdown's open list, or the list of what a typed dropdown or a tags
//! part offers: one line per label over the window; a press on one picks
//! it. The part that opens it keeps the list's state with it (the lines,
//! the marked one, how far it is scrolled, the one picked), so it can work
//! the list with its keys while the screen holds the list as an overlay.
//! A pick closes the list; the part hears of it in `on_overlay_closed` and
//! reads the line picked from the state.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element, ElementId, Node};
use crate::element_node;
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::theme::ThemeLook;

/// How high a line is.
pub const LINE: f64 = 28.0;

/// How many lines show at most; the rest scroll.
pub const MOST_SHOWN: usize = 8;

/// How far apart the columns of a line with cells are.
const COLUMN_GAP: f64 = 16.0;

/// What a list shows and where the person is in it.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub labels: Vec<String>,
    /// Each line's cells, shown in columns as wide as their widest cell (a
    /// search's records with their fields); none: each line is its label.
    pub cells: Option<Vec<Vec<String>>>,
    /// The line marked, which Enter picks; none for none.
    pub highlighted: Option<usize>,
    /// How far the lines are scrolled.
    pub top: f64,
    /// How high the place for the lines is.
    pub height: f64,
    /// The line the person picked, once they did.
    pub picked: Option<usize>,
}

impl ListState {
    pub fn new(labels: Vec<String>, highlighted: Option<usize>) -> ListState {
        ListState { labels, highlighted, ..ListState::default() }
    }

    pub fn count(&self) -> usize {
        self.labels.len()
    }

    fn scrollable(&self) -> f64 {
        (self.labels.len() as f64 * LINE - self.height).max(0.0)
    }

    /// Marks the line `by` lines on, and scrolls it into view.
    pub fn move_by(&mut self, by: i64) {
        let count = self.labels.len() as i64;
        if count == 0 {
            return;
        }
        let next = match self.highlighted {
            None if by > 0 => by - 1,
            None => count + by,
            Some(at) => at as i64 + by,
        };
        self.highlighted = Some(next.clamp(0, count - 1) as usize);
        self.reveal();
    }

    /// Scrolls the marked line into view.
    pub fn reveal(&mut self) {
        let Some(at) = self.highlighted else { return };
        let at = at as f64;
        if at * LINE < self.top {
            self.top = at * LINE;
        } else if (at + 1.0) * LINE > self.top + self.height {
            self.top = (at + 1.0) * LINE - self.height;
        }
        self.top = self.top.clamp(0.0, self.scrollable());
    }
}

/// The state a part and its open list share.
pub type SharedList = Rc<RefCell<ListState>>;

/// Where a list of `count` lines lies: below `under`, else above it when there is more room there.
pub fn below(whole: Rect, under: Rect, width: f64, count: usize) -> Rect {
    let height = count.clamp(1, MOST_SHOWN) as f64 * LINE + 2.0;
    let x = under.x.min(whole.width - width).max(0.0);
    let below = under.bottom() + 2.0;
    if below + height > whole.height && under.y - 2.0 - height >= 0.0 {
        return Rect::new(x, under.y - 2.0 - height, width, height);
    }
    Rect::new(x, below, width, height)
}

/// Opens a list of `state` at `place` over the window, opened from the
/// element `opener` (whose own box is no press outside), which keeps the
/// focus while the person picks. Answers the list's id.
pub fn open(cx: &mut Cx, state: SharedList, place: Rect, opener: ElementId) -> ElementId {
    {
        let mut s = state.borrow_mut();
        s.height = (place.height - 2.0).max(0.0);
        s.reveal();
    }
    let list = ChoiceList::new(state, Some(opener));
    let mut overlay = Overlay::new(Box::new(list), place);
    overlay.anchor = Some(opener);
    overlay.opener = Some(opener);
    cx.open(overlay)
}

/// A value as a line of text: nothing for none, a text as it is, anything else as written.
pub fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The list itself, as an overlay.
pub struct ChoiceList {
    node: Node,
    pub state: SharedList,
    /// The part it belongs to, which keeps the focus while the person picks.
    pub holder: Option<ElementId>,
    pressed_at: Option<usize>,
}

impl ChoiceList {
    pub fn new(state: SharedList, holder: Option<ElementId>) -> ChoiceList {
        ChoiceList { node: Node::new(), state, holder, pressed_at: None }
    }

    /// The line at a height on the screen; none for none.
    fn line_at(&self, y: f64) -> Option<usize> {
        let state = self.state.borrow();
        let at = ((y - self.node.bounds.y - 1.0 + state.top) / LINE).floor();
        (at >= 0.0 && (at as usize) < state.labels.len()).then_some(at as usize)
    }

    /// Where each cell of line `line` is drawn, on the screen; none without cells.
    pub fn cell_boxes(&self, line: usize, fonts: &Fonts, look: &ThemeLook) -> Vec<Rect> {
        boxes_of(&self.state.borrow(), self.node.bounds, line, fonts, look)
    }
}

/// Where each cell of line `line` of a list at `bounds` is drawn; none without cells.
fn boxes_of(state: &ListState, bounds: Rect, line: usize, fonts: &Fonts, look: &ThemeLook) -> Vec<Rect> {
    let Some(cells) = state.cells.as_ref() else { return Vec::new() };
    let Some(row) = cells.get(line) else { return Vec::new() };
    let size = look.part("list", None).size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
    let pad = look.padding() * 2.0;
    let count = cells.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<f64> = (0..count).map(|i| cells.iter().map(|c| c.get(i).map(|t| fonts.measure(t, "text", size, false)).unwrap_or(0.0)).fold(0.0, f64::max)).collect();
    let y = bounds.y + 1.0 + line as f64 * LINE - state.top;
    let mut x = bounds.x + 1.0 + pad;
    let mut boxes = Vec::new();
    for (i, width) in widths.iter().enumerate().take(row.len()) {
        let right = if i == count - 1 { bounds.right() - 1.0 - pad } else { x + width };
        boxes.push(Rect::new(x, y, (right.min(bounds.right() - 1.0 - pad) - x).max(0.0), LINE));
        x += width + COLUMN_GAP;
    }
    boxes
}

impl Element for ChoiceList {
    element_node!();

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.node.bounds = area;
        let mut state = self.state.borrow_mut();
        state.height = (area.height - 2.0).max(0.0);
        state.top = state.top.clamp(0.0, state.scrollable());
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(line) = self.line_at(at.screen_y) {
            let mut state = self.state.borrow_mut();
            if state.highlighted != Some(line) {
                state.highlighted = Some(line);
                cx.invalidate();
            }
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pressed_at = self.line_at(at.screen_y);
        if let Some(holder) = self.holder {
            cx.focus(Some(holder));
        }
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.state.borrow_mut().picked = Some(at);
            cx.close(self.node.id());
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let mut state = self.state.borrow_mut();
        let next = (state.top + dy).clamp(0.0, state.scrollable());
        if next == state.top {
            return false;
        }
        state.top = next;
        cx.invalidate();
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let plain = look.part("list", None);
        let hover = look.part("list", Some("hover"));
        let radius = look.radius("input");
        let b = self.node.bounds;
        p.fill(b, plain.background.unwrap_or_else(|| look.color("panel")), radius);
        {
            let _clip = p.clip(b.inset(1.0));
            let state = self.state.borrow();
            let pad = look.padding() * 2.0;
            let first = (state.top / LINE).floor().max(0.0) as usize;
            for i in first..state.labels.len() {
                if i as f64 * LINE - state.top >= b.height {
                    break;
                }
                let line = Rect::new(b.x + 1.0, b.y + 1.0 + i as f64 * LINE - state.top, b.width - 2.0, LINE);
                let marked = state.highlighted == Some(i);
                let values = if marked { &hover } else { &plain };
                if marked {
                    p.fill(line, values.background.unwrap_or_else(|| look.color("selection")), 0.0);
                }
                let color = values.text.unwrap_or_else(|| look.color("text"));
                let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
                let font = values.font.clone().unwrap_or_else(|| "text".to_string());
                let bold = values.bold();
                match state.cells.as_ref().and_then(|c| c.get(i)) {
                    Some(row) => {
                        // The first cell in the text's color, the fields after it muted.
                        for (c, place) in boxes_of(&state, b, i, p.fonts, look).iter().enumerate() {
                            let shown = p.fit(&row[c], place.width, size, &font, bold);
                            p.text(&shown, *place, if c == 0 { color } else { look.color("muted") }, size, &font, bold, TextAlign::Left);
                        }
                    }
                    None => {
                        let place = Rect { x: line.x + pad, width: (line.width - 2.0 * pad).max(0.0), ..line };
                        let shown = p.fit(&state.labels[i], place.width, size, &font, bold);
                        p.text(&shown, place, color, size, &font, bold, TextAlign::Left);
                    }
                }
            }
        }
        p.stroke(b, plain.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
    }
}

//! A panel or a form: a frame with its caption in a header (the theme's
//! header look) around a grid of its parts, as wide as it spans. A form
//! without a caption has no frame, and its grid lines up with the grid
//! around it. A foldable one folds to its header when the person presses
//! the header, or presses Enter or Space while it has the focus, as its
//! part's state keeps (conformance/folding.json). Enter in an input of a
//! form runs the form's `on submit` (conformance/forms.json, `submit`).

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::input::{Key, KeyInput, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, LookNow, Page};

pub struct PanelPart {
    base: PartBase,
    folded_here: bool,
    pressed_header: bool,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let columns = container::columns_of(&spec);
    let grid = container::grid_of(&spec.children, columns, GAP, &host).owned(spec.text(crate::markup::DESIGN_ITEM));
    let folded_here = spec.flag("folded");
    let mut base = PartBase::new(spec, host);
    base.node.add(Box::new(Page::new(Box::new(grid))));
    let mut panel = PanelPart { base, folded_here, pressed_header: false, look: Cell::new(LookNow::default()) };
    let folded = panel.is_folded();
    panel.page_mut().shown = !folded;
    Box::new(panel)
}

impl PanelPart {
    fn page(&self) -> &Page {
        self.base.node.children()[0].as_any().downcast_ref::<Page>().expect("a panel holds its page")
    }

    fn page_mut(&mut self) -> &mut Page {
        self.base.node.children_mut()[0].as_any_mut().downcast_mut::<Page>().expect("a panel holds its page")
    }

    /// Whether a list, table or tree in it shows no border of its own: every container frames what it holds.
    pub fn frames_parts(&self) -> bool {
        true
    }

    /// The grid of its parts.
    pub fn inner(&self) -> &GridPanel {
        self.page().grid()
    }

    pub fn foldable(&self) -> bool {
        self.base.spec.flag("foldable")
    }

    /// A form without a caption: no frame and no header.
    pub fn bare(&self) -> bool {
        self.base.spec.kind == "form" && self.base.spec.text("caption").is_none_or(str::is_empty) && !self.foldable()
    }

    /// Folded, as the person or a handler left it in its part's state, else as designed.
    pub fn is_folded(&self) -> bool {
        self.foldable() && self.base.state().and_then(|s| s.folded).unwrap_or(self.folded_here)
    }

    /// Folds it, or unfolds it, as a press on its header does.
    pub fn toggle_fold(&mut self, cx: &mut Cx) {
        if !self.foldable() || !self.base.enabled() {
            return;
        }
        let folded = !self.is_folded();
        match self.base.name() {
            Some(name) if self.base.state().is_some() => crate::parts::change_state(&*self.base.host, name, |s| s.folded = Some(folded)),
            _ => self.folded_here = folded,
        }
        // Folded, what it holds takes no pointer from now on, not only once it is laid out again.
        self.page_mut().shown = !folded;
        cx.invalidate();
    }

    fn header(&self) -> f64 {
        if self.bare() { 0.0 } else { self.look.get().header() }
    }

    /// Its caption: as a handler set it, else as designed.
    fn caption(&self) -> String {
        self.base.caption().unwrap_or_default()
    }
}

impl Element for PanelPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.foldable() && self.base.enabled()
    }

    /// The room it needs beyond its rows: its header and padding, when its grid needs more than its rows give.
    fn around(&self) -> f64 {
        let frame = if self.bare() { 0.0 } else { self.header() + 2.0 * self.look.get().padding };
        container::around_for(&self.base.spec, frame, container::least_of(self.inner()))
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.look.set(LookNow::of(cx.look()));
        let folded = self.is_folded();
        let header = self.header();
        self.page_mut().shown = !folded;
        self.base.node.bounds = if folded { Rect { height: area.height.min(header), ..area } } else { area };
        let inner = if self.bare() {
            area
        } else {
            let pad = self.look.get().padding;
            Rect::new(area.x, area.y + header + pad, area.width, (area.height - header - 2.0 * pad).max(0.0))
        };
        element::arrange_safely(self.base.node.children_mut()[0].as_mut(), inner, cx);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.pressed_header = at.y < self.header();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.pressed_header {
            self.toggle_fold(cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !self.foldable() || !key.modifiers.is_empty() || !matches!(key.key, Key::Enter | Key::Space) {
            return false;
        }
        self.toggle_fold(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        if !self.bare() {
            let look = p.look;
            let radius = look.radius("panel");
            let frame = container::frame_of(bounds);
            let header = look.part("header", None);
            let header_box = Rect { height: container::header_height(look), ..frame };
            {
                let _clip = p.clip(header_box);
                p.fill(frame.inset(look.border()), header.background.unwrap_or_else(|| look.color("raised")), radius);
            }
            p.stroke(frame, look.color("strong_border"), look.border(), radius);
            if self.base.node.focused {
                p.stroke(header_box.inset(2.0), look.color("focus"), 2.0, radius);
            }
            let color = header.text.unwrap_or_else(|| look.color("muted"));
            let size = header.size.map(|s| s as f64).unwrap_or_else(|| look.size("small"));
            let font = header.font.clone().unwrap_or_else(|| "text".to_string());
            let mut x = frame.x + 2.0 * look.padding();
            if self.foldable() {
                container::chevron(p, Rect::new(x, header_box.y, 10.0, header_box.height), color, !self.is_folded());
                x += 16.0;
            }
            let text = Rect::new(x, header_box.y, (frame.right() - x - 2.0 * look.padding()).max(0.0), header_box.height);
            p.text(&p.fit(&self.caption(), text.width, size, &font, header.bold()), text, color, size, &font, header.bold(), TextAlign::Left);
        }
        let _framed = container::framing();
        element::draw_children(&self.base.node, p);
    }
}

impl Part for PanelPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A folded part grows no more: it is its header.
    fn grows(&self) -> bool {
        !self.is_folded() && container::grid_grows(self.inner())
    }
}

/// Enter in an input: the form it is in runs its `on submit`, or the
/// sign-in part it is in signs in with the input's strategy. False when it
/// is in neither, or the form has no submit. `from` is the input, in the
/// tree of `root`; the nearest form around it is the one.
pub fn submit(root: &dyn Element, from: ElementId) -> bool {
    let Some(ids) = element::chain(root, from) else { return false };
    for id in ids.iter().rev().skip(1) {
        let Some(part) = element::find(root, *id).and_then(|e| e.as_part()) else { continue };
        let base = part.base();
        // In a sign-in part, Enter signs in with the strategy of the input.
        if base.spec.kind == "signin" {
            let field = element::find(root, from).and_then(|e| e.as_part()).and_then(|p| p.base().name().map(str::to_string));
            return crate::parts::signin::enter(part, field.as_deref());
        }
        if base.spec.kind == "form" {
            if base.spec.on("submit").next().is_none() {
                return false;
            }
            base.fire("submit");
            return true;
        }
    }
    false
}

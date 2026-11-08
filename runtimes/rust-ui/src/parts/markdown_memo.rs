//! A memo of Markdown (`memo ... markdown`): an editor with a preview. On its
//! top a switch, Write and Preview (Clarity's words `markdownWrite` and
//! `markdownPreview`); under it the memo, its Markdown colored as it is
//! written with Ctrl+B and Ctrl+I for bold and italic, or the markdown part's
//! page of the same text. Each of the two is a part of its own, bound to the
//! same field, so each keeps its own keys, scrolling and links.

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{draw_children, Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::PointerInput;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::markdown;
use crate::parts::text_box::TextBoxPart;
use crate::parts::{Part, PartBase, PartHost};

/// How high the switch on its top is.
pub const SWITCH_HEIGHT: f64 = 30.0;

/// The room a caption of the switch has on each side.
const CAPTION_PAD: f64 = 12.0;

/// One of the two: shown while the memo shows what it is for.
struct Shown {
    node: Node,
    preview: Rc<Cell<bool>>,
    /// Whether it is the preview (else the editor).
    is_preview: bool,
}

impl Element for Shown {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.preview.get() == self.is_preview
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            crate::element::arrange_safely(child.as_mut(), area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        draw_children(&self.node, p);
    }
}

pub struct MarkdownMemoPart {
    base: PartBase,
    /// Whether it shows the preview, else the editor.
    preview: Rc<Cell<bool>>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(MarkdownMemoPart::new(spec, host))
}

impl MarkdownMemoPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> MarkdownMemoPart {
        let preview = Rc::new(Cell::new(false));
        let editor = TextBoxPart::new(spec.clone(), host.clone());
        let page = markdown::make(spec.clone(), host.clone());
        let mut base = PartBase::new(spec, host);
        for (child, is_preview) in [(Box::new(editor) as Box<dyn Element>, false), (page as Box<dyn Element>, true)] {
            let mut node = Node::new();
            node.add(child);
            base.node.add(Box::new(Shown { node, preview: preview.clone(), is_preview }));
        }
        MarkdownMemoPart { base, preview }
    }

    /// Whether it shows the preview.
    pub fn previewing(&self) -> bool {
        self.preview.get()
    }

    /// Where the person writes.
    pub fn editor(&self) -> &TextBoxPart {
        self.base.node.children()[0].node().children()[0].as_any().downcast_ref::<TextBoxPart>().expect("the editor")
    }

    fn editor_mut(&mut self) -> &mut TextBoxPart {
        self.base.node.children_mut()[0].node_mut().children_mut()[0].as_any_mut().downcast_mut::<TextBoxPart>().expect("the editor")
    }

    /// The page that shows the text as it reads.
    pub fn page(&self) -> &markdown::MarkdownPart {
        self.base.node.children()[1].node().children()[0].as_any().downcast_ref::<markdown::MarkdownPart>().expect("the page")
    }

    /// The switch's two captions and where each lies on the screen: Write, then Preview.
    pub fn switch(&self, cx_fonts: &crate::fonts::Fonts, size: f64) -> [(String, Rect); 2] {
        let b = self.base.node.bounds;
        let write = locale::translate("markdownWrite", "Write");
        let preview = locale::translate("markdownPreview", "Preview");
        let wide = |text: &str| cx_fonts.measure(text, "text", size, false) + 2.0 * CAPTION_PAD;
        let first = Rect::new(b.x, b.y, wide(&write), SWITCH_HEIGHT);
        let second = Rect::new(first.right(), b.y, wide(&preview), SWITCH_HEIGHT);
        [(write, first), (preview, second)]
    }

    /// Shows the preview or the editor; what was typed is taken first, so the page shows it.
    pub fn show_preview(&mut self, on: bool, cx: &mut Cx) {
        if self.preview.get() == on {
            return;
        }
        if on {
            self.editor_mut().take();
        }
        self.preview.set(on);
        cx.invalidate();
    }
}

impl Element for MarkdownMemoPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let below = Rect::new(area.x, area.y + SWITCH_HEIGHT, area.width, (area.height - SWITCH_HEIGHT).max(0.0));
        for child in self.base.node.children_mut() {
            let place = if child.is_shown() { below } else { Rect::new(below.x, below.y, 0.0, 0.0) };
            crate::element::arrange_safely(child.as_mut(), place, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let size = look.size("text");
        let _dim = p.dimmed(self.base.enabled());
        for (index, (caption, place)) in self.switch(p.fonts, size).iter().enumerate() {
            let on = (index == 1) == self.preview.get();
            let color = if on { look.color("text") } else { look.color("muted") };
            p.text(caption, *place, color, size, "text", on, TextAlign::Center);
            if on {
                p.fill(Rect::new(place.x + CAPTION_PAD / 2.0, place.bottom() - 3.0, place.width - CAPTION_PAD, 2.0), look.color("accent"), 1.0);
            }
        }
        draw_children(&self.base.node, p);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if !self.base.enabled() {
            return;
        }
        let b = self.base.node.bounds;
        let size = cx.look().size("text");
        let switch = self.switch(cx.fonts(), size);
        let (x, y) = (b.x + at.x, b.y + at.y);
        if let Some(index) = switch.iter().position(|(_, place)| place.contains(x, y)) {
            self.show_preview(index == 1, cx);
        }
    }
}

impl Part for MarkdownMemoPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}

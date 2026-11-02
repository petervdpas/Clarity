//! A dialog shown over the window that opened it (DESIGN 7.9), where the
//! host opens no desktop window of its own for it: the window under it
//! dimmed, the dialog's title, and its window in a panel sized to the
//! dialog, as its own desktop window would be. Its keys are its own: its
//! shortcuts, Enter for its default button and Escape for its cancel
//! button (without one, Escape closes it as its close does); Ctrl+Z and Ctrl+Y take back and do again the dialog's own
//! edits. In another language its window is made again.

use std::rc::Rc;

use crate::dialogs::backdrop;
use crate::element::{self, Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::element::GridPanel;
use crate::fonts::Fonts;
use crate::grid::{GAP, ROW_HEIGHT};
use crate::markup::{PartSpec, STANDARD_COLUMNS};
use crate::theme::ThemeLook;
use crate::input::KeyInput;
use crate::painter::{Painter, TextAlign};
use crate::window::view::{WindowView, CONTENT_PADDING};
use crate::window::ProgramHost;

/// The height of the dialog's title above its window.
pub const TITLE_HEIGHT: f64 = 40.0;

/// The width a dialog that names no size opens at, at least.
pub const DIALOG_WIDTH: f64 = 560.0;

/// How wide a dialog grows for its captions, at most.
pub const MAX_DIALOG_WIDTH: f64 = 1000.0;

/// Makes the dialog's window anew, as in another language.
pub type MakeView = Box<dyn Fn() -> WindowView>;

pub struct DialogOverlay {
    node: Node,
    /// The dialog's title, as shown.
    pub title: String,
    /// The size its model names; none for its rows' own.
    size: Option<(f64, f64)>,
    make: MakeView,
    language: String,
    panel: Rect,
    /// The dialog window's host, which takes back its edits; none for a dialog without a program.
    host: Option<Rc<ProgramHost>>,
}

/// The size of a dialog's window: as its model says, else as wide as its
/// parts' captions need, 560 to 1000, and as high as its rows with the
/// window's padding, at most 720. Measured in Clarity's own look; a dialog
/// on a screen measures in the screen's (`dialog_size_in`).
pub fn dialog_size(view: &WindowView, size: Option<(f64, f64)>) -> (f64, f64) {
    let look = ThemeLook::clarity(false);
    let fonts = Fonts::new(look.clone());
    dialog_size_in(view, size, &fonts, &look)
}

/// The size of a dialog's window, its captions measured in `fonts` and `look`.
pub fn dialog_size_in(view: &WindowView, size: Option<(f64, f64)>, fonts: &Fonts, look: &ThemeLook) -> (f64, f64) {
    if let Some(size) = size {
        return size;
    }
    let width = content_width(view, fonts, look).clamp(DIALOG_WIDTH, MAX_DIALOG_WIDTH);
    let rows = view.content_height(width);
    (width, rows.clamp(ROW_HEIGHT + 2.0 * CONTENT_PADDING, 720.0))
}

/// The width a window's grid needs so each part's caption fits in the
/// columns it spans, with the padding around. The parts in a container
/// count too (a form's labels, the parts of every tab, open or not): a
/// container's grid has a column for each it spans, so its columns are as
/// wide as the window's.
fn content_width(view: &WindowView, fonts: &Fonts, look: &ThemeLook) -> f64 {
    fn all<'a>(specs: &mut dyn Iterator<Item = &'a PartSpec>, out: &mut Vec<&'a PartSpec>) {
        for spec in specs {
            out.push(spec);
            all(&mut spec.children.iter(), out);
            all(&mut spec.pages.iter().flat_map(|p| p.children.iter()), out);
        }
    }
    let parts = view.parts();
    let mut specs = Vec::new();
    all(&mut parts.iter().map(|p| &p.base().spec), &mut specs);
    let columns = element::in_order(view).into_iter().find_map(|e| e.as_any().downcast_ref::<GridPanel>()).map(|g| g.columns).unwrap_or(STANDARD_COLUMNS) as f64;
    let host = view.host();
    let mut widest: f64 = 0.0;
    for spec in specs {
        let Some(caption) = spec.text("caption").filter(|c| !c.is_empty()) else { continue };
        let needs = fonts.measure(&host.text(Some(caption)), "text", look.size("text"), false) + 4.0 * look.padding() + 8.0;
        let span = spec.number("width").unwrap_or(1.0).max(1.0).floor();
        let column = (needs - (span - 1.0) * GAP) / span;
        widest = widest.max(column * columns + (columns - 1.0) * GAP);
    }
    widest + 2.0 * CONTENT_PADDING
}

impl DialogOverlay {
    pub fn new(title: String, size: Option<(f64, f64)>, make: MakeView) -> DialogOverlay {
        let mut node = Node::new();
        node.add(Box::new(make()));
        DialogOverlay { node, title, size, make, language: crate::choices::language(), panel: Rect::default(), host: None }
    }

    /// The dialog with its window's host, through which Ctrl+Z and Ctrl+Y take back and do again its edits.
    pub fn with_host(mut self, host: Rc<ProgramHost>) -> DialogOverlay {
        self.host = Some(host);
        self
    }

    /// The dialog's window.
    pub fn view(&self) -> &WindowView {
        self.node.children()[0].as_any().downcast_ref::<WindowView>().expect("a dialog holds its window")
    }

    fn view_mut(&mut self) -> &mut WindowView {
        self.node.children_mut()[0].as_any_mut().downcast_mut::<WindowView>().expect("a dialog holds its window")
    }

    /// The first part that takes the focus, which has it as the dialog opens.
    pub fn first(&self) -> Option<ElementId> {
        element::focus_takers(self.view()).first().copied()
    }

    /// Where the dialog is: its title and its window, in the middle of the window under it.
    pub fn panel(&self) -> Rect {
        self.panel
    }
}

impl Element for DialogOverlay {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        // The language changed: its window is made again, so its texts show in it.
        let language = crate::choices::language();
        if language != self.language {
            self.language = language;
            let mut old = self.node.children_mut().remove(0);
            element::detach_all(old.as_mut(), cx);
            self.node.children_mut().insert(0, Box::new((self.make)()));
        }
        let (width, height) = dialog_size_in(self.view(), self.size, cx.fonts(), cx.look());
        let width = width.min((area.width - 32.0).max(160.0));
        let height = (height + TITLE_HEIGHT).min((area.height - 32.0).max(TITLE_HEIGHT + ROW_HEIGHT));
        self.panel = Rect::new(area.x + (area.width - width) / 2.0, area.y + (area.height - height) / 2.0, width, height);
        let inside = Rect::new(self.panel.x, self.panel.y + TITLE_HEIGHT, self.panel.width, self.panel.height - TITLE_HEIGHT);
        self.view_mut().arrange(inside, cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        backdrop(p, self.node.bounds, self.panel);
        let title = Rect::new(self.panel.x + CONTENT_PADDING, self.panel.y, self.panel.width - 2.0 * CONTENT_PADDING, TITLE_HEIGHT);
        p.text(&self.title, title, look.color("text"), look.size("heading"), "heading", true, TextAlign::Left);
        p.fill(Rect::new(self.panel.x, self.panel.y + TITLE_HEIGHT - 1.0, self.panel.width, 1.0), look.color("border"), 0.0);
        element::draw_children(&self.node, p);
    }

    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.view_mut().key(key, cx) {
            return true;
        }
        let host = self.host.clone();
        if crate::window::undo_key(self, host.as_ref(), key, cx) {
            return true;
        }
        // Escape that nothing in it took closes it, as its close does (DESIGN 5.14).
        if key.key == crate::input::Key::Escape && key.modifiers == crate::input::Modifiers::NONE {
            cx.close(self.node.id());
            return true;
        }
        false
    }
}

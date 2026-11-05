//! A whole window of the program (DESIGN 7.9): its parts in the window's
//! grid, with the content padding every stack keeps around it, scrolling
//! when the window is smaller than its rows. Around the parts is the
//! window's frame (its menu bar, toolbar, ribbon, sidebars and footer, its
//! own or its app's). The window's own keys are here too: a part's
//! shortcut, a frame command's or one of a part's menu's items, F10 and Alt
//! alone for the menu bar, Enter for its default button (or a form's
//! submit, or a wizard's Next or Finish) and Escape for its cancel button.
//! The frame comes before the grid in Tab order, as it is docked around it.

use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel, Node};
use crate::frame::{self, Access, FrameFooter, FrameMenuBar, FrameRibbon, FrameSidebar, FrameSpec, FrameToolbar, Showing, ShownFrame, WindowFrame};
use crate::geometry::Rect;
use crate::grid::row_height;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::{CommandSpec, WindowSpec};
use crate::painter::{Painter, TextAlign};
use crate::parts::containers::container::ContainerButton;
use crate::parts::containers::slideout::{Side, SlideoutHandle, SlideoutPart, HANDLE_WIDTH};
use crate::parts::containers::steps::StepsPart;
use crate::parts::{self, PartHost};
use crate::shortcuts;
use crate::xml::XmlElement;

/// The space around the grid of a running window (clarity_spec::layout CONTENT_PADDING).
pub const CONTENT_PADDING: f64 = 12.0;

pub struct WindowView {
    node: Node,
    host: Rc<dyn PartHost>,
    own_frame: Vec<XmlElement>,
    frame: Rc<WindowFrame>,
    /// The window's name (`Main`), by which the person's keys know its commands.
    pub name: Option<String>,
    /// Why the window shows nothing: its markup could not be read.
    pub problem: Option<String>,
    /// How far the window's content is scrolled down, and how far it can be.
    pub scroll_y: f64,
    pub max_scroll_y: f64,
    /// Where the pointer took the bar's thumb, and how far the window was scrolled then.
    drag_from: Option<(f64, f64)>,
    hover_x: f64,
    /// The strips of slide-out handles at its edges, as last placed, with the side each is on.
    strips: Vec<(Rect, Side)>,
}

/// A part that may answer a key: where it is, its name and what its markup says of keys.
struct Keyed {
    id: ElementId,
    name: Option<String>,
    shortcut: Option<String>,
    button: bool,
    default: bool,
    cancel: bool,
    steps: bool,
    menu: Vec<CommandSpec>,
}

impl WindowView {
    /// The window of `spec` over `host`: each of its parts made by its kind,
    /// at its place in the grid, in the frame of its own regions.
    pub fn new(spec: &WindowSpec, host: Rc<dyn PartHost>) -> WindowView {
        // Its rows, and what follows them, are as high as its project says.
        crate::grid::set_row_height(spec.row_height);
        let mut grid = GridPanel::new(spec.columns).owned(spec.item.as_deref());
        for part in &spec.parts {
            let made = parts::make(part.clone(), host.clone());
            let place = parts::place_of(made.as_ref());
            grid.add(made, place);
        }
        let frame = WindowFrame::new(host.clone(), ShownFrame::of(None, &FrameSpec::read(&spec.frame)), spec.name.clone());
        let filling = frame.clone();
        // While a sidebar fills the window, its own parts make room.
        let body = Showing::new(Box::new(grid), move || filling.filling().is_none());
        let mut node = Node::new();
        node.add(Box::new(body));
        let mut view = WindowView { node, host, own_frame: spec.frame.clone(), frame, name: spec.name.clone(), problem: None, scroll_y: 0.0, max_scroll_y: 0.0, drag_from: None, hover_x: 0.0, strips: Vec::new() };
        view.add_regions();
        view
    }

    /// The same window in its app's frame: the regions of the app's markup
    /// (`markup::read_app` of `apps/Shop.cxaml`, see `frame::app_frame_of`),
    /// which it shows where it has none of its own; none for a dialog or a
    /// window without an app.
    pub fn with_frame(mut self, app: Option<Vec<XmlElement>>) -> WindowView {
        self.set_frame(app.as_deref(), None);
        self
    }

    /// Gives the window its app's frame after it is shown; its open menus close.
    pub fn set_app_frame(&mut self, app: Option<&[XmlElement]>, cx: &mut Cx) {
        self.set_frame(app, Some(cx));
        cx.invalidate();
    }

    fn set_frame(&mut self, app: Option<&[XmlElement]>, mut cx: Option<&mut Cx>) {
        if let Some(cx) = cx.as_deref_mut() {
            self.frame.menus().close_all(cx);
        }
        // The regions go; the body stays, as the last child.
        while self.node.children().len() > 1 {
            match cx.as_deref_mut() {
                Some(cx) => {
                    self.node.remove(0, cx);
                }
                None => {
                    self.node.children_mut().remove(0);
                }
            }
        }
        let app = app.map(FrameSpec::read);
        let shown = ShownFrame::of(app.as_ref(), &FrameSpec::read(&self.own_frame));
        let frame = WindowFrame::new(self.host.clone(), shown, self.name.clone());
        let filling = frame.clone();
        self.frame = frame;
        self.body_mut().set_shows(move || filling.filling().is_none());
        self.add_regions();
    }

    /// The frame's regions go before the body: they come first in Tab order,
    /// as they are docked around the grid; then the handles of the slide-outs,
    /// which the frame leaves alone and the window places at its edges.
    fn add_regions(&mut self) {
        let mut at = 0;
        for region in frame::regions(&self.frame) {
            self.node.children_mut().insert(at, region);
            at += 1;
        }
        let slides: Vec<_> = self.grid().node().children().iter().filter_map(|c| c.as_any().downcast_ref::<SlideoutPart>()).map(|s| s.slide().clone()).filter(|s| s.has_handle()).collect();
        for slide in slides {
            self.node.children_mut().insert(at, Box::new(SlideoutHandle::new(slide)));
            at += 1;
        }
    }

    /// The frame's regions, in the order they are drawn.
    fn regions(&self) -> &[Box<dyn Element>] {
        let children = self.node.children();
        &children[..children.len() - 1]
    }

    /// A window whose markup could not be read: it says why in its place.
    pub fn failed(problem: String, host: Rc<dyn PartHost>) -> WindowView {
        let spec = WindowSpec { frame: Vec::new(), parts: Vec::new(), name: None, columns: crate::markup::STANDARD_COLUMNS, item: None, row_height: crate::grid::row_height() };
        WindowView { problem: Some(problem), ..WindowView::new(&spec, host) }
    }

    pub fn host(&self) -> &Rc<dyn PartHost> {
        &self.host
    }

    /// The frame around the window's parts: what its regions share and the menus open over it.
    pub fn frame(&self) -> &Rc<WindowFrame> {
        &self.frame
    }

    fn body(&self) -> &Showing {
        self.node.children().last().and_then(|c| c.as_any().downcast_ref::<Showing>()).expect("a window view holds its body")
    }

    fn body_mut(&mut self) -> &mut Showing {
        self.node.children_mut().last_mut().and_then(|c| c.as_any_mut().downcast_mut::<Showing>()).expect("a window view holds its body")
    }

    fn grid(&self) -> &GridPanel {
        self.body().child().as_any().downcast_ref::<GridPanel>().expect("a window view holds its grid")
    }

    /// The window's own parts, in the order of its markup.
    pub fn parts(&self) -> Vec<&dyn parts::Part> {
        self.grid().node().children().iter().filter_map(|c| c.as_part()).collect()
    }

    fn region<T: Element>(&self) -> Option<&T> {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<T>())
    }

    fn region_mut<T: Element>(&mut self, which: impl Fn(&T) -> bool) -> Option<&mut T> {
        self.node.children_mut().iter_mut().filter_map(|c| c.as_any_mut().downcast_mut::<T>()).find(|t| which(t))
    }

    pub fn menu_bar(&self) -> Option<&FrameMenuBar> {
        self.region()
    }

    fn open_by_letter(&mut self, letter: char, cx: &mut Cx) -> bool {
        self.region_mut::<FrameMenuBar>(|_| true).is_some_and(|bar| bar.open_by_letter(letter, cx))
    }

    /// The menu bar takes the focus with its first top item marked: false when there is none, or it shows nothing.
    pub fn focus_menu_bar(&mut self, cx: &mut Cx) -> bool {
        self.region_mut::<FrameMenuBar>(|_| true).is_some_and(|bar| bar.focus_first(cx))
    }

    /// The menu bar's access letters before any other use of a key: Alt
    /// shows them; after Alt went alone the letter alone opens its menu,
    /// and any other key ends them (Escape doing nothing more). None when
    /// the key goes on as any key.
    fn access_key(&mut self, key: KeyInput, cx: &mut Cx) -> Option<bool> {
        let access = self.frame.access();
        if key.key == Key::Alt {
            if !self.menu_bar().is_some_and(|bar| bar.is_shown()) {
                return None;
            }
            let used = matches!(access, Access::Held { used: true });
            self.frame.set_access(Access::Held { used });
            cx.invalidate();
            return Some(true);
        }
        match access {
            Access::Kept => {
                self.frame.set_access(Access::Off);
                cx.invalidate();
                if key.modifiers == Modifiers::NONE {
                    if let Some(letter) = frame::rules::access_char(key.key) {
                        if self.open_by_letter(letter, cx) {
                            return Some(true);
                        }
                    }
                }
                (key.key == Key::Escape).then_some(true)
            }
            Access::Held { used: false } => {
                self.frame.set_access(Access::Held { used: true });
                None
            }
            _ => None,
        }
    }

    /// A key let go: Alt let go alone keeps the access letters for a plain
    /// letter and puts the focus on the menu bar, its first menu marked, as
    /// a desktop menu bar does; after another key it hides them.
    pub fn key_up(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Alt {
            return false;
        }
        match self.frame.access() {
            Access::Held { used: false } => {
                self.frame.set_access(Access::Kept);
                self.focus_menu_bar(cx);
            }
            Access::Held { used: true } => self.frame.set_access(Access::Off),
            _ => return false,
        }
        cx.invalidate();
        true
    }

    pub fn toolbar(&self) -> Option<&FrameToolbar> {
        self.region()
    }

    pub fn toolbar_mut(&mut self) -> Option<&mut FrameToolbar> {
        self.region_mut(|_| true)
    }

    pub fn ribbon(&self) -> Option<&FrameRibbon> {
        self.region()
    }

    pub fn ribbon_mut(&mut self) -> Option<&mut FrameRibbon> {
        self.region_mut(|_| true)
    }

    /// The sidebar on the left (`left`) or right.
    pub fn sidebar(&self, left: bool) -> Option<&FrameSidebar> {
        self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<FrameSidebar>()).find(|s| s.is_left() == left)
    }

    pub fn sidebar_mut(&mut self, left: bool) -> Option<&mut FrameSidebar> {
        self.region_mut(move |s: &FrameSidebar| s.is_left() == left)
    }

    pub fn footer(&self) -> Option<&FrameFooter> {
        self.region()
    }

    /// Where the window's own parts are, inside the frame's regions, as last placed.
    pub fn body_place(&self) -> Rect {
        self.frame.body()
    }

    /// The place of the grid: inside the frame, inside the padding.
    fn inside(&self) -> Rect {
        self.frame.body().inset(CONTENT_PADDING)
    }

    /// How high the window's rows and its frame's regions are with no spare height: what a window that names no height needs.
    pub fn content_height(&self, width: f64) -> f64 {
        let (high, wide) = self.frame.room(self.regions());
        self.grid().content_for((width - wide - 2.0 * CONTENT_PADDING).max(0.0), 0.0) + 2.0 * CONTENT_PADDING + high
    }

    /// Where the scroll bar runs: the padding at the window's right, beside the content.
    fn track(&self) -> Rect {
        let inside = self.inside();
        Rect::new(self.node.bounds.right() - CONTENT_PADDING, inside.y, CONTENT_PADDING, inside.height)
    }

    /// The part of the bar that says what shows: as long as the share shown, where it shows.
    pub fn thumb(&self) -> Rect {
        let inside = self.inside();
        let content = self.grid().content.max(1.0);
        let height = (inside.height * inside.height / content).max(16.0);
        let at = if self.max_scroll_y <= 0.0 { 0.0 } else { self.scroll_y / self.max_scroll_y };
        Rect::new(self.node.bounds.right() - CONTENT_PADDING / 2.0 - 2.0, inside.y + (inside.height - height) * at, 4.0, height)
    }

    /// Scrolls the window's content to `y`, within what it can; true when it moved.
    pub fn scroll_to(&mut self, y: f64, cx: &mut Cx) -> bool {
        let to = y.clamp(0.0, self.max_scroll_y);
        if to == self.scroll_y {
            return false;
        }
        self.scroll_y = to;
        if self.node.bounds.width > 0.0 {
            let bounds = self.node.bounds;
            self.arrange(bounds, cx);
        }
        cx.invalidate();
        true
    }

    /// One page down (1) or up (-1): the height shown, less a row, as PageDown does.
    pub fn page(&mut self, by: f64, cx: &mut Cx) -> bool {
        let row = row_height();
        let step = row.max(self.inside().height - row);
        self.scroll_to(self.scroll_y + by * step, cx)
    }

    /// How wide its sidebars are where they show, as designed or dragged (folded, a strip), even while one fills the window.
    pub fn sidebars_width(&self) -> f64 {
        [true, false].into_iter().filter(|left| self.frame.sidebar_shows(*left)).filter_map(|left| self.sidebar(left)).map(|s| s.wanted()).sum()
    }

    /// The parts that show and take clicks, in the order they are drawn: the window's own, then its sidebars' and footer's.
    fn keyed(&self) -> Vec<Keyed> {
        fn walk(e: &dyn Element, out: &mut Vec<Keyed>) {
            if !e.is_shown() {
                return;
            }
            if let Some(part) = e.as_part() {
                let base = part.base();
                if base.enabled() {
                    let spec = &base.spec;
                    out.push(Keyed {
                        id: e.node().id(),
                        name: spec.name().map(str::to_string),
                        shortcut: spec.text("shortcut").filter(|s| !s.is_empty()).map(str::to_string),
                        button: spec.kind == "button",
                        default: spec.flag("default"),
                        cancel: spec.flag("cancel"),
                        steps: e.as_any().is::<StepsPart>(),
                        menu: spec.menu.as_ref().map(|m| all_items(&m.items)).unwrap_or_default(),
                    });
                }
            }
            for child in e.node().children() {
                walk(child.as_ref(), out);
            }
        }
        let mut out = Vec::new();
        walk(self.body(), &mut out);
        for region in self.regions() {
            walk(region.as_ref(), &mut out);
        }
        out
    }

    /// The keys a part's command answers: the person's where they set their own, else its designed one.
    fn keys_of(&self, part: &Keyed) -> Vec<String> {
        frame::rules::keys_of(self.host.as_ref(), self.name.as_deref(), part.name.as_deref(), false, part.shortcut.as_deref())
    }

    fn press(&mut self, id: ElementId, cx: &mut Cx) {
        if let Some(part) = element::find_mut(self, id).and_then(|e| e.as_part_mut()) {
            part.press(cx);
        }
    }

    /// A key, before the part with the focus has it: a shortcut of a shown,
    /// enabled part runs it, then one of the frame's commands (its menu,
    /// toolbar and ribbon, while they show and take clicks), then one of a
    /// part's menu items; Enter runs the submit of the form the input with
    /// the focus is in, else presses the default button, and Escape the
    /// cancel button, once the part with the focus did not take them. True
    /// when the window took the key, or asked the part with the focus for it;
    /// false for Escape in a window without a cancel button.
    pub fn key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let focused = cx.focused();
        let mine = focused.and_then(|f| element::find(self, f));
        let typing = mine.is_some_and(|e| e.typing());
        // The focus is in something over the window (an open menu or list): it has its keys.
        let elsewhere = focused.is_some() && mine.is_none();
        if let Some(took) = self.access_key(key, cx) {
            return took;
        }
        let keyed = self.keyed();
        for part in &keyed {
            if self.keys_of(part).iter().any(|k| shortcuts::matches(k, &key, typing)) {
                // What is typed in the input with the focus is taken first, as a click on the button does.
                element::take_typed_from(self, focused, cx);
                self.press(part.id, cx);
                return true;
            }
        }
        // While a menu of the frame is open, its commands wait for it to close, as a desktop menu's do.
        let commands = if self.frame.menus().is_open() { Vec::new() } else { frame::rules::keyed(self.frame.shown(), self.host.as_ref()) };
        for (item, app) in commands {
            if self.frame.keys_of(&item, app).iter().any(|k| shortcuts::matches(k, &key, typing)) {
                element::take_typed_from(self, focused, cx);
                let frame = self.frame.clone();
                frame.run_by_key(&item, cx);
                return true;
            }
        }
        for part in &keyed {
            for item in &part.menu {
                if item.text("shortcut").is_some_and(|s| !s.is_empty() && shortcuts::matches(s, &key, typing)) {
                    element::take_typed_from(self, focused, cx);
                    parts::pick(self.host.as_ref(), part.name.as_deref(), item);
                    return true;
                }
            }
        }
        // F10 puts the focus on the menu bar, once no shortcut took it.
        if key.typed.is_none() && key.key == Key::F10 && key.modifiers == Modifiers::NONE && self.focus_menu_bar(cx) {
            return true;
        }
        // Alt with a top item's access letter opens its menu, once no shortcut took the key.
        if key.modifiers == Modifiers::ALT {
            if let Some(letter) = frame::rules::access_char(key.key) {
                if self.open_by_letter(letter, cx) {
                    return true;
                }
            }
        }
        // PageUp and PageDown that the part with the focus left scroll the window a page.
        if !elsewhere && key.typed.is_none() && key.modifiers == Modifiers::NONE && matches!(key.key, Key::PageUp | Key::PageDown) && self.max_scroll_y > 0.0 {
            self.page(if key.key == Key::PageDown { 1.0 } else { -1.0 }, cx);
            return true;
        }
        if elsewhere || key.modifiers != Modifiers::NONE || !matches!(key.key, Key::Enter | Key::Escape) {
            return false;
        }
        // The part with the focus is asked here, so the screen asks it no more.
        if let Some(e) = focused.and_then(|f| element::find_mut(self, f)) {
            if e.on_key(key, cx) {
                return true;
            }
            if key.key == Key::Enter && e.multiline() {
                return true;
            }
        }
        // Enter in an input of a form runs the form's submit, and presses no default button.
        if key.key == Key::Enter {
            if let Some(from) = focused {
                if parts::containers::panel::submit(&*self, from) {
                    return true;
                }
            }
        }
        let wanted = |p: &&Keyed| p.button && if key.key == Key::Enter { p.default } else { p.cancel };
        match keyed.iter().find(wanted) {
            Some(pressed) => {
                element::take_typed_from(self, focused, cx);
                self.press(pressed.id, cx);
            }
            // A wizard's Next or Finish is the default button of its step.
            None if key.key == Key::Enter => {
                if let Some(wizard) = keyed.iter().find(|p| p.steps && self.default_of(p.id).is_some()) {
                    element::take_typed_from(self, focused, cx);
                    if let Some(button) = self.default_of(wizard.id) {
                        if let Some(b) = element::find_mut(self, button).and_then(|e| e.as_any_mut().downcast_mut::<ContainerButton>()) {
                            b.press(cx);
                        }
                    }
                }
            }
            // Escape without a cancel button is left to the dialog the window is in, which closes as its close does.
            None if key.key == Key::Escape => return false,
            None => {}
        }
        true
    }
}

impl WindowView {
    /// Gives each slide-out of the window its band at its side of `body`, the
    /// window's content inside the frame; a pinned open one lies beside the
    /// window's parts, which make room for it. Where the parts go now.
    fn place_slideouts(&mut self, body: Rect, cx: &mut Cx) -> Rect {
        let mut room = body;
        let size = cx.look().size("small");
        self.strips.clear();
        // The handles of each side stand in a strip at its edge, one after the other: those of the bottom on top of the footer.
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let lengths: Vec<f64> = self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<SlideoutHandle>()).filter(|h| h.slide().side() == side).map(|h| h.length(cx.fonts(), size)).collect();
            if lengths.is_empty() {
                continue;
            }
            let strip = match side {
                Side::Left => Rect::new(room.x, room.y, HANDLE_WIDTH.min(room.width), room.height),
                Side::Right => Rect::new(room.right() - HANDLE_WIDTH.min(room.width), room.y, HANDLE_WIDTH.min(room.width), room.height),
                Side::Top => Rect::new(room.x, room.y, room.width, HANDLE_WIDTH.min(room.height)),
                Side::Bottom => Rect::new(room.x, room.bottom() - HANDLE_WIDTH.min(room.height), room.width, HANDLE_WIDTH.min(room.height)),
            };
            let mut at = 0.0;
            let mut lengths = lengths.into_iter();
            for child in self.node.children_mut() {
                let Some(handle) = child.as_any().downcast_ref::<SlideoutHandle>() else { continue };
                if handle.slide().side() != side {
                    continue;
                }
                let along = if side.across() { strip.width } else { strip.height };
                let length = lengths.next().unwrap_or(0.0).min((along - at).max(0.0));
                let place = if side.across() { Rect::new(strip.x + at, strip.y, length, strip.height) } else { Rect::new(strip.x, strip.y + at, strip.width, length) };
                child.arrange(place, cx);
                at += length;
            }
            self.strips.push((strip, side));
            room = match side {
                Side::Left => Rect::new(room.x + strip.width, room.y, room.width - strip.width, room.height),
                Side::Right => Rect::new(room.x, room.y, room.width - strip.width, room.height),
                Side::Top => Rect::new(room.x, room.y + strip.height, room.width, room.height - strip.height),
                Side::Bottom => Rect::new(room.x, room.y, room.width, room.height - strip.height),
            };
        }
        let grid = self.body_mut().child_mut();
        for child in grid.node_mut().children_mut() {
            let Some(slideout) = child.as_any().downcast_ref::<SlideoutPart>() else { continue };
            let slide = slideout.slide();
            let side = slide.side();
            let deep = slide.size().min(if side.across() { room.height } else { room.width }).max(0.0);
            // Docked ones line up from the edge in; one that floats lies at the edge of what is left.
            let band = match side {
                Side::Left => Rect::new(room.x, room.y, deep, room.height),
                Side::Right => Rect::new(room.right() - deep, room.y, deep, room.height),
                Side::Top => Rect::new(room.x, room.y, room.width, deep),
                Side::Bottom => Rect::new(room.x, room.bottom() - deep, room.width, deep),
            };
            slideout.set_band(band);
            if slideout.docked() {
                room = match side {
                    Side::Left => Rect::new(room.x + deep, room.y, (room.width - deep).max(0.0), room.height),
                    Side::Right => Rect::new(room.x, room.y, (room.width - deep).max(0.0), room.height),
                    Side::Top => Rect::new(room.x, room.y + deep, room.width, (room.height - deep).max(0.0)),
                    Side::Bottom => Rect::new(room.x, room.y, room.width, (room.height - deep).max(0.0)),
                };
            }
        }
        room
    }

    /// The button Enter presses in the wizard of that id: its Next, or on its last step its Finish; none when neither shows or can be pressed.
    fn default_of(&self, wizard: ElementId) -> Option<ElementId> {
        let steps = element::find(self, wizard)?.as_any().downcast_ref::<StepsPart>()?;
        [steps.next(), steps.finish()].into_iter().find(|b| b.is_shown() && b.is_usable()).map(|b| b.node().id())
    }
}

/// A menu's items and the items under them.
fn all_items(items: &[CommandSpec]) -> Vec<CommandSpec> {
    let mut all = Vec::new();
    for item in items {
        all.push(item.clone());
        all.extend(all_items(&item.items));
    }
    all
}

impl Element for WindowView {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let frame = self.frame.clone();
        let regions = self.node.children().len() - 1;
        let body = frame.arrange(&mut self.node.children_mut()[..regions], area, cx);
        frame.set_body(self.place_slideouts(body, cx));
        let inside = self.inside();
        let content = self.grid().content_for(inside.width, inside.height);
        self.max_scroll_y = (content - inside.height).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y);
        let place = if self.max_scroll_y > 0.0 { Rect::new(inside.x, inside.y - self.scroll_y, inside.width, content) } else { inside };
        let body = self.body_mut();
        body.node_mut().bounds = inside;
        body.child_mut().arrange(place, cx);
    }

    fn draw(&self, p: &Painter) {
        let inside = self.inside();
        let shows = self.body().is_shown();
        if shows {
            let _clip = p.clip(inside);
            self.grid().draw(p);
        }
        let content = self.grid().content;
        if self.max_scroll_y > 0.0 && content > 0.0 && shows {
            // A bar at the right says how much of the content shows, and where; it is wider while the pointer is on it or drags it.
            let (thumb, track) = (self.thumb(), self.track());
            let wide = self.drag_from.is_some() || (self.node.hovered && self.hover_x >= track.x);
            let bar = if wide { Rect::new(track.x + 2.0, thumb.y, track.width - 4.0, thumb.height) } else { thumb };
            p.fill(bar, p.look.color(if wide { "muted" } else { "strong_border" }), bar.width / 2.0);
        }
        if let Some(problem) = &self.problem {
            p.text(problem, Rect::new(inside.x, inside.y, inside.width, row_height()), p.look.color("error"), p.look.size("text"), "text", false, TextAlign::Left);
        }
        // A strip of handles has the panel's color and a line toward the window's parts.
        for (strip, side) in &self.strips {
            p.fill(*strip, p.look.color("panel"), 0.0);
            p.fill(side.inner_edge(*strip, p.look.border()), p.look.color("border"), 0.0);
        }
        for region in self.regions() {
            if region.is_shown() {
                element::draw_safely(region.as_ref(), p);
            }
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.scroll_to(self.scroll_y + dy, cx)
    }

    /// A press on the bar: on what shows it drags it, above or below it pages there, as a scroll bar does.
    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.drag_from = None;
        if self.max_scroll_y <= 0.0 || !self.body().is_shown() || !self.track().contains(at.screen_x, at.screen_y) {
            return;
        }
        let thumb = self.thumb();
        if at.screen_y < thumb.y {
            self.page(-1.0, cx);
        } else if at.screen_y > thumb.bottom() {
            self.page(1.0, cx);
        } else {
            self.drag_from = Some((at.screen_y, self.scroll_y));
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.hover_x = at.screen_x;
        if let (Some((from_y, from_scroll)), true) = (self.drag_from, self.node.pressed) {
            let room = (self.inside().height - self.thumb().height).max(1.0);
            self.scroll_to(from_scroll + (at.screen_y - from_y) * self.max_scroll_y / room, cx);
        } else if self.max_scroll_y > 0.0 {
            cx.invalidate();
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        self.drag_from = None;
        cx.invalidate();
    }

    /// The focus moved: a part of the window's grid that is out of sight
    /// scrolls into view, as Avalonia's ScrollViewer brought it; not when a
    /// press of the pointer moved it (a part half out of view stays put).
    fn focus_moved(&mut self, focused: Option<ElementId>, pointing: bool, cx: &mut Cx) {
        let Some(focused) = focused.filter(|_| !pointing && self.max_scroll_y > 0.0) else { return };
        let grid = self.grid().node().id();
        let Some(ids) = element::chain(self, focused) else { return };
        let Some(at) = ids.iter().position(|i| *i == grid) else { return };
        // A part in a part (a text box's editor) shows with the part it is in, up to the window's grid.
        let outer = ids[at + 1..].iter().filter_map(|i| element::find(self, *i)).find(|e| e.as_part().is_some()).or_else(|| element::find(self, focused));
        let Some(place) = outer.map(|e| e.node().bounds) else { return };
        let inside = self.inside();
        if place.y < inside.y || place.height > inside.height {
            self.scroll_to(self.scroll_y - (inside.y - place.y), cx);
        } else if place.bottom() > inside.bottom() {
            self.scroll_to(self.scroll_y + (place.bottom() - inside.bottom()), cx);
        }
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.key(key, cx)
    }

    fn window_key_up(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.key_up(key, cx)
    }

    /// The window is no longer shown: its menus close.
    fn on_detached(&mut self, cx: &mut Cx) {
        self.frame.menus().close_all(cx);
    }
}

//! A whole window of the program (DESIGN 7.9): its parts in the window's
//! grid, with the content padding every stack keeps around it, scrolling
//! when the window is smaller than its rows. Around the parts is the
//! window's frame (its menu bar, toolbar, ribbon, sidebars and footer, its
//! own or its app's). The window's own keys are here too: a part's
//! shortcut, a frame command's or one of a part's menu's items, Enter for
//! its default button (or a form's submit) and Escape for its cancel button.

use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel, Node};
use crate::frame::{self, Access, FrameFooter, FrameMenuBar, FrameRibbon, FrameSidebar, FrameSpec, FrameToolbar, Showing, ShownFrame, WindowFrame};
use crate::geometry::Rect;
use crate::grid::ROW_HEIGHT;
use crate::input::{Key, KeyInput, Modifiers};
use crate::markup::{CommandSpec, WindowSpec};
use crate::painter::{Painter, TextAlign};
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
}

/// A part that may answer a key: where it is, its name and what its markup says of keys.
struct Keyed {
    id: ElementId,
    name: Option<String>,
    shortcut: Option<String>,
    button: bool,
    default: bool,
    cancel: bool,
    menu: Vec<CommandSpec>,
}

impl WindowView {
    /// The window of `spec` over `host`: each of its parts made by its kind,
    /// at its place in the grid, in the frame of its own regions.
    pub fn new(spec: &WindowSpec, host: Rc<dyn PartHost>) -> WindowView {
        let mut grid = GridPanel::new(spec.columns);
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
        let mut view = WindowView { node, host, own_frame: spec.frame.clone(), frame, name: spec.name.clone(), problem: None, scroll_y: 0.0, max_scroll_y: 0.0 };
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
        // The regions go; the body stays, as the first child.
        while self.node.children().len() > 1 {
            let last = self.node.children().len() - 1;
            match cx.as_deref_mut() {
                Some(cx) => {
                    self.node.remove(last, cx);
                }
                None => {
                    self.node.children_mut().pop();
                }
            }
        }
        let app = app.map(FrameSpec::read);
        let shown = ShownFrame::of(app.as_ref(), &FrameSpec::read(&self.own_frame));
        let frame = WindowFrame::new(self.host.clone(), shown, self.name.clone());
        let filling = frame.clone();
        self.frame = frame;
        let body = self.node.children_mut()[0].as_any_mut().downcast_mut::<Showing>().expect("a window view holds its body");
        body.set_shows(move || filling.filling().is_none());
        self.add_regions();
    }

    fn add_regions(&mut self) {
        for region in frame::regions(&self.frame) {
            self.node.add(region);
        }
    }

    /// A window whose markup could not be read: it says why in its place.
    pub fn failed(problem: String, host: Rc<dyn PartHost>) -> WindowView {
        let spec = WindowSpec { frame: Vec::new(), parts: Vec::new(), name: None, columns: crate::markup::STANDARD_COLUMNS };
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
        self.node.children()[0].as_any().downcast_ref::<Showing>().expect("a window view holds its body")
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

    /// A key let go: Alt let go alone keeps the access letters for a plain letter, after another key it hides them.
    pub fn key_up(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Alt {
            return false;
        }
        match self.frame.access() {
            Access::Held { used: false } => self.frame.set_access(Access::Kept),
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
        let (high, wide) = self.frame.room(&self.node.children()[1..]);
        self.grid().content_for((width - wide - 2.0 * CONTENT_PADDING).max(0.0), 0.0) + 2.0 * CONTENT_PADDING + high
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
                        menu: spec.menu.as_ref().map(|m| all_items(&m.items)).unwrap_or_default(),
                    });
                }
            }
            for child in e.node().children() {
                walk(child.as_ref(), out);
            }
        }
        let mut out = Vec::new();
        for child in self.node.children() {
            walk(child.as_ref(), &mut out);
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
        for (item, app) in frame::rules::keyed(self.frame.shown(), self.host.as_ref()) {
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
        // Alt with a top item's access letter opens its menu, once no shortcut took the key.
        if key.modifiers == Modifiers::ALT {
            if let Some(letter) = frame::rules::access_char(key.key) {
                if self.open_by_letter(letter, cx) {
                    return true;
                }
            }
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
            // Escape without a cancel button is left to the dialog the window is in, which closes as its close does.
            None if key.key == Key::Escape => return false,
            None => {}
        }
        true
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
        frame.arrange(&mut self.node.children_mut()[1..], area, cx);
        let inside = self.inside();
        let content = self.grid().content_for(inside.width, inside.height);
        self.max_scroll_y = (content - inside.height).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y);
        let place = if self.max_scroll_y > 0.0 { Rect::new(inside.x, inside.y - self.scroll_y, inside.width, content) } else { inside };
        let body = self.node.children_mut()[0].as_any_mut().downcast_mut::<Showing>().expect("a window view holds its body");
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
            // A thin bar at the right says how much of the content shows, and where.
            let shown = inside.height / content;
            let bar = Rect::new(self.frame.body().right() - CONTENT_PADDING / 2.0 - 2.0, inside.y + inside.height * (self.scroll_y / content), 4.0, (inside.height * shown).max(16.0));
            p.fill(bar, p.look.color("strong_border"), 2.0);
        }
        if let Some(problem) = &self.problem {
            p.text(problem, Rect::new(inside.x, inside.y, inside.width, ROW_HEIGHT), p.look.color("error"), p.look.size("text"), "text", false, TextAlign::Left);
        }
        for region in &self.node.children()[1..] {
            if region.is_shown() {
                region.draw(p);
            }
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let to = (self.scroll_y + dy).clamp(0.0, self.max_scroll_y);
        if to == self.scroll_y {
            return false;
        }
        self.scroll_y = to;
        cx.invalidate();
        true
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

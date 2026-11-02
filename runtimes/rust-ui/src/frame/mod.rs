//! The frame around a window (DESIGN 7.9): the menu bar, toolbar and ribbon
//! at the top, the sidebars at the sides, the footer at the bottom, and the
//! window's own parts in between. Each region is the window's own where its
//! markup has one, else the app's; a dialog shows no app frame. Clicking an
//! item runs its handler or opens the window it targets; the `language` and
//! `theme` targets pick in place from a menu of choices.
//!
//! The regions are elements in the window view's tree; what they share (the
//! host, the regions shown, the menus open over the window, the folding of
//! an unnamed sidebar) is the `WindowFrame` they hold together.

pub mod buttons;
pub mod footer;
pub mod menu_bar;
pub mod menus;
pub mod ribbon;
pub mod rules;
pub mod sidebar;
pub mod spec;
pub mod toolbar;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clarity_runtime::ui::PartState;

use crate::app_files::AppFiles;
use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::markup::{self, CommandSpec};
use crate::painter::Painter;
use crate::parts::{self, PartHost};
use crate::xml::XmlElement;

pub use footer::FrameFooter;
pub use menu_bar::FrameMenuBar;
pub use menus::{FrameMenuList, FrameMenus, MenuEntry};
pub use ribbon::FrameRibbon;
pub use rules::{FramePicker, PickerChoice};
pub use sidebar::FrameSidebar;
pub use spec::{FrameSpec, ShownFrame};
pub use toolbar::FrameToolbar;

/// The regions of the app's frame a window shows where it has none of its
/// own: the first app's (`AppFiles::first_app`); none for a dialog, or a
/// program without an app. What `WindowView::with_frame` takes.
pub fn app_frame_of(files: &AppFiles, window: &str) -> Option<Vec<XmlElement>> {
    if crate::facts::ModelFacts::of(files.model).is_dialog(window) {
        return None;
    }
    files.first_app().and_then(|app| markup::read_app(app).ok())
}

/// What the regions of one window's frame share: its host, the regions it
/// shows, the menus open over the window, the pickers of the `language` and
/// `theme` targets, and the folding of an unnamed sidebar.
pub struct WindowFrame {
    host: Rc<dyn PartHost>,
    shown: ShownFrame,
    window: Option<String>,
    menus: FrameMenus,
    language: RefCell<FramePicker>,
    theme: RefCell<FramePicker>,
    /// Whether an unnamed sidebar is folded and fills the window, left then right.
    own_sides: [Cell<(bool, bool)>; 2],
    body: Cell<Rect>,
    access: Cell<Access>,
}

/// Whether the menu bar's access letters show: off, while Alt is held
/// (`used` once a key went with it), or kept after Alt went alone, when
/// the letter alone opens its menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Off,
    Held { used: bool },
    Kept,
}

impl WindowFrame {
    /// The frame of `shown` over `host`, for the window named `window` (by which the person's keys know its own commands).
    pub fn new(host: Rc<dyn PartHost>, shown: ShownFrame, window: Option<String>) -> Rc<WindowFrame> {
        let own = |side: &Option<spec::Shown<spec::SidebarSpec>>| Cell::new((side.as_ref().is_some_and(|s| s.value.collapsed), false));
        let own_sides = [own(&shown.left), own(&shown.right)];
        Rc::new(WindowFrame {
            host,
            window,
            menus: FrameMenus::new(),
            language: RefCell::new(FramePicker::language()),
            theme: RefCell::new(FramePicker::theme()),
            own_sides,
            body: Cell::new(Rect::default()),
            access: Cell::new(Access::Off),
            shown,
        })
    }

    pub fn host(&self) -> &Rc<dyn PartHost> {
        &self.host
    }

    /// The regions it shows, each the window's own or the app's.
    pub fn shown(&self) -> &ShownFrame {
        &self.shown
    }

    pub fn window(&self) -> Option<&str> {
        self.window.as_deref()
    }

    /// The menus open over the window: a menu bar's, a picker's.
    pub fn menus(&self) -> &FrameMenus {
        &self.menus
    }

    /// Whether the menu bar's access letters show, and how.
    pub fn access(&self) -> Access {
        self.access.get()
    }

    pub fn set_access(&self, access: Access) {
        self.access.set(access);
    }

    /// Where the window's own parts are, inside the regions, as last placed.
    pub fn body(&self) -> Rect {
        self.body.get()
    }

    /// What the `language` target picks from; a test gives its own.
    pub fn set_language(&self, picker: FramePicker) {
        *self.language.borrow_mut() = picker;
    }

    /// What the `theme` target picks from.
    pub fn set_theme(&self, picker: FramePicker) {
        *self.theme.borrow_mut() = picker;
    }

    pub fn picker_of(&self, target: &str) -> FramePicker {
        if target == "theme" { self.theme.borrow().clone() } else { self.language.borrow().clone() }
    }

    /// The keys a command of a region answers; `app`: the region is the app's.
    pub fn keys_of(&self, item: &CommandSpec, app: bool) -> Vec<String> {
        rules::keys_of(self.host.as_ref(), self.window.as_deref(), item.text("name"), app, item.text("shortcut"))
    }

    // -- Sidebars ----------------------------------------------------------------

    /// The sidebar on the left (`left`) or right; none where the frame has none.
    pub fn sidebar(&self, left: bool) -> Option<&spec::SidebarSpec> {
        if left { self.shown.left.as_ref() } else { self.shown.right.as_ref() }.map(|s| &s.value)
    }

    fn sidebar_state(&self, left: bool) -> Option<PartState> {
        self.sidebar(left).and_then(|s| s.name.as_deref()).and_then(|name| self.host.state(Some(name)))
    }

    fn own_side(&self, left: bool) -> &Cell<(bool, bool)> {
        &self.own_sides[if left { 0 } else { 1 }]
    }

    /// Whether the sidebar there shows: its state is not hidden.
    pub fn sidebar_shows(&self, left: bool) -> bool {
        self.sidebar(left).is_some_and(|s| rules::shows(self.host.as_ref(), s.name.as_deref()))
    }

    /// Whether the sidebar there is folded to its strip.
    pub fn collapsed(&self, left: bool) -> bool {
        let Some(sidebar) = self.sidebar(left) else { return false };
        let state = self.sidebar_state(left);
        let asked = match &state {
            Some(state) => state.collapsed,
            None => Some(self.own_side(left).get().0),
        };
        rules::sidebar_now(sidebar, asked, None).0
    }

    /// Whether the sidebar there fills the window; a folded one does not.
    pub fn maximized(&self, left: bool) -> bool {
        let Some(sidebar) = self.sidebar(left) else { return false };
        let asked = match self.sidebar_state(left) {
            Some(state) => state.maximized,
            None => Some(self.own_side(left).get().1),
        };
        rules::sidebar_now(sidebar, None, asked).1 && !self.collapsed(left)
    }

    /// The sidebar that fills the window, `Some(true)` the left one; the window's own parts and the other sidebar make room.
    pub fn filling(&self) -> Option<bool> {
        [true, false].into_iter().find(|left| self.sidebar_shows(*left) && self.maximized(*left))
    }

    /// Folds the sidebar there to its strip or shows it again; whether it
    /// fills the window stays as it was (folded it does not show), as
    /// Avalonia's did: shown again, it fills the window again.
    pub fn fold(&self, left: bool, folded: bool) {
        let Some(sidebar) = self.sidebar(left) else { return };
        if !sidebar.collapsible {
            return;
        }
        match sidebar.name.clone() {
            Some(name) => crate::parts::change_state(&*self.host, &name, |s| s.collapsed = Some(folded)),
            None => {
                let (_, filling) = self.own_side(left).get();
                self.own_side(left).set((folded, filling));
            }
        }
    }

    /// Lets the sidebar there fill the window, or gives the window back.
    pub fn fill(&self, left: bool, filling: bool) {
        let Some(sidebar) = self.sidebar(left) else { return };
        if !sidebar.maximizable {
            return;
        }
        match sidebar.name.clone() {
            Some(name) => crate::parts::change_state(&*self.host, &name, |s| s.maximized = Some(filling)),
            None => {
                let (folded, _) = self.own_side(left).get();
                self.own_side(left).set((folded, filling));
            }
        }
    }

    // -- Running commands --------------------------------------------------------

    /// The person picked a command at `at`, of `owner`: the `language` or
    /// `theme` target opens its choices there; any other runs its handler or
    /// opens the window it targets.
    pub fn run(&self, item: &CommandSpec, at: Rect, owner: menus::MenuOwner, cx: &mut Cx) {
        if let Some(target) = rules::picker_of(item) {
            self.menus.open(owner, choices(&self.picker_of(target)), at, None, cx);
            return;
        }
        self.menus.close_all(cx);
        parts::pick(self.host.as_ref(), item.text("name"), item);
    }

    /// A command's key was pressed: it runs as a click on it does, a picker's choices opening at the top of the window's parts.
    pub fn run_by_key(&self, item: &CommandSpec, cx: &mut Cx) {
        let body = self.body();
        self.run(item, Rect::new(body.x, body.y, 0.0, 0.0), (0, usize::MAX), cx);
    }

    /// The items of a menu as they show: hidden ones left out; the
    /// `language` and `theme` targets with their choices as a sub menu, the
    /// one shown checked; any other runs its handler or opens its window.
    /// Each shows the key it answers (the person's, else its designed one).
    pub fn entries_of(&self, items: &[CommandSpec]) -> Vec<MenuEntry> {
        let app = self.shown.menu.as_ref().is_some_and(|m| m.inherited);
        let host = self.host.as_ref();
        let mut entries = Vec::new();
        for item in items {
            let name = item.text("name");
            if !rules::shows(host, name) {
                continue;
            }
            if rules::is_separator(item) {
                entries.push(MenuEntry::line());
                continue;
            }
            let sub = if !item.items.is_empty() {
                self.entries_of(&item.items)
            } else {
                rules::picker_of(item).map(|p| choices(&self.picker_of(p))).unwrap_or_default()
            };
            let run: Option<Rc<dyn Fn()>> = if sub.is_empty() {
                let (host, item) = (self.host.clone(), item.clone());
                Some(Rc::new(move || parts::pick(host.as_ref(), item.text("name"), &item)))
            } else {
                None
            };
            entries.push(MenuEntry {
                caption: host.text(item.text("caption")),
                icon: item.text("icon").filter(|i| !i.is_empty()).and_then(|i| host.icon_path(i)),
                key: self.keys_of(item, app).into_iter().next(),
                checked: rules::checked(host, name),
                enabled: rules::takes(host, name),
                separator: false,
                items: sub,
                run,
                wake: None,
            });
        }
        entries
    }

    /// Places the frame's regions (`children`, the window view's) in `area`,
    /// and says where the window's own parts go: the menu bar, toolbar and
    /// ribbon at the top, the footer at the bottom, the sidebars at the sides
    /// of what is left; a sidebar that fills the window takes the middle.
    pub fn arrange(&self, children: &mut [Box<dyn Element>], area: Rect, cx: &mut Cx) -> Rect {
        let mut top = area.y;
        for child in children.iter_mut() {
            let wanted = match region_of(child.as_ref()) {
                Some(Region::Top(wanted)) if child.is_shown() => wanted,
                _ => continue,
            };
            child.arrange(Rect::new(area.x, top, area.width, wanted), cx);
            top += wanted;
        }
        let mut bottom = area.bottom();
        for child in children.iter_mut() {
            if let Some(Region::Bottom(wanted)) = region_of(child.as_ref()).filter(|_| child.is_shown()) {
                bottom = top.max(bottom - wanted);
                child.arrange(Rect::new(area.x, bottom, area.width, wanted), cx);
            }
        }
        let middle = Rect::new(area.x, top, area.width, (bottom - top).max(0.0));
        let filling = self.filling();
        let (mut left, mut right) = (middle.x, middle.right());
        for side in [true, false] {
            for child in children.iter_mut() {
                let Some(Region::Side(on_left, wanted)) = region_of(child.as_ref()) else { continue };
                if on_left != side || !child.is_shown() {
                    continue;
                }
                let fills = filling == Some(side);
                if side {
                    let wide = if fills { middle.width } else { wanted.min(middle.width) };
                    child.arrange(Rect::new(left, middle.y, wide, middle.height), cx);
                    left += wide;
                } else {
                    let wide = if fills { middle.width } else { wanted.min((right - left).max(0.0)) };
                    child.arrange(Rect::new(right - wide, middle.y, wide, middle.height), cx);
                    right -= wide;
                }
            }
        }
        let body = if filling.is_none() { Rect::new(left, middle.y, (right - left).max(0.0), middle.height) } else { Rect::new(middle.x, middle.y, 0.0, 0.0) };
        self.body.set(body);
        body
    }

    /// How high its regions at the top and bottom want to be now, and how wide its sidebars.
    pub fn room(&self, children: &[Box<dyn Element>]) -> (f64, f64) {
        let (mut high, mut wide) = (0.0, 0.0);
        for child in children.iter().filter(|c| c.is_shown()) {
            match region_of(child.as_ref()) {
                Some(Region::Top(w)) | Some(Region::Bottom(w)) => high += w,
                Some(Region::Side(_, w)) => wide += w,
                None => {}
            }
        }
        (high, wide)
    }
}

/// The choices of a picker as items of a menu, the one shown checked.
pub fn choices(picker: &FramePicker) -> Vec<MenuEntry> {
    (picker.choices)()
        .into_iter()
        .map(|c| {
            let choose = picker.choose.clone();
            let value = c.value.clone();
            MenuEntry { caption: c.label, checked: c.shown, enabled: true, run: Some(Rc::new(move || choose(&value))), ..MenuEntry::default() }
        })
        .collect()
}

/// The regions of a frame as elements, in the order they are drawn.
pub fn regions(frame: &Rc<WindowFrame>) -> Vec<Box<dyn Element>> {
    let shown = frame.shown();
    let mut out: Vec<Box<dyn Element>> = Vec::new();
    if shown.menu.is_some() {
        out.push(Box::new(FrameMenuBar::new(frame.clone())));
    }
    if shown.toolbar.is_some() {
        out.push(Box::new(FrameToolbar::new(frame.clone())));
    }
    if shown.ribbon.is_some() {
        out.push(Box::new(FrameRibbon::new(frame.clone())));
    }
    if shown.left.is_some() {
        out.push(Box::new(FrameSidebar::new(frame.clone(), true)));
    }
    if shown.right.is_some() {
        out.push(Box::new(FrameSidebar::new(frame.clone(), false)));
    }
    if shown.footer.is_some() {
        out.push(Box::new(FrameFooter::new(frame.clone())));
    }
    out
}

/// Where a region lies and the room it wants: its height at the top or bottom, its width at a side.
enum Region {
    Top(f64),
    Bottom(f64),
    /// On the left (true) or right, this wide.
    Side(bool, f64),
}

fn region_of(e: &dyn Element) -> Option<Region> {
    let any = e.as_any();
    if let Some(bar) = any.downcast_ref::<FrameMenuBar>() {
        return Some(Region::Top(bar.wanted()));
    }
    if let Some(toolbar) = any.downcast_ref::<FrameToolbar>() {
        return Some(Region::Top(toolbar.wanted()));
    }
    if let Some(ribbon) = any.downcast_ref::<FrameRibbon>() {
        return Some(Region::Top(ribbon.wanted()));
    }
    if let Some(footer) = any.downcast_ref::<FrameFooter>() {
        return Some(Region::Bottom(footer.wanted()));
    }
    any.downcast_ref::<FrameSidebar>().map(|s| Region::Side(s.is_left(), s.wanted()))
}

/// Whether an element is a region of a frame.
pub fn is_region(e: &dyn Element) -> bool {
    region_of(e).is_some()
}

/// The frame's own background, with a line on the side toward the window's parts.
pub(crate) fn plate(p: &Painter, area: Rect, line: Rect) {
    p.fill(area, p.look.color("frame"), 0.0);
    p.fill(line, p.look.color("border"), 0.0);
}

/// Whether the pointer, last at `pointer`, is over `area` of a region the pointer is over.
pub(crate) fn over(node: &Node, pointer: Option<(f64, f64)>, area: Rect) -> bool {
    node.hovered && pointer.is_some_and(|(x, y)| area.contains(x, y))
}

/// An element that holds others and shows while `shows` says so: a folded
/// sidebar's parts, the window's parts while a sidebar fills it.
pub struct Showing {
    node: Node,
    shows: Box<dyn Fn() -> bool>,
}

impl Showing {
    pub fn new(child: Box<dyn Element>, shows: impl Fn() -> bool + 'static) -> Showing {
        let mut node = Node::new();
        node.add(child);
        Showing { node, shows: Box::new(shows) }
    }

    /// It shows from now on while `shows` says so.
    pub fn set_shows(&mut self, shows: impl Fn() -> bool + 'static) {
        self.shows = Box::new(shows);
    }

    pub fn child(&self) -> &dyn Element {
        self.node.children()[0].as_ref()
    }

    pub fn child_mut(&mut self) -> &mut dyn Element {
        self.node.children_mut()[0].as_mut()
    }
}

impl Element for Showing {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        (self.shows)()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }
}

/// The id of a region's element, by which an open menu knows what it belongs to.
pub(crate) fn owner(node: &Node, at: usize) -> menus::MenuOwner {
    (node.id() as ElementId, at)
}

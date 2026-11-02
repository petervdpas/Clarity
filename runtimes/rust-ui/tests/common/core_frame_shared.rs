//! What the frame's tests share: a window with its frame on a screen of its
//! own, over a host that also knows the person's keys and the icons.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::frame::FrameMenuList;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::markup::{self, HandlerCall};
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::theme::ThemeLook;
use clarity_ui::window::WindowView;
use clarity_ui::Screen;
use serde_json::Value;

/// A test host with the person's keys (`Main.find` -> `F3`) and the icons the program carries.
#[derive(Default)]
pub struct FrameHost {
    pub base: TestHost,
    pub keys: RefCell<BTreeMap<String, Vec<String>>>,
    pub icons: RefCell<BTreeMap<String, String>>,
}

impl FrameHost {
    pub fn new() -> Rc<FrameHost> {
        Rc::new(FrameHost::default())
    }

    pub fn set_state(&self, name: &str, change: impl FnOnce(&mut PartState)) {
        self.base.set_state(name, change);
    }

    pub fn state_of(&self, name: &str) -> PartState {
        self.base.states.borrow().get(name).cloned().unwrap_or_default()
    }

    pub fn log(&self) -> Vec<String> {
        self.base.logged()
    }
}

impl PartHost for FrameHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.base.read(path)
    }
    fn write(&self, part: &str, path: &str, value: Value) {
        self.base.write(part, path, value)
    }
    fn edited(&self, part: &str) {
        self.base.edited(part)
    }
    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.base.state(name)
    }
    fn set_state(&self, name: &str, state: PartState) {
        clarity_ui::PartHost::set_state(&self.base, name, state)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.base.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.base.text(written)
    }
    fn field(&self, path: &str) -> Option<FieldFacts> {
        self.base.field(path)
    }
    fn changes(&self) -> u64 {
        self.base.changes()
    }
    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        match self.keys.borrow().get(command) {
            Some(own) => own.clone(),
            None => designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default(),
        }
    }
    fn icon_path(&self, name: &str) -> Option<String> {
        self.icons.borrow().get(name).cloned()
    }
}

pub const WIDTH: f64 = 1000.0;
pub const HEIGHT: f64 = 600.0;

/// A window named `name` of `own` markup, in the frame of `app` (an app's regions), on a screen of 1000 by 600.
pub fn window_named(own: &str, app: Option<&str>, host: Rc<FrameHost>, name: &str) -> Screen {
    let spec = markup::read(&format!("<Window name=\"{name}\">{own}</Window>")).expect("the window's markup reads");
    let app = app.map(|a| markup::read_app(&format!("<App name=\"Shop\">{a}</App>")).expect("the app's markup reads"));
    let as_host: Rc<dyn PartHost> = host;
    let view = WindowView::new(&spec, as_host).with_frame(app);
    let mut screen = Screen::new(Box::new(view), ThemeLook::clarity(false));
    screen.resize(WIDTH, HEIGHT, 1.0);
    screen
}

pub fn window(own: &str, app: Option<&str>) -> (Screen, Rc<FrameHost>) {
    let host = FrameHost::new();
    (window_named(own, app, host.clone(), "Main"), host)
}

pub fn view(screen: &Screen) -> &WindowView {
    screen.root().as_any().downcast_ref::<WindowView>().expect("the screen shows a window view")
}

/// Draws the screen, as the host does, so every region is placed and drawn.
pub fn draw(screen: &mut Screen) -> Shot {
    Shot::of(screen, WIDTH as i32, HEIGHT as i32, 1.0)
}

pub fn click_at(screen: &mut Screen, place: Rect, clicks: u32) {
    let (x, y) = (place.x + place.width / 2.0, place.y + place.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, clicks);
    screen.pointer_released(x, y, PointerButton::Left);
}

pub fn click(screen: &mut Screen, place: Rect) {
    click_at(screen, place, 1);
}

pub fn press(screen: &mut Screen, key: Key, held: Modifiers) {
    screen.key_pressed(KeyInput::new(key, held));
}

/// The menus open over the window, the first menu first.
pub fn lists(screen: &Screen) -> Vec<&FrameMenuList> {
    view(screen).frame().menus().lists().into_iter().filter_map(|id| screen.element(id)).filter_map(|e| e.as_any().downcast_ref::<FrameMenuList>()).collect()
}

pub fn captions(list: &FrameMenuList) -> Vec<String> {
    list.entries().iter().map(|e| e.caption.clone()).collect()
}

/// Where the item with `caption` is in a menu.
pub fn row(list: &FrameMenuList, caption: &str) -> Rect {
    list.row_of(list.entries().iter().position(|e| e.caption == caption).unwrap_or_else(|| panic!("no item {caption}")))
}

/// Where the scratchpad keeps the shots to look at.
pub fn shot_path(name: &str) -> String {
    format!("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/{name}.png")
}

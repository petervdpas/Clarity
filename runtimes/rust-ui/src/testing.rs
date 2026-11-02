//! What tests draw with: a screen drawn into pixels on Skia's raster (CPU)
//! surface, as a window shows it, to look at its colors or keep as a PNG.

use std::path::Path;

use skia_safe::{surfaces, Color, EncodedImageFormat, Surface};

use crate::screen::Screen;

/// A screen drawn into pixels.
pub struct Shot {
    surface: Surface,
}

impl Shot {
    /// Draws `screen` at `width` by `height` pixels, at `scale` pixels per unit.
    pub fn of(screen: &mut Screen, width: i32, height: i32, scale: f32) -> Shot {
        screen.resize(width as f64 / scale as f64, height as f64 / scale as f64, scale);
        let mut surface = surfaces::raster_n32_premul((width, height)).expect("Skia makes a raster surface");
        screen.draw(surface.canvas());
        Shot { surface }
    }

    pub fn width(&self) -> i32 {
        self.surface.width()
    }

    pub fn height(&self) -> i32 {
        self.surface.height()
    }

    /// The color of the pixel at `x`, `y`.
    pub fn pixel(&mut self, x: i32, y: i32) -> Color {
        let pixmap = self.surface.peek_pixels().expect("a raster surface has its pixels");
        pixmap.get_color((x, y))
    }

    /// Whether any pixel of a place differs from `color`.
    pub fn any_not(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) -> bool {
        let pixmap = self.surface.peek_pixels().expect("a raster surface has its pixels");
        (y..y + height).any(|py| (x..x + width).any(|px| pixmap.get_color((px, py)) != color))
    }

    /// The picture as PNG.
    pub fn png(&mut self) -> Vec<u8> {
        let image = self.surface.image_snapshot();
        image.encode(None, EncodedImageFormat::PNG, None).map(|d| d.as_bytes().to_vec()).unwrap_or_default()
    }

    /// Writes the picture as PNG to `path`, to look at.
    pub fn save(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        if let Some(dir) = path.as_ref().parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.png())
    }
}

/// Whether two colors are near: each channel within `by`.
pub fn near(a: Color, b: Color, by: u8) -> bool {
    a.r().abs_diff(b.r()) <= by && a.g().abs_diff(b.g()) <= by && a.b().abs_diff(b.b()) <= by
}

/// A window's values as the parts' tests give them: values by path, part
/// states, texts and fields in maps, and a log of what the parts did (each
/// write and each handler run; apart, each edit of a part's own state and
/// each address opened). Shared by every part's tests.
#[derive(Default)]
pub struct TestHost {
    pub values: std::cell::RefCell<std::collections::BTreeMap<String, serde_json::Value>>,
    pub states: std::cell::RefCell<std::collections::BTreeMap<String, clarity_runtime::ui::PartState>>,
    pub texts: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
    pub fields: std::cell::RefCell<std::collections::BTreeMap<String, crate::parts::FieldFacts>>,
    pub binds: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
    pub refusing: std::cell::RefCell<std::collections::BTreeSet<String>>,
    pub log: std::cell::RefCell<Vec<String>>,
    pub edits: std::cell::RefCell<Vec<String>>,
    pub opened: std::cell::RefCell<Vec<String>>,
    pub changed: std::cell::Cell<u64>,
}

impl TestHost {
    pub fn new() -> std::rc::Rc<TestHost> {
        std::rc::Rc::new(TestHost::default())
    }

    /// Sets a value, as a handler does: the parts read it again.
    pub fn set(&self, path: &str, value: serde_json::Value) {
        self.values.borrow_mut().insert(path.to_string(), value);
        self.change();
    }

    /// Changes a part's state, as code does.
    pub fn set_state(&self, name: &str, change: impl FnOnce(&mut clarity_runtime::ui::PartState)) {
        change(self.states.borrow_mut().entry(name.to_string()).or_default());
        self.change();
    }

    /// The values changed, as a handler changes them.
    pub fn change(&self) {
        self.changed.set(self.changed.get() + 1);
    }

    /// What the parts did, in order.
    pub fn logged(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

impl crate::parts::PartHost for TestHost {
    fn read(&self, path: &str) -> Option<serde_json::Value> {
        self.values.borrow().get(path).cloned()
    }

    fn write(&self, part: &str, path: &str, value: serde_json::Value) {
        self.log.borrow_mut().push(format!("write {part} {path} {value}"));
        self.values.borrow_mut().insert(path.to_string(), value);
        self.change();
    }

    fn edited(&self, part: &str) {
        self.edits.borrow_mut().push(part.to_string());
        self.change();
    }

    /// A named part has a state the first time it is asked for, as a window has one per named part.
    fn state(&self, name: Option<&str>) -> Option<clarity_runtime::ui::PartState> {
        let name = name?;
        Some(self.states.borrow_mut().entry(name.to_string()).or_default().clone())
    }

    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut clarity_runtime::ui::PartState)) {
        change(self.states.borrow_mut().entry(name.to_string()).or_default());
        self.change();
    }

    /// Logs the run; a handler named in `refusing` refuses.
    fn run(&self, part: Option<&str>, call: &crate::markup::HandlerCall) -> bool {
        self.log.borrow_mut().push(format!("run {} {} {}", part.unwrap_or(""), call.event, call.written()));
        !self.refusing.borrow().contains(&call.handler)
    }

    fn text(&self, written: Option<&str>) -> String {
        match written {
            None => String::new(),
            Some(w) => match w.strip_prefix('@') {
                Some(key) => self.texts.borrow().get(key).cloned().unwrap_or_else(|| key.to_string()),
                None => w.to_string(),
            },
        }
    }

    fn field(&self, path: &str) -> Option<crate::parts::FieldFacts> {
        self.fields.borrow().get(path).cloned()
    }

    fn open_address(&self, address: &str) {
        self.opened.borrow_mut().push(address.to_string());
    }

    fn changes(&self) -> u64 {
        self.changed.get()
    }

    fn bound_of(&self, part: &str) -> Option<String> {
        self.binds.borrow().get(part).cloned()
    }
}

/// A window's markup made into its parts in a grid of 12 columns on a screen, with `host` behind them, in Clarity's light look.
pub fn window_of(parts: &str, host: std::rc::Rc<dyn crate::parts::PartHost>) -> Screen {
    let markup = format!("<Window name=\"Test\">{parts}</Window>");
    let spec = crate::markup::read(&markup).expect("the test's markup reads");
    let mut grid = crate::element::GridPanel::new(spec.columns);
    for part in spec.parts {
        let made = crate::parts::make(part, host.clone());
        let place = crate::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    Screen::new(Box::new(grid), crate::theme::ThemeLook::clarity(false))
}

/// The id of the part named `name` on a screen.
pub fn part_named(screen: &Screen, name: &str) -> crate::element::ElementId {
    crate::element::in_order(screen.root())
        .into_iter()
        .find(|e| e.node().id() != screen.root().node().id() && part_name(*e) == Some(name))
        .map(|e| e.node().id())
        .unwrap_or_else(|| panic!("no part named {name}"))
}

/// The name of a part, by its spec; none for an element that is no part Clarity.UI knows.
pub fn part_name(element: &dyn crate::element::Element) -> Option<&str> {
    crate::parts::as_part(element).and_then(|p| p.base().spec.name())
}

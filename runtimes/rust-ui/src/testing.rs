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

    /// The color of the pixel at `x`, `y`; one past the edge fails the test.
    pub fn pixel(&mut self, x: i32, y: i32) -> Color {
        let pixmap = self.surface.peek_pixels().expect("a raster surface has its pixels");
        pixel_at(&pixmap, x, y)
    }

    /// Whether any pixel of a place differs from `color`; a place past the edge fails the test.
    pub fn any_not(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) -> bool {
        let pixmap = self.surface.peek_pixels().expect("a raster surface has its pixels");
        (y..y + height).any(|py| (x..x + width).any(|px| pixel_at(&pixmap, px, py) != color))
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

/// The color of the pixel at `x`, `y` of `pixels`: the one way tests read a
/// pixel. One past the edge fails the test with where it was, instead of
/// reading memory that is not the picture's.
pub fn pixel_at(pixels: &skia_safe::Pixmap, x: i32, y: i32) -> Color {
    let (width, height) = (pixels.width(), pixels.height());
    assert!(x >= 0 && y >= 0 && x < width && y < height, "the pixel {x},{y} is past the edge of a picture of {width} by {height}");
    pixels.get_color((x, y))
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
    /// The chosen row of each list, table or tree, by its part's name.
    pub selected: std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
    /// What code told custom parts, waiting to be heard.
    pub hears: std::cell::RefCell<Vec<clarity_runtime::ui::Hear>>,
    /// What custom parts and engines told, each as `part tell values`.
    pub told: std::cell::RefCell<Vec<String>>,
    /// The files of the program's resources, by name.
    pub resources: std::cell::RefCell<std::collections::BTreeMap<String, Vec<u8>>>,
    /// The keys the person set for commands, by the command (`Main.save`).
    pub keys: std::cell::RefCell<std::collections::BTreeMap<String, Vec<String>>>,
    /// Handlers that cannot run now (C# TestHost.CannotRun), by name.
    pub cannot_run: std::cell::RefCell<std::collections::BTreeSet<String>>,
    /// The SVG paths of the icons the program carries, by name.
    pub icons: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
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

    fn set_state(&self, name: &str, state: clarity_runtime::ui::PartState) {
        self.states.borrow_mut().insert(name.to_string(), state);
        self.change();
    }

    fn can_run(&self, _part: Option<&str>, call: &crate::markup::HandlerCall) -> bool {
        !self.cannot_run.borrow().contains(&call.handler)
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

    fn selected(&self, part: &str) -> Option<usize> {
        self.selected.borrow().get(part).copied()
    }

    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.resources.borrow().get(name).cloned()
    }

    fn icon_path(&self, name: &str) -> Option<String> {
        self.icons.borrow().get(name).cloned()
    }

    /// The person's keys where they set their own for `command`, else the designed one.
    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        if let Some(own) = self.keys.borrow().get(command) {
            return own.clone();
        }
        designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default()
    }

    fn select(&self, part: &str, place: Option<usize>) {
        match place {
            Some(place) => self.selected.borrow_mut().insert(part.to_string(), place),
            None => self.selected.borrow_mut().remove(part),
        };
        self.change();
    }

    fn hears(&self, part: &str) -> Vec<clarity_runtime::ui::Hear> {
        let mut hears = self.hears.borrow_mut();
        let (theirs, others): (Vec<_>, Vec<_>) = hears.drain(..).partition(|h| h.part == part);
        *hears = others;
        theirs
    }

    fn told(&self, part: &str, tell: &str, values: &[serde_json::Value]) {
        let values: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        self.told.borrow_mut().push(format!("{part} {tell} {}", values.join(" ")).trim_end().to_string());
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

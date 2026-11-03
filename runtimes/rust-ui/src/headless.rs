//! A program driven without a window (DESIGN 1.6, "Used as a person uses
//! it"): the generated program's own `Program` and files on a screen drawn
//! on Skia's raster surface, in the program's own process, with no display.
//! A test sends what a person does (a click, keys, typed text, a drag) and
//! reads what the screen shows and what the window holds, one step at a
//! time. `desktop::run` drives the program this way when the environment
//! variable CLARITY_HEADLESS is set: each line on standard input is a step
//! as JSON, and each answer is a line of JSON on standard output.
//!
//! The steps, by their `do`:
//! - `click`: the middle of the part `part` (in a dialog or menu over the
//!   window first, else a button of the window's frame by its name), the
//!   window scrolled with the wheel first until it is in sight; of the
//!   button that says `text` (scrolled to as well), or the point `x`, `y`;
//!   `clicks` 2 for a double click, `button` "right" for the context button.
//! - `press`, `move`, `release`: the main button at `x`, `y`, for a drag.
//! - `drag`: from `from` to `to` (`[x, y]`), moved in `steps` steps.
//! - `key`: the key `key` (`Enter`, `Tab`, `Down`, `Z`, `F2`), with `ctrl`,
//!   `shift`, `alt` held when true.
//! - `show`: the window scrolled until the part `part` is in sight, as
//!   before a click; answers its place `at`.
//! - `focus`: the focus to the part `part` (its first element that takes
//!   the focus), as Tab would bring it there, without a click.
//! - `type`: the text `text`, as typed.
//! - `wheel`: turns the wheel at `x`, `y` by `dy` (and `dx`).
//! - `settle`: lets `ms` milliseconds pass: timers wake, work off the
//!   screen's thread is taken.
//! - `size`: the screen at `width` by `height`.
//! - `dark`: the system shows dark when `dark` is true.
//! - `file`: the next file or folder a picker asks for is `path`.
//! - `read`: what shows (below).
//! - `shot`: the screen as a PNG at `path`.
//! - `quit`: the run ends.
//!
//! Every answer has `ok` (and `error` when a step could not be done); a
//! `read` also has the open windows (`windows`), whether the program ended
//! (`ended`) and by quitting (`quit`), whether Clarity's keys window shows
//! (`keys`), the one on top (`window`),
//! its data (`data`), its parts' states (`parts`), the rows chosen
//! (`selected`), the dialog over it (`dialog`, with its `data`), the part
//! with the focus (`focused`), every part and named frame button that shows
//! with its place (`shown`), every button that shows with what it says (`buttons`), the
//! notes and the messages said (`notes`, `said`), how many overlays are
//! open (`overlays`), the parts that show in the top one (`top`), and the
//! rows of each list, table and tree that show, with their places (`rows`),
//! and how each canvas maps its units onto the screen (`canvases`).

use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_runtime::program::Program;
use serde_json::{Map, Value, json};

use crate::app_files::AppFiles;
use crate::element::{self, Element, ElementId};
use crate::files::{ChosenFile, FileFilter, FilePicker};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerButton};
use crate::screen::Screen;
use crate::window::WindowStack;

/// The environment variable that drives a program without a window.
pub const VARIABLE: &str = "CLARITY_HEADLESS";

/// The size a driven screen starts at, as a desktop window of 1280 by 800.
const START_SIZE: (f64, f64) = (1280.0, 800.0);

/// The files a driven run's pickers give, in turn, as the system's file dialog would.
#[derive(Default)]
struct Given(Mutex<Vec<String>>);

impl Given {
    fn next(&self) -> Option<String> {
        let mut given = self.0.lock().ok()?;
        if given.is_empty() { None } else { Some(given.remove(0)) }
    }
}

impl FilePicker for Given {
    fn picks_folders(&self) -> bool {
        true
    }

    fn open_file(&self, _title: &str, _filter: Option<&FileFilter>) -> Option<ChosenFile> {
        let path = self.next()?;
        let bytes = std::fs::read(&path).ok()?;
        let name = Path::new(&path).file_name()?.to_string_lossy().into_owned();
        Some(ChosenFile { name, bytes })
    }

    fn save_file(&self, _title: &str, name: &str, bytes: &[u8]) -> Option<String> {
        let folder = self.next()?;
        let path = Path::new(&folder).join(crate::files::safe_name(name));
        std::fs::write(&path, bytes).ok()?;
        Some(path.display().to_string())
    }

    fn choose_folder(&self, _title: &str) -> Option<String> {
        self.next()
    }

    fn show(&self, _name: &str, _bytes: &[u8]) {}
}

/// A program on a screen without a window, driven step by step.
pub struct Driven {
    screen: Screen,
    given: Arc<Given>,
}

impl Driven {
    /// Starts `program` as the desktop does (its language and theme, its main
    /// window), every window in this one screen: a dialog over the window, a
    /// window `in new window` in place.
    pub fn start(program: Arc<dyn Program>, files: AppFiles) -> Driven {
        let kept = clarity_runtime::settings::file(files.title).and_then(|f| f.parent().map(|p| p.to_path_buf()));
        crate::choices::start(&files, kept);
        let given = Arc::new(Given::default());
        crate::files::set_system(given.clone());
        let stack = WindowStack::start(program, files);
        let mut screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
        screen.follow(Box::new(move |dark| crate::choices::look(&files, dark)));
        screen.resize(START_SIZE.0, START_SIZE.1, 1.0);
        let mut driven = Driven { screen, given };
        driven.settle(Duration::ZERO);
        driven
    }

    /// Drives a screen of its own (a window over a host of the caller's,
    /// as the C interface makes one), with no program behind it.
    pub fn over(screen: Screen) -> Driven {
        let given = Arc::new(Given::default());
        crate::files::set_system(given.clone());
        let mut driven = Driven { screen, given };
        driven.settle(Duration::ZERO);
        driven
    }

    /// The screen it drives.
    pub fn screen(&mut self) -> &mut Screen {
        &mut self.screen
    }

    /// Does one step and answers it.
    pub fn step(&mut self, step: &Value) -> Value {
        match self.run(step) {
            Ok(mut answer) => {
                answer.insert("ok".into(), Value::Bool(true));
                Value::Object(answer)
            }
            Err(error) => json!({ "ok": false, "error": error }),
        }
    }

    fn run(&mut self, step: &Value) -> Result<Map<String, Value>, String> {
        let number = |name: &str| step.get(name).and_then(Value::as_f64).ok_or_else(|| format!("the step has no number {name}"));
        let point = |name: &str| -> Result<(f64, f64), String> {
            let at = step.get(name).and_then(Value::as_array).ok_or_else(|| format!("the step has no point {name}"))?;
            Ok((at.first().and_then(Value::as_f64).unwrap_or(0.0), at.get(1).and_then(Value::as_f64).unwrap_or(0.0)))
        };
        let mut answer = Map::new();
        match step.get("do").and_then(Value::as_str).unwrap_or_default() {
            "click" => {
                let (x, y) = self.target(step)?;
                let clicks = step.get("clicks").and_then(Value::as_u64).unwrap_or(1) as u32;
                let button = if step.get("button").and_then(Value::as_str) == Some("right") { PointerButton::Right } else { PointerButton::Left };
                self.screen.pointer_moved(x, y);
                for click in 1..=clicks {
                    self.screen.pointer_pressed_with(x, y, button, click, Modifiers::NONE);
                    self.screen.pointer_released(x, y, button);
                }
                self.settle(Duration::ZERO);
            }
            "press" => {
                let (x, y) = (number("x")?, number("y")?);
                self.screen.pointer_moved(x, y);
                self.screen.pointer_pressed(x, y, PointerButton::Left, 1);
                self.settle(Duration::ZERO);
            }
            "move" => {
                self.screen.pointer_moved(number("x")?, number("y")?);
                self.settle(Duration::ZERO);
            }
            "release" => {
                self.screen.pointer_released(number("x")?, number("y")?, PointerButton::Left);
                self.settle(Duration::ZERO);
            }
            "drag" => {
                let (from, to) = (point("from")?, point("to")?);
                let steps = step.get("steps").and_then(Value::as_u64).unwrap_or(8).max(1);
                self.screen.pointer_moved(from.0, from.1);
                self.screen.pointer_pressed(from.0, from.1, PointerButton::Left, 1);
                self.settle(Duration::ZERO);
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    self.screen.pointer_moved(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
                    self.settle(Duration::ZERO);
                }
                self.screen.pointer_released(to.0, to.1, PointerButton::Left);
                self.settle(Duration::ZERO);
            }
            "key" => {
                let name = step.get("key").and_then(Value::as_str).unwrap_or_default();
                let key = key_named(name).ok_or_else(|| format!("no key {name}"))?;
                let mut held = Modifiers::NONE;
                for (flag, modifier) in [("ctrl", Modifiers::CONTROL), ("shift", Modifiers::SHIFT), ("alt", Modifiers::ALT)] {
                    if step.get(flag).and_then(Value::as_bool).unwrap_or(false) {
                        held = held | modifier;
                    }
                }
                let input = KeyInput::new(key, held);
                self.screen.key_pressed(input);
                self.screen.key_released(input);
                self.settle(Duration::ZERO);
            }
            "show" => {
                let name = step.get("part").and_then(Value::as_str).unwrap_or_default();
                let b = self.in_sight(name)?;
                answer.insert("at".into(), json!([b.x, b.y, b.width, b.height]));
            }
            "focus" => {
                let name = step.get("part").and_then(Value::as_str).unwrap_or_default();
                let id = self.part(name).ok_or_else(|| format!("no part {name} shows"))?;
                let inner = self.screen.element(id).and_then(|e| shown_in(e).into_iter().find(|e| e.focusable()).map(|e| e.node().id()));
                self.screen.focus(Some(inner.unwrap_or(id)));
                self.settle(Duration::ZERO);
            }
            "type" => {
                let text = step.get("text").and_then(Value::as_str).unwrap_or_default();
                self.screen.text_typed(text);
                self.settle(Duration::ZERO);
            }
            "wheel" => {
                let dx = step.get("dx").and_then(Value::as_f64).unwrap_or(0.0);
                self.screen.wheel(number("x")?, number("y")?, dx, number("dy")?, Modifiers::NONE);
                self.settle(Duration::ZERO);
            }
            "settle" => self.settle(Duration::from_millis(number("ms")? as u64)),
            "size" => {
                self.screen.resize(number("width")?, number("height")?, 1.0);
                self.settle(Duration::ZERO);
            }
            "dark" => {
                self.screen.set_dark(step.get("dark").and_then(Value::as_bool).unwrap_or(false));
                self.settle(Duration::ZERO);
            }
            "file" => {
                let path = step.get("path").and_then(Value::as_str).ok_or("the step has no path")?;
                self.given.0.lock().map_err(|_| "the picker is not available")?.push(path.to_string());
            }
            "read" => answer = self.read(),
            "shot" => {
                let path = step.get("path").and_then(Value::as_str).ok_or("the step has no path")?;
                let png = self.png().ok_or("the screen could not be drawn")?;
                std::fs::write(path, png).map_err(|e| e.to_string())?;
            }
            "quit" => {}
            other => return Err(format!("no step {other}")),
        }
        Ok(answer)
    }

    /// Where a click goes: the middle of a part, of a button by what it says, or a point.
    fn target(&mut self, step: &Value) -> Result<(f64, f64), String> {
        if let Some(name) = step.get("part").and_then(Value::as_str) {
            return self.in_sight(name).map(middle);
        }
        if let Some(text) = step.get("text").and_then(Value::as_str) {
            return self.button_in_sight(text).map(middle);
        }
        match (step.get("x").and_then(Value::as_f64), step.get("y").and_then(Value::as_f64)) {
            (Some(x), Some(y)) => Ok((x, y)),
            _ => Err("a click names a part, a button's text or a point".into()),
        }
    }

    /// The place of a part, or a frame button, scrolled into sight first
    /// as a person scrolls to it: the wheel turned over the window's margin
    /// until the whole of it shows in the window's body, where it can be.
    fn in_sight(&mut self, name: &str) -> Result<Rect, String> {
        self.scrolled_to(&|d: &Driven| d.place_of(name), &|d: &Driven, _| d.body_of(name)).ok_or_else(|| format!("no part {name} shows"))
    }

    /// The button that says `text` (the last drawn of those that do), scrolled into sight as a part is.
    fn button_in_sight(&mut self, text: &str) -> Result<Rect, String> {
        let find = |d: &Driven| d.buttons().into_iter().rev().find(|(says, _)| says == text).map(|(_, b)| b);
        self.scrolled_to(&find, &|d: &Driven, b| d.body_holding(b)).ok_or_else(|| format!("no button {text} shows"))
    }

    /// Turns the wheel over the margin of the window that shows a place
    /// (`find`) until the place is inside that window's body (`body`), at
    /// most twenty turns; the place found last, none when it is gone.
    fn scrolled_to(&mut self, find: &dyn Fn(&Driven) -> Option<Rect>, body: &dyn Fn(&Driven, Rect) -> Option<Rect>) -> Option<Rect> {
        for _ in 0..20 {
            let b = find(self)?;
            let Some(body) = body(self, b) else { return Some(b) };
            let by = if b.y < body.y {
                b.y - body.y - 10.0
            } else if b.bottom() > body.bottom() {
                (b.bottom() - body.bottom() + 10.0).min(b.y - body.y)
            } else {
                return Some(b);
            };
            // Over the window's own margin, beside its parts, so the window scrolls and no part in it.
            self.screen.wheel(body.x + crate::window::view::CONTENT_PADDING / 2.0, body.y + body.height / 2.0, 0.0, by, Modifiers::NONE);
            self.settle(Duration::ZERO);
            if find(self) == Some(b) {
                break;
            }
        }
        find(self)
    }

    /// The window views on the screen, the top one first.
    fn views(&self) -> Vec<&crate::window::view::WindowView> {
        self.trees().into_iter().rev().flat_map(|tree| shown_in(tree).into_iter().rev().filter_map(|e| e.as_any().downcast_ref::<crate::window::view::WindowView>())).collect()
    }

    /// The body of the window view that shows the part `name`: where its parts show, inside its frame.
    fn body_of(&self, name: &str) -> Option<Rect> {
        self.views().into_iter().find(|view| shown_in(*view).into_iter().any(|e| part_name(e) == Some(name))).map(|view| view.body_place())
    }

    /// The body of the window view that holds an element at the place `b`.
    fn body_holding(&self, b: Rect) -> Option<Rect> {
        self.views().into_iter().find(|view| shown_in(*view).into_iter().skip(1).any(|e| e.node().bounds == b)).map(|view| view.body_place())
    }

    /// The trees on the screen, the window first and the overlays over it after.
    fn trees(&self) -> Vec<&dyn Element> {
        std::iter::once(self.screen.root()).chain(self.screen.overlays().iter().map(|o| o.content.as_ref())).collect()
    }

    /// The part named `name` that shows, the one in the top overlay first.
    fn part(&self, name: &str) -> Option<ElementId> {
        self.trees().into_iter().rev().find_map(|tree| shown_in(tree).into_iter().find(|e| part_name(*e) == Some(name)).map(|e| e.node().id()))
    }

    /// The place of a part, or of a button of the window's frame (its
    /// toolbar, ribbon or menu bar) by its name.
    fn place_of(&self, name: &str) -> Option<Rect> {
        self.part(name).map(|id| self.bounds(id)).or_else(|| self.commands().into_iter().find(|(n, _)| n == name).map(|(_, b)| b))
    }

    /// The named buttons and top menu items of the window's frame that
    /// show, each with its place: the toolbar's, the ribbon's open tab's and
    /// the menu bar's.
    fn commands(&self) -> Vec<(String, Rect)> {
        let mut out = Vec::new();
        for e in self.trees().into_iter().flat_map(shown_in) {
            let any = e.as_any();
            let placed = if let Some(toolbar) = any.downcast_ref::<crate::frame::toolbar::FrameToolbar>() {
                toolbar.placed()
            } else if let Some(ribbon) = any.downcast_ref::<crate::frame::ribbon::FrameRibbon>() {
                ribbon.buttons()
            } else if let Some(bar) = any.downcast_ref::<crate::frame::menu_bar::FrameMenuBar>() {
                bar.placed()
            } else {
                continue;
            };
            out.extend(placed.into_iter().filter_map(|(item, b)| item.text("name").map(|n| (n.to_string(), b))));
        }
        out
    }

    fn bounds(&self, id: ElementId) -> Rect {
        self.screen.element(id).map(|e| e.node().bounds).unwrap_or_default()
    }

    /// Every button that shows and what it says, in the order drawn: a
    /// window's buttons, a dialog's, a wizard's, tabs, a canvas's tools, the
    /// items of an open menu, and the lines of an open list (a split
    /// button's menu, a dropdown's choices).
    fn buttons(&self) -> Vec<(String, Rect)> {
        let mut out = Vec::new();
        for e in self.trees().into_iter().flat_map(shown_in) {
            let any = e.as_any();
            let b = e.node().bounds;
            if let Some(button) = any.downcast_ref::<crate::dialogs::DialogButton>() {
                out.push((button.text.clone(), b));
            } else if let Some(button) = any.downcast_ref::<crate::parts::containers::container::ContainerButton>() {
                out.push((button.text(), b));
            } else if let Some(button) = any.downcast_ref::<crate::parts::button::ButtonPart>() {
                out.push((button.caption(), b));
            } else if let Some(tab) = any.downcast_ref::<crate::parts::containers::tabs::TabButton>() {
                out.push((tab.title(), b));
            } else if let Some(canvas) = any.downcast_ref::<crate::parts::data::canvas::CanvasPart>() {
                // A drawing's or a picture's toolbar: its tools by their words.
                for (tool, place) in canvas.tool_buttons(self.screen.fonts(), self.screen.look()) {
                    out.push((crate::parts::data::canvas::tool_word(tool), place));
                }
            } else if let Some(list) = any.downcast_ref::<crate::parts::inputs::choice_list::ChoiceList>() {
                // An open list: a split button's menu, a dropdown's choices; each line in sight.
                use crate::parts::inputs::choice_list::LINE;
                let state = list.state.borrow();
                for (i, label) in state.labels.iter().enumerate() {
                    let line = Rect::new(b.x + 1.0, b.y + 1.0 + i as f64 * LINE - state.top, b.width - 2.0, LINE);
                    if line.y >= b.y && line.bottom() <= b.bottom() + 1.0 {
                        out.push((label.clone(), line));
                    }
                }
            } else if let Some(menu) = any.downcast_ref::<crate::frame::menus::FrameMenuList>() {
                for (i, entry) in menu.entries().iter().enumerate().filter(|(_, e)| !e.separator) {
                    out.push((entry.caption.clone(), menu.row_of(i)));
                }
            }
        }
        out.retain(|(says, _)| !says.is_empty());
        out
    }

    /// What shows and what the window holds.
    fn read(&self) -> Map<String, Value> {
        let mut out = Map::new();
        let stack = self.screen.root().as_any().downcast_ref::<WindowStack>();
        if let Some(stack) = stack {
            out.insert("windows".into(), json!(stack.windows()));
            out.insert("ended".into(), json!(stack.ended()));
            out.insert("quit".into(), json!(stack.quitting()));
            if let Some(host) = stack.current() {
                let state = host.state_now();
                out.insert("window".into(), json!(state.window));
                out.insert("data".into(), Value::Object(state.data.clone()));
                out.insert("parts".into(), serde_json::to_value(&state.parts).unwrap_or_default());
                out.insert("selected".into(), json!(state.selected));
            }
            if let Some(dialog) = stack.dialog_host() {
                let state = dialog.state_now();
                out.insert("dialog".into(), json!({ "window": state.window, "data": state.data, "parts": state.parts, "selected": state.selected }));
            }
            out.insert("notes".into(), json!(stack.notes().iter().map(|m| m.text.show()).collect::<Vec<_>>()));
            out.insert("said".into(), json!(stack.said().iter().map(|m| m.text.show()).collect::<Vec<_>>()));
        }
        out.insert("keys".into(), json!(self.screen.find::<crate::dialogs::KeysWindow>().is_some()));
        let focused = self.screen.focused().and_then(|id| self.owning_part(id));
        out.insert("focused".into(), json!(focused));
        let mut shown = Map::new();
        for tree in self.trees() {
            for e in shown_in(tree) {
                if let Some(name) = part_name(e) {
                    let b = e.node().bounds;
                    shown.insert(name.to_string(), json!([b.x, b.y, b.width, b.height]));
                }
            }
        }
        for (name, b) in self.commands() {
            shown.entry(name).or_insert_with(|| json!([b.x, b.y, b.width, b.height]));
        }
        out.insert("shown".into(), Value::Object(shown));
        out.insert("rows".into(), Value::Object(self.rows()));
        out.insert("canvases".into(), Value::Object(self.canvases()));
        out.insert("buttons".into(), json!(self.buttons().into_iter().map(|(says, b)| json!({ "text": says, "at": [b.x, b.y, b.width, b.height] })).collect::<Vec<_>>()));
        let top = self.screen.overlays().last().map(|o| shown_in(o.content.as_ref()).into_iter().filter_map(part_name).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default();
        out.insert("top".into(), json!(top));
        out.insert("overlays".into(), json!(self.screen.overlays().len()));
        out
    }

    /// The rows each list, table and tree that shows has on the screen, top
    /// to bottom: what each row says (a table's cells between spaces, a
    /// list's fields between bars), its record's place in the data, its box,
    /// and a tree row's toggle box. A table also gives its headers' boxes
    /// by field and its page buttons' boxes, under its name with `.header`
    /// and `.pages`.
    fn rows(&self) -> Map<String, Value> {
        use crate::parts::data::list::ListPart;
        use crate::parts::data::table::{PageButton, TablePart};
        let at = |b: Rect| json!([b.x, b.y, b.width, b.height]);
        let mut out = Map::new();
        for e in self.trees().into_iter().flat_map(shown_in) {
            let Some(name) = part_name(e) else { continue };
            let any = e.as_any();
            if let Some(table) = any.downcast_ref::<TablePart>() {
                let fields: Vec<String> = table.columns().iter().map(|c| c.field.clone()).collect();
                let rows: Vec<Value> = table
                    .shown_places()
                    .into_iter()
                    .filter_map(|place| {
                        let b = table.row_box(place)?;
                        let text = fields.iter().map(|f| table.shown(place, f)).collect::<Vec<_>>().join(" ");
                        let mut row = json!({ "text": text.trim(), "place": place, "at": at(b) });
                        if let Some(toggle) = table.toggle_box(place) {
                            row["toggle"] = at(toggle);
                        }
                        Some(row)
                    })
                    .collect();
                out.insert(name.to_string(), Value::Array(rows));
                let headers: Map<String, Value> = fields.iter().filter_map(|f| table.header_box(f).map(|b| (f.clone(), at(b)))).collect();
                out.insert(format!("{name}.header"), Value::Object(headers));
                let mut pages = Map::new();
                for (word, button) in [("first", PageButton::First), ("previous", PageButton::Previous), ("next", PageButton::Next), ("last", PageButton::Last)] {
                    if let Some(b) = table.pager_box(button) {
                        pages.insert(word.into(), at(b));
                    }
                }
                out.insert(format!("{name}.pages"), Value::Object(pages));
            } else if let Some(list) = any.downcast_ref::<ListPart>() {
                let rows: Vec<Value> = list
                    .records()
                    .iter()
                    .enumerate()
                    .filter_map(|(place, record)| list.row_box(place).map(|b| json!({ "text": list.text_of(record), "place": place, "at": at(b) })))
                    .collect();
                out.insert(name.to_string(), Value::Array(rows));
            }
        }
        out
    }

    /// How each canvas that shows maps its units onto the screen: a unit
    /// `u` is at `left + (u - x) * scale` across and `top + (u - y) * scale`
    /// down, `left` and `top` being its box's.
    fn canvases(&self) -> Map<String, Value> {
        let mut out = Map::new();
        for e in self.trees().into_iter().flat_map(shown_in) {
            let (Some(name), Some(canvas)) = (part_name(e), e.as_any().downcast_ref::<crate::parts::data::canvas::CanvasPart>()) else { continue };
            let v = canvas.viewport();
            let b = e.node().bounds;
            out.insert(name.to_string(), json!({ "scale": v.scale, "x": v.x, "y": v.y, "at": [b.x, b.y, b.width, b.height] }));
        }
        out
    }

    /// The name of the part an element is, or is in.
    fn owning_part(&self, id: ElementId) -> Option<String> {
        for tree in self.trees() {
            if let Some(chain) = element::chain(tree, id) {
                return chain.iter().rev().find_map(|i| element::find(tree, *i).and_then(part_name).map(str::to_string));
            }
        }
        None
    }

    /// Lets `time` pass, at least once: work off the screen's thread is
    /// taken, timers wake, and the screen is drawn so every place is current.
    fn settle(&mut self, time: Duration) {
        let end = Instant::now() + time;
        loop {
            if self.screen.later_running() > 0 {
                self.screen.finish_later();
            }
            self.screen.take_later();
            self.screen.wake_due(Instant::now());
            self.draw();
            if Instant::now() >= end {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn draw(&mut self) -> Option<skia_safe::Surface> {
        let (width, height) = (self.screen.width().max(1.0) as i32, self.screen.height().max(1.0) as i32);
        let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))?;
        self.screen.draw(surface.canvas());
        Some(surface)
    }

    fn png(&mut self) -> Option<Vec<u8>> {
        let mut surface = self.draw()?;
        let image = surface.image_snapshot();
        let data = image.encode(None, skia_safe::EncodedImageFormat::PNG, None)?;
        Some(data.as_bytes().to_vec())
    }
}

fn middle(b: Rect) -> (f64, f64) {
    (b.x + b.width / 2.0, b.y + b.height / 2.0)
}

fn part_name(e: &dyn Element) -> Option<&str> {
    e.as_part().and_then(|p| p.base().spec.name())
}

/// Every element of a tree that shows (with a place), in the order drawn.
fn shown_in(root: &dyn Element) -> Vec<&dyn Element> {
    fn walk<'a>(e: &'a dyn Element, out: &mut Vec<&'a dyn Element>) {
        if !e.is_shown() {
            return;
        }
        let b = e.node().bounds;
        if b.width > 0.0 && b.height > 0.0 {
            out.push(e);
        }
        for child in e.node().children() {
            walk(child.as_ref(), out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// A key by the name a step gives it: `Enter`, `Down`, `F2`, `A`, `7`.
pub fn key_named(name: &str) -> Option<Key> {
    let named = match name {
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Space" | " " => Key::Space,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Insert" => Key::Insert,
        "Left" | "ArrowLeft" => Key::Left,
        "Right" | "ArrowRight" => Key::Right,
        "Up" | "ArrowUp" => Key::Up,
        "Down" | "ArrowDown" => Key::Down,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "+" => Key::Plus,
        "-" => Key::Minus,
        _ => {
            if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=12).contains(n)) {
                return Some(Key::FUNCTIONS[n - 1]);
            }
            let mut chars = name.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else { return None };
            if c.is_ascii_alphabetic() {
                return Some(Key::LETTERS[(c.to_ascii_uppercase() as u8 - b'A') as usize]);
            }
            if c.is_ascii_digit() {
                return Some(Key::DIGITS[(c as u8 - b'0') as usize]);
            }
            return None;
        }
    };
    Some(named)
}

/// Runs `program` driven from standard input, answering on standard output,
/// until a `quit` step or the end of the input.
pub fn run(program: impl Program, files: AppFiles) -> std::process::ExitCode {
    run_shared(Arc::new(program), files)
}

/// Runs a program held elsewhere too (one the C interface gives) driven
/// from standard input, as `run` does.
pub fn run_shared(program: Arc<dyn Program>, files: AppFiles) -> std::process::ExitCode {
    // The fonts the project carries come before the system's, as on the desktop.
    crate::fonts::carry_fallbacks(files.resources);
    let mut driven = Driven::start(program, files);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let step: Value = match serde_json::from_str(&line) {
            Ok(step) => step,
            Err(e) => json!({ "do": "", "error": e.to_string() }),
        };
        let answer = driven.step(&step);
        if writeln!(stdout, "{answer}").and_then(|_| stdout.flush()).is_err() {
            break;
        }
        if step.get("do").and_then(Value::as_str) == Some("quit") {
            break;
        }
    }
    driven.screen.dispose();
    std::process::ExitCode::SUCCESS
}

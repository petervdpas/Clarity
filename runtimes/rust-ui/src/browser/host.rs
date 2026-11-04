//! The browser host's screen: the program's windows on one screen, as the
//! desktop shows them in one desktop window, a dialog over the window and a
//! window `in new window` in place. What the person does in the page comes
//! in by the host's methods, in CSS pixels of the canvas (the screen's
//! units); what only the page can do goes out as messages (`page`). The
//! drawing surface is the caller's: WebGL in the browser (`web`), Skia's
//! raster surface in tests.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clarity_runtime::program::Program;
use serde_json::{Value, json};

use crate::app_files::AppFiles;
use crate::element::Composition;
use crate::files::ChosenFile;
use crate::geometry::Rect;
use crate::headless::Driven;
use crate::input::{Cursor, KeyInput, Modifiers, PointerButton, PressCounter};
use crate::window::WindowStack;

use super::input::{button_of, cursor_css, key_of, modifiers_of, typed_text, wheel_of};
use super::page::{PageClipboard, PageFiles, PageSound, Sounds, Tell};

/// A pointer event of the page, by its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Press,
    Release,
    /// The pointer left the canvas.
    Leave,
    /// The page took the press away (a touch turned into a scroll).
    Cancel,
}

impl PointerKind {
    /// The kind the page sends as a number: 0 move, 1 press, 2 release, 3 leave, 4 cancel.
    pub fn of(kind: i32) -> Option<PointerKind> {
        Some(match kind {
            0 => PointerKind::Move,
            1 => PointerKind::Press,
            2 => PointerKind::Release,
            3 => PointerKind::Leave,
            4 => PointerKind::Cancel,
            _ => return None,
        })
    }
}

/// The program's windows on the page's canvas.
pub struct BrowserHost {
    driven: Driven,
    program: Arc<dyn Program>,
    kept: BTreeMap<String, String>,
    asker: Option<crate::later::AskLater>,
    /// What owns the files the screen is made of, when the host was given it.
    owned: Option<super::OwnedFiles>,
    tell: Tell,
    started: Instant,
    presses: PressCounter,
    clipboard: Rc<RefCell<Option<String>>>,
    sounds: Rc<RefCell<Sounds>>,
    files: Arc<PageFiles>,
    /// Where the pointer last went down: a file the page gives is taken by a click there again.
    pressed_at: Option<(f64, f64)>,
    title: Option<String>,
    cursor: Option<Cursor>,
    caret: Option<Option<Rect>>,
    ended: bool,
    /// The page's accessible mirror (A1 phase 2): none while it is off, else the tree it was last told.
    access: Option<Option<crate::access::AccessTree>>,
}

impl BrowserHost {
    /// Starts `program` with its files on a canvas of `width` by `height`
    /// CSS pixels at `scale` device pixels each, light or `dark` as the
    /// browser shows; `tell` reaches the page.
    pub fn start(program: Arc<dyn Program>, files: AppFiles, width: f64, height: f64, scale: f32, dark: bool, tell: Tell) -> BrowserHost {
        BrowserHost::start_kept(program, files, width, height, scale, dark, tell, BTreeMap::new())
    }

    /// Starts as `start` does, with the person's choices (language, theme,
    /// columns) the page kept from an earlier visit (`kept`); each choice
    /// made goes to the page to keep (`choices`).
    #[allow(clippy::too_many_arguments)]
    pub fn start_kept(program: Arc<dyn Program>, files: AppFiles, width: f64, height: f64, scale: f32, dark: bool, tell: Tell, kept: BTreeMap<String, String>) -> BrowserHost {
        let mut driven = BrowserHost::driven(program.clone(), files, &tell, kept.clone());
        let clipboard: Rc<RefCell<Option<String>>> = Rc::default();
        let sounds: Rc<RefCell<Sounds>> = Rc::default();
        let page_files = Arc::new(PageFiles::new(tell.clone()));
        crate::files::set_system(page_files.clone());
        crate::parts::sound::set_output(Rc::new(PageSound::new(tell.clone(), sounds.clone())));
        let opener = tell.clone();
        // Only what a link opens on the desktop (`address::opens`): never javascript:, data: and the like.
        crate::window::address::set_opener(Some(Rc::new(move |address: &str| {
            if crate::window::address::opens(address) {
                opener(json!({ "tell": "open", "address": address }));
            }
        })));
        let screen = driven.screen();
        screen.set_clipboard(Box::new(PageClipboard::new(tell.clone(), clipboard.clone())));
        screen.set_dark(dark);
        screen.resize(width.max(1.0), height.max(1.0), scale);
        let mut host = BrowserHost {
            driven,
            program,
            kept,
            asker: None,
            owned: None,
            tell,
            started: Instant::now(),
            presses: PressCounter::new(),
            clipboard,
            sounds,
            files: page_files,
            pressed_at: None,
            title: None,
            cursor: None,
            caret: None,
            ended: false,
            access: None,
        };
        host.tell_changes();
        host
    }

    fn driven(program: Arc<dyn Program>, files: AppFiles, tell: &Tell, kept: BTreeMap<String, String>) -> Driven {
        crate::fonts::carry_fallbacks(files.resources);
        let keeps = tell.clone();
        let keeper: crate::choices::Keeper = Arc::new(move |choices: &BTreeMap<String, String>| keeps(json!({ "tell": "choices", "choices": choices })));
        Driven::start_keeping(program, files, Some((kept, keeper)))
    }

    /// Starts the program again from new files (its windows' markup, its
    /// themes, its model) on the same canvas, at the same size, light or
    /// dark as it was: how the IDE's designer and preview follow a change of
    /// the design without loading the library again. The program is asked
    /// to start again (`started`, `start_app`, its main window).
    pub fn reload(&mut self, files: AppFiles) {
        self.reload_owned(files, None);
    }

    /// The files the host started with are owned by `owned`, kept until the next reload.
    pub fn keep_files(&mut self, owned: super::OwnedFiles) {
        self.owned = Some(owned);
    }

    /// The person's choices (a language, a theme) the next reload starts with:
    /// the IDE's designer and preview show the language chosen there.
    pub fn choose(&mut self, kept: BTreeMap<String, String>) {
        self.kept = kept;
    }

    /// `reload` with files owned by `owned`: the files before are given up
    /// once the screen made of them is, so reloads do not build up.
    pub fn reload_owned(&mut self, files: AppFiles, owned: Option<super::OwnedFiles>) {
        let (width, height, scale, dark) = {
            let screen = self.screen();
            (screen.width(), screen.height(), screen.scale(), screen.dark())
        };
        self.driven = BrowserHost::driven(self.program.clone(), files, &self.tell, self.kept.clone());
        let screen = self.driven.screen();
        screen.set_clipboard(Box::new(PageClipboard::new(self.tell.clone(), self.clipboard.clone())));
        screen.set_dark(dark);
        screen.resize(width, height, scale);
        if let Some(asker) = self.asker.clone() {
            screen.set_asker(asker);
        }
        // The screen made of the files before is gone: so may they be.
        drop(std::mem::replace(&mut self.owned, owned));
        self.title = None;
        self.cursor = None;
        self.caret = None;
        self.ended = false;
        // A new screen: the page's mirror starts again from its whole tree.
        if self.access.is_some() {
            self.access = Some(None);
        }
        self.screen().take_later();
        self.screen().invalidate();
        self.tell_changes();
    }

    /// The page keeps an accessible mirror of the screen (true) or not:
    /// while on, each drawn frame tells it what changed in the
    /// accessibility tree (`access`, see `access::mirror::update`), the
    /// whole tree first.
    pub fn set_access(&mut self, on: bool) {
        self.access = on.then_some(None);
        if on {
            self.screen().invalidate();
        }
    }

    /// What a screen reader asked of the mirror's node `node` (its id as
    /// text): `action` by its word (`click`, `setValue`...), with `value`
    /// for setValue. Done as the person's input (`access::act`); false when
    /// there is no such node or it does not take that action now.
    pub fn access_act(&mut self, node: &str, action: &str, value: Option<&str>) -> bool {
        let (Ok(id), Some(action)) = (node.parse::<u64>(), crate::access::AccessAction::from_word(action)) else { return false };
        let done = crate::access::act::act(self.screen(), id, action, value);
        self.after();
        done
    }

    /// Tells the page what changed in the tree since it was told last, when it keeps a mirror.
    fn tell_access(&mut self) {
        let Some(last) = self.access.as_ref() else { return };
        let now = crate::access::tree(self.driven.screen());
        if let Some(update) = crate::access::mirror::update(last.as_ref(), &now) {
            (self.tell)(json!({ "tell": "access", "update": update }));
        }
        self.access = Some(Some(now));
    }

    /// Where the design's parts and grids show (the driven `layout` step), for the IDE's designer.
    pub fn layout(&mut self) -> Value {
        Value::Object(self.driven.layout())
    }

    /// A long call's request goes to the server through `asker` without
    /// waiting: the page draws on, its bar asks how far it is and Stop
    /// stops it, and the answer is taken when it comes.
    pub fn set_asker(&mut self, asker: crate::later::AskLater) {
        self.asker = Some(asker.clone());
        self.screen().set_asker(asker);
    }

    /// The screen, for a test's look and for drawing.
    pub fn screen(&mut self) -> &mut crate::screen::Screen {
        self.driven.screen()
    }

    /// The canvas is `width` by `height` CSS pixels now, at `scale` device pixels each.
    pub fn resize(&mut self, width: f64, height: f64, scale: f32) {
        self.screen().resize(width.max(1.0), height.max(1.0), scale);
    }

    /// The browser shows dark (true) or light.
    pub fn set_dark(&mut self, dark: bool) {
        self.screen().set_dark(dark);
    }

    /// The pointer at `x`, `y` (CSS pixels of the canvas), with `button` (the event's) and the keys held (`held`, the page's bits).
    pub fn pointer(&mut self, kind: PointerKind, x: f64, y: f64, button: i32, held: u32) {
        let held = modifiers_of(held);
        match kind {
            PointerKind::Move => self.screen().pointer_moved(x, y),
            PointerKind::Leave => self.screen().pointer_left(),
            PointerKind::Cancel => self.screen().pointer_cancelled(),
            PointerKind::Press => {
                let Some(button) = button_of(button) else { return };
                let elapsed = self.started.elapsed().as_millis() as i64;
                let clicks = self.presses.press(x, y, button, elapsed);
                if button == PointerButton::Left {
                    self.pressed_at = Some((x, y));
                }
                let screen = self.screen();
                screen.pointer_moved(x, y);
                screen.pointer_pressed_with(x, y, button, clicks, held);
            }
            PointerKind::Release => {
                let Some(button) = button_of(button) else { return };
                self.screen().pointer_released(x, y, button);
            }
        }
        self.after();
    }

    /// The wheel turned at `x`, `y` by the event's `dx`, `dy` in its `mode` (`deltaMode`).
    pub fn wheel(&mut self, x: f64, y: f64, dx: f64, dy: f64, mode: u32, held: u32) {
        let page = (self.screen().width(), self.screen().height());
        let (dx, dy) = wheel_of(dx, dy, mode, page);
        let held = modifiers_of(held);
        self.screen().wheel(x, y, dx, dy, held);
        self.after();
    }

    /// A key went down (or up) with the keyboard event's `key` and `code`;
    /// a key that writes text the page sends as typed text (`text`) itself.
    /// Whether the screen took it.
    pub fn key(&mut self, down: bool, key: &str, code: &str, held: u32) -> bool {
        let input = KeyInput::new(key_of(key, code), modifiers_of(held));
        let taken = if down {
            self.screen().key_pressed(input)
        } else {
            self.screen().key_released(input);
            false
        };
        self.after();
        taken
    }

    /// Text typed (the page's input, or an input method's committed text).
    pub fn text(&mut self, text: &str) {
        if let Some(text) = typed_text(text, Modifiers::NONE) {
            let screen = self.screen();
            screen.compose(None);
            screen.text_typed(&text);
        }
        self.after();
    }

    /// An input method composes `text` (none when it ended), the marked part in bytes of it.
    pub fn compose(&mut self, text: Option<&str>, marked: Option<(usize, usize)>) {
        let composing = text.filter(|t| !t.is_empty()).map(|text| Composition { text: text.to_string(), marked });
        self.screen().compose(composing);
        self.after();
    }

    /// The page's clipboard holds `text` (told as the person pastes).
    pub fn pasted(&mut self, text: &str) {
        *self.clipboard.borrow_mut() = Some(text.to_string());
    }

    /// A sound the page played reached its end.
    pub fn sound_ended(&mut self, id: u64) {
        self.sounds.borrow_mut().ended(id);
    }

    /// The person chose a file in the page's file input: the click that asked for it is done again, and the picker gives it.
    pub fn file_chosen(&mut self, name: &str, bytes: Vec<u8>) {
        self.files.give(ChosenFile { name: crate::wire::clean_name(name), bytes });
        if let Some((x, y)) = self.pressed_at {
            let screen = self.screen();
            screen.pointer_pressed_with(x, y, PointerButton::Left, 1, Modifiers::NONE);
            screen.pointer_released(x, y, PointerButton::Left);
        }
        self.after();
    }

    /// A frame of the page's: time goes on, work done is taken. Whether the screen is to be drawn now.
    pub fn frame(&mut self) -> bool {
        let screen = self.screen();
        screen.take_later();
        screen.wake_due(Instant::now());
        let draw = screen.take_redraw() || screen.animating() > 0;
        screen.take_wakes_changed();
        self.tell_changes();
        draw
    }

    /// How long until the screen wants a frame again by itself; none while it waits for the person.
    pub fn next_wake(&mut self) -> Option<Duration> {
        let screen = self.screen();
        if screen.animating() > 0 {
            return Some(Duration::ZERO);
        }
        screen.next_wake().map(|at| at.saturating_duration_since(Instant::now()))
    }

    /// Draws the screen on `canvas`, which is the canvas's whole size in device pixels.
    pub fn draw(&mut self, canvas: &skia_safe::Canvas) {
        self.screen().draw(canvas);
        // Laid out as drawn: the mirror's places are those shown.
        self.tell_access();
    }

    /// A step as a driven program takes it (DESIGN 1.6: `read`, `click`, `type`...): how a test looks at the page's screen.
    pub fn step(&mut self, step: &Value) -> Value {
        let answer = self.driven.step(step);
        self.after();
        answer
    }

    /// Whether the program's windows closed.
    pub fn ended(&self) -> bool {
        self.ended
    }

    fn after(&mut self) {
        self.screen().take_later();
        self.tell_changes();
    }

    /// Tells the page what changed that it shows: the title, the pointer's shape, where an input method writes, the end.
    fn tell_changes(&mut self) {
        let screen = self.driven.screen();
        let (title, ended, quit) = match screen.root().as_any().downcast_ref::<WindowStack>() {
            Some(stack) => (Some(stack.title()), stack.ended(), stack.quitting()),
            None => (None, false, false),
        };
        let cursor = screen.cursor();
        screen.take_cursor_changed();
        let caret = screen.caret_area();
        if title.is_some() && title != self.title {
            (self.tell)(json!({ "tell": "title", "title": title }));
            self.title = title;
        }
        if self.cursor != Some(cursor) {
            (self.tell)(json!({ "tell": "cursor", "cursor": cursor_css(cursor) }));
            self.cursor = Some(cursor);
        }
        if self.caret != Some(caret) {
            let at = caret.map(|c| json!([c.x, c.y, c.width, c.height]));
            (self.tell)(json!({ "tell": "caret", "at": at }));
            self.caret = Some(caret);
        }
        if (ended || quit) && !self.ended {
            self.ended = true;
            (self.tell)(json!({ "tell": "ended", "quit": quit }));
        }
    }
}

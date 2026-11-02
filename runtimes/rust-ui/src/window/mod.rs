//! The windows of a program on Clarity.UI: each window's view (its parts in
//! its grid, read from its `.cxaml`), the host its parts read and change
//! (the runtime's `WindowState`, run through the program's `Program`), and
//! the stack of windows in one desktop window: `open` shows a window in
//! place, on top of the one before; `close` goes back to the one below,
//! which is told it is on top again. Closing the last one ends the program.
//! The messages belong to the stack, so one said just before a window closes
//! shows in the window below.

pub mod host;
pub mod messages;
pub mod paths;
pub mod view;

use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Level, Message, Text};
use serde_json::Map;

use crate::app_files::AppFiles;
use crate::element::{self, Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::markup;
use crate::overlay::Overlay;
use crate::parts::PartHost;

pub use host::{Effect, ProgramHost};
pub use messages::{NoteList, SaidDialog};
pub use view::WindowView;

/// One open window: its host, and its view while it is not on top (the top one is the stack's child).
struct Shown {
    host: Rc<ProgramHost>,
    parked: Option<Box<dyn Element>>,
}

/// The windows of a program, the one on top shown: the root of the desktop window's screen.
pub struct WindowStack {
    node: Node,
    program: Arc<dyn Program>,
    files: AppFiles,
    stack: Vec<Shown>,
    /// The warnings and errors waiting for their dialog, the first one shown.
    said: Vec<Message>,
    saying: Option<ElementId>,
    /// Effects of the start, before the main window is on a screen.
    early: Vec<Effect>,
    /// How often the window on top's values had changed when the screen last drew them.
    seen: u64,
    ended: bool,
}

impl WindowStack {
    /// Starts the program: its texts load, its `on start` runs and its main window opens.
    pub fn start(program: Arc<dyn Program>, files: AppFiles) -> WindowStack {
        clarity_runtime::i18n::load(files.model);
        let mut node = Node::new();
        node.add(Box::new(NoteList::default()));
        let mut stack = WindowStack { node, program, files, stack: Vec::new(), said: Vec::new(), saying: None, early: Vec::new(), seen: 0, ended: false };
        let started = stack.program.started();
        match stack.program.start_app() {
            Ok(reply) => {
                if !reply.messages.is_empty() {
                    stack.early.push(Effect::Say(reply.messages));
                }
            }
            Err(text) => stack.early.push(Effect::Say(vec![Message { level: Level::Error, text }])),
        }
        stack.open_window(&started.main, Map::new(), None);
        stack
    }

    /// The desktop window's title now.
    pub fn title(&self) -> String {
        self.files.title.to_string()
    }

    /// The size the program asks its desktop window to open at; none for the host's own.
    pub fn wanted_size(&self) -> Option<(f64, f64)> {
        (self.files.width > 0.0 && self.files.height > 0.0).then_some((self.files.width, self.files.height))
    }

    /// Whether the last window closed: the host ends.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// The program it runs.
    pub fn program(&self) -> &Arc<dyn Program> {
        &self.program
    }

    /// The names of the open windows, from the bottom to the top.
    pub fn windows(&self) -> Vec<String> {
        self.stack.iter().map(|s| s.host.window()).collect()
    }

    /// The host of the window on top.
    pub fn current(&self) -> Option<&Rc<ProgramHost>> {
        self.stack.last().map(|s| &s.host)
    }

    /// The view of the window on top.
    pub fn view(&self) -> Option<&WindowView> {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<WindowView>())
    }

    fn view_index(&self) -> Option<usize> {
        self.node.children().iter().position(|c| c.as_any().is::<WindowView>())
    }

    fn notes_mut(&mut self) -> &mut NoteList {
        self.node.children_mut().iter_mut().find_map(|c| c.as_any_mut().downcast_mut::<NoteList>()).expect("the stack holds its notes")
    }

    /// The info messages on screen.
    pub fn notes(&self) -> Vec<Message> {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<NoteList>()).map(NoteList::notes).unwrap_or_default()
    }

    /// The warning or error its dialog shows now, and those waiting after it.
    pub fn said(&self) -> &[Message] {
        &self.said
    }

    /// Makes the view of a window from its markup; one the program does not carry, or that does not read, says why in its place.
    fn make_view(&self, host: &Rc<ProgramHost>, name: &str) -> WindowView {
        let as_host: Rc<dyn PartHost> = host.clone();
        let Some(markup) = self.files.window(name) else {
            let why = locale::fill(&locale::translate("windowMissing", "The program carries no window {window}."), "window", name);
            return WindowView::failed(why, as_host);
        };
        match markup::read(markup) {
            Ok(spec) => WindowView::new(&spec, as_host),
            Err(err) => WindowView::failed(err.0, as_host),
        }
    }

    /// Opens window `name` on top: the program opens it (its `on open` runs), then it shows.
    fn open_window(&mut self, name: &str, with: Map<String, serde_json::Value>, cx: Option<&mut Cx>) {
        let reply = match self.program.open_window(name, with) {
            Ok(reply) => reply,
            Err(text) => {
                self.early.push(Effect::Say(vec![Message { level: Level::Error, text }]));
                return;
            }
        };
        let parts = self.files.window(name).and_then(|m| markup::read(m).ok()).map(|s| s.parts).unwrap_or_default();
        let host = Rc::new(ProgramHost::new(self.program.clone(), self.files, reply.state.clone(), &parts));
        host.apply(reply);
        let view = self.make_view(&host, name);
        // The window below is parked while this one is on top.
        if let Some(index) = self.view_index() {
            let mut below = self.node.children_mut().remove(index);
            if let Some(cx) = cx {
                element::detach_all(below.as_mut(), cx);
                cx.focus(None);
            }
            if let Some(top) = self.stack.last_mut() {
                top.parked = Some(below);
            }
        }
        self.node.children_mut().insert(0, Box::new(view));
        self.stack.push(Shown { host, parked: None });
    }

    /// Closes the window on top: its `on close` runs, the window below shows and is told; the last one ends the program.
    fn close_top(&mut self, cx: &mut Cx) {
        let Some(top) = self.stack.pop() else { return };
        let effects = top.host.closing();
        if let Some(index) = self.view_index() {
            self.node.remove(index, cx);
        }
        cx.focus(None);
        if self.stack.is_empty() {
            self.ended = true;
            cx.invalidate();
            return;
        }
        if let Some(view) = self.stack.last_mut().and_then(|s| s.parked.take()) {
            self.node.children_mut().insert(0, view);
        }
        let below = self.stack.last().map(|s| s.host.clone());
        if let Some(below) = below {
            below.returned();
        }
        self.take(effects, cx);
        cx.invalidate();
    }

    /// Does what replies asked: messages show, windows open and close, text goes on the clipboard.
    fn take(&mut self, effects: Vec<Effect>, cx: &mut Cx) {
        for effect in effects {
            match effect {
                Effect::Say(messages) => {
                    for message in messages {
                        if message.level == Level::Info {
                            let until = cx.now() + messages::INFO_DURATION;
                            self.notes_mut().add(message, until, cx);
                        } else {
                            self.said.push(message);
                        }
                    }
                }
                Effect::Open(window, with) => {
                    self.open_window(&window, with, Some(cx));
                    cx.invalidate();
                }
                Effect::Close => self.close_top(cx),
                Effect::Copy(text) => cx.clipboard().set_text(&text),
            }
        }
        self.say_next(cx);
    }

    /// The next warning or error, in Clarity's dialog over the window, one at a time.
    fn say_next(&mut self, cx: &mut Cx) {
        if self.saying.is_some() {
            return;
        }
        let Some(message) = self.said.first().cloned() else { return };
        let dialog = SaidDialog::new(message);
        let ok = dialog.ok();
        let overlay = Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(dialog)) };
        self.saying = Some(cx.open(overlay));
        cx.focus(Some(ok));
    }

    /// Says something among the messages, as a handler's reply does.
    pub fn say(&mut self, level: Level, text: Text, cx: &mut Cx) {
        self.take(vec![Effect::Say(vec![Message { level, text }])], cx);
    }

    /// The part with the focus, or the first part it is in: its name.
    fn focused_part(&self, focused: Option<ElementId>) -> Option<String> {
        let focused = focused?;
        let ids = element::chain(self, focused)?;
        ids.iter().rev().filter_map(|id| element::find(self, *id)).find_map(|e| e.as_part().and_then(|p| p.base().spec.name().map(str::to_string)))
    }
}

impl Element for WindowStack {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn after_event(&mut self, cx: &mut Cx) {
        let mut effects = std::mem::take(&mut self.early);
        if let Some(host) = self.stack.last().map(|s| s.host.clone()) {
            host.flush();
            effects.extend(host.take_effects());
            // The window's values changed: the screen draws them again.
            if host.changes() != self.seen {
                self.seen = host.changes();
                cx.invalidate();
            }
        }
        if !effects.is_empty() {
            self.take(effects, cx);
            cx.invalidate();
        }
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        if self.saying == Some(overlay) {
            self.saying = None;
            if !self.said.is_empty() {
                self.said.remove(0);
            }
            self.say_next(cx);
            cx.invalidate();
        }
    }

    /// The window's own keys: the window on top's (shortcuts, the default and
    /// cancel buttons), then Ctrl+Z takes back the last edit of the scope of
    /// the part with the focus, Ctrl+Y or Ctrl+Shift+Z does it again; a part
    /// that takes the key itself (typing not yet an edit) has it first.
    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let Some(index) = self.view_index() else { return false };
        if self.node.children_mut()[index].window_key(key, cx) {
            return true;
        }
        let command = key.control() || key.meta();
        if !command || key.alt() {
            return false;
        }
        let back = key.key == Key::Z && !key.shift();
        let again = key.key == Key::Y || (key.key == Key::Z && key.shift());
        if !back && !again {
            return false;
        }
        let focused = cx.focused();
        if let Some(e) = focused.and_then(|f| element::find_mut(self, f)) {
            if e.on_key(key, cx) {
                return true;
            }
        }
        let part = self.focused_part(focused);
        if let (Some(host), Some(part)) = (self.current().cloned(), part) {
            if let Some(scope) = host.scope(&part) {
                host.undo(&scope, back);
            }
        }
        true
    }
}

/// Keys the stack takes itself, for a host that wires them by hand: none beyond `window_key` yet.
pub fn plain(key: Key) -> KeyInput {
    KeyInput::new(key, Modifiers::NONE)
}

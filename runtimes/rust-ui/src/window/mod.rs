//! The windows of a program on Clarity.UI: each window's view (its parts in
//! its grid, read from its `.cxaml`), the host its parts read and change
//! (the runtime's `WindowState`, run through the program's `Program`), and
//! the stack of windows in one desktop window: `open` shows a window in
//! place, on top of the one before; `close` goes back to the one below,
//! which is told it is on top again. Closing the last one ends the program,
//! or the desktop window of its own it was in.
//!
//! A dialog, and a target marked `in new window`, open in a desktop window
//! of their own where the host gives them (`Running::set_apart`, then
//! `take_aparts`); elsewhere a dialog shows over the window as a modal
//! overlay and a new window opens in place. A question a handler asks shows
//! in Clarity's own dialog, and a handler of a program that watches its long
//! calls runs off the screen's thread with the bar of a long call. The
//! messages belong to the stack, so one said just before a window closes
//! shows in the window below. When the language changes every window is
//! made again, so its texts show in it. When the databases did not open,
//! what keeps the connection shows first (manual/data.settings.md); when people sign
//! in first, the sign-in window (docs/ledger/signin.md).

pub mod address;
pub mod history;
pub mod host;
pub mod messages;
pub mod paths;
pub mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use clarity_runtime::keys::PersonKeys;
use clarity_runtime::program::{Answer, Program, Unconnected};
use clarity_runtime::ui::{Ask, AskForm, Level, Message, Reply, Text, WindowState};
use serde_json::{Map, Value};

use crate::app_files::AppFiles;
use crate::dialogs::ask::{self, Answered, AskDialog};
use crate::dialogs::keys::{self, KeysWindow};
use crate::dialogs::long_call::LongCallBar;
use crate::dialogs::over::DialogOverlay;
use crate::element::{self, Cx, Element, ElementId, Node};
use crate::facts::ModelFacts;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::markup;
use crate::overlay::Overlay;
use crate::parts::PartHost;

pub use host::{Call, Effect, ProgramHost};
pub use messages::{ToastList, SaidDialog};
pub use view::WindowView;

/// What every window of a program shares, in every desktop window: the
/// program, its files and model, the person's keys, whether a long call
/// runs, and whether the host opens windows of their own.
pub struct Running {
    pub program: Arc<dyn Program>,
    pub files: AppFiles,
    pub facts: Arc<ModelFacts>,
    keys: RefCell<Option<PersonKeys>>,
    calling: Cell<bool>,
    apart: Cell<bool>,
}

impl Running {
    pub fn new(program: Arc<dyn Program>, files: AppFiles) -> Rc<Running> {
        Rc::new(Running {
            facts: ModelFacts::of(files.model),
            program,
            files,
            keys: RefCell::new(None),
            calling: Cell::new(false),
            apart: Cell::new(false),
        })
    }

    /// Whether a long call runs now: no other handler runs meanwhile.
    pub fn calling(&self) -> bool {
        self.calling.get()
    }

    /// The host opens dialogs, the keys window and new windows in desktop windows of their own.
    pub fn set_apart(&self, apart: bool) {
        self.apart.set(apart);
    }

    /// The person's keys, as the settings keep them (read once).
    pub fn person_keys(&self) -> PersonKeys {
        self.keys.borrow_mut().get_or_insert_with(|| PersonKeys::read(&self.program.read_settings())).clone()
    }

    /// The keys a command answers: the person's where they set their own, else the design's.
    pub fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        if self.keys.borrow().is_none() {
            self.person_keys();
        }
        self.keys.borrow().as_ref().map(|k| k.of(command, designed)).unwrap_or_default()
    }

    /// Keeps the person's keys with the program's settings, the other settings as they were.
    pub fn keep_keys(&self, keys: &PersonKeys) -> Answer<()> {
        *self.keys.borrow_mut() = Some(keys.clone());
        let mut settings = self.program.read_settings();
        keys.keep(&mut settings);
        self.program.keep_settings(settings)
    }

    /// Clarity's keys window over the program's commands; `root` when it is a screen's root.
    pub fn keys_window(running: &Rc<Running>, root: bool) -> KeysWindow {
        let keeper = running.clone();
        let keep: keys::KeepKeys = Rc::new(move |keys| {
            if let Err(why) = keeper.keep_keys(keys) {
                // Fail safe: keys that cannot be kept still work in this run.
                eprintln!("{}", why.show());
            }
        });
        KeysWindow::new(keys::catalog(running.files.model), running.person_keys(), keep, root)
    }

    /// A window's title as shown: its own, else the program's.
    /// The title a window shows: its own, else the app's (as the app's
    /// markup says it), else the project's name; a key in the person's language.
    pub fn title_of(&self, window: &str) -> String {
        let own = self.facts.window(window).and_then(|w| w.title.clone()).filter(|t| !t.is_empty());
        match own.or_else(|| self.files.app_title()) {
            Some(title) => crate::facts::shown(&title, &title),
            None => self.files.title.to_string(),
        }
    }
}

/// A question whose answer is the value a dialog window closes with: the handler it runs, with its values.
#[derive(Debug, Clone, PartialEq)]
pub struct Asked {
    pub handler: String,
    pub values: Vec<Value>,
}

/// A window to open in a desktop window of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct ApartWindow {
    pub window: String,
    pub with: Map<String, Value>,
    /// The open window that opened it (its instance), told when it closes.
    pub opener: u64,
    pub asked: Option<Asked>,
    /// It is a dialog: sized to it, and the window under it takes nothing while it is open.
    pub dialog: bool,
}

/// What the stack asks the host to open in a desktop window of its own.
#[derive(Debug, Clone, PartialEq)]
pub enum Apart {
    Window(ApartWindow),
    Keys,
    /// The main window, in a desktop window of its own, with what `on start`
    /// answered: a splash that doubles as the sign-in window was signed in on.
    Main(Answer<Reply>),
    /// The sign-in window, after a signout on the desktop: in a desktop
    /// window of its own as at the start (`signin_window`).
    Signin(String),
}

/// A desktop window of its own closed: what the window that opened it is told.
#[derive(Debug, Clone, PartialEq)]
pub struct ApartClosed {
    pub opener: u64,
    pub result: Option<Value>,
    pub asked: Option<Asked>,
    pub said: Vec<Message>,
}

/// One open window: its host, and its view while it is not on top (the top one is the stack's child).
struct Shown {
    host: Rc<ProgramHost>,
    parked: Option<Box<dyn Element>>,
}

/// A dialog over the window: its host, its overlay, the window that opened it, and the question it answers.
struct Dialog {
    host: Rc<ProgramHost>,
    overlay: ElementId,
    opener: Option<u64>,
    asked: Option<Asked>,
}

/// A grant an AI agent asks for: the dialog's title and lines, and where the choice goes.
struct Granting {
    title: String,
    lines: Vec<String>,
    answer: Box<dyn FnOnce(crate::dialogs::GrantChoice)>,
}

/// The question shown now: its overlay, what the person answers, and the window that asked.
struct Asking {
    overlay: ElementId,
    answered: Answered,
    ask: Ask,
    host: Rc<ProgramHost>,
}

/// While the databases are not open (manual/data.settings.md): what to show, and the settings when it showed.
struct Unconnecting {
    unconnected: Unconnected,
    settings: Map<String, Value>,
}

/// What a long call gives back on the screen's thread, and the number of its watch.
struct LongDone(Answer<Reply>, u64);

/// The part of a splash page that says what is loading.
pub const SPLASH_STATUS: &str = "status";

/// The number of the next long call's watch (`Program::start_watch`), unique in the process.
static NEXT_WATCH: AtomicU64 = AtomicU64::new(1);

/// The windows of a program, the one on top shown: the root of a desktop window's screen.
mod agents;
mod asking;
mod opening;
mod signin;
mod starting;

pub struct WindowStack {
    node: Node,
    running: Rc<Running>,
    stack: Vec<Shown>,
    dialogs: Vec<Dialog>,
    /// The warnings and errors waiting for their dialog, the first one shown.
    said: Vec<Message>,
    saying: Option<ElementId>,
    /// The questions waiting their turn, and the one shown.
    asks: Vec<(Ask, Rc<ProgramHost>)>,
    asking: Option<Asking>,
    /// What AI agents ask the person to allow (docs/ledger/app-mcp.md), one at a time.
    grants: Vec<Granting>,
    granting: Option<(ElementId, crate::dialogs::grant::Granted, Granting)>,
    /// What the Undo of an agent's change answered, until the window takes it.
    agent_undone: Rc<RefCell<Option<Answer<()>>>>,
    keys_overlay: Option<ElementId>,
    /// The long call running: the window it runs for, and its bar.
    long: Option<(Rc<ProgramHost>, ElementId)>,
    /// Effects of the start, before the main window is on a screen.
    early: Vec<Effect>,
    /// How often the windows' values had changed when the screen last drew them.
    seen: u64,
    ended: bool,
    quitting: bool,
    aparts: Vec<Apart>,
    /// The window that opened this desktop window of its own, and the question it answers.
    opener: Option<(u64, Option<Asked>)>,
    closed: Option<ApartClosed>,
    /// This desktop window of its own shows a dialog: it is sized to it.
    dialog: bool,
    unconnected: Option<Unconnecting>,
    /// The sign-in window, once the program said people sign in first; shown again after `signout`.
    signin: Option<String>,
    /// The splash is the sign-in window too: it stays once `on start` answered, until someone signed in (`signed`).
    splash_signs_in: bool,
    signed: bool,
    /// This desktop window of its own shows the sign-in window (or the splash that is it):
    /// a sign-in on it asks for the main window in a desktop window of its own.
    own_signin_window: bool,
    language: String,
    /// The app starting behind its splash page in this same screen (the browser's whole page).
    starting: Option<InPlace>,
}

/// While the app starts behind its splash page in the same screen: how long
/// the splash shows at least and since when, whether `on start` was asked,
/// and what it answered once it did.
struct InPlace {
    at_least: std::time::Duration,
    opened: std::time::Instant,
    asked: bool,
    reply: Option<Answer<Reply>>,
}

/// What the app's `on start` answered, asked without waiting.
struct StartDone(Answer<Reply>);

/// The view of window `name` over `host`, from its markup; one the program does not carry, or that does not read, says why in its place.
pub fn view_of(files: AppFiles, host: &Rc<ProgramHost>, name: &str) -> WindowView {
    let as_host: Rc<dyn PartHost> = host.clone();
    let Some(markup) = files.window(name) else {
        let why = locale::fill(&locale::translate("windowMissing", "The program carries no window {window}."), "window", name);
        return WindowView::failed(why, as_host);
    };
    match markup::read(markup) {
        // Every window but a dialog or the splash shows the app's frame where it has none of its own.
        Ok(spec) => WindowView::new(&spec, as_host).with_frame(crate::frame::app_frame_of(&files, name)),
        Err(err) => WindowView::failed(err.0, as_host),
    }
}

impl WindowStack {
    fn empty(running: Rc<Running>) -> WindowStack {
        let mut node = Node::new();
        node.add(Box::new(ToastList::default()));
        WindowStack {
            node,
            running,
            stack: Vec::new(),
            dialogs: Vec::new(),
            said: Vec::new(),
            saying: None,
            asks: Vec::new(),
            asking: None,
            grants: Vec::new(),
            granting: None,
            agent_undone: Rc::default(),
            keys_overlay: None,
            long: None,
            early: Vec::new(),
            seen: 0,
            ended: false,
            quitting: false,
            aparts: Vec::new(),
            opener: None,
            closed: None,
            dialog: false,
            unconnected: None,
            signin: None,
            splash_signs_in: false,
            signed: false,
            own_signin_window: false,
            language: crate::choices::language(),
            starting: None,
        }
    }

    /// What the window on top's markup says of its desktop window; the system's own window for one that says nothing.
    pub fn window_look(&self) -> crate::markup::WindowLook {
        self.current().map(|h| self.running.files.window_look(&h.window())).unwrap_or_default()
    }

    /// What the splash screen says is loading: the caption of its part `status`.
    pub fn show_status(&self, text: Text) {
        let Some(host) = self.current() else { return };
        let mut state = host.state(Some(SPLASH_STATUS)).unwrap_or_default();
        if state.caption.as_ref() != Some(&text) {
            state.caption = Some(text);
            host.set_state(SPLASH_STATUS, state);
        }
    }

    /// What every window of the program shares.
    pub fn running(&self) -> &Rc<Running> {
        &self.running
    }

    /// The desktop window's title now: the title of the window on top, else the program's.
    pub fn title(&self) -> String {
        match self.stack.last() {
            Some(top) => self.running.title_of(&top.host.window()),
            None => self.running.files.title.to_string(),
        }
    }

    /// The size the program asks its desktop window to open at; none for the host's own.
    /// A dialog of its own is sized to the dialog: as its model says, else 480 wide and as high as its rows.
    pub fn wanted_size(&self) -> Option<(f64, f64)> {
        let top = self.stack.first().map(|s| s.host.window());
        let size = top.as_deref().and_then(|w| self.running.facts.window(w)).and_then(|w| w.size);
        if self.dialog {
            // Measured in the program's own look, as the view shows in it.
            let look = self.running.files.look(false);
            let fonts = crate::fonts::Fonts::new(look.clone());
            return self.view().map(|v| crate::dialogs::over::dialog_size_in(v, size, &fonts, &look));
        }
        if self.opener.is_some() {
            return size;
        }
        // A window whose markup gives its size (a splash, as an Avalonia window's Width and Height) opens at it.
        if let Some(size) = top.as_deref().and_then(|w| self.running.files.window_look(w).size) {
            return Some(size);
        }
        let files = &self.running.files;
        (files.width > 0.0 && files.height > 0.0).then_some((files.width, files.height))
    }

    /// Whether this desktop window of its own shows a dialog.
    pub fn is_dialog(&self) -> bool {
        self.dialog
    }

    /// Whether the last window closed: the host closes the desktop window it is in.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Whether the program ends (the built-in `quit`): the host closes every window.
    pub fn quitting(&self) -> bool {
        self.quitting
    }

    /// The program it runs.
    pub fn program(&self) -> &Arc<dyn Program> {
        &self.running.program
    }

    /// The names of the open windows, from the bottom to the top, the dialogs over them last.
    pub fn windows(&self) -> Vec<String> {
        self.stack.iter().map(|s| s.host.window()).chain(self.dialogs.iter().map(|d| d.host.window())).collect()
    }

    /// The host of the window on top.
    pub fn current(&self) -> Option<&Rc<ProgramHost>> {
        self.stack.last().map(|s| &s.host)
    }

    /// The host of the dialog on top, over the window.
    pub fn dialog_host(&self) -> Option<&Rc<ProgramHost>> {
        self.dialogs.last().map(|d| &d.host)
    }

    /// Whether the open window of number `instance` is in this stack.
    pub fn holds(&self, instance: u64) -> bool {
        self.host_of(instance).is_some()
    }

    fn host_of(&self, instance: u64) -> Option<Rc<ProgramHost>> {
        self.stack.iter().map(|s| &s.host).chain(self.dialogs.iter().map(|d| &d.host)).find(|h| h.instance() == instance).cloned()
    }

    /// The view of the window on top.
    pub fn view(&self) -> Option<&WindowView> {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<WindowView>())
    }

    fn view_index(&self) -> Option<usize> {
        self.node.children().iter().position(|c| c.as_any().is::<WindowView>())
    }

    fn toasts_mut(&mut self) -> &mut ToastList {
        self.node.children_mut().iter_mut().find_map(|c| c.as_any_mut().downcast_mut::<ToastList>()).expect("the stack holds its toasts")
    }

    /// The info messages on screen.
    pub fn toasts(&self) -> Vec<Message> {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<ToastList>()).map(ToastList::toasts).unwrap_or_default()
    }

    /// The warning or error its dialog shows now, and those waiting after it.
    pub fn said(&self) -> &[Message] {
        &self.said
    }

    /// Whether a long call runs, with its bar over the window.
    pub fn long_call(&self) -> bool {
        self.long.is_some()
    }

    /// Makes a window's host from what opening it answered.
    fn host_for(&self, name: &str, reply: Reply) -> Rc<ProgramHost> {
        let files = self.running.files;
        let parts = files.window(name).and_then(|m| markup::read(m).ok()).map(|s| s.parts).unwrap_or_default();
        let host = Rc::new(ProgramHost::new(self.running.program.clone(), files, reply.state.clone(), &parts));
        host.set_running(self.running.clone());
        host.apply(reply);
        host
    }

    /// Clarity's keys window, where the project lets people set their keys: in a desktop window of its own where the host gives them, else over the window.
    fn show_keys(&mut self, cx: &mut Cx) {
        if !self.running.facts.keys_editable() {
            return;
        }
        if self.running.apart.get() {
            self.aparts.push(Apart::Keys);
            return;
        }
        if self.keys_overlay.is_some() {
            return;
        }
        let window = Running::keys_window(&self.running, false);
        let overlay = cx.open(Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(window)) });
        cx.focus(Some(overlay));
        self.keys_overlay = Some(overlay);
    }

    /// A handler of `host` runs off the screen's thread, its long calls
    /// watched: the bar lies over the window until it ended, says how far
    /// it is, and Stop stops it.
    fn start_call(&mut self, host: Rc<ProgramHost>, call: Call, cx: &mut Cx) {
        if self.long.is_some() {
            return;
        }
        // The program knows the call's watch by number: the bar asks it how far the call is and stops it by that number.
        let watch = NEXT_WATCH.fetch_add(1, Ordering::SeqCst);
        let program = self.running.program.clone();
        let (asks, stops) = (program.clone(), program.clone());
        let bar = LongCallBar::watching(Rc::new(move || asks.watch_progress(watch)), Rc::new(move || stops.stop_watch(watch)));
        let bar = cx.open(Overlay::dialog(Box::new(bar)));
        self.running.calling.set(true);
        self.long = Some((host.clone(), bar));
        let state = host.state_now();
        // Where the host sends requests without waiting (the browser), the
        // handler's request names its watch and its answer comes later; else
        // the handler runs on a thread of its own between its watch's start and end.
        let (command, args) = call.wire(&state, watch);
        let answered: crate::later::Answered = Box::new(move |answer| Box::new(LongDone(crate::program_wire::read(command, answer), watch)) as crate::later::Done);
        let work: crate::later::Work = Box::new(move || {
            program.start_watch(watch);
            let answer = call.run(program.as_ref(), state);
            program.end_watch(watch);
            Box::new(LongDone(answer, watch)) as crate::later::Done
        });
        cx.later_ask(self.node.id(), crate::later::Asked { command: command.to_string(), args, answered, work });
    }

    /// Makes every window's view again, as in another language; a dialog over the window makes its own.
    fn remake(&mut self, cx: &mut Cx) {
        let files = self.running.files;
        // The part with the focus keeps it in the window made again, so the next Tab goes on from it.
        let focused = cx.focused().and_then(|f| self.view().and_then(|v| part_of(v, f)));
        let count = self.stack.len();
        for i in 0..count {
            let host = self.stack[i].host.clone();
            host.reshow();
            let view = view_of(files, &host, &host.window());
            if i + 1 == count {
                if let Some(index) = self.view_index() {
                    self.node.remove(index, cx);
                }
                self.node.children_mut().insert(0, Box::new(view));
            } else {
                self.stack[i].parked = Some(Box::new(view));
            }
        }
        for dialog in &self.dialogs {
            dialog.host.reshow();
        }
        if let Some(name) = focused {
            let again = self.view().and_then(|view| {
                let part = element::in_order(view).into_iter().find(|e| e.as_part().and_then(|p| p.base().spec.name()) == Some(name.as_str()))?;
                element::focus_takers(part).into_iter().next()
            });
            if again.is_some() {
                cx.focus(again);
            }
        }
        cx.invalidate();
    }

    /// The part with the focus, or the first part it is in, when it is no button: its name.
    /// The part with the focus, in the window or in a slide-out or popover over it; none for a button.
    fn focused_input(&self, cx: &Cx) -> Option<String> {
        let found = match cx.focused().and_then(|f| focused_part_in(self, f)) {
            Some(part) => Some(part),
            None => cx.focused_part.clone(),
        }?;
        (!found.button).then_some(found.name)
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
        // A stack whose desktop window closed hears nothing more: no language, no replies.
        if self.ended {
            return;
        }
        self.take_agent_undone(cx);
        // The language changed: the program hears it, and every window is made again.
        let language = crate::choices::language();
        if language != self.language {
            self.language = language.clone();
            let written = self.running.facts.formats().language;
            let picked = if written.as_deref() == Some(language.as_str()) { None } else { Some(language.as_str()) };
            self.running.program.set_language(picked);
            self.remake(cx);
        }
        let early = std::mem::take(&mut self.early);
        if !early.is_empty() {
            self.take(None, early, cx);
            cx.invalidate();
        }
        // The part with the focus gives its undo scope, which `-> undo` and
        // `-> redo` act on; a button that runs them keeps the one before.
        if let (Some(host), Some(part)) = (self.current().cloned(), self.focused_input(cx)) {
            host.set_focused(&part);
        }
        let hosts: Vec<Rc<ProgramHost>> = self.stack.last().map(|s| s.host.clone()).into_iter().chain(self.dialogs.iter().map(|d| d.host.clone())).collect();
        let mut changes = 0;
        for host in hosts {
            host.flush();
            let effects = host.take_effects();
            changes += host.changes();
            if !effects.is_empty() {
                self.take(Some(host), effects, cx);
                cx.invalidate();
            }
        }
        // A parked window's sounds hear what code told them at once (as C# SoundPart.Hear).
        for shown in &mut self.stack {
            if let Some(view) = shown.parked.as_mut() {
                hear_parked_sounds(view.as_mut());
            }
        }
        // The event is done: where the window is now is a step of its history, for Back and Forward.
        if let Some(host) = self.current().cloned() {
            host.note_place();
        }
        // The windows' values changed: the screen draws them again.
        if changes != self.seen {
            self.seen = changes;
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
        if self.asking.as_ref().is_some_and(|a| a.overlay == overlay) {
            let Some(asking) = self.asking.take() else { return };
            let values = asking.answered.borrow_mut().take();
            // Cancelling runs nothing.
            if let Some(values) = values {
                asking.host.answer(&asking.ask.handler, values);
            }
            self.ask_next(cx);
            cx.invalidate();
        }
        if self.granting.as_ref().is_some_and(|(open, _, _)| *open == overlay) {
            if let Some((_, granted, granting)) = self.granting.take() {
                (granting.answer)(granted.get().unwrap_or(crate::dialogs::GrantChoice::Refuse));
            }
            self.grant_next(cx);
            cx.invalidate();
        }
        if self.keys_overlay == Some(overlay) {
            self.keys_overlay = None;
        }
        // A dialog over the window closed by itself (Escape): it closes as its close does.
        if let Some(index) = self.dialogs.iter().position(|d| d.overlay == overlay) {
            self.close_dialog(index, cx);
        }
    }

    /// `on start` is asked at the first wake, once the splash page is on the
    /// screen and the host has given how it sends requests (the browser's asker).
    fn on_attached(&mut self, cx: &mut Cx) {
        if self.starting.is_some() {
            cx.wake(self.node.id(), std::time::Instant::now());
        }
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.follow_start(cx);
    }

    /// A long call ended: its bar goes, and its reply is the window's. Or
    /// the app's `on start` answered while its splash page shows.
    fn on_later(&mut self, done: crate::later::Done, cx: &mut Cx) {
        let done = match done.downcast::<StartDone>() {
            Ok(started) => {
                if let Some(starting) = self.starting.as_mut() {
                    starting.reply = Some(started.0);
                }
                self.follow_start(cx);
                return;
            }
            Err(other) => other,
        };
        let done = match done.downcast::<signin::ProviderDone>() {
            Ok(provider) => return self.provider_done(*provider, cx),
            Err(other) => other,
        };
        let Ok(done) = done.downcast::<LongDone>() else { return };
        // A Stop pressed as the call ended is forgotten with its watch.
        self.running.program.end_watch(done.1);
        self.running.calling.set(false);
        if let Some((host, bar)) = self.long.take() {
            cx.close(bar);
            host.answered(done.0);
        }
        cx.invalidate();
    }

    /// The window's own keys: the window on top's (shortcuts, the default and
    /// cancel buttons), then Ctrl+Z takes back the last edit of the scope of
    /// the part with the focus, Ctrl+Y or Ctrl+Shift+Z does it again; a part
    /// that takes the key itself (typing not yet an edit) has it first.
    fn window_key_up(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let Some(index) = self.view_index() else { return false };
        self.node.children_mut()[index].window_key_up(key, cx)
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let Some(index) = self.view_index() else { return false };
        if self.node.children_mut()[index].window_key(key, cx) {
            return true;
        }
        let host = self.current().cloned();
        if undo_key(self, host.as_ref(), key, cx) {
            return true;
        }
        // Alt and Left or Right go Back or Forward through the places the person was in this window.
        if key.modifiers == Modifiers::ALT && matches!(key.key, Key::Left | Key::Right) {
            if let Some(host) = host.as_ref().filter(|h| h.can_go(key.key == Key::Left)) {
                host.go(key.key == Key::Left);
                cx.invalidate();
                return true;
            }
        }
        // A dialog in a desktop window of its own closes on Escape that nothing in it took, as its close does (manual/code.statements.md).
        if self.dialog && key.key == Key::Escape && key.modifiers == Modifiers::NONE {
            if self.stack.len() > 1 {
                self.close_top(cx);
            } else {
                self.closing(cx);
                cx.invalidate();
            }
            return true;
        }
        false
    }
}

/// Ctrl+Z takes back the last edit of the scope of the part with the focus
/// in `root` (a window, or a dialog over it), Ctrl+Y or Ctrl+Shift+Z does it
/// again, through that window's `host`; a part that takes the key itself
/// (typing not yet an edit) has it first. False for any other key.
pub(crate) fn undo_key(root: &mut dyn Element, host: Option<&Rc<ProgramHost>>, key: KeyInput, cx: &mut Cx) -> bool {
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
    if let Some(e) = focused.and_then(|f| element::find_mut(root, f)) {
        if e.on_key(key, cx) {
            return true;
        }
    }
    let part = focused.and_then(|f| part_of(root, f)).or_else(|| cx.focused_part.as_ref().map(|p| p.name.clone()));
    if let (Some(host), Some(part)) = (host, part) {
        if let Some(scope) = host.scope(&part) {
            host.undo(&scope, back);
        }
    }
    true
}

/// The part `focused` is, or the first part it is in, under `root`: its name.
/// The part `focused` is, or the first part it is in, under `root`.
pub(crate) fn focused_part_in(root: &dyn Element, focused: ElementId) -> Option<element::FocusedPart> {
    let ids = element::chain(root, focused)?;
    let part = ids.iter().rev().filter_map(|id| element::find(root, *id)).find_map(|e| e.as_part())?;
    let name = part.base().spec.name()?.to_string();
    Some(element::FocusedPart { name, button: part.base().spec.kind.eq_ignore_ascii_case("button") })
}

fn part_of(root: &dyn Element, focused: ElementId) -> Option<String> {
    let ids = element::chain(root, focused)?;
    ids.iter().rev().filter_map(|id| element::find(root, *id)).find_map(|e| e.as_part().and_then(|p| p.base().spec.name().map(str::to_string)))
}

/// Keys the stack takes itself, for a host that wires them by hand: none beyond `window_key` yet.
pub fn plain(key: Key) -> KeyInput {
    KeyInput::new(key, Modifiers::NONE)
}

/// Every sound of a parked view hears what code told it since it last heard.
fn hear_parked_sounds(view: &mut dyn Element) {
    if let Some(sound) = view.as_any_mut().downcast_mut::<crate::parts::sound::SoundPart>() {
        sound.hear_now();
        return;
    }
    for child in view.node_mut().children_mut().iter_mut() {
        hear_parked_sounds(child.as_mut());
    }
}

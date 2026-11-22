//! How a program starts in its window stack: the splash, asking to start,
//! the sign-in window (docs/ledger/signin.md) before the main window, and a
//! start whose databases did not open, which shows what keeps the connection
//! and reconnects (manual/data.settings.md).

use super::*;

impl WindowStack {
    /// Starts the program: its texts load, its `on start` runs and its main
    /// window opens; when its databases did not open, what keeps the
    /// connection shows first.
    pub fn start(program: Arc<dyn Program>, files: AppFiles) -> WindowStack {
        let running = WindowStack::prepare(program, files);
        let started = running.program.started();
        WindowStack::begin(running, started)
    }

    /// Starts a prepared program as `start` does, from how it `started`.
    pub fn begin(running: Rc<Running>, started: clarity_runtime::program::Started) -> WindowStack {
        let mut stack = WindowStack::empty(running);
        match (started.unconnected, started.signin) {
            (Some(unconnected), _) => stack.start_unconnected(unconnected, None),
            (None, Some(page)) => stack.show_signin(page, None),
            (None, None) => stack.start_main(None),
        }
        stack
    }

    /// What the program's windows share, its texts loaded and its language
    /// told, before anything of it runs.
    pub fn prepare(program: Arc<dyn Program>, files: AppFiles) -> Rc<Running> {
        clarity_runtime::i18n::load(files.model);
        // A language kept from an earlier run: the program hears it before
        // it answers anything, as when it is chosen (a server keeps it per browser).
        if let Some(picked) = crate::choices::language_picked() {
            program.set_language(Some(&picked));
        }
        Running::new(program, files)
    }

    /// The splash screen `name`, shown while the app's `on start` runs elsewhere.
    pub fn splash(running: Rc<Running>, name: &str) -> WindowStack {
        let mut stack = WindowStack::empty(running);
        stack.open_window(name, Map::new(), None);
        stack
    }

    /// The splash page `splash` fills this screen while the app's `on start`
    /// runs, asked without waiting where the host can (the browser), else on
    /// a thread of its own; the main window takes its place once it answered
    /// and the splash showed its time. Where the databases did not open, it starts as `begin` does.
    pub fn start_in_place(running: Rc<Running>, splash: &crate::app_files::Splash) -> WindowStack {
        let started = running.program.started();
        // A sign-in page other than the splash comes first, before `on start`.
        let doubles = started.signin.as_deref() == Some(splash.page.as_str());
        if started.unconnected.is_some() || (started.signin.is_some() && !doubles) {
            return WindowStack::begin(running, started);
        }
        let mut stack = if doubles { WindowStack::splash_signing_in(running, &splash.page) } else { WindowStack::splash(running, &splash.page) };
        stack.starting = Some(InPlace { at_least: splash.at_least, opened: std::time::Instant::now(), asked: false, reply: None });
        stack
    }

    /// `on start` answered `reply` while this splash shows (the desktop host
    /// runs it): a splash that is the sign-in window keeps it until someone
    /// signed in; else the main window follows.
    pub fn splash_started(&mut self, reply: Answer<Reply>, cx: &mut Cx) {
        self.starting = Some(InPlace { at_least: std::time::Duration::ZERO, opened: std::time::Instant::now(), asked: true, reply: Some(reply) });
        self.follow_start(cx);
    }

    /// Whether the splash page still shows while the app starts.
    pub fn starting(&self) -> bool {
        self.starting.is_some()
    }

    /// `on start` is asked to run, once.
    pub(super) fn ask_start(&mut self, cx: &mut Cx) {
        let Some(starting) = self.starting.as_mut() else { return };
        starting.asked = true;
        let program = self.running.program.clone();
        let answered: crate::later::Answered = Box::new(|answer| Box::new(StartDone(crate::program_wire::read("start_app", answer))) as crate::later::Done);
        let work: crate::later::Work = Box::new(move || Box::new(StartDone(program.start_app())) as crate::later::Done);
        cx.later_ask(self.node.id(), crate::later::Asked { command: "start_app".into(), args: Map::new(), answered, work });
        cx.wake(self.node.id(), std::time::Instant::now() + crate::splash::STATUS_EVERY);
    }

    /// While the splash shows: what loads is shown, and once `on start`
    /// answered and the splash showed its time, the main window takes its place.
    pub(super) fn follow_start(&mut self, cx: &mut Cx) {
        let Some(starting) = self.starting.as_ref() else { return };
        if !starting.asked {
            return self.ask_start(cx);
        }
        let shown_for = starting.opened.elapsed();
        let answered = starting.reply.is_some();
        if !crate::splash::closes(answered, shown_for, starting.at_least) {
            // Asked until the splash goes: a quick `on start` still leaves what it said last.
            if let Some(text) = self.running.program.splash_status() {
                self.show_status(text);
                cx.invalidate();
            }
            let next = crate::splash::next_look(shown_for, starting.at_least, answered);
            cx.wake(self.node.id(), std::time::Instant::now() + next);
            return;
        }
        // The splash that is the sign-in window stays, keeping what `on start` answered, until someone signed in.
        if self.splash_signs_in && !self.signed {
            return;
        }
        let Some(reply) = self.starting.take().and_then(|s| s.reply) else { return };
        self.splash_signs_in = false;
        self.signed = false;
        if let Some(top) = self.stack.pop() {
            let _ = top.host.closing();
        }
        if let Some(index) = self.view_index() {
            self.node.remove(index, cx);
        }
        cx.focus(None);
        // On the desktop the splash is a desktop window of its own: the main window gets one too, and the splash ends.
        if self.running.apart.get() {
            self.aparts.push(Apart::Main(reply));
            self.ended = true;
        } else {
            self.open_main(reply, Some(cx));
        }
        cx.invalidate();
    }

    /// The main window, once the app's `on start` ran elsewhere (while the splash showed) and answered `reply`.
    pub fn started(running: Rc<Running>, reply: Answer<Reply>) -> WindowStack {
        let mut stack = WindowStack::empty(running);
        stack.open_main(reply, None);
        stack
    }

    /// The app's `on start`, then its main window.
    pub(super) fn start_main(&mut self, cx: Option<&mut Cx>) {
        let reply = self.running.program.start_app();
        self.open_main(reply, cx);
    }

    /// What the app's `on start` said, then its main window.
    pub(super) fn open_main(&mut self, reply: Answer<Reply>, cx: Option<&mut Cx>) {
        let main = self.running.program.started().main;
        match reply {
            Ok(reply) => {
                if !reply.messages.is_empty() {
                    self.early.push(Effect::Say(reply.messages));
                }
            }
            Err(text) => self.early.push(Effect::Say(vec![Message { level: Level::Error, text, toast: None }])),
        }
        self.open_window(&main, Map::new(), cx);
    }

    /// The databases did not open: the app's `on unconnected` runs with why,
    /// and what it opens shows; without one, or when it opens nothing, the
    /// window that keeps the connection shows, saying why. With neither, the
    /// program says why and ends.
    pub(super) fn start_unconnected(&mut self, unconnected: Unconnected, mut cx: Option<&mut Cx>) {
        let settings = self.running.program.read_settings();
        if let Some(handler) = &unconnected.handler {
            // The handler reads why as the person does, in their language.
            let why = Value::from(unconnected.problem.show());
            match self.running.program.answer_event(WindowState::new("", Map::new()), handler, vec![why]) {
                Ok(reply) => {
                    if !reply.messages.is_empty() {
                        self.early.push(Effect::Say(reply.messages));
                    }
                    for open in reply.open {
                        self.open_window(&open.window, open.with, cx.as_deref_mut());
                    }
                }
                Err(text) => self.early.push(Effect::Say(vec![Message { level: Level::Error, text, toast: None }])),
            }
        }
        if self.stack.is_empty() {
            self.early.push(Effect::Say(vec![Message { level: Level::Error, text: unconnected.problem.clone(), toast: None }]));
            if let Some(window) = &unconnected.window {
                self.open_window(window, Map::new(), cx.as_deref_mut());
            }
        }
        if self.stack.is_empty() {
            eprintln!("{}", unconnected.problem.show());
            self.ended = true;
            return;
        }
        self.unconnected = Some(Unconnecting { unconnected, settings });
    }

    /// The last window closed while the databases are not open: when a
    /// setting changed the program connects again and its main window opens;
    /// unchanged, the program ends.
    pub(super) fn reconnect(&mut self, cx: &mut Cx) -> bool {
        let Some(was) = self.unconnected.take() else { return false };
        if self.running.program.read_settings() == was.settings {
            return false;
        }
        match self.running.program.connect() {
            Ok(()) => self.start_main(Some(cx)),
            Err(problem) => self.start_unconnected(Unconnected { problem, ..was.unconnected }, Some(cx)),
        }
        !self.stack.is_empty()
    }
}

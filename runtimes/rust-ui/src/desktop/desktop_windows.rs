//! The desktop windows of a program: adding one, which is active and which
//! blocked, opening what waits, fitting and widening to a window's size and a
//! sidebar's room, closing, telling the window below, following the window
//! stacks, and a window opened apart.

use super::*;

impl Host {
    /// A screen in a desktop window of its own, shown once the loop runs; over `owner` when given.
    pub(super) fn add(&mut self, mut screen: Screen, title: String, size: (f64, f64), owner: Option<u64>, modal: bool) {
        if let Some(system) = clipboard::SystemClipboard::open() {
            screen.set_clipboard(Box::new(system));
        }
        if let Some(looks) = &self.looks {
            let looks = looks.clone();
            screen.follow(Box::new(move |dark| looks(dark)));
        }
        if let Some(dark) = self.dark {
            screen.set_dark(dark);
        }
        screen.set_waker(self.waker.clone());
        let key = self.next_key;
        self.next_key += 1;
        let look = screen.root().as_any().downcast_ref::<WindowStack>().map(WindowStack::window_look).unwrap_or_default();
        self.wins.push(Win {
            key,
            screen,
            shown: None,
            title,
            size,
            owner,
            modal,
            held: Modifiers::NONE,
            pointer: PointerPlace::new(self.untold),
            inside: false,
            presses: PressCounter::new(),
            caret: None,
            sidebars: None,
            reader: None,
            splash: false,
            look,
        });
    }

    pub(super) fn index_of(&self, id: WindowId) -> Option<usize> {
        self.wins.iter().position(|w| w.shown.as_ref().is_some_and(|s| s.window.id() == id))
    }

    pub(super) fn index_by_key(&self, key: u64) -> Option<usize> {
        self.wins.iter().position(|w| w.key == key)
    }

    /// Whether a window takes no input: a dialog of its own is open over it, or a file dialog of the system's.
    pub(super) fn blocked(&self, i: usize) -> bool {
        let key = self.wins[i].key;
        blocks_input(self.wins.iter().any(|w| w.owner == Some(key) && w.modal), &self.files)
    }

    /// The person works in the window `key` now: file dialogs belong to it.
    pub(super) fn set_active(&mut self, key: Option<u64>) {
        self.active = key;
        let owner = key.and_then(|k| self.index_by_key(k)).and_then(|i| self.wins[i].shown.as_ref()).and_then(|s| window_system::handle(&s.window));
        self.files.set_owner(owner);
    }

    /// Shows the windows not shown yet; a window opened over another lies in its middle.
    pub(super) fn open_waiting(&mut self, event_loop: &ActiveEventLoop) {
        for i in 0..self.wins.len() {
            if self.wins[i].shown.is_some() {
                continue;
            }
            let win = &self.wins[i];
            let over = win.owner.and_then(|o| self.index_by_key(o)).and_then(|o| self.wins[o].shown.as_ref());
            // The window's units as the screen's: as the window it opens over has them, until it knows its own.
            let per_unit = over.map(Shown::per_unit).unwrap_or(1.0);
            let mut attributes = WindowAttributes::default()
                .with_title(win.title.clone())
                .with_inner_size(LogicalSize::new(win.size.0.max(120.0), win.size.1.max(80.0)))
                .with_resizable(!win.modal)
                .with_window_icon(self.icon.clone())
                // Shown once the screen reader's adapter is made, as AccessKit asks.
                .with_visible(false);
            // What the window's markup says of its desktop window, as an Avalonia window's XAML does (a splash says it all).
            let look = win.look;
            attributes = attributes.with_decorations(look.decorations);
            if !look.resizable {
                attributes = attributes.with_resizable(false);
            }
            if look.topmost {
                attributes = attributes.with_window_level(winit::window::WindowLevel::AlwaysOnTop);
            }
            if !look.taskbar {
                attributes = not_in_taskbar(attributes, look.decorations);
            }
            if look.centre && over.is_none() {
                if let Some(monitor) = event_loop.primary_monitor().or_else(|| event_loop.available_monitors().next()) {
                    let scale = monitor.scale_factor();
                    let size = ((win.size.0 * scale) as u32, (win.size.1 * scale) as u32);
                    let at = monitor.position();
                    let room = monitor.size();
                    let (x, y) = crate::splash::centred((at.x, at.y), (room.width, room.height), size);
                    attributes = attributes.with_position(PhysicalPosition::new(x, y));
                }
            }
            if let Some(over) = over {
                if let Ok(at) = over.window.outer_position() {
                    let there = over.window.outer_size();
                    let x = at.x as f64 + (there.width as f64 - win.size.0 * per_unit) / 2.0;
                    let y = at.y as f64 + (there.height as f64 - win.size.1 * per_unit) / 2.0;
                    attributes = attributes.with_position(PhysicalPosition::new(x.max(0.0) as i32, y.max(0.0) as i32));
                }
            }
            let software = window_system::software_asked(|name| std::env::var(name).ok());
            let first = window_system::first_context(software, self.context);
            match Shown::open(event_loop, attributes, self.config.as_ref(), first, software) {
                Ok((shown, context)) => {
                    self.context = context;
                    let adapter = accesskit_winit::Adapter::with_event_loop_proxy(event_loop, &shown.window, self.proxy.clone());
                    shown.window.set_visible(true);
                    self.wins[i].reader = Some(Reader { adapter, active: false, told: None });
                    if self.config.is_none() {
                        self.config = Some(shown.config.clone());
                        // Where the window system says nothing (X11), the system is asked as C# asks it, and followed by its change event or every 3 s.
                        let told = shown.window.theme().or_else(|| event_loop.system_theme());
                        let theme = told.or_else(|| Some(if crate::system_theme::dark_here() { Theme::Dark } else { Theme::Light }));
                        self.wins[i].shown = Some(shown);
                        self.follow_theme(theme);
                        if told.is_none() {
                            let proxy = Mutex::new(self.proxy.clone());
                            let now = theme == Some(Theme::Dark);
                            let listen: crate::system_theme::Listen = Box::new(theme_events::listen);
                            self.theme_following = Some(crate::system_theme::follow(now, crate::system_theme::dark_here, listen, Duration::from_secs(3), move |dark| {
                                if let Ok(proxy) = proxy.lock() {
                                    let _ = proxy.send_event(Wake::Dark(dark));
                                }
                            }));
                        }
                    } else {
                        self.wins[i].shown = Some(shown);
                    }
                    let key = self.wins[i].key;
                    self.set_active(Some(key));
                    if self.wins[i].splash
                        && let Some(starting) = self.starting.as_mut()
                    {
                        starting.opened = Some(Instant::now());
                    }
                    self.safely(|host| host.fit(i));
                    if let Some(shown) = &self.wins[i].shown {
                        shown.window.request_redraw();
                    }
                }
                Err(why) => {
                    self.failure = Some(why);
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    /// The splash finished: the main window opens with what `on start` answered, its own desktop window.
    pub(super) fn open_main(&mut self, running: Rc<Running>, reply: clarity_runtime::program::Answer<clarity_runtime::ui::Reply>) {
        let files = running.files;
        let stack = WindowStack::started(running, reply);
        let title = stack.title();
        let size = grow::open_size(&stack);
        let screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
        self.add(screen, title, size, None, false);
    }

    /// The screen takes the window's size in its units, at the system's content scale.
    pub(super) fn fit(&mut self, i: usize) {
        let win = &mut self.wins[i];
        let Some(shown) = win.shown.as_mut() else { return };
        let _ = shown.gl_context.make_current(&shown.gl_surface);
        let size = shown.window.inner_size();
        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            shown.gl_surface.resize(&shown.gl_context, w, h);
        }
        match Shown::surface_of(&mut shown.drawing, &shown.config, size) {
            Ok(surface) => shown.surface = surface,
            Err(why) => {
                self.failure = Some(why);
                return;
            }
        }
        let (scale, per_unit) = shown.scale();
        win.screen.resize(size.width.max(1) as f64 / per_unit, size.height.max(1) as f64 / per_unit, scale);
    }

    /// A sidebar of the window shown opened: the desktop window grows by the room it takes (a dialog keeps its size).
    pub(super) fn widen(&mut self, i: usize) {
        let win = &mut self.wins[i];
        if win.modal {
            return;
        }
        let Some(shown) = &win.shown else { return };
        let Some(stack) = win.screen.root().as_any().downcast_ref::<WindowStack>() else { return };
        let Some(view) = stack.view() else { return };
        let (id, is) = (view.node().id(), grow::sidebars_of(view));
        if let Some((was_view, was)) = win.sidebars {
            if was_view == id {
                let now = shown.size();
                let grown = grow::sidebars_grown(now, &was, &is, shown.room());
                if grown != now {
                    shown.resize(grown);
                }
            }
        }
        win.sidebars = Some((id, is));
    }

    /// Closes a desktop window, the windows opened over it first; what it
    /// shows hears its `on close` unless it ended by itself, what it said
    /// and still shows goes to the window below, and the window that opened
    /// it is told.
    pub(super) fn close(&mut self, key: u64) {
        let children: Vec<u64> = self.wins.iter().filter(|w| w.owner == Some(key)).map(|w| w.key).collect();
        for child in children {
            self.close(child);
        }
        let Some(i) = self.index_by_key(key) else { return };
        let owner = self.wins[i].owner;
        let screen = &mut self.wins[i].screen;
        let closed = catch_unwind(AssertUnwindSafe(|| {
            screen.act_root::<WindowStack, _>(|stack, cx| {
                let mut shown = Vec::new();
                if !stack.ended() {
                    shown = stack.toasts();
                    shown.extend(stack.said().iter().cloned());
                    stack.closing(cx);
                }
                let mut closed = stack.take_closed();
                if let Some(closed) = closed.as_mut() {
                    hand_down(&mut closed.said, &mut shown);
                    closed.said = shown;
                }
                closed
            })
        }));
        let mut win = self.wins.remove(i);
        // What its elements started (timers, sounds, engines) stops as they leave the screen.
        let _ = catch_unwind(AssertUnwindSafe(|| win.screen.dispose()));
        drop(win.shown.take());
        if self.active == Some(key) {
            let next = owner.or_else(|| self.wins.last().map(|w| w.key));
            self.set_active(next);
        }
        match closed {
            Ok(Some(Some(closed))) => self.tell_opener(closed),
            Err(payload) => {
                let now = self.now();
                self.fail_safe.failed(&panic_text(payload.as_ref()), now);
            }
            _ => {}
        }
    }

    /// A window of its own closed: the window that opened it is told.
    pub(super) fn tell_opener(&mut self, closed: ApartClosed) {
        for win in self.wins.iter_mut() {
            let holds = win.screen.act_root::<WindowStack, _>(|stack, _| stack.holds(closed.opener)).unwrap_or(false);
            if holds {
                win.screen.act_root::<WindowStack, _>(move |stack, cx| stack.apart_closed(closed, cx));
                return;
            }
        }
    }

    /// What each program's stack asks of its desktop window: its title (and
    /// the size its window names), windows of their own, its end.
    pub(super) fn follow_stacks(&mut self, event_loop: &ActiveEventLoop) {
        let mut i = 0;
        while i < self.wins.len() {
            let key = self.wins[i].key;
            // The keys window closes once the person closed it.
            let keys_done = self.wins[i].screen.root().as_any().downcast_ref::<KeysWindow>().is_some_and(|k| k.done());
            if keys_done {
                self.close(key);
                continue;
            }
            let asked = self.wins[i].screen.act_root::<WindowStack, _>(|stack, _| {
                let mut closed = stack.take_closed();
                // A window of its own that ended by itself: the error it still shows goes below too.
                if let (Some(closed), true) = (closed.as_mut(), stack.ended()) {
                    let mut shown = stack.said().to_vec();
                    hand_down(&mut closed.said, &mut shown);
                    closed.said = shown;
                }
                (stack.title(), grow::top_size(stack), stack.ended(), stack.quitting(), stack.take_aparts(), closed, stack.running().clone())
            });
            let Some((title, wanted, ended, quitting, aparts, closed, running)) = asked else {
                i += 1;
                continue;
            };
            if quitting {
                event_loop.exit();
                return;
            }
            if let Some(closed) = closed {
                self.tell_opener(closed);
            }
            // The main window a signed-in splash asked for, or the splash a signout shows again, opens
            // before the window that asked goes, so the program always has a window.
            let (mains, aparts): (Vec<Apart>, Vec<Apart>) = aparts.into_iter().partition(|a| matches!(a, Apart::Main(_) | Apart::Signin(_)));
            for main in mains {
                self.open_apart(main, &running, key);
            }
            if ended {
                self.close(key);
                continue;
            }
            if title != self.wins[i].title {
                let win = &self.wins[i];
                if let Some(shown) = &win.shown {
                    shown.window.set_title(&title);
                    // A window that names its size grows the desktop window to it, as far as the screen has room.
                    if !win.modal {
                        let now = shown.size();
                        let grown = grow::grown(now, wanted, shown.room());
                        if grown != now {
                            shown.resize(grown);
                        }
                    }
                }
                self.wins[i].title = title;
            }
            for apart in aparts {
                self.open_apart(apart, &running, key);
            }
            i += 1;
        }
        // The program's own windows closed: it ends.
        if !self.wins.iter().any(|w| w.owner.is_none()) {
            event_loop.exit();
        }
    }

    /// Opens what a stack asked for in a desktop window of its own, over the window it came from.
    pub(super) fn open_apart(&mut self, apart: Apart, running: &Rc<Running>, owner: u64) {
        let look = self.looks.as_ref().map(|l| l(self.dark.unwrap_or(false))).unwrap_or_else(|| ThemeLook::clarity(false));
        match apart {
            Apart::Window(request) => {
                let modal = request.dialog;
                let stack = WindowStack::apart(running.clone(), request);
                let title = stack.title();
                let size = grow::open_size(&stack);
                let mut screen = Screen::new(Box::new(stack), look);
                // A dialog's first part has the focus, as over the window in a page.
                if modal {
                    let first = grow::first_focus(screen.root());
                    screen.focus(first);
                }
                self.add(screen, title, size, Some(owner), modal);
            }
            Apart::Main(reply) => self.open_main(running.clone(), reply),
            // At its size, as its markup says, and owned by no window: it stays when the one that asked closes.
            Apart::Signin(page) => {
                let stack = WindowStack::signin_window(running.clone(), &page);
                let title = stack.title();
                let size = grow::open_size(&stack);
                self.add(Screen::new(Box::new(stack), look), title, size, None, false);
            }
            Apart::Keys => {
                if self.wins.iter().any(|w| w.screen.root().as_any().is::<KeysWindow>()) {
                    return;
                }
                let window = Running::keys_window(running, true);
                let size = window.wanted();
                self.add(Screen::new(Box::new(window), look), keys::title(), size, Some(owner), true);
            }
        }
    }

    /// The pointer's place in the screen's units.
    pub(super) fn at(&self, i: usize, x: f64, y: f64) -> (f64, f64) {
        let per_unit = self.wins[i].shown.as_ref().map(Shown::per_unit).unwrap_or(1.0);
        (x / per_unit, y / per_unit)
    }
}

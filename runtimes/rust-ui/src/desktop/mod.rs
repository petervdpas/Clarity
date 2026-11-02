//! The desktop host (DESIGN 7.9): each screen in a window of the system's
//! own (winit), drawn by Skia through OpenGL (glutin) when it changed, at
//! the window's own scale. What the person does with the pointer, the wheel,
//! keys and an input method goes to its screen in its units (logical
//! pixels). The theme follows the system's light or dark, at start and when
//! it changes. A window wakes at the time an element asked for, when work
//! off the screen's thread is done, and draws every frame while one
//! animates; otherwise it waits.
//!
//! `run` shows a generated program: its windows open in place in their
//! desktop window, while a dialog, the keys window and a target marked `in
//! new window` get a desktop window of their own, each its own screen and
//! OpenGL surface. A dialog's window is sized to the dialog, and the window
//! under it takes no input while it is open. `show` shows any screen. Both
//! end when the program's own windows closed.

mod clipboard;
pub mod files;
pub mod input;
pub mod sound;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use skia_safe::gpu::{self, backend_render_targets, gl::FramebufferInfo, DirectContext, SurfaceOrigin};
use skia_safe::{ColorType, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Theme, Window, WindowAttributes, WindowId};

use clarity_runtime::program::Program;

use crate::app_files::AppFiles;
use crate::dialogs::keys::{self, KeysWindow};
use crate::element::Composition;
use crate::geometry::Rect;
use crate::input::{KeyInput, Modifiers, PointerButton, PressCounter};
use crate::locale::{fill, translate};
use crate::screen::Screen;
use crate::theme::ThemeLook;
use crate::window::{Apart, ApartClosed, Running, WindowStack};

use self::input::{key_of, modifiers_of, typed_text, wheel_of, PointerPlace};

/// The size a window that names none opens at, in the screen's units, on every stack.
pub const STANDARD_SIZE: (f64, f64) = (1100.0, 760.0);

/// The look for light (false) or dark (true), as the system's theme asks for it.
pub type Looks = Box<dyn Fn(bool) -> Arc<ThemeLook>>;

/// How a screen shows in its desktop window: its title, the size it opens
/// at in the screen's units, and the looks it follows the system's theme with.
pub struct ShowOptions {
    pub title: String,
    pub width: f64,
    pub height: f64,
    /// The look for the system's light or dark; none keeps the screen's own look.
    pub looks: Option<Looks>,
}

impl ShowOptions {
    /// A window with this title at the standard size, in the screen's own look.
    pub fn new(title: impl Into<String>) -> ShowOptions {
        ShowOptions { title: title.into(), width: STANDARD_SIZE.0, height: STANDARD_SIZE.1, looks: None }
    }

    /// The size the window opens at, in the screen's units.
    pub fn size(mut self, width: f64, height: f64) -> ShowOptions {
        self.width = width;
        self.height = height;
        self
    }

    /// The look follows the system's light or dark.
    pub fn following(mut self, looks: impl Fn(bool) -> Arc<ThemeLook> + 'static) -> ShowOptions {
        self.looks = Some(Box::new(looks));
        self
    }
}

/// Runs a generated program on Clarity.UI: its windows in desktop windows,
/// in its look for the system's light or dark. Ends when its own windows
/// closed or it quit.
pub fn run(program: impl Program, files: AppFiles) -> ExitCode {
    // Driven without a window, step by step from standard input (DESIGN 1.6).
    if std::env::var_os(crate::headless::VARIABLE).is_some_and(|v| !v.is_empty()) {
        return crate::headless::run(program, files);
    }
    // The language and theme the person chose in an earlier run, kept beside the program's settings.
    let kept = clarity_runtime::settings::file(files.title).and_then(|f| f.parent().map(|p| p.to_path_buf()));
    crate::choices::start(&files, kept);
    crate::files::set_system(Arc::new(self::files::SystemFilePicker));
    crate::parts::sound::set_output(Rc::new(self::sound::Speakers::default()));
    let stack = WindowStack::start(Arc::new(program), files);
    // Dialogs, the keys window and new windows get desktop windows of their own.
    stack.running().set_apart(true);
    let title = stack.title();
    let (width, height) = stack.wanted_size().unwrap_or(STANDARD_SIZE);
    let screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
    show(screen, ShowOptions::new(title).size(width, height).following(move |dark| crate::choices::look(&files, dark)))
}

/// Shows a screen in a desktop window until the window closes (or, for a
/// program's `WindowStack`, until its own windows closed).
pub fn show(screen: Screen, mut options: ShowOptions) -> ExitCode {
    let event_loop = match EventLoop::<Wake>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(why) => return failed(&why.to_string()),
    };
    let looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>> = options.looks.take().map(Rc::from);
    let mut host = Host::new(event_loop.create_proxy(), looks);
    host.untold = untold_moves(&event_loop);
    host.add(screen, options.title.clone(), (options.width, options.height), None, false);
    let ran = event_loop.run_app(&mut host);
    for win in host.wins.iter_mut() {
        win.screen.dispose();
        // Skia's context goes before the window it draws in.
        win.shown = None;
    }
    match (ran, host.failure) {
        (_, Some(why)) => failed(&why),
        (Err(why), None) => failed(&why.to_string()),
        (Ok(()), None) => ExitCode::SUCCESS,
    }
}

/// Says why the window could not show, and ends with a failure.
fn failed(why: &str) -> ExitCode {
    eprintln!("{}", fill(&translate("desktopWindowFailed", "The window could not show: {why}"), "why", why));
    ExitCode::FAILURE
}

/// Whether the window system leaves a move back to where the pointer last
/// moved untold: winit on X11 (see `PointerPlace`).
fn untold_moves(event_loop: &EventLoop<Wake>) -> bool {
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopExtX11;
        event_loop.is_x11()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = event_loop;
        false
    }
}

/// What wakes the host from outside its loop: work off a screen's thread
/// is done, or the system's light or dark changed (as the watch over it
/// tells where the window system does not, X11).
#[derive(Debug, Clone, Copy)]
enum Wake {
    Done,
    Dark(bool),
}

/// The window as it shows: the system's window, its OpenGL surface and
/// context, and Skia's context and surface over them.
struct Shown {
    // Dropped in this order: Skia's surface and context before OpenGL's, the window last.
    surface: Surface,
    gl_surface: GlutinSurface<WindowSurface>,
    skia: DirectContext,
    gl_context: PossiblyCurrentContext,
    config: Config,
    window: Window,
}

impl Drop for Shown {
    fn drop(&mut self) {
        // Some drivers crash when Skia frees its resources after the context went.
        let _ = self.gl_context.make_current(&self.gl_surface);
        self.skia.release_resources_and_abandon();
    }
}

impl Shown {
    /// Opens the window with an OpenGL context and Skia over it; the first
    /// window finds the config every later one uses.
    fn open(event_loop: &ActiveEventLoop, attributes: WindowAttributes, config: Option<&Config>) -> Result<Shown, String> {
        let (window, config) = match config {
            Some(config) => (glutin_winit::finalize_window(event_loop, attributes, config).map_err(|why| why.to_string())?, config.clone()),
            None => {
                let template = ConfigTemplateBuilder::new().with_alpha_size(8);
                // The config with the fewest samples: Skia smooths edges itself.
                let (window, config) = DisplayBuilder::new()
                    .with_window_attributes(Some(attributes))
                    .build(event_loop, template, |configs| configs.reduce(|a, b| if b.num_samples() < a.num_samples() { b } else { a }).expect("the system offers an OpenGL config"))
                    .map_err(|why| why.to_string())?;
                (window.ok_or_else(|| translate("desktopNoWindow", "the system made no window"))?, config)
            }
        };
        let handle = window.window_handle().map_err(|why| why.to_string())?.as_raw();
        let display = config.display();
        // OpenGL first, else OpenGL ES, which some systems have alone.
        let context = unsafe {
            display
                .create_context(&config, &ContextAttributesBuilder::new().build(Some(handle)))
                .or_else(|_| display.create_context(&config, &ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(Some(handle))))
                .map_err(|why| why.to_string())?
        };
        let surface_attributes = window.build_surface_attributes(Default::default()).map_err(|why| why.to_string())?;
        let gl_surface = unsafe { display.create_window_surface(&config, &surface_attributes).map_err(|why| why.to_string())? };
        let gl_context = context.make_current(&gl_surface).map_err(|why| why.to_string())?;
        // Paced by the display: a frame while an element animates, no faster.
        let _ = gl_surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::MIN));
        let interface = gpu::gl::Interface::new_load_with(|name| {
            if name == "eglGetCurrentDisplay" {
                return std::ptr::null();
            }
            match CString::new(name) {
                Ok(name) => display.get_proc_address(name.as_c_str()),
                Err(_) => std::ptr::null(),
            }
        })
        .ok_or_else(|| translate("desktopNoOpenGl", "OpenGL has not what Skia needs"))?;
        let mut skia = gpu::direct_contexts::make_gl(interface, None).ok_or_else(|| translate("desktopNoSkia", "Skia could not draw with OpenGL"))?;
        let surface = Self::surface_of(&mut skia, &config, window.inner_size())?;
        // Input methods compose in it (the person types Chinese, Japanese, accents).
        window.set_ime_allowed(true);
        Ok(Shown { surface, gl_surface, skia, gl_context, config, window })
    }

    /// Skia's surface over the window's own framebuffer, in its pixels.
    fn surface_of(skia: &mut DirectContext, config: &Config, size: PhysicalSize<u32>) -> Result<Surface, String> {
        let info = FramebufferInfo { fboid: 0, format: gpu::gl::Format::RGBA8.into(), ..Default::default() };
        let pixels = (size.width.max(1) as i32, size.height.max(1) as i32);
        let target = backend_render_targets::make_gl(pixels, config.num_samples() as usize, config.stencil_size() as usize, info);
        gpu::surfaces::wrap_backend_render_target(skia, &target, SurfaceOrigin::BottomLeft, ColorType::RGBA8888, None, None)
            .ok_or_else(|| translate("desktopNoSurface", "Skia could not draw in the window"))
    }

    /// The window's scale: its pixels per screen unit.
    fn scale(&self) -> f64 {
        let scale = self.window.scale_factor();
        if scale > 0.0 { scale } else { 1.0 }
    }
}

/// One desktop window: its screen, the system's window once it shows, the
/// window it was opened over, and what its input gave last.
struct Win {
    /// Its own number, by which the windows opened over it know it.
    key: u64,
    screen: Screen,
    shown: Option<Shown>,
    title: String,
    size: (f64, f64),
    /// The window it was opened over; none for the program's own.
    owner: Option<u64>,
    /// The window it was opened over takes no input while it is open.
    modal: bool,
    held: Modifiers,
    /// Where the pointer is over it, kept apart from a move the system leaves untold (X11).
    pointer: PointerPlace,
    /// Whether the pointer is over it.
    inside: bool,
    presses: PressCounter,
    caret: Option<Rect>,
}

/// Every desktop window, and what they share.
struct Host {
    wins: Vec<Win>,
    looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>>,
    waker: crate::later::Waker,
    config: Option<Config>,
    failure: Option<String>,
    dark: Option<bool>,
    started: Instant,
    next_key: u64,
    proxy: EventLoopProxy<Wake>,
    /// The watch over the system's light or dark, where the window system tells no change (X11).
    theme_watch: Option<crate::system_theme::Watch>,
    /// Whether the window system leaves a move back to where the pointer last moved untold (X11).
    untold: bool,
}

impl Host {
    fn new(proxy: EventLoopProxy<Wake>, looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>>) -> Host {
        let shared = Mutex::new(proxy.clone());
        let waker: crate::later::Waker = Arc::new(move || {
            if let Ok(proxy) = shared.lock() {
                let _ = proxy.send_event(Wake::Done);
            }
        });
        Host { wins: Vec::new(), looks, waker, config: None, failure: None, dark: None, started: Instant::now(), next_key: 1, proxy, theme_watch: None, untold: false }
    }

    /// A screen in a desktop window of its own, shown once the loop runs; over `owner` when given.
    fn add(&mut self, mut screen: Screen, title: String, size: (f64, f64), owner: Option<u64>, modal: bool) {
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
        self.wins.push(Win { key, screen, shown: None, title, size, owner, modal, held: Modifiers::NONE, pointer: PointerPlace::new(self.untold), inside: false, presses: PressCounter::new(), caret: None });
    }

    fn index_of(&self, id: WindowId) -> Option<usize> {
        self.wins.iter().position(|w| w.shown.as_ref().is_some_and(|s| s.window.id() == id))
    }

    fn index_by_key(&self, key: u64) -> Option<usize> {
        self.wins.iter().position(|w| w.key == key)
    }

    /// Whether a window of its own is open over it as a dialog: it takes no input.
    fn blocked(&self, i: usize) -> bool {
        let key = self.wins[i].key;
        self.wins.iter().any(|w| w.owner == Some(key) && w.modal)
    }

    /// Shows the windows not shown yet; a window opened over another lies in its middle.
    fn open_waiting(&mut self, event_loop: &ActiveEventLoop) {
        for i in 0..self.wins.len() {
            if self.wins[i].shown.is_some() {
                continue;
            }
            let win = &self.wins[i];
            let mut attributes = WindowAttributes::default()
                .with_title(win.title.clone())
                .with_inner_size(LogicalSize::new(win.size.0.max(120.0), win.size.1.max(80.0)))
                .with_resizable(!win.modal);
            let over = win.owner.and_then(|o| self.index_by_key(o)).and_then(|o| self.wins[o].shown.as_ref());
            if let Some(over) = over {
                if let Ok(at) = over.window.outer_position() {
                    let scale = over.scale();
                    let there = over.window.outer_size();
                    let x = at.x as f64 + (there.width as f64 - win.size.0 * scale) / 2.0;
                    let y = at.y as f64 + (there.height as f64 - win.size.1 * scale) / 2.0;
                    attributes = attributes.with_position(PhysicalPosition::new(x.max(0.0) as i32, y.max(0.0) as i32));
                }
            }
            match Shown::open(event_loop, attributes, self.config.as_ref()) {
                Ok(shown) => {
                    if self.config.is_none() {
                        self.config = Some(shown.config.clone());
                        // Where the window system says nothing (X11), the system is asked as C# asks it, and asked again every 3 s.
                        let told = shown.window.theme().or_else(|| event_loop.system_theme());
                        let theme = told.or_else(|| Some(if crate::system_theme::dark_here() { Theme::Dark } else { Theme::Light }));
                        self.wins[i].shown = Some(shown);
                        self.follow_theme(theme);
                        if told.is_none() {
                            let proxy = Mutex::new(self.proxy.clone());
                            let now = theme == Some(Theme::Dark);
                            self.theme_watch = Some(crate::system_theme::watch(now, Duration::from_secs(3), move |dark| {
                                if let Ok(proxy) = proxy.lock() {
                                    let _ = proxy.send_event(Wake::Dark(dark));
                                }
                            }));
                        }
                    } else {
                        self.wins[i].shown = Some(shown);
                    }
                    self.fit(i);
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

    /// The screen takes the window's size in its units, at the window's scale.
    fn fit(&mut self, i: usize) {
        let win = &mut self.wins[i];
        let Some(shown) = win.shown.as_mut() else { return };
        let _ = shown.gl_context.make_current(&shown.gl_surface);
        let size = shown.window.inner_size();
        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            shown.gl_surface.resize(&shown.gl_context, w, h);
        }
        match Shown::surface_of(&mut shown.skia, &shown.config, size) {
            Ok(surface) => shown.surface = surface,
            Err(why) => {
                self.failure = Some(why);
                return;
            }
        }
        let scale = shown.scale();
        win.screen.resize(size.width.max(1) as f64 / scale, size.height.max(1) as f64 / scale, scale as f32);
    }

    /// The look for the system's light or dark, when it changed: every window follows it.
    fn follow_theme(&mut self, theme: Option<Theme>) {
        let dark = theme == Some(Theme::Dark);
        if self.dark == Some(dark) {
            return;
        }
        self.dark = Some(dark);
        for win in self.wins.iter_mut() {
            win.screen.set_dark(dark);
        }
    }

    fn draw(&mut self, i: usize) {
        let win = &mut self.wins[i];
        let Some(shown) = win.shown.as_mut() else { return };
        if let Err(why) = shown.gl_context.make_current(&shown.gl_surface) {
            self.failure = Some(why.to_string());
            return;
        }
        win.screen.draw(shown.surface.canvas());
        // A frame that did not reach OpenGL is left out, not shown half drawn.
        if shown.skia.flush_and_submit().is_ok() {
            let _ = shown.gl_surface.swap_buffers(&shown.gl_context);
        }
    }

    /// Closes a desktop window, the windows opened over it first; what it
    /// shows hears its `on close` unless it ended by itself, and the window
    /// that opened it is told.
    fn close(&mut self, key: u64) {
        let children: Vec<u64> = self.wins.iter().filter(|w| w.owner == Some(key)).map(|w| w.key).collect();
        for child in children {
            self.close(child);
        }
        let Some(i) = self.index_by_key(key) else { return };
        let closed = self.wins[i].screen.act_root::<WindowStack, _>(|stack, cx| {
            if !stack.ended() {
                stack.closing(cx);
            }
            stack.take_closed()
        });
        let mut win = self.wins.remove(i);
        win.screen.dispose();
        drop(win.shown.take());
        if let Some(Some(closed)) = closed {
            self.tell_opener(closed);
        }
    }

    /// A window of its own closed: the window that opened it is told.
    fn tell_opener(&mut self, closed: ApartClosed) {
        for win in self.wins.iter_mut() {
            let holds = win.screen.act_root::<WindowStack, _>(|stack, _| stack.holds(closed.opener)).unwrap_or(false);
            if holds {
                win.screen.act_root::<WindowStack, _>(move |stack, cx| stack.apart_closed(closed, cx));
                return;
            }
        }
    }

    /// What each program's stack asks of its desktop window: its title, windows of their own, its end.
    fn follow_stacks(&mut self, event_loop: &ActiveEventLoop) {
        let mut i = 0;
        while i < self.wins.len() {
            let key = self.wins[i].key;
            // The keys window closes once the person closed it.
            let keys_done = self.wins[i].screen.root().as_any().downcast_ref::<KeysWindow>().is_some_and(|k| k.done());
            if keys_done {
                self.close(key);
                continue;
            }
            let asked = self.wins[i].screen.act_root::<WindowStack, _>(|stack, _| (stack.title(), stack.ended(), stack.quitting(), stack.take_aparts(), stack.take_closed(), stack.running().clone()));
            let Some((title, ended, quitting, aparts, closed, running)) = asked else {
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
            if ended {
                self.close(key);
                continue;
            }
            if title != self.wins[i].title {
                if let Some(shown) = &self.wins[i].shown {
                    shown.window.set_title(&title);
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
    fn open_apart(&mut self, apart: Apart, running: &Rc<Running>, owner: u64) {
        let look = self.looks.as_ref().map(|l| l(self.dark.unwrap_or(false))).unwrap_or_else(|| ThemeLook::clarity(false));
        match apart {
            Apart::Window(request) => {
                let modal = request.dialog;
                let stack = WindowStack::apart(running.clone(), request);
                let title = stack.title();
                let size = stack.wanted_size().unwrap_or(STANDARD_SIZE);
                self.add(Screen::new(Box::new(stack), look), title, size, Some(owner), modal);
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
    fn at(&self, i: usize, x: f64, y: f64) -> (f64, f64) {
        let scale = self.wins[i].shown.as_ref().map(Shown::scale).unwrap_or(1.0);
        (x / scale, y / scale)
    }

    /// The input method's window lies by the caret of the part that types.
    fn place_ime(&mut self) {
        for win in self.wins.iter_mut() {
            let caret = win.screen.caret_area();
            if caret == win.caret {
                continue;
            }
            win.caret = caret;
            if let (Some(caret), Some(shown)) = (caret, &win.shown) {
                shown.window.set_ime_cursor_area(LogicalPosition::new(caret.x, caret.y), LogicalSize::new(caret.width.max(1.0), caret.height.max(1.0)));
            }
        }
    }
}

fn button_of(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        _ => None,
    }
}

/// Whether an event is the person's input, which a window under a dialog of its own does not take.
fn is_input(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } | WindowEvent::KeyboardInput { .. } | WindowEvent::Ime(_)
    )
}

impl ApplicationHandler<Wake> for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.open_waiting(event_loop);
    }

    /// Work off a screen's thread is done: what it gave goes to its element.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Wake) {
        if let Wake::Dark(dark) = event {
            self.follow_theme(Some(if dark { Theme::Dark } else { Theme::Light }));
            for win in self.wins.iter() {
                if let Some(shown) = &win.shown {
                    shown.window.request_redraw();
                }
            }
            return;
        }
        for win in self.wins.iter_mut() {
            win.screen.take_later();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(i) = self.index_of(id) else { return };
        // Where the pointer is, kept also while a dialog over the window takes its input.
        match &event {
            WindowEvent::CursorEntered { .. } => {
                self.wins[i].inside = true;
                self.wins[i].pointer.entered();
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].inside = false,
            WindowEvent::Focused(true) => self.wins[i].pointer.focused(),
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = self.at(i, position.x, position.y);
                self.wins[i].pointer.told(x, y);
            }
            _ => {}
        }
        if self.blocked(i) && (is_input(&event) || matches!(event, WindowEvent::CloseRequested)) {
            // A window with a dialog of its own open over it stays, and takes nothing.
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                let key = self.wins[i].key;
                // The program's main window closing ends the program.
                if self.wins[i].owner.is_none() && self.wins.iter().filter(|w| w.owner.is_none()).count() == 1 {
                    let owned: Vec<u64> = self.wins.iter().map(|w| w.key).collect();
                    for key in owned.into_iter().rev() {
                        self.close(key);
                    }
                    event_loop.exit();
                    return;
                }
                self.close(key);
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.fit(i),
            WindowEvent::ThemeChanged(theme) => self.follow_theme(Some(theme)),
            WindowEvent::RedrawRequested => self.draw(i),
            WindowEvent::ModifiersChanged(modifiers) => self.wins[i].held = modifiers_of(modifiers.state()),
            WindowEvent::Focused(false) => self.wins[i].held = Modifiers::NONE,
            WindowEvent::CursorMoved { .. } => {
                let win = &mut self.wins[i];
                let (x, y) = win.pointer.at();
                win.screen.pointer_moved(x, y);
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].screen.pointer_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(button) = button_of(button) else { return };
                let elapsed = self.started.elapsed().as_millis() as i64;
                let win = &mut self.wins[i];
                // A move the system left untold went where the pointer last moved.
                if let Some((x, y)) = win.pointer.settle() {
                    win.screen.pointer_moved(x, y);
                }
                let (x, y) = win.pointer.at();
                match state {
                    // The window gives only presses: a double or triple click is counted here.
                    ElementState::Pressed => {
                        let clicks = win.presses.press(x, y, button, elapsed);
                        win.screen.pointer_pressed_with(x, y, button, clicks, win.held);
                    }
                    ElementState::Released => win.screen.pointer_released(x, y, button),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self.wins[i].shown.as_ref().map(Shown::scale).unwrap_or(1.0);
                let (dx, dy) = wheel_of(delta, scale);
                let win = &mut self.wins[i];
                if let Some((x, y)) = win.pointer.settle() {
                    win.screen.pointer_moved(x, y);
                }
                let (x, y) = win.pointer.at();
                win.screen.wheel(x, y, dx, dy, win.held);
            }
            // Keys the system repeats for a key held as the window got the focus are not the person's.
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => {
                let win = &mut self.wins[i];
                let key = KeyInput::new(key_of(&event.logical_key, event.physical_key), win.held);
                match event.state {
                    ElementState::Pressed => {
                        win.screen.key_pressed(key);
                        if let Some(text) = typed_text(event.text.as_deref(), win.held) {
                            win.screen.text_typed(&text);
                        }
                    }
                    ElementState::Released => win.screen.key_released(key),
                }
            }
            // An input method composes text, then commits it as typed text.
            WindowEvent::Ime(ime) => {
                let screen = &mut self.wins[i].screen;
                match ime {
                    Ime::Preedit(text, cursor) if !text.is_empty() => screen.compose(Some(Composition { text, marked: cursor })),
                    Ime::Preedit(..) | Ime::Disabled => screen.compose(None),
                    Ime::Commit(text) => {
                        screen.compose(None);
                        screen.text_typed(&text);
                    }
                    Ime::Enabled => {}
                }
            }
            _ => {}
        }
    }

    /// The pointer moved, told to a window or not: the window it is over keeps that it moved.
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { .. } = event {
            for win in self.wins.iter_mut().filter(|w| w.inside) {
                win.pointer.raw_motion();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.failure.is_some() {
            event_loop.exit();
            return;
        }
        self.follow_stacks(event_loop);
        self.open_waiting(event_loop);
        self.place_ime();
        let now = Instant::now();
        let mut animating = false;
        let mut next: Option<Instant> = None;
        for win in self.wins.iter_mut() {
            if win.screen.next_wake().is_some_and(|at| at <= now) {
                win.screen.wake_due(now);
            }
            let moving = win.screen.animating() > 0;
            animating |= moving;
            if win.screen.take_redraw() || moving {
                if let Some(shown) = &win.shown {
                    shown.window.request_redraw();
                }
            }
            // The next wake-up is asked for anew on every turn, so a changed time needs nothing more.
            win.screen.take_wakes_changed();
            next = match (next, win.screen.next_wake()) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        let flow = match next {
            Some(at) if !animating => ControlFlow::WaitUntil(at),
            _ => ControlFlow::Wait,
        };
        event_loop.set_control_flow(flow);
    }
}

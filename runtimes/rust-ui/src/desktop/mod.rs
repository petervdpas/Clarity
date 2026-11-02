//! The desktop host (DESIGN 7.9): each screen in a window of the system's
//! own (winit), drawn by Skia through OpenGL (glutin) when it changed, at
//! the system's content scale. What the person does with the pointer, the
//! wheel, keys and an input method goes to its screen in its units; the
//! pointer takes the system's shape of what is under it, and a press held
//! as the window loses the focus is dropped. The theme follows the system's
//! light or dark, at start and when it changes (the system's own event where
//! it has one, else read every 3 s). A window wakes at the time an element
//! asked for, when work off the screen's thread is done, and draws every
//! frame while one animates; otherwise it waits.
//!
//! `run` shows a generated program: its windows open in place in their
//! desktop window, while a dialog, the keys window and a target marked `in
//! new window` get a desktop window of their own, each its own screen and
//! OpenGL surface. A dialog's window is sized to the dialog with its first
//! part focused, and the window under it takes no input while it is open;
//! what it says as it closes shows in the window below. A window that names
//! its size grows the desktop window to it, and a sidebar that opens grows
//! it by its room. The file dialogs are the system's own, over the window
//! the person works in, which takes no input meanwhile. Whatever fails on
//! the loop (what the person did, drawing, a resize's layout, time going
//! on) is said as an error message and the program goes on (`FailSafe`). A
//! window shows the application's icon, and gets the best OpenGL the system
//! has, or draws in software (`window_system`). `show` shows any screen.
//! Both end when the program's own windows closed.

mod clipboard;
pub mod fail_safe;
pub mod files;
pub mod grow;
pub mod input;
pub mod sound;
pub mod theme_events;
pub mod window_system;

use std::cell::RefCell;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext, Version};
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
use winit::window::{CursorIcon, Icon, Theme, Window, WindowAttributes, WindowId};

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Level, Message, Text};

use crate::app_files::AppFiles;
use crate::dialogs::keys::{self, KeysWindow};
use crate::element::{Composition, Element, ElementId};
use crate::geometry::Rect;
use crate::input::{Cursor, KeyInput, Modifiers, PointerButton, PressCounter};
use crate::locale::{fill, translate};
use crate::screen::Screen;
use crate::theme::ThemeLook;
use crate::window::{Apart, ApartClosed, Running, WindowStack};

use self::fail_safe::{panic_text, FailSafe};
use self::files::SystemFilePicker;
use self::input::{key_of, modifiers_of, typed_text, wheel_of, PointerPlace};
use self::window_system::{SoftwareSurface, CONTEXTS};

pub use self::input::WHEEL_NOTCH;

/// The size a window that names none opens at, in the screen's units, on every stack.
pub const STANDARD_SIZE: (f64, f64) = (1100.0, 760.0);

/// The look for light (false) or dark (true), as the system's theme asks for it.
pub type Looks = Box<dyn Fn(bool) -> Arc<ThemeLook>>;

/// How a screen shows in its desktop window: its title, the size it opens
/// at in the screen's units, the looks it follows the system's theme with,
/// and its icon.
pub struct ShowOptions {
    pub title: String,
    pub width: f64,
    pub height: f64,
    /// The look for the system's light or dark; none keeps the screen's own look.
    pub looks: Option<Looks>,
    /// The application's icon (a PNG); none shows Clarity's.
    pub icon: Option<&'static [u8]>,
}

impl ShowOptions {
    /// A window with this title at the standard size, in the screen's own look.
    pub fn new(title: impl Into<String>) -> ShowOptions {
        ShowOptions { title: title.into(), width: STANDARD_SIZE.0, height: STANDARD_SIZE.1, looks: None, icon: None }
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

    /// The icon its windows show, a PNG; none shows Clarity's.
    pub fn icon(mut self, png: Option<&'static [u8]>) -> ShowOptions {
        self.icon = png;
        self
    }
}

thread_local! {
    static FILES: RefCell<Option<Arc<SystemFilePicker>>> = const { RefCell::new(None) };
}

/// What the desktop gives the parts before its first window: sound through
/// rodio (the speakers opened when a sound first plays) and the system's
/// file dialogs, which every window's parts use. The picker given is the
/// one the host asks whether a dialog is open.
pub fn prepare() -> Arc<SystemFilePicker> {
    crate::parts::sound::set_output(Rc::new(self::sound::Speakers::default()));
    desktop_files()
}

/// The system's file dialogs the desktop gave its parts; made and given the first time.
fn desktop_files() -> Arc<SystemFilePicker> {
    FILES.with(|f| {
        f.borrow_mut()
            .get_or_insert_with(|| {
                let files = Arc::new(SystemFilePicker::default());
                crate::files::set_system(files.clone());
                files
            })
            .clone()
    })
}

/// Runs a generated program on Clarity.UI: its windows in desktop windows,
/// in its look for the system's light or dark. Ends when its own windows
/// closed or it quit.
pub fn run(program: impl Program, files: AppFiles) -> ExitCode {
    // Driven without a window, step by step from standard input (DESIGN 1.6).
    if std::env::var_os(crate::headless::VARIABLE).is_some_and(|v| !v.is_empty()) {
        return crate::headless::run(program, files);
    }
    // The fonts the project carries (`fonts/*.ttf` in its resources) come before the system's.
    crate::fonts::carry_fallbacks(files.resources);
    // The language and theme the person chose in an earlier run, kept beside the program's settings.
    let kept = clarity_runtime::settings::file(files.title).and_then(|f| f.parent().map(|p| p.to_path_buf()));
    crate::choices::start(&files, kept);
    prepare();
    let stack = WindowStack::start(Arc::new(program), files);
    // Dialogs, the keys window and new windows get desktop windows of their own.
    stack.running().set_apart(true);
    let title = stack.title();
    let (width, height) = grow::open_size(&stack);
    let screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
    let options = ShowOptions::new(title).size(width, height).following(move |dark| crate::choices::look(&files, dark));
    show(screen, options.icon(files.resource("icon.png")))
}

/// Shows a screen in a desktop window until the window closes (or, for a
/// program's `WindowStack`, until its own windows closed).
pub fn show(screen: Screen, mut options: ShowOptions) -> ExitCode {
    let event_loop = match EventLoop::<Wake>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(why) => return failed(&why.to_string()),
    };
    let looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>> = options.looks.take().map(Rc::from);
    let mut host = Host::new(event_loop.create_proxy(), looks, window_system::window_icon(options.icon));
    host.untold = untold_moves(&event_loop);
    host.add(screen, options.title.clone(), (options.width, options.height), None, false);
    let ran = event_loop.run_app(&mut host);
    // The system's light or dark is no longer followed once the windows are gone.
    host.theme_following = None;
    for win in host.wins.iter_mut() {
        let _ = catch_unwind(AssertUnwindSafe(|| win.screen.dispose()));
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

/// The system's pointer of a shape.
pub fn standard_of(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Hand => CursorIcon::Pointer,
        Cursor::Text => CursorIcon::Text,
        Cursor::ResizeLeftRight => CursorIcon::EwResize,
        Cursor::ResizeUpDown => CursorIcon::NsResize,
        Cursor::Move => CursorIcon::Move,
        Cursor::Crosshair => CursorIcon::Crosshair,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Default => CursorIcon::Default,
    }
}

/// What a window of its own said and still shows as it closes (a dialog's
/// handler that says a message and closes it) goes to the window below.
pub fn hand_down(from: &mut Vec<Message>, to: &mut Vec<Message>) {
    to.append(from);
}

/// Whether a window takes no input: a dialog of its own is open over it, or a file dialog of the system's.
pub fn blocks_input(modal_child: bool, files: &SystemFilePicker) -> bool {
    modal_child || files.showing()
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
/// is done, or the system's light or dark changed (as following it tells
/// where the window system does not, X11).
#[derive(Debug, Clone, Copy)]
enum Wake {
    Done,
    Dark(bool),
}

/// How Skia draws in a window: through its OpenGL, or in memory put on the window as pixels.
enum Drawing {
    Gpu(DirectContext),
    Software(SoftwareSurface),
}

/// The window as it shows: the system's window, its OpenGL surface and
/// context, and Skia's drawing and surface over them.
struct Shown {
    // Dropped in this order: Skia's surface and context before OpenGL's, the window last.
    surface: Surface,
    gl_surface: GlutinSurface<WindowSurface>,
    drawing: Drawing,
    gl_context: PossiblyCurrentContext,
    config: Config,
    window: Window,
}

impl Drop for Shown {
    fn drop(&mut self) {
        // Some drivers crash when Skia frees its resources after the context went.
        let _ = self.gl_context.make_current(&self.gl_surface);
        if let Drawing::Gpu(skia) = &mut self.drawing {
            skia.release_resources_and_abandon();
        }
    }
}

impl Shown {
    /// Opens the window with the first OpenGL context of `CONTEXTS` from
    /// `first` on that the system gives, and Skia over it (drawing in memory
    /// where it cannot use that OpenGL, or `software` asks); the first window
    /// finds the config every later one uses. With it, the context it got.
    fn open(event_loop: &ActiveEventLoop, attributes: WindowAttributes, config: Option<&Config>, first: usize, software: bool) -> Result<(Shown, usize), String> {
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
        // OpenGL 3.3 core first, then older contexts; else OpenGL ES, which some systems have alone.
        let mut found = None;
        for (i, wanted) in CONTEXTS.iter().enumerate().skip(first) {
            let mut attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(wanted.major, wanted.minor))));
            if wanted.core {
                attributes = attributes.with_profile(GlProfile::Core);
            }
            match unsafe { display.create_context(&config, &attributes.build(Some(handle))) } {
                Ok(context) => {
                    found = Some((context, i));
                    break;
                }
                Err(why) => eprintln!("{}", fill(&fill(&translate("desktopNoContext", "OpenGL {version} is not there: {why}"), "version", &format!("{}.{}", wanted.major, wanted.minor)), "why", &why.to_string())),
            }
        }
        let (context, got) = match found {
            Some(found) => found,
            None => {
                let gles = unsafe { display.create_context(&config, &ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(Some(handle))) };
                (gles.map_err(|why| fill(&translate("desktopNoGl", "this system gives no OpenGL to draw a window with: {why}"), "why", &why.to_string()))?, first)
            }
        };
        let surface_attributes = window.build_surface_attributes(Default::default()).map_err(|why| why.to_string())?;
        let gl_surface = unsafe { display.create_window_surface(&config, &surface_attributes).map_err(|why| why.to_string())? };
        let gl_context = context.make_current(&gl_surface).map_err(|why| why.to_string())?;
        // Paced by the display: a frame while an element animates, no faster.
        let _ = gl_surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::MIN));
        let find = |name: &std::ffi::CStr| display.get_proc_address(name);
        let skia = if software {
            None
        } else {
            gpu::gl::Interface::new_load_with(|name| {
                if name == "eglGetCurrentDisplay" {
                    return std::ptr::null();
                }
                match CString::new(name) {
                    Ok(name) => display.get_proc_address(name.as_c_str()),
                    Err(_) => std::ptr::null(),
                }
            })
            .and_then(|interface| gpu::direct_contexts::make_gl(interface, None))
        };
        let mut drawing = match skia {
            Some(skia) => Drawing::Gpu(skia),
            None => {
                // Skia cannot use this OpenGL (below 2.0), or the person asked: it draws in memory, put on the window as pixels.
                let soft = SoftwareSurface::new(find).ok_or_else(|| translate("desktopNoSoftware", "this OpenGL can show nothing Clarity draws"))?;
                eprintln!("{}", translate("desktopSoftware", "Clarity draws in software"));
                Drawing::Software(soft)
            }
        };
        let surface = Self::surface_of(&mut drawing, &config, window.inner_size())?;
        // Input methods compose in it (the person types Chinese, Japanese, accents).
        window.set_ime_allowed(true);
        Ok((Shown { surface, gl_surface, drawing, gl_context, config, window }, got))
    }

    /// Skia's surface over the window's own framebuffer, in its pixels; in memory when it draws in software.
    fn surface_of(drawing: &mut Drawing, config: &Config, size: PhysicalSize<u32>) -> Result<Surface, String> {
        let no_surface = || translate("desktopNoSurface", "Skia could not draw in the window");
        match drawing {
            Drawing::Software(_) => SoftwareSurface::fit(size.width, size.height).ok_or_else(no_surface),
            Drawing::Gpu(skia) => {
                let info = FramebufferInfo { fboid: 0, format: gpu::gl::Format::RGBA8.into(), ..Default::default() };
                let pixels = (size.width.max(1) as i32, size.height.max(1) as i32);
                let target = backend_render_targets::make_gl(pixels, config.num_samples() as usize, config.stencil_size() as usize, info);
                gpu::surfaces::wrap_backend_render_target(skia, &target, SurfaceOrigin::BottomLeft, ColorType::RGBA8888, None, None).ok_or_else(no_surface)
            }
        }
    }

    /// The scale the window draws at and its pixels per screen unit, at the system's content scale.
    fn scale(&self) -> (f32, f64) {
        let size = self.window.inner_size();
        window_system::scale_of((size.width, size.height), (size.width, size.height), self.window.scale_factor() as f32)
    }

    /// The window's pixels per screen unit.
    fn per_unit(&self) -> f64 {
        self.scale().1
    }

    /// The window's size in the screen's units.
    fn size(&self) -> (f64, f64) {
        let size = self.window.inner_size();
        let per_unit = self.per_unit();
        (size.width as f64 / per_unit, size.height as f64 / per_unit)
    }

    /// Asks for the window's size, given in the screen's units.
    fn resize(&self, size: (f64, f64)) {
        let per_unit = self.per_unit();
        let _ = self.window.request_inner_size(PhysicalSize::new((size.0 * per_unit).round() as u32, (size.1 * per_unit).round() as u32));
    }

    /// The room the window's screen has, in the screen's units, less the system's own bars; none when it is not known.
    fn room(&self) -> Option<(f64, f64)> {
        let monitor = self.window.current_monitor()?.size();
        grow::room_of((monitor.width, monitor.height), self.per_unit())
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
    /// The window view whose sidebars were last seen, and how wide they were: one that opens grows the desktop window.
    sidebars: Option<(ElementId, f64)>,
}

/// Every desktop window, and what they share.
struct Host {
    wins: Vec<Win>,
    looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>>,
    waker: crate::later::Waker,
    config: Option<Config>,
    /// The first of `CONTEXTS` the system gave, which every later window asks for first.
    context: usize,
    failure: Option<String>,
    dark: Option<bool>,
    started: Instant,
    next_key: u64,
    proxy: EventLoopProxy<Wake>,
    /// The system's light or dark followed, where the window system tells no change (X11).
    theme_following: Option<crate::system_theme::Following>,
    /// Whether the window system leaves a move back to where the pointer last moved untold (X11).
    untold: bool,
    /// The window the person works in: what fails is said there, and file dialogs belong to it.
    active: Option<u64>,
    files: Arc<SystemFilePicker>,
    icon: Option<Icon>,
    fail_safe: FailSafe,
    /// What failed and is to be said in the window the person works in.
    failures: Rc<RefCell<Vec<String>>>,
}

impl Host {
    fn new(proxy: EventLoopProxy<Wake>, looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>>, icon: Option<Icon>) -> Host {
        let shared = Mutex::new(proxy.clone());
        let waker: crate::later::Waker = Arc::new(move || {
            if let Ok(proxy) = shared.lock() {
                let _ = proxy.send_event(Wake::Done);
            }
        });
        let failures: Rc<RefCell<Vec<String>>> = Rc::default();
        let said = failures.clone();
        let fail_safe = FailSafe::new(move |what| said.borrow_mut().push(what.to_string()));
        Host {
            wins: Vec::new(),
            looks,
            waker,
            config: None,
            context: 0,
            failure: None,
            dark: None,
            started: Instant::now(),
            next_key: 1,
            proxy,
            theme_following: None,
            untold: false,
            active: None,
            files: desktop_files(),
            icon,
            fail_safe,
            failures,
        }
    }

    /// Milliseconds since the host started: the fail safe's clock.
    fn now(&self) -> i64 {
        self.started.elapsed().as_millis() as i64
    }

    /// Runs `act`; whatever fails in it is said in the window the person works in, and the program goes on.
    fn safely(&mut self, act: impl FnOnce(&mut Host)) {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| act(self))) {
            let now = self.now();
            self.fail_safe.failed(&panic_text(payload.as_ref()), now);
        }
    }

    /// What failed is said as an error message in the window the person works in (or the first program window).
    fn say_failures(&mut self) {
        let said: Vec<String> = self.failures.borrow_mut().drain(..).collect();
        if said.is_empty() {
            return;
        }
        let active = self.active.and_then(|k| self.index_by_key(k));
        let at = active.filter(|i| self.wins[*i].screen.root().as_any().is::<WindowStack>()).or_else(|| self.wins.iter().position(|w| w.screen.root().as_any().is::<WindowStack>()));
        let Some(i) = at else { return };
        for what in said {
            let screen = &mut self.wins[i].screen;
            // Saying it must not fail in turn; if it does, the error output has it.
            let _ = catch_unwind(AssertUnwindSafe(|| screen.act_root::<WindowStack, _>(|stack, cx| stack.say(Level::Error, Text::plain(what), cx))));
        }
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
        });
    }

    fn index_of(&self, id: WindowId) -> Option<usize> {
        self.wins.iter().position(|w| w.shown.as_ref().is_some_and(|s| s.window.id() == id))
    }

    fn index_by_key(&self, key: u64) -> Option<usize> {
        self.wins.iter().position(|w| w.key == key)
    }

    /// Whether a window takes no input: a dialog of its own is open over it, or a file dialog of the system's.
    fn blocked(&self, i: usize) -> bool {
        let key = self.wins[i].key;
        blocks_input(self.wins.iter().any(|w| w.owner == Some(key) && w.modal), &self.files)
    }

    /// The person works in the window `key` now: file dialogs belong to it.
    fn set_active(&mut self, key: Option<u64>) {
        self.active = key;
        let owner = key.and_then(|k| self.index_by_key(k)).and_then(|i| self.wins[i].shown.as_ref()).and_then(|s| window_system::handle(&s.window));
        self.files.set_owner(owner);
    }

    /// Shows the windows not shown yet; a window opened over another lies in its middle.
    fn open_waiting(&mut self, event_loop: &ActiveEventLoop) {
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
                .with_window_icon(self.icon.clone());
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

    /// The screen takes the window's size in its units, at the system's content scale.
    fn fit(&mut self, i: usize) {
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
        match &mut shown.drawing {
            Drawing::Software(soft) => {
                soft.present(&mut shown.surface);
                let _ = shown.gl_surface.swap_buffers(&shown.gl_context);
            }
            // A frame that did not reach OpenGL is left out, not shown half drawn.
            Drawing::Gpu(skia) => {
                if skia.flush_and_submit().is_ok() {
                    let _ = shown.gl_surface.swap_buffers(&shown.gl_context);
                }
            }
        }
    }

    /// A sidebar of the window shown opened: the desktop window grows by the room it takes (a dialog keeps its size).
    fn widen(&mut self, i: usize) {
        let win = &mut self.wins[i];
        if win.modal {
            return;
        }
        let Some(shown) = &win.shown else { return };
        let Some(stack) = win.screen.root().as_any().downcast_ref::<WindowStack>() else { return };
        let Some(view) = stack.view() else { return };
        let (id, width) = (view.node().id(), grow::sidebars_width(view));
        if let Some((was_view, was)) = win.sidebars {
            if was_view == id && width > was {
                let now = shown.size();
                let grown = grow::sidebar_grown(now, was, width, shown.room());
                if grown != now {
                    shown.resize(grown);
                }
            }
        }
        win.sidebars = Some((id, width));
    }

    /// Closes a desktop window, the windows opened over it first; what it
    /// shows hears its `on close` unless it ended by itself, what it said
    /// and still shows goes to the window below, and the window that opened
    /// it is told.
    fn close(&mut self, key: u64) {
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
                    shown = stack.notes();
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
    fn tell_opener(&mut self, closed: ApartClosed) {
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
    fn open_apart(&mut self, apart: Apart, running: &Rc<Running>, owner: u64) {
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
        let per_unit = self.wins[i].shown.as_ref().map(Shown::per_unit).unwrap_or(1.0);
        (x / per_unit, y / per_unit)
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

    /// The pointer takes the system's shape of what is under it.
    fn shape_pointers(&mut self) {
        for win in self.wins.iter_mut() {
            if win.screen.take_cursor_changed() {
                if let Some(shown) = &win.shown {
                    shown.window.set_cursor(standard_of(win.screen.cursor()));
                }
            }
        }
    }

    /// What the system says of a window, and what the person did on it.
    fn on_window(&mut self, event_loop: &ActiveEventLoop, i: usize, event: WindowEvent) {
        // Where the pointer is, kept also while a dialog over the window takes its input.
        match &event {
            WindowEvent::CursorEntered { .. } => {
                self.wins[i].inside = true;
                self.wins[i].pointer.entered();
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].inside = false,
            WindowEvent::Focused(true) => {
                self.wins[i].pointer.focused();
                let key = self.wins[i].key;
                self.set_active(Some(key));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = self.at(i, position.x, position.y);
                self.wins[i].pointer.told(x, y);
            }
            _ => {}
        }
        if self.blocked(i) && (is_input(&event) || matches!(event, WindowEvent::CloseRequested)) {
            // A window with a dialog over it (its own or a file dialog) stays, and takes nothing.
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
            WindowEvent::Focused(false) => {
                let win = &mut self.wins[i];
                win.held = Modifiers::NONE;
                // Losing the focus mid-press (Alt+Tab during a drag) drops the drag.
                win.screen.pointer_cancelled();
            }
            WindowEvent::CursorMoved { .. } => {
                let win = &mut self.wins[i];
                let (x, y) = win.pointer.at();
                win.screen.pointer_moved(x, y);
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].screen.pointer_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(button) = button_of(button) else { return };
                let elapsed = self.now();
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
                let per_unit = self.wins[i].shown.as_ref().map(Shown::per_unit).unwrap_or(1.0);
                let (dx, dy) = wheel_of(delta, per_unit);
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
                        let _ = win.screen.key_pressed(key);
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
}

fn button_of(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        _ => None,
    }
}

/// Whether an event is the person's input, which a window under a dialog does not take.
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
            self.safely(|host| host.follow_theme(Some(if dark { Theme::Dark } else { Theme::Light })));
            for win in self.wins.iter() {
                if let Some(shown) = &win.shown {
                    shown.window.request_redraw();
                }
            }
            return;
        }
        for i in 0..self.wins.len() {
            self.safely(|host| {
                host.wins[i].screen.take_later();
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(i) = self.index_of(id) else { return };
        self.safely(|host| host.on_window(event_loop, i, event));
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
        self.safely(|host| host.follow_stacks(event_loop));
        self.open_waiting(event_loop);
        self.place_ime();
        let now = Instant::now();
        for i in 0..self.wins.len() {
            // An element whose time came wakes (a trend moves on, a tooltip shows).
            if self.wins[i].screen.next_wake().is_some_and(|at| at <= now) {
                self.safely(|host| host.wins[i].screen.wake_due(now));
            }
            self.safely(|host| host.widen(i));
        }
        self.say_failures();
        self.shape_pointers();
        let mut animating = false;
        let mut next: Option<Instant> = None;
        for win in self.wins.iter_mut() {
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

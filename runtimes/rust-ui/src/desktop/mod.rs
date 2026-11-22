//! The desktop host (manual/start.how-it-works.md): each screen in a window of the system's
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

mod agents;
mod clipboard;
mod desktop_windows;
mod reader;
mod starting;
mod window_events;
pub mod fail_safe;
pub mod files;
pub mod grow;
pub mod input;
#[cfg(feature = "audio")]
pub mod music;
pub mod sound;
pub mod theme_events;
#[cfg(feature = "video")]
pub mod videos;
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
use self::files::SystemStorageProvider;
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
    static FILES: RefCell<Option<Arc<SystemStorageProvider>>> = const { RefCell::new(None) };
}

/// What the desktop gives the parts before its first window: sound through
/// rodio (the speakers opened when a sound first plays) and the system's
/// file dialogs, which every window's parts use. The picker given is the
/// one the host asks whether a dialog is open. With the `audio` feature,
/// songs play through Clarity.Audio.
pub fn prepare() -> Arc<SystemStorageProvider> {
    crate::parts::sound::set_output(Rc::new(self::sound::Speakers::default()));
    #[cfg(feature = "audio")]
    crate::parts::sound::set_music(Some(Rc::new(self::music::ClarityAudio::default())));
    #[cfg(feature = "video")]
    crate::parts::video::set_videos(Some(Rc::new(self::videos::ClarityVideo)));
    desktop_files()
}

/// The system's file dialogs the desktop gave its parts; made and given the first time.
fn desktop_files() -> Arc<SystemStorageProvider> {
    FILES.with(|f| {
        f.borrow_mut()
            .get_or_insert_with(|| {
                let files = Arc::new(SystemStorageProvider::default());
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
    run_shared(Arc::new(program), files)
}

/// Runs a program held elsewhere too (one the C interface gives), as `run` does.
pub fn run_shared(program: Arc<dyn Program>, files: AppFiles) -> ExitCode {
    crate::app_files::set_running(files);
    // Driven without a window, step by step from standard input (manual/start.how-it-works.md).
    if std::env::var_os(crate::headless::VARIABLE).is_some_and(|v| !v.is_empty()) {
        return crate::headless::run_shared(program, files);
    }
    // The fonts the project carries (`fonts/*.ttf` in its resources) come before the system's.
    crate::fonts::carry_fallbacks(files.resources);
    // The language and theme the person chose in an earlier run, kept beside the program's settings.
    let kept = clarity_runtime::settings::file(files.title).and_then(|f| f.parent().map(|p| p.to_path_buf()));
    crate::choices::start(&files, kept);
    prepare();
    let running = WindowStack::prepare(program, files);
    let started = running.program.started();
    // The splash screen the app names shows first, unless the databases did
    // not open or people sign in first on another window (it comes before
    // `on start`). A splash that is the sign-in window shows, and stays.
    if let (Some(splash), None) = (files.splash(), &started.unconnected) {
        let doubles = started.signin.as_deref() == Some(splash.page.as_str());
        if started.signin.is_none() || doubles {
            return show_splash(running, splash, files, doubles);
        }
    }
    // People sign in first on a sign-in window of its own: a desktop window at
    // its size; the main window gets one of its own once someone signed in.
    let stack = match (&started.unconnected, &started.signin) {
        (None, Some(page)) => {
            running.set_apart(true);
            WindowStack::signin_window(running, page)
        }
        _ => WindowStack::begin(running, started),
    };
    // Dialogs, the keys window and new windows get desktop windows of their own.
    stack.running().set_apart(true);
    let title = stack.title();
    let (width, height) = grow::open_size(&stack);
    let screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
    let options = ShowOptions::new(title).size(width, height).following(move |dark| crate::choices::look(&files, dark));
    show(screen, options.icon(files.icon()))
}

/// Shows the app's splash screen while its `on start` runs, then its main window (see `splash`).
fn show_splash(running: Rc<Running>, splash: crate::app_files::Splash, files: AppFiles, signs_in: bool) -> ExitCode {
    running.set_apart(true);
    let stack = if signs_in { WindowStack::splash_signing_in(running.clone(), &splash.page) } else { WindowStack::splash(running.clone(), &splash.page) };
    let (width, height) = grow::open_size(&stack);
    let screen = Screen::new(Box::new(stack), crate::choices::look(&files, false));
    let options = ShowOptions::new(files.title).size(width, height).following(move |dark| crate::choices::look(&files, dark)).icon(files.icon());
    let starting = Starting { running, at_least: splash.at_least, opened: None, asked: false, reply: None, next_status: Instant::now(), signs_in };
    show_starting(screen, options, Some(starting))
}

/// Shows a screen in a desktop window until the window closes (or, for a
/// program's `WindowStack`, until its own windows closed).
pub fn show(screen: Screen, options: ShowOptions) -> ExitCode {
    show_starting(screen, options, None)
}

/// As `show`; with `starting`, the screen is the app's splash and the main window follows.
fn show_starting(screen: Screen, mut options: ShowOptions, starting: Option<Starting>) -> ExitCode {
    let event_loop = match EventLoop::<Wake>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(why) => return failed(&why.to_string()),
    };
    let looks: Option<Rc<dyn Fn(bool) -> Arc<ThemeLook>>> = options.looks.take().map(Rc::from);
    let mut host = Host::new(event_loop.create_proxy(), looks, window_system::window_icon(options.icon));
    *OUTSIDE.lock().unwrap_or_else(|p| p.into_inner()) = Some(event_loop.create_proxy());
    host.untold = untold_moves(&event_loop);
    host.add(screen, options.title.clone(), (options.width, options.height), None, false);
    host.listen_for_agents(event_loop.create_proxy());
    if starting.is_some() {
        host.wins[0].splash = true;
        host.starting = starting;
    }
    let ran = event_loop.run_app(&mut host);
    *OUTSIDE.lock().unwrap_or_else(|p| p.into_inner()) = None;
    // The system's light or dark is no longer followed once the windows are gone.
    host.theme_following = None;
    for win in host.wins.iter_mut() {
        let _ = catch_unwind(AssertUnwindSafe(|| win.screen.dispose()));
        // The screen reader's adapter and Skia's context go before the window.
        win.reader = None;
        win.shown = None;
    }
    match (ran, host.failure) {
        (_, Some(why)) => failed(&why),
        (Err(why), None) => failed(&why.to_string()),
        (Ok(()), None) => ExitCode::SUCCESS,
    }
}

/// The desktop window stays out of the taskbar: on Windows by its own
/// setting; on X11 a borderless one is a splash window, which taskbars leave
/// out. Wayland and macOS have no such setting for a single window.
#[allow(unused_mut)]
fn not_in_taskbar(mut attributes: WindowAttributes, decorations: bool) -> WindowAttributes {
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attributes = attributes.with_skip_taskbar(true);
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    if !decorations {
        use winit::platform::x11::{WindowAttributesExtX11, WindowType};
        attributes = attributes.with_x11_window_type(vec![WindowType::Splash]);
    }
    let _ = decorations;
    attributes
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
        Cursor::ResizeCorner => CursorIcon::NwseResize,
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
pub fn blocks_input(modal_child: bool, files: &SystemStorageProvider) -> bool {
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
#[derive(Debug)]
enum Wake {
    Done,
    /// The app's `on start` ran on its own thread while the splash showed: what it answered.
    Started(clarity_runtime::program::Answer<clarity_runtime::ui::Reply>),
    Dark(bool),
    /// Woken from another thread (`wake`): every window draws anew.
    Outside,
    /// An AI agent's grant to ask the person, or its call to run (crate::agents).
    Agent(crate::agents::AgentWork),
    /// The system's screen reader asks for the tree, or for an action (manual/start.how-it-works.md).
    Access(accesskit_winit::Event),
}

impl From<accesskit_winit::Event> for Wake {
    fn from(event: accesskit_winit::Event) -> Wake {
        Wake::Access(event)
    }
}

/// A window's link to the system's screen reader: AccessKit's adapter,
/// whether a screen reader reads it now, and the tree it was told last.
struct Reader {
    adapter: accesskit_winit::Adapter,
    active: bool,
    told: Option<crate::access::AccessTree>,
}

/// The loop that runs now, to wake from another thread; none while none runs.
static OUTSIDE: Mutex<Option<EventLoopProxy<Wake>>> = Mutex::new(None);

/// Wakes the desktop that runs now from any thread: every window reads its
/// values again and draws. Does nothing while no desktop runs.
pub fn wake() {
    if let Some(proxy) = OUTSIDE.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        let _ = proxy.send_event(Wake::Outside);
    }
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
    /// What the system's screen reader reads of it; none until it shows. It goes before the window.
    reader: Option<Reader>,
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
    /// The window view whose sidebars were last seen, and how: one that opens grows the desktop window.
    sidebars: Option<(ElementId, grow::Sidebars)>,
    /// It is the app's splash screen, which closes once the main window shows.
    splash: bool,
    /// What its window's markup says of the desktop window: borderless, centred, on top...
    look: crate::markup::WindowLook,
}

/// While the app starts behind its splash screen: what its windows share,
/// how long the splash shows at least and since when, whether `on start`
/// was asked to run, what it answered once it did, and when the status is
/// asked for next.
struct Starting {
    running: Rc<Running>,
    at_least: Duration,
    opened: Option<Instant>,
    asked: bool,
    reply: Option<clarity_runtime::program::Answer<clarity_runtime::ui::Reply>>,
    next_status: Instant,
    /// The splash is the sign-in window too: it gets what `on start` answered and stays.
    signs_in: bool,
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
    files: Arc<SystemStorageProvider>,
    icon: Option<Icon>,
    /// The app starting behind its splash screen; none once the splash closed, or without one.
    starting: Option<Starting>,
    fail_safe: FailSafe,
    /// What failed and is to be said in the window the person works in.
    failures: Rc<RefCell<Vec<String>>>,
    /// The app listening for AI agents while it runs (docs/ledger/app-mcp.md); its running.json goes with it.
    agents: Option<clarity_app_mcp::relay::Listening>,
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
            starting: None,
            fail_safe,
            failures,
            agents: None,
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
        let event = match event {
            Wake::Agent(work) => {
                self.safely(|host| host.agent_work(work));
                return;
            }
            Wake::Access(asked) => {
                self.safely(|host| host.on_reader(asked));
                return;
            }
            Wake::Started(reply) => {
                // Kept until the splash finishes; looked at on the next turn.
                if let Some(starting) = self.starting.as_mut() {
                    starting.reply = Some(reply);
                }
                return;
            }
            other => other,
        };
        if let Wake::Dark(dark) = event {
            self.safely(|host| host.follow_theme(Some(if dark { Theme::Dark } else { Theme::Light })));
            for win in self.wins.iter() {
                if let Some(shown) = &win.shown {
                    shown.window.request_redraw();
                }
            }
            return;
        }
        let outside = matches!(event, Wake::Outside);
        for i in 0..self.wins.len() {
            self.safely(|host| {
                host.wins[i].screen.take_later();
                if outside {
                    host.wins[i].screen.invalidate();
                }
            });
            if outside {
                if let Some(shown) = &self.wins[i].shown {
                    shown.window.request_redraw();
                }
            }
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
        let mut splash_look = None;
        self.safely(|host| splash_look = host.follow_start());
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
        next = match (next, splash_look) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let flow = match next {
            Some(at) if !animating => ControlFlow::WaitUntil(at),
            _ => ControlFlow::Wait,
        };
        event_loop.set_control_flow(flow);
    }
}

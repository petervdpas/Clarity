//! The desktop host (DESIGN 7.9): a screen in a window of the system's own
//! (winit), drawn by Skia through OpenGL (glutin) when it changed, at the
//! window's own scale. What the person does with the pointer, the wheel and
//! keys goes to the screen in its units (logical pixels). The theme follows
//! the system's light or dark, at start and when it changes. The window
//! wakes at the time an element asked for and draws every frame while one
//! animates; otherwise it waits.
//!
//! `run` shows a generated program (its `WindowStack`); `show` shows any
//! screen. Both end when the window closes, or when the program's last
//! window closed.

mod clipboard;
pub mod input;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use skia_safe::gpu::{self, backend_render_targets, gl::FramebufferInfo, DirectContext, SurfaceOrigin};
use skia_safe::{ColorType, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Theme, Window, WindowAttributes, WindowId};

use clarity_runtime::program::Program;

use crate::app_files::AppFiles;
use crate::input::{KeyInput, Modifiers, PointerButton, PressCounter};
use crate::locale::{fill, translate};
use crate::screen::Screen;
use crate::theme::ThemeLook;
use crate::window::WindowStack;

use self::input::{key_of, modifiers_of, typed_text, wheel_of};

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

/// Runs a generated program on Clarity.UI: its windows in one desktop window,
/// in its look for the system's light or dark. Ends when the window closes or
/// the program's last window closed.
pub fn run(program: impl Program, files: AppFiles) -> ExitCode {
    let stack = WindowStack::start(Arc::new(program), files);
    let title = stack.title();
    let (width, height) = stack.wanted_size().unwrap_or(STANDARD_SIZE);
    let screen = Screen::new(Box::new(stack), files.look(false));
    show(screen, ShowOptions::new(title).size(width, height).following(move |dark| files.look(dark)))
}

/// Shows a screen in a desktop window until the window closes (or, for a
/// program's `WindowStack`, until its last window closed).
pub fn show(mut screen: Screen, options: ShowOptions) -> ExitCode {
    if let Some(system) = clipboard::SystemClipboard::open() {
        screen.set_clipboard(Box::new(system));
    }
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(why) => return failed(&why.to_string()),
    };
    let mut host = Host::new(screen, options);
    let ran = event_loop.run_app(&mut host);
    host.screen.dispose();
    // Skia's context goes before the window it draws in.
    host.shown = None;
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
        self.skia.release_resources_and_abandon();
    }
}

impl Shown {
    /// Opens the window with an OpenGL context and Skia over it.
    fn open(event_loop: &ActiveEventLoop, options: &ShowOptions) -> Result<Shown, String> {
        let attributes = WindowAttributes::default()
            .with_title(options.title.clone())
            .with_inner_size(LogicalSize::new(options.width.max(1.0), options.height.max(1.0)));
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        // The config with the fewest samples: Skia smooths edges itself.
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attributes))
            .build(event_loop, template, |configs| configs.reduce(|a, b| if b.num_samples() < a.num_samples() { b } else { a }).expect("the system offers an OpenGL config"))
            .map_err(|why| why.to_string())?;
        let window = window.ok_or_else(|| translate("desktopNoWindow", "the system made no window"))?;
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

/// The screen in its desktop window, and what the window gave last.
struct Host {
    screen: Screen,
    options: ShowOptions,
    shown: Option<Shown>,
    failure: Option<String>,
    title: String,
    dark: Option<bool>,
    held: Modifiers,
    pointer: (f64, f64),
    presses: PressCounter,
    started: Instant,
}

impl Host {
    fn new(screen: Screen, options: ShowOptions) -> Host {
        let title = options.title.clone();
        Host { screen, options, shown: None, failure: None, title, dark: None, held: Modifiers::NONE, pointer: (0.0, 0.0), presses: PressCounter::new(), started: Instant::now() }
    }

    /// The screen takes the window's size in its units, at the window's scale.
    fn fit(&mut self) {
        let Some(shown) = self.shown.as_mut() else { return };
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
        self.screen.resize(size.width.max(1) as f64 / scale, size.height.max(1) as f64 / scale, scale as f32);
    }

    /// The look for the system's light or dark, when it changed.
    fn follow_theme(&mut self, theme: Option<Theme>) {
        let dark = theme == Some(Theme::Dark);
        if self.dark == Some(dark) {
            return;
        }
        self.dark = Some(dark);
        if let Some(looks) = &self.options.looks {
            self.screen.show(looks(dark));
        }
    }

    /// The program's title in the window's bar, and the end once its last window closed.
    fn follow_stack(&mut self, event_loop: &ActiveEventLoop) {
        let Some((title, ended)) = self.screen.act_root::<WindowStack, _>(|stack, _| (stack.title(), stack.ended())) else { return };
        if ended {
            event_loop.exit();
            return;
        }
        if title != self.title {
            if let Some(shown) = &self.shown {
                shown.window.set_title(&title);
            }
            self.title = title;
        }
    }

    fn draw(&mut self) {
        let Some(shown) = self.shown.as_mut() else { return };
        if let Err(why) = shown.gl_context.make_current(&shown.gl_surface) {
            self.failure = Some(why.to_string());
            return;
        }
        self.screen.draw(shown.surface.canvas());
        // A frame that did not reach OpenGL is left out, not shown half drawn.
        if shown.skia.flush_and_submit().is_ok() {
            let _ = shown.gl_surface.swap_buffers(&shown.gl_context);
        }
    }

    /// The pointer's place in the screen's units.
    fn at(&self, x: f64, y: f64) -> (f64, f64) {
        let scale = self.shown.as_ref().map(Shown::scale).unwrap_or(1.0);
        (x / scale, y / scale)
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

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        match Shown::open(event_loop, &self.options) {
            Ok(shown) => {
                let theme = shown.window.theme().or_else(|| event_loop.system_theme());
                self.shown = Some(shown);
                self.follow_theme(theme);
                self.fit();
                if let Some(shown) = &self.shown {
                    shown.window.request_redraw();
                }
            }
            Err(why) => {
                self.failure = Some(why);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.fit(),
            WindowEvent::ThemeChanged(theme) => self.follow_theme(Some(theme)),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ModifiersChanged(modifiers) => self.held = modifiers_of(modifiers.state()),
            WindowEvent::Focused(false) => self.held = Modifiers::NONE,
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = self.at(position.x, position.y);
                self.pointer = (x, y);
                self.screen.pointer_moved(x, y);
            }
            WindowEvent::CursorLeft { .. } => self.screen.pointer_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(button) = button_of(button) else { return };
                let (x, y) = self.pointer;
                match state {
                    // The window gives only presses: a double or triple click is counted here.
                    ElementState::Pressed => {
                        let clicks = self.presses.press(x, y, button, self.started.elapsed().as_millis() as i64);
                        self.screen.pointer_pressed_with(x, y, button, clicks, self.held);
                    }
                    ElementState::Released => self.screen.pointer_released(x, y, button),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self.shown.as_ref().map(Shown::scale).unwrap_or(1.0);
                let (dx, dy) = wheel_of(delta, scale);
                let (x, y) = self.pointer;
                self.screen.wheel(x, y, dx, dy, self.held);
            }
            // Keys the system repeats for a key held as the window got the focus are not the person's.
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => {
                let key = KeyInput::new(key_of(&event.logical_key, event.physical_key), self.held);
                match event.state {
                    ElementState::Pressed => {
                        self.screen.key_pressed(key);
                        if let Some(text) = typed_text(event.text.as_deref(), self.held) {
                            self.screen.text_typed(&text);
                        }
                    }
                    ElementState::Released => self.screen.key_released(key),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.failure.is_some() {
            event_loop.exit();
            return;
        }
        let now = Instant::now();
        if self.screen.next_wake().is_some_and(|at| at <= now) {
            self.screen.wake_due(now);
        }
        self.follow_stack(event_loop);
        let animating = self.screen.animating() > 0;
        if self.screen.take_redraw() || animating {
            if let Some(shown) = &self.shown {
                shown.window.request_redraw();
            }
        }
        // The next wake-up is asked for anew on every turn, so a changed time needs nothing more.
        self.screen.take_wakes_changed();
        let flow = match self.screen.next_wake() {
            Some(at) if !animating => ControlFlow::WaitUntil(at),
            _ => ControlFlow::Wait,
        };
        event_loop.set_control_flow(flow);
    }
}

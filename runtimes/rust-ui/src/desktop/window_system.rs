//! What a desktop window needs of the system beyond winit's own (DESIGN
//! 7.9): the OpenGL context it can get, drawing in software where Skia
//! cannot use it, the window's real content scale, and its icon.
//!
//! OpenGL: a window asks for OpenGL 3.3 core first, as Skia draws best with
//! it; a driver that has none (an old GPU, a virtual machine, Windows over
//! Remote Desktop, whose own OpenGL is 1.1) gets the newest compatibility
//! context it has, down to 1.1, and else OpenGL ES. Skia draws through the
//! context when it can (OpenGL 2.0 and on); below that, or where
//! CLARITY_RENDER=software says so, Skia draws in memory and the pixels go
//! to the window with glDrawPixels, which every desktop OpenGL has. Only a
//! system with no OpenGL at all shows no window, and says why.

use std::ffi::{c_void, CStr};

use skia_safe::{surfaces, AlphaType, CubicResampler, Data, Image, ImageInfo, Paint, Rect, SamplingOptions, Surface};

/// An OpenGL context a window asks for: its version, and whether it is the core profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlContext {
    pub major: u8,
    pub minor: u8,
    /// The core profile; below 3.2 there are no profiles, and any context of the version is asked for.
    pub core: bool,
}

/// The OpenGL contexts a window asks for, in turn, until the system gives one.
pub const CONTEXTS: [GlContext; 3] = [
    GlContext { major: 3, minor: 3, core: true },
    GlContext { major: 2, minor: 1, core: false },
    GlContext { major: 1, minor: 1, core: false },
];

/// The name of the environment variable that asks for drawing in software.
pub const RENDER: &str = "CLARITY_RENDER";

/// Whether the person asked for drawing in software (CLARITY_RENDER=software), for a driver Skia draws wrongly on.
pub fn software_asked(env: impl Fn(&str) -> Option<String>) -> bool {
    env(RENDER).is_some_and(|v| v.trim().eq_ignore_ascii_case("software"))
}

/// The first of `CONTEXTS` to ask for: drawing in software puts pixels with
/// glDrawPixels, which only a compatibility context has.
pub fn first_context(software: bool, found: usize) -> usize {
    if software { found.max(1) } else { found }
}

/// The scale a window draws at and how many of the system's window units
/// one of the screen's units is: `size` is the window in the system's units,
/// `pixels` its framebuffer, and `content` the content scale the system
/// gives the window (0 when it gave none). On macOS and Wayland the window's
/// units are the screen's already (the framebuffer is larger); on X11 and
/// Windows they are pixels, and the content scale (150 percent, 200) says
/// how large the screen's units are. winit gives the window in pixels, so
/// there its size and framebuffer are the same and the content scale is
/// its scale factor.
pub fn scale_of(size: (u32, u32), pixels: (u32, u32), content: f32) -> (f32, f64) {
    let ratio = if size.0 > 0 && pixels.0 > 0 { pixels.0 as f32 / size.0 as f32 } else { 1.0 };
    let mut scale = if content > 0.0 { content.max(ratio) } else { ratio };
    if scale.is_nan() || scale <= 0.0 {
        scale = 1.0;
    }
    (scale, scale as f64 / ratio as f64)
}

/// One size of a window's icon: its width and height and its pixels, RGBA, not premultiplied.
#[derive(Debug, Clone, PartialEq)]
pub struct IconImage {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// Clarity's own icon, which a window shows when the application carries none.
pub const CLARITY_ICON: &[u8] = include_bytes!("icon.png");

/// The sizes systems pick a window's icon from.
pub const ICON_SIZES: [u32; 5] = [16, 32, 48, 64, 128];

/// The window's icon: the one the application carries (`icon.png` among its
/// resources), else Clarity's; at the sizes systems show, none larger than
/// the picture itself (16 at least). Empty when neither can be read.
pub fn icon(app: Option<&[u8]>) -> Vec<IconImage> {
    let image = app.and_then(|bytes| Image::from_encoded(Data::new_copy(bytes))).or_else(|| Image::from_encoded(Data::new_copy(CLARITY_ICON)));
    let Some(image) = image else { return Vec::new() };
    let largest = image.width().max(16) as u32;
    ICON_SIZES.iter().filter(|s| **s <= largest).filter_map(|s| sized(&image, *s)).collect()
}

/// The picture drawn at `size` by `size`, smoothly.
fn sized(image: &Image, size: u32) -> Option<IconImage> {
    let side = size as i32;
    let mut surface = surfaces::raster(&ImageInfo::new((side, side), skia_safe::ColorType::RGBA8888, AlphaType::Premul, None), None, None)?;
    let canvas = surface.canvas();
    canvas.clear(skia_safe::Color::TRANSPARENT);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    canvas.draw_image_rect_with_sampling_options(image, None, Rect::from_wh(size as f32, size as f32), SamplingOptions::from(CubicResampler::mitchell()), &paint);
    let info = ImageInfo::new((side, side), skia_safe::ColorType::RGBA8888, AlphaType::Unpremul, None);
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    surface.read_pixels(&info, &mut rgba, size as usize * 4, (0, 0)).then_some(IconImage { size, rgba })
}

/// The icon a system window shows: the largest size, as the system scales it down for its bars.
pub fn window_icon(app: Option<&[u8]>) -> Option<winit::window::Icon> {
    let largest = icon(app).into_iter().last()?;
    winit::window::Icon::from_rgba(largest.rgba, largest.size, largest.size).ok()
}

/// The system's window, as its file dialogs name an owner; none for a window the system names no handle of.
pub fn handle(window: &winit::window::Window) -> Option<super::files::SystemWindow> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let raw = window.window_handle().ok()?.as_raw();
    match raw {
        RawWindowHandle::Xlib(x) => Some(super::files::SystemWindow { x11: x.window as u64, win32: 0 }),
        RawWindowHandle::Xcb(x) => Some(super::files::SystemWindow { x11: x.window.get() as u64, win32: 0 }),
        RawWindowHandle::Win32(w) => Some(super::files::SystemWindow { x11: 0, win32: w.hwnd.get() }),
        _ => None,
    }
}

// OpenGL's numbers for what glDrawPixels is given.
const RGBA: u32 = 0x1908;
const UNSIGNED_BYTE: u32 = 0x1401;
const COLOR_BUFFER_BIT: u32 = 0x4000;

type Viewport = unsafe extern "system" fn(i32, i32, i32, i32);
type RasterPos = unsafe extern "system" fn(f32, f32);
type PixelZoom = unsafe extern "system" fn(f32, f32);
type DrawPixels = unsafe extern "system" fn(i32, i32, u32, u32, *const c_void);
type Clear = unsafe extern "system" fn(u32);
type Flush = unsafe extern "system" fn();

/// Skia's pixels drawn in memory and put on the window with OpenGL's
/// glDrawPixels: the window's picture where Skia cannot draw through the
/// window's OpenGL (a driver below OpenGL 2.0, Windows over Remote Desktop).
pub struct SoftwareSurface {
    viewport: Viewport,
    raster_pos: RasterPos,
    pixel_zoom: PixelZoom,
    draw_pixels: DrawPixels,
    clear: Option<Clear>,
    flush: Option<Flush>,
}

impl SoftwareSurface {
    /// The surface over this OpenGL's functions, which `function` finds by
    /// name in the window's current context; none when it has not what it takes.
    pub fn new(function: impl Fn(&CStr) -> *const c_void) -> Option<SoftwareSurface> {
        let find = |name: &CStr| {
            let at = function(name);
            (!at.is_null()).then_some(at)
        };
        // Each found function has the type OpenGL gives it.
        unsafe {
            Some(SoftwareSurface {
                viewport: std::mem::transmute::<*const c_void, Viewport>(find(c"glViewport")?),
                raster_pos: std::mem::transmute::<*const c_void, RasterPos>(find(c"glRasterPos2f")?),
                pixel_zoom: std::mem::transmute::<*const c_void, PixelZoom>(find(c"glPixelZoom")?),
                draw_pixels: std::mem::transmute::<*const c_void, DrawPixels>(find(c"glDrawPixels")?),
                clear: find(c"glClear").map(|at| std::mem::transmute::<*const c_void, Clear>(at)),
                flush: find(c"glFlush").map(|at| std::mem::transmute::<*const c_void, Flush>(at)),
            })
        }
    }

    /// The surface Skia draws on in memory, as large as the framebuffer, in OpenGL's byte order.
    pub fn fit(width: u32, height: u32) -> Option<Surface> {
        let info = ImageInfo::new((width.max(1) as i32, height.max(1) as i32), skia_safe::ColorType::RGBA8888, AlphaType::Premul, None);
        surfaces::raster(&info, None, None)
    }

    /// Puts what Skia drew on the window: from the top left, its rows downward.
    pub fn present(&self, surface: &mut Surface) {
        let Some(pixels) = surface.peek_pixels() else { return };
        let (width, height) = (pixels.width(), pixels.height());
        // Skia's rows are packed (width * 4 bytes), as glDrawPixels reads them by default.
        if pixels.row_bytes() != width as usize * 4 {
            return;
        }
        unsafe {
            (self.viewport)(0, 0, width, height);
            if let Some(clear) = self.clear {
                clear(COLOR_BUFFER_BIT);
            }
            // OpenGL's rows go up from the bottom: start at the top left and draw downward.
            (self.raster_pos)(-1.0, 1.0);
            (self.pixel_zoom)(1.0, -1.0);
            (self.draw_pixels)(width, height, RGBA, UNSIGNED_BYTE, pixels.addr());
            if let Some(flush) = self.flush {
                flush();
            }
        }
    }
}

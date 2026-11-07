//! The browser host in the page's worker (wasm32-unknown-emscripten): the
//! host on the WebGL2 context the worker made current, the program asked
//! through the worker's synchronous requests, the page told through the
//! worker's messages. The functions the worker script provides are imported
//! here (runtimes/rust-ui-browser/js/library.js); the entry points it calls
//! are the browser crate's (runtimes/rust-ui-browser/src/main.rs), which
//! call these.
//!
//! Whatever fails in what the person did or in drawing is said in the
//! window as an error message, and the page goes on, as on the desktop.

use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Level, Text};
use serde_json::{Map, Value};
use skia_safe::gpu::{self, DirectContext, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo};
use skia_safe::{ColorType, EncodedImageFormat, Surface};

use super::host::{BrowserHost, PointerKind};
use super::page::Tell;
use crate::program_wire::{Asks, WireProgram};
use crate::window::WindowStack;

unsafe extern "C" {
    /// Posts `{"command", "args"}` to the server and waits: its answer as text the caller frees, NULL for none.
    fn clarity_browser_ask(command: *const u8, command_len: usize, args: *const u8, args_len: usize) -> *mut c_char;
    /// Posts `{"command", "args"}` to the server without waiting; the worker
    /// gives its answer to `clarity_ui_answered` with `id` when it comes.
    fn clarity_browser_ask_later(id: u32, command: *const u8, command_len: usize, args: *const u8, args_len: usize);
    /// Sends the page a message (JSON).
    fn clarity_browser_tell(json: *const u8, json_len: usize);
    /// The bytes of a resource of the program by its name, fetched from the server; NULL when it has none.
    fn clarity_browser_resource(name: *const u8, name_len: usize, len: *mut usize) -> *mut u8;
    fn free(pointer: *mut c_void);
}

/// The program on the server, asked through the worker.
struct ServerAsks;

impl Asks for ServerAsks {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        let args = Value::Object(args).to_string();
        let answer = unsafe { clarity_browser_ask(command.as_ptr(), command.len(), args.as_ptr(), args.len()) };
        if answer.is_null() {
            return None;
        }
        let text = unsafe { CStr::from_ptr(answer) }.to_string_lossy().into_owned();
        unsafe { free(answer as *mut c_void) };
        serde_json::from_str(&text).ok()
    }
}

/// What waits for the answer of a request sent without waiting, by its number.
type Waiting = Box<dyn FnOnce(Option<Value>)>;

thread_local! {
    static WAITING: RefCell<(u32, std::collections::HashMap<u32, Waiting>)> = RefCell::new((0, std::collections::HashMap::new()));
}

/// Sends a long call's request without waiting; `answered` gets its answer.
fn ask_later(command: &str, args: Map<String, Value>, answered: Waiting) {
    let id = WAITING.with(|w| {
        let mut w = w.borrow_mut();
        w.0 = w.0.wrapping_add(1).max(1);
        let id = w.0;
        w.1.insert(id, answered);
        id
    });
    let args = Value::Object(args).to_string();
    unsafe { clarity_browser_ask_later(id, command.as_ptr(), command.len(), args.as_ptr(), args.len()) };
}

/// The answer to request `id` sent without waiting came (its text; none when there was none).
pub fn answered(id: u32, answer: Option<&str>) {
    let Some(waiting) = WAITING.with(|w| w.borrow_mut().1.remove(&id)) else { return };
    // Taken by the next frame, which the worker asks for.
    waiting(answer.and_then(|text| serde_json::from_str(text).ok()));
}

fn tell(message: Value) {
    let text = message.to_string();
    unsafe { clarity_browser_tell(text.as_ptr(), text.len()) };
}

fn resource(name: &str) -> Option<Vec<u8>> {
    let mut len = 0usize;
    let bytes = unsafe { clarity_browser_resource(name.as_ptr(), name.len(), &mut len) };
    if bytes.is_null() {
        return None;
    }
    let copy = unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec();
    unsafe { free(bytes as *mut c_void) };
    Some(copy)
}

/// The host and what it draws with.
struct Web {
    host: BrowserHost,
    context: DirectContext,
    surface: Option<Surface>,
    /// The canvas in device pixels.
    pixels: (i32, i32),
}

thread_local! {
    static WEB: RefCell<Option<Web>> = const { RefCell::new(None) };
}

fn surface(context: &mut DirectContext, (width, height): (i32, i32)) -> Option<Surface> {
    let info = FramebufferInfo { fboid: 0, format: gpu::gl::Format::RGBA8.into(), ..Default::default() };
    let target = backend_render_targets::make_gl((width.max(1), height.max(1)), 0, 8, info);
    gpu::surfaces::wrap_backend_render_target(context, &target, SurfaceOrigin::BottomLeft, ColorType::RGBA8888, None, None)
}

fn device_pixels(width: f64, height: f64, scale: f32) -> (i32, i32) {
    ((width * scale as f64).round().max(1.0) as i32, (height * scale as f64).round().max(1.0) as i32)
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_else(|| "Clarity.UI stopped on a fault of its own".to_string())
}

/// Runs `act` on the host; what fails in it is said in the window (and to the page), and the page goes on.
fn with<R: Default>(act: impl FnOnce(&mut Web) -> R) -> R {
    WEB.with(|web| {
        let mut web = web.borrow_mut();
        let Some(web) = web.as_mut() else { return R::default() };
        match catch_unwind(AssertUnwindSafe(|| act(web))) {
            Ok(done) => done,
            Err(payload) => {
                let what = panic_text(payload.as_ref());
                tell(serde_json::json!({ "tell": "failed", "what": what }));
                let _ = catch_unwind(AssertUnwindSafe(|| web.host.screen().act_root::<WindowStack, _>(|stack, cx| stack.say(Level::Error, Text::plain(what), cx))));
                R::default()
            }
        }
    })
}

/// Starts the program: its files as `files.json` gives them, on a canvas of
/// `width` by `height` CSS pixels at `scale`, in the WebGL2 context the
/// worker made current. Why it could not start, when it could not.
/// `choices` are the person's choices the page kept (JSON, an object of texts).
pub fn start(files: &str, choices: &str, width: f64, height: f64, scale: f32, dark: bool) -> Result<(), String> {
    let files: Value = serde_json::from_str(files).map_err(|e| format!("files.json does not read: {e}"))?;
    let kept = kept_of(choices);
    let (files, owned) = super::owned_files_from_json(&files, &resource);
    let interface = gpu::gl::Interface::new_native().ok_or("WebGL2 gives Skia no interface")?;
    let mut context = gpu::direct_contexts::make_gl(interface, None).ok_or("Skia cannot draw with this WebGL2 context")?;
    let pixels = device_pixels(width, height, scale);
    let surface = surface(&mut context, pixels);
    let program: Arc<dyn Program> = Arc::new(WireProgram::new(ServerAsks));
    let teller: Tell = Arc::new(tell);
    let mut host = catch_unwind(AssertUnwindSafe(|| BrowserHost::start_kept(program, files, width, height, scale, dark, teller, kept))).map_err(|p| panic_text(p.as_ref()))?;
    host.set_asker(std::rc::Rc::new(ask_later));
    host.keep_files(owned);
    WEB.with(|web| *web.borrow_mut() = Some(Web { host, context, surface, pixels }));
    Ok(())
}

/// Starts the program again from new files (`files.json`'s text) on the
/// same canvas: the IDE's designer and preview after a change of the design.
/// Why it could not, when it could not.
/// `choices` are the person's choices to start with (JSON, an object of texts): the language the IDE shows.
pub fn reload(files: &str, choices: &str) -> Result<(), String> {
    let files: Value = serde_json::from_str(files).map_err(|e| format!("files.json does not read: {e}"))?;
    if WEB.with(|web| web.borrow().is_none()) {
        return Err("the program has not started".to_string());
    }
    let (files, owned) = super::owned_files_from_json(&files, &resource);
    let kept = kept_of(choices);
    with(|web| {
        web.host.choose(kept);
        web.host.reload_owned(files, Some(owned));
    });
    Ok(())
}

fn kept_of(choices: &str) -> std::collections::BTreeMap<String, String> {
    serde_json::from_str::<Map<String, Value>>(choices).unwrap_or_default().into_iter().filter_map(|(key, value)| value.as_str().map(|text| (key, text.to_string()))).collect()
}

/// Where the design's parts and grids show, as JSON (the driven `layout` step).
/// Drawn first, so the places are those of the screen as it is now.
pub fn layout() -> String {
    with(|web| {
        if let Some(surface) = web.surface.as_mut() {
            web.host.draw(surface.canvas());
            let _ = web.context.flush_and_submit();
        }
        web.host.layout().to_string()
    })
}

/// The canvas is `width` by `height` CSS pixels at `scale` now; the worker sized the OffscreenCanvas to match.
pub fn resize(width: f64, height: f64, scale: f32) {
    with(|web| {
        let pixels = device_pixels(width, height, scale);
        if pixels != web.pixels || web.surface.is_none() {
            web.surface = surface(&mut web.context, pixels);
            web.pixels = pixels;
        }
        web.host.resize(width, height, scale);
    })
}

pub fn set_dark(dark: bool) {
    with(|web| web.host.set_dark(dark))
}

pub fn pointer(kind: i32, x: f64, y: f64, button: i32, held: u32) {
    let Some(kind) = PointerKind::of(kind) else { return };
    with(|web| web.host.pointer(kind, x, y, button, held))
}

pub fn wheel(x: f64, y: f64, dx: f64, dy: f64, mode: u32, held: u32) {
    with(|web| web.host.wheel(x, y, dx, dy, mode, held))
}

pub fn key(down: bool, key: &str, code: &str, held: u32) -> bool {
    with(|web| web.host.key(down, key, code, held))
}

pub fn text(text: &str) {
    with(|web| web.host.text(text))
}

pub fn compose(text: Option<&str>, marked: Option<(usize, usize)>) {
    with(|web| web.host.compose(text, marked))
}

pub fn pasted(text: &str) {
    with(|web| web.host.pasted(text))
}

pub fn song_said(id: u64, said: super::page::SongSaid) {
    with(|web| web.host.song_said(id, said))
}

pub fn sound_ended(id: u64) {
    with(|web| web.host.sound_ended(id))
}

pub fn file_chosen(name: &str, bytes: Vec<u8>) {
    with(|web| web.host.file_chosen(name, bytes))
}

/// The page keeps an accessible mirror of the screen (A1 phase 2) or not.
pub fn access(on: bool) {
    with(|web| web.host.set_access(on))
}

/// A screen reader's action on the mirror's node (its id as text): whether it was done.
pub fn act(node: &str, action: &str, value: Option<&str>) -> bool {
    with(|web| web.host.access_act(node, action, value))
}

/// A frame: time goes on and the screen is drawn when it changed. Whether it drew.
pub fn frame() -> bool {
    with(|web| {
        if !web.host.frame() {
            return false;
        }
        let Some(surface) = web.surface.as_mut() else { return false };
        web.host.draw(surface.canvas());
        let _ = web.context.flush_and_submit();
        true
    })
}

/// Milliseconds until the screen wants a frame by itself; -1 while it waits for the person.
pub fn next_wake() -> f64 {
    with(|web| web.host.next_wake().map(|d| d.as_secs_f64() * 1000.0).unwrap_or(-1.0))
}

/// A driven step (DESIGN 1.6) on the page's screen, answered as JSON.
pub fn step(step: &str) -> String {
    let step: Value = match serde_json::from_str(step) {
        Ok(step) => step,
        Err(why) => return serde_json::json!({ "ok": false, "error": why.to_string() }).to_string(),
    };
    WEB.with(|web| web.borrow().is_some()).then(|| with(|web| web.host.step(&step).to_string())).unwrap_or_else(|| serde_json::json!({ "ok": false, "error": "the program has not started" }).to_string())
}

/// What the canvas shows, drawn again now and read back from WebGL as a PNG.
pub fn shot() -> Option<Vec<u8>> {
    with(|web| {
        let surface = web.surface.as_mut()?;
        web.host.draw(surface.canvas());
        let _ = web.context.flush_and_submit();
        // Read straight from WebGL's drawing buffer, in this same task, before the page shows it.
        let info = skia_safe::ImageInfo::new_n32_premul((surface.width(), surface.height()), None);
        let row = info.min_row_bytes();
        let mut pixels = vec![0u8; row * surface.height() as usize];
        if !surface.read_pixels(&info, &mut pixels, row, (0, 0)) {
            return None;
        }
        let image = skia_safe::images::raster_from_data(&info, skia_safe::Data::new_copy(&pixels), row)?;
        image.encode(None, EncodedImageFormat::PNG, None).map(|data| data.as_bytes().to_vec())
    })
}

//! The desktop through Clarity.UI's ready-made library (DESIGN 7.9): its C
//! interface (`clarity_ui.h`), as Go, C and C++ programs use it, so a Rust
//! program need not build Clarity.UI and Skia itself. The program answers the
//! library's commands through `wire::answer`, the same answers the web wire
//! gives. The generated program's build script links the library from the
//! folder CLARITY_UI names. Custom parts written in Rust register here
//! (`register_part`), as a table of callbacks, as Go, C and C++ parts do.

use std::ffi::{c_char, c_void};
use std::process::ExitCode;
use std::sync::Arc;

use crate::program::Program;

/// The windows, app frames, themes, model and resources the library draws
/// from, and the main window's title and size: as `clarity_ui::AppFiles`.
pub struct AppFiles {
    /// Each window's markup, by the window's name.
    pub windows: &'static [(&'static str, &'static str)],
    /// Each app's frame, by the app's name.
    pub apps: &'static [(&'static str, &'static str)],
    /// The resolved themes, by their name and mode (`shop.light`).
    pub themes: &'static [(&'static str, &'static str)],
    /// The model the program is made with, as JSON.
    pub model: &'static str,
    /// The files of the resources folder by their names.
    pub resources: &'static [(&'static str, &'static [u8])],
    /// The title of the desktop window.
    pub title: &'static str,
    /// The size the main window opens at; 0 for the host's own.
    pub width: f64,
    pub height: f64,
}

type Status = i32;
const OK: Status = 0;

#[repr(C)]
struct Files {
    _private: [u8; 0],
}

#[repr(C)]
struct NativeProgram {
    _private: [u8; 0],
}

#[repr(C)]
struct Answer {
    _private: [u8; 0],
}

#[repr(C)]
struct Callbacks {
    user: *mut c_void,
    call: Option<unsafe extern "C" fn(user: *mut c_void, command: *const c_char, command_len: usize, args: *const c_char, args_len: usize, answer: *mut Answer)>,
    release: Option<unsafe extern "C" fn(user: *mut c_void)>,
}

// Linked by the program's build script, from the folder CLARITY_UI names (clarity_ui.dll.lib on Windows).
unsafe extern "C" {
    fn clarity_last_error() -> *mut c_char;
    fn clarity_string_free(text: *mut c_char);
    fn clarity_answer_set(answer: *mut Answer, data: *const u8, len: usize) -> Status;
    fn clarity_files_new(out: *mut *mut Files) -> Status;
    fn clarity_files_free(files: *mut Files);
    fn clarity_files_window(files: *mut Files, name: *const c_char, name_len: usize, markup: *const c_char, markup_len: usize) -> Status;
    fn clarity_files_app(files: *mut Files, name: *const c_char, name_len: usize, markup: *const c_char, markup_len: usize) -> Status;
    fn clarity_files_theme(files: *mut Files, name: *const c_char, name_len: usize, json: *const c_char, json_len: usize) -> Status;
    fn clarity_files_model(files: *mut Files, json: *const c_char, json_len: usize) -> Status;
    fn clarity_files_resource(files: *mut Files, name: *const c_char, name_len: usize, bytes: *const u8, bytes_len: usize) -> Status;
    fn clarity_files_title(files: *mut Files, title: *const c_char, title_len: usize) -> Status;
    fn clarity_files_size(files: *mut Files, width: f64, height: f64) -> Status;
    fn clarity_program_new(callbacks: *const Callbacks, out: *mut *mut NativeProgram) -> Status;
    fn clarity_program_free(program: *mut NativeProgram);
    fn clarity_ui_use_audio(api: *const c_void) -> Status;
    fn clarity_ui_use_video(api: *const c_void) -> Status;
    fn clarity_desktop_run(program: *const NativeProgram, files: *const Files, exit_code: *mut i32) -> Status;
    fn clarity_desktop_wake();
    fn clarity_translate(key: *const c_char, key_len: usize, english: *const c_char, english_len: usize, out: *mut *mut c_char) -> Status;
    fn clarity_agent_ask(question: *const c_char, question_len: usize, form: *const c_char, form_len: usize, out: *mut *mut c_char) -> Status;
    fn clarity_custom_register(kind: *const c_char, kind_len: usize, callbacks: *const CustomCallbacks) -> Status;
    fn clarity_custom_bounds(cx: *const RawCx, x: *mut f64, y: *mut f64, width: *mut f64, height: *mut f64) -> Status;
    fn clarity_custom_enabled(cx: *const RawCx, enabled: *mut i32) -> Status;
    fn clarity_custom_invalidate(cx: *mut RawCx) -> Status;
    fn clarity_custom_wake_in(cx: *mut RawCx, ms: f64) -> Status;
    fn clarity_custom_animate(cx: *mut RawCx, on: i32) -> Status;
    fn clarity_custom_focus(cx: *mut RawCx) -> Status;
    fn clarity_custom_change(cx: *mut RawCx, value: *const c_char, value_len: usize) -> Status;
    fn clarity_custom_tell(cx: *mut RawCx, tell: *const c_char, tell_len: usize, values: *const c_char, values_len: usize) -> Status;
    fn clarity_canvas_fill(canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64, color: u32, radius: f64) -> Status;
    fn clarity_canvas_stroke(canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64, color: u32, line_width: f64, radius: f64) -> Status;
    fn clarity_canvas_line(canvas: *mut RawCanvas, x1: f64, y1: f64, x2: f64, y2: f64, color: u32, line_width: f64) -> Status;
    fn clarity_canvas_oval(canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64, color: u32, filled: i32, line_width: f64) -> Status;
    fn clarity_canvas_path(canvas: *mut RawCanvas, svg: *const c_char, svg_len: usize, color: u32, filled: i32, line_width: f64) -> Status;
    fn clarity_canvas_text(canvas: *mut RawCanvas, text: *const c_char, text_len: usize, x: f64, y: f64, width: f64, height: f64, color: u32, size: f64, bold: i32, align: i32) -> Status;
    fn clarity_canvas_measure(canvas: *mut RawCanvas, text: *const c_char, text_len: usize, size: f64, bold: i32, width: *mut f64) -> Status;
    fn clarity_canvas_pixels(canvas: *mut RawCanvas, rgba: *const u8, width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64) -> Status;
    fn clarity_canvas_clip(canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64) -> Status;
    fn clarity_canvas_restore(canvas: *mut RawCanvas) -> Status;
    fn clarity_canvas_look_color(canvas: *mut RawCanvas, role: *const c_char, role_len: usize, color: *mut u32) -> Status;
}

/// Why the last call of the library on this thread failed.
fn last_error(call: &str) -> String {
    let text = unsafe { clarity_last_error() };
    if text.is_null() {
        return format!("{call} failed");
    }
    let why = unsafe { std::ffi::CStr::from_ptr(text) }.to_string_lossy().into_owned();
    unsafe { clarity_string_free(text) };
    format!("{call}: {why}")
}

fn checked(status: Status, call: &str) -> Result<(), String> {
    if status == OK { Ok(()) } else { Err(last_error(call)) }
}

/// The program as the callbacks hold it.
type Held = Arc<dyn Program>;

unsafe extern "C" fn call(user: *mut c_void, command: *const c_char, command_len: usize, args: *const c_char, args_len: usize, answer: *mut Answer) {
    // A fault of the program never crosses into the library: wire::answer turns a panic into an error answer.
    let program = unsafe { &*(user as *const Held) };
    let text = |p: *const c_char, n: usize| -> String {
        if p.is_null() || n == 0 {
            return String::new();
        }
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(p as *const u8, n) }).into_owned()
    };
    let command = text(command, command_len);
    let args = serde_json::from_str(&text(args, args_len)).unwrap_or(serde_json::Value::Object(Default::default()));
    let reply = crate::wire::answer(program.as_ref(), &command, args).to_string();
    unsafe { clarity_answer_set(answer, reply.as_ptr(), reply.len()) };
}

unsafe extern "C" fn release(user: *mut c_void) {
    drop(unsafe { Box::from_raw(user as *mut Held) });
}

fn files_of(app: &AppFiles) -> Result<*mut Files, String> {
    let mut files: *mut Files = std::ptr::null_mut();
    checked(unsafe { clarity_files_new(&mut files) }, "clarity_files_new")?;
    let filled = (|| {
        for (name, markup) in app.windows {
            checked(unsafe { clarity_files_window(files, name.as_ptr() as _, name.len(), markup.as_ptr() as _, markup.len()) }, "clarity_files_window")?;
        }
        for (name, markup) in app.apps {
            checked(unsafe { clarity_files_app(files, name.as_ptr() as _, name.len(), markup.as_ptr() as _, markup.len()) }, "clarity_files_app")?;
        }
        for (name, json) in app.themes {
            checked(unsafe { clarity_files_theme(files, name.as_ptr() as _, name.len(), json.as_ptr() as _, json.len()) }, "clarity_files_theme")?;
        }
        checked(unsafe { clarity_files_model(files, app.model.as_ptr() as _, app.model.len()) }, "clarity_files_model")?;
        for (name, bytes) in app.resources {
            checked(unsafe { clarity_files_resource(files, name.as_ptr() as _, name.len(), bytes.as_ptr(), bytes.len()) }, "clarity_files_resource")?;
        }
        checked(unsafe { clarity_files_title(files, app.title.as_ptr() as _, app.title.len()) }, "clarity_files_title")?;
        checked(unsafe { clarity_files_size(files, app.width, app.height) }, "clarity_files_size")
    })();
    match filled {
        Ok(()) => Ok(files),
        Err(why) => {
            unsafe { clarity_files_free(files) };
            Err(why)
        }
    }
}

/// Songs of the sound parts play through Clarity.Audio: `api` is its
/// `clarity_audio_api()`, which the program links when it plays songs.
pub fn use_audio(api: *const c_void) {
    if unsafe { clarity_ui_use_audio(api) } != OK {
        eprintln!("{}", last_error("clarity_ui_use_audio"));
    }
}

/// Video parts play through Clarity.Video: `api` is its `clarity_video_api()`.
pub fn use_video(api: *const c_void) {
    if unsafe { clarity_ui_use_video(api) } != OK {
        eprintln!("{}", last_error("clarity_ui_use_video"));
    }
}

/// The head of Clarity.Video's table (`clarity_video.h`, ClarityVideoApi) up
/// to `add_codec`, its fields in their order.
#[repr(C)]
struct VideoApi {
    interface: u32,
    open: *const c_void,
    free: *const c_void,
    size: *const c_void,
    length_ms: *const c_void,
    frame_ms: *const c_void,
    picture_at: *const c_void,
    ended_at: *const c_void,
    problem: *const c_void,
    add_codec: Option<unsafe extern "C" fn(file: *const c_char, file_len: usize) -> i32>,
}

/// Adds the codec library `file` the program carries (its `codecs` folder,
/// docs/VIDEO-PLAN.md) to Clarity.Video: `api` is its `clarity_video_api()`.
pub fn add_codec(api: *const c_void, file: &str) -> Result<(), String> {
    if api.is_null() {
        return Err(format!("codec {file}: Clarity.Video is not there"));
    }
    let table = unsafe { &*(api as *const VideoApi) };
    let Some(add) = table.add_codec.filter(|_| table.interface >= 2) else {
        return Err(format!("codec {file}: this Clarity.Video adds no codec libraries"));
    };
    match unsafe { add(file.as_ptr() as *const c_char, file.len()) } {
        0 => Ok(()),
        code => Err(format!("codec {file} was not added (Clarity.Video says {code})")),
    }
}

/// Has the running desktop read every window's values again and draw, from any thread.
pub fn wake() {
    unsafe { clarity_desktop_wake() }
}

/// Shows the program's windows in desktop windows until they closed, on the
/// calling (main) thread; with CLARITY_HEADLESS set the library drives it
/// from standard input instead (DESIGN 1.6), as `clarity_ui::desktop::run`.
/// Asks the agent whose command runs now, through Clarity.UI (docs/ledger/app-mcp.md).
fn agent_ask(question: &str, form: &str) -> Option<String> {
    let mut out: *mut c_char = std::ptr::null_mut();
    // SAFETY: both texts are (pointer, length); `out` is ours to read and free.
    let status = unsafe { clarity_agent_ask(question.as_ptr().cast(), question.len(), form.as_ptr().cast(), form.len(), &mut out) };
    if status != OK || out.is_null() {
        return None;
    }
    let answer = unsafe { std::ffi::CStr::from_ptr(out) }.to_string_lossy().into_owned();
    unsafe { clarity_string_free(out) };
    Some(answer)
}

pub fn run(program: impl Program, app: AppFiles) -> ExitCode {
    let _ = crate::wire::AGENT_ASK.set(agent_ask);
    match run_native(Arc::new(program), &app) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code.clamp(1, 255) as u8),
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

fn run_native(program: Held, app: &AppFiles) -> Result<i32, String> {
    let files = files_of(app)?;
    let user = Box::into_raw(Box::new(program)) as *mut c_void;
    let callbacks = Callbacks { user, call: Some(call), release: Some(release) };
    let mut native: *mut NativeProgram = std::ptr::null_mut();
    if let Err(why) = checked(unsafe { clarity_program_new(&callbacks, &mut native) }, "clarity_program_new") {
        // The library holds nothing: the program goes here.
        unsafe { release(user) };
        unsafe { clarity_files_free(files) };
        return Err(why);
    }
    let mut code = 0i32;
    let ran = checked(unsafe { clarity_desktop_run(native, files, &mut code) }, "clarity_desktop_run");
    unsafe { clarity_program_free(native) };
    unsafe { clarity_files_free(files) };
    ran.map(|()| code)
}

// -- Custom parts (DESIGN 5.19, 7.9) ----------------------------------------
//
// A custom part written in Rust is a table of callbacks to the library, as a
// Go, C or C++ part is: the program's glue wraps the part's crate in a
// `CustomPart` and registers it by its kind's name. The part draws on the
// `Canvas` the library gives and acts through its `Cx`.

#[repr(C)]
struct RawCx {
    _private: [u8; 0],
}

#[repr(C)]
struct RawCanvas {
    _private: [u8; 0],
}

#[repr(C)]
struct RawPointer {
    x: f64,
    y: f64,
    modifiers: u32,
    clicks: u32,
}

type C = *const c_char;

#[repr(C)]
struct CustomCallbacks {
    user: *mut c_void,
    make: Option<unsafe extern "C" fn(user: *mut c_void) -> *mut c_void>,
    set: Option<unsafe extern "C" fn(part: *mut c_void, name: C, name_len: usize, value: C, value_len: usize, cx: *mut RawCx)>,
    hear: Option<unsafe extern "C" fn(part: *mut c_void, hear: C, hear_len: usize, values: C, values_len: usize, cx: *mut RawCx)>,
    draw: Option<unsafe extern "C" fn(part: *mut c_void, canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64)>,
    start: Option<unsafe extern "C" fn(part: *mut c_void, cx: *mut RawCx)>,
    focusable: Option<unsafe extern "C" fn(part: *mut c_void) -> i32>,
    pointer_pressed: Option<unsafe extern "C" fn(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx)>,
    pointer_moved: Option<unsafe extern "C" fn(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx)>,
    pointer_released: Option<unsafe extern "C" fn(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx)>,
    click: Option<unsafe extern "C" fn(part: *mut c_void, cx: *mut RawCx)>,
    wheel: Option<unsafe extern "C" fn(part: *mut c_void, dx: f64, dy: f64, modifiers: u32, cx: *mut RawCx) -> i32>,
    key: Option<unsafe extern "C" fn(part: *mut c_void, name: C, name_len: usize, modifiers: u32, cx: *mut RawCx) -> i32>,
    text: Option<unsafe extern "C" fn(part: *mut c_void, text: C, text_len: usize, cx: *mut RawCx)>,
    focus_changed: Option<unsafe extern "C" fn(part: *mut c_void, focused: i32, cx: *mut RawCx)>,
    wake: Option<unsafe extern "C" fn(part: *mut c_void, cx: *mut RawCx)>,
    stop: Option<unsafe extern "C" fn(part: *mut c_void)>,
    tooltip_at: Option<unsafe extern "C" fn(part: *mut c_void, x: f64, y: f64, tip: *mut Answer)>,
    destroy: Option<unsafe extern "C" fn(part: *mut c_void)>,
}

/// Keys held, as bits of `modifiers`.
pub const SHIFT: u32 = 1;
pub const CONTROL: u32 = 2;
pub const ALT: u32 = 4;
pub const META: u32 = 8;

/// Where the pointer is, in the screen's units, with the keys held and the clicks counted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
    pub modifiers: u32,
    pub clicks: u32,
}

/// What a part draws on, in the screen's units, valid during its `draw`. Colors are 0xAARRGGBB.
pub struct Canvas {
    raw: *mut RawCanvas,
}

impl Canvas {
    pub fn fill(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, radius: f64) {
        unsafe { clarity_canvas_fill(self.raw, x, y, width, height, color, radius) };
    }

    pub fn stroke(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, line_width: f64, radius: f64) {
        unsafe { clarity_canvas_stroke(self.raw, x, y, width, height, color, line_width, radius) };
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: u32, line_width: f64) {
        unsafe { clarity_canvas_line(self.raw, x1, y1, x2, y2, color, line_width) };
    }

    pub fn oval(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, filled: bool, line_width: f64) {
        unsafe { clarity_canvas_oval(self.raw, x, y, width, height, color, filled as i32, line_width) };
    }

    /// An SVG path's data ("M0 0 L10 10 Z"), filled or stroked.
    pub fn path(&mut self, svg: &str, color: u32, filled: bool, line_width: f64) {
        unsafe { clarity_canvas_path(self.raw, svg.as_ptr() as C, svg.len(), color, filled as i32, line_width) };
    }

    /// Text in a box: align 0 left, 1 center, 2 right; size 0 is the look's text size.
    pub fn text(&mut self, text: &str, x: f64, y: f64, width: f64, height: f64, color: u32, size: f64, bold: bool, align: i32) {
        unsafe { clarity_canvas_text(self.raw, text.as_ptr() as C, text.len(), x, y, width, height, color, size, bold as i32, align) };
    }

    /// How wide text is at a size (0: the look's text size).
    pub fn measure(&mut self, text: &str, size: f64, bold: bool) -> f64 {
        let mut width = 0.0;
        unsafe { clarity_canvas_measure(self.raw, text.as_ptr() as C, text.len(), size, bold as i32, &mut width) };
        width
    }

    /// RGBA pixels (unpremultiplied, rows of `stride` bytes) drawn into a box.
    pub fn pixels(&mut self, rgba: &[u8], width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64) {
        // The library reads `height` rows of `stride` bytes: never more than the slice holds.
        if (height as usize).saturating_mul(stride) > rgba.len() || (width as usize).saturating_mul(4) > stride {
            return;
        }
        unsafe { clarity_canvas_pixels(self.raw, rgba.as_ptr(), width, height, stride, x, y, box_width, box_height) };
    }

    /// Clips what follows to a box until `restore`.
    pub fn clip(&mut self, x: f64, y: f64, width: f64, height: f64) {
        unsafe { clarity_canvas_clip(self.raw, x, y, width, height) };
    }

    pub fn restore(&mut self) {
        unsafe { clarity_canvas_restore(self.raw) };
    }

    /// A color of the look by its role ("accent", "text", "border").
    pub fn look_color(&mut self, role: &str) -> u32 {
        let mut color = 0;
        unsafe { clarity_canvas_look_color(self.raw, role.as_ptr() as C, role.len(), &mut color) };
        color
    }
}

/// What a part does through Clarity.UI, valid during the call it is given to.
pub struct Cx {
    raw: *mut RawCx,
}

impl Cx {
    /// Its place in the window: x, y, width and height.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let (mut x, mut y, mut width, mut height) = (0.0, 0.0, 0.0, 0.0);
        unsafe { clarity_custom_bounds(self.raw, &mut x, &mut y, &mut width, &mut height) };
        (x, y, width, height)
    }

    pub fn enabled(&self) -> bool {
        let mut enabled = 1;
        unsafe { clarity_custom_enabled(self.raw, &mut enabled) };
        enabled != 0
    }

    /// Has it drawn again.
    pub fn invalidate(&mut self) {
        unsafe { clarity_custom_invalidate(self.raw) };
    }

    /// Has its `wake` called once after `ms` milliseconds.
    pub fn wake_in(&mut self, ms: f64) {
        unsafe { clarity_custom_wake_in(self.raw, ms) };
    }

    /// Has it drawn every frame while on.
    pub fn animate(&mut self, on: bool) {
        unsafe { clarity_custom_animate(self.raw, on as i32) };
    }

    pub fn focus(&mut self) {
        unsafe { clarity_custom_focus(self.raw) };
    }

    /// The person changed the value it binds: what it binds takes it and `on change` runs.
    pub fn change(&mut self, value: &serde_json::Value) {
        let value = value.to_string();
        unsafe { clarity_custom_change(self.raw, value.as_ptr() as C, value.len()) };
    }

    /// It tells `tell` with its values: the handler wired to it runs.
    pub fn tell(&mut self, tell: &str, values: &[serde_json::Value]) {
        let values = serde_json::Value::Array(values.to_vec()).to_string();
        unsafe { clarity_custom_tell(self.raw, tell.as_ptr() as C, tell.len(), values.as_ptr() as C, values.len()) };
    }

    /// A text of the program's translations by its key, in the language shown; `english` when there is none.
    pub fn translate(&self, key: &str, english: &str) -> String {
        let mut out: *mut c_char = std::ptr::null_mut();
        if unsafe { clarity_translate(key.as_ptr() as C, key.len(), english.as_ptr() as C, english.len(), &mut out) } != OK || out.is_null() {
            return english.to_string();
        }
        let text = unsafe { std::ffi::CStr::from_ptr(out) }.to_string_lossy().into_owned();
        unsafe { clarity_string_free(out) };
        text
    }
}

/// A custom part as the library holds it: what the program's glue makes of
/// the part's crate. Every method is called on the screen's thread.
pub trait CustomPart {
    /// A value of its line, a property, what it binds (`value`) or `enabled`, as JSON.
    fn set(&mut self, name: &str, value: &serde_json::Value, cx: &mut Cx);
    /// What code tells it, with its values.
    fn hear(&mut self, hear: &str, values: &[serde_json::Value], cx: &mut Cx);
    /// Draws it in its place.
    fn draw(&self, canvas: &mut Canvas, x: f64, y: f64, width: f64, height: f64);
    fn start(&mut self, cx: &mut Cx) {
        let _ = cx;
    }
    fn focusable(&self) -> bool {
        false
    }
    fn pointer_pressed(&mut self, at: Pointer, cx: &mut Cx) {
        let _ = (at, cx);
    }
    fn pointer_moved(&mut self, at: Pointer, cx: &mut Cx) {
        let _ = (at, cx);
    }
    fn pointer_released(&mut self, at: Pointer, cx: &mut Cx) {
        let _ = (at, cx);
    }
    fn click(&mut self, cx: &mut Cx) {
        let _ = cx;
    }
    fn wheel(&mut self, dx: f64, dy: f64, modifiers: u32, cx: &mut Cx) -> bool {
        let _ = (dx, dy, modifiers, cx);
        false
    }
    /// A key by its name ("Enter", "Up", "A", "+") while it has the focus.
    fn key(&mut self, name: &str, modifiers: u32, cx: &mut Cx) -> bool {
        let _ = (name, modifiers, cx);
        false
    }
    fn text(&mut self, text: &str, cx: &mut Cx) {
        let _ = (text, cx);
    }
    fn focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        let _ = (focused, cx);
    }
    fn wake(&mut self, cx: &mut Cx) {
        let _ = cx;
    }
    fn stop(&mut self) {}
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let _ = (x, y);
        None
    }
}

type Make = fn() -> Box<dyn CustomPart>;
type HeldPart = Box<dyn CustomPart>;

/// Runs a part's code; a panic of the part never crosses into the library: it is said and dropped.
fn shielded<R>(fallback: R, act: impl FnOnce() -> R) -> R {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(act)).unwrap_or_else(|_| {
        eprintln!("a custom part failed (see the message above); it goes on as it was");
        fallback
    })
}

fn text_of<'a>(p: C, n: usize) -> &'a str {
    if p.is_null() || n == 0 {
        return "";
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(p as *const u8, n) }).unwrap_or("")
}

fn json_of(p: C, n: usize) -> serde_json::Value {
    serde_json::from_str(text_of(p, n)).unwrap_or(serde_json::Value::Null)
}

/// The part behind the library's pointer.
unsafe fn held<'a>(part: *mut c_void) -> &'a mut HeldPart {
    unsafe { &mut *(part as *mut HeldPart) }
}

fn pointer_of(at: *const RawPointer) -> Pointer {
    let at = unsafe { &*at };
    Pointer { x: at.x, y: at.y, modifiers: at.modifiers, clicks: at.clicks }
}

unsafe extern "C" fn part_make(user: *mut c_void) -> *mut c_void {
    let make: Make = unsafe { std::mem::transmute::<*mut c_void, Make>(user) };
    shielded(std::ptr::null_mut(), || Box::into_raw(Box::new(make())) as *mut c_void)
}

unsafe extern "C" fn part_set(part: *mut c_void, name: C, name_len: usize, value: C, value_len: usize, cx: *mut RawCx) {
    let value = json_of(value, value_len);
    shielded((), || unsafe { held(part) }.set(text_of(name, name_len), &value, &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_hear(part: *mut c_void, hear: C, hear_len: usize, values: C, values_len: usize, cx: *mut RawCx) {
    let values = match json_of(values, values_len) {
        serde_json::Value::Array(values) => values,
        _ => Vec::new(),
    };
    shielded((), || unsafe { held(part) }.hear(text_of(hear, hear_len), &values, &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_draw(part: *mut c_void, canvas: *mut RawCanvas, x: f64, y: f64, width: f64, height: f64) {
    shielded((), || unsafe { held(part) }.draw(&mut Canvas { raw: canvas }, x, y, width, height));
}

unsafe extern "C" fn part_start(part: *mut c_void, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.start(&mut Cx { raw: cx }));
}

unsafe extern "C" fn part_focusable(part: *mut c_void) -> i32 {
    shielded(0, || unsafe { held(part) }.focusable() as i32)
}

unsafe extern "C" fn part_pressed(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.pointer_pressed(pointer_of(at), &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_moved(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.pointer_moved(pointer_of(at), &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_released(part: *mut c_void, at: *const RawPointer, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.pointer_released(pointer_of(at), &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_click(part: *mut c_void, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.click(&mut Cx { raw: cx }));
}

unsafe extern "C" fn part_wheel(part: *mut c_void, dx: f64, dy: f64, modifiers: u32, cx: *mut RawCx) -> i32 {
    shielded(0, || unsafe { held(part) }.wheel(dx, dy, modifiers, &mut Cx { raw: cx }) as i32)
}

unsafe extern "C" fn part_key(part: *mut c_void, name: C, name_len: usize, modifiers: u32, cx: *mut RawCx) -> i32 {
    shielded(0, || unsafe { held(part) }.key(text_of(name, name_len), modifiers, &mut Cx { raw: cx }) as i32)
}

unsafe extern "C" fn part_text(part: *mut c_void, text: C, text_len: usize, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.text(text_of(text, text_len), &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_focus_changed(part: *mut c_void, focused: i32, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.focus_changed(focused != 0, &mut Cx { raw: cx }));
}

unsafe extern "C" fn part_wake(part: *mut c_void, cx: *mut RawCx) {
    shielded((), || unsafe { held(part) }.wake(&mut Cx { raw: cx }));
}

unsafe extern "C" fn part_stop(part: *mut c_void) {
    shielded((), || unsafe { held(part) }.stop());
}

unsafe extern "C" fn part_tooltip(part: *mut c_void, x: f64, y: f64, tip: *mut Answer) {
    if let Some(text) = shielded(None, || unsafe { held(part) }.tooltip_at(x, y)) {
        unsafe { clarity_answer_set(tip, text.as_ptr(), text.len()) };
    }
}

unsafe extern "C" fn part_destroy(part: *mut c_void) {
    if !part.is_null() {
        shielded((), || drop(unsafe { Box::from_raw(part as *mut HeldPart) }));
    }
}

/// Registers a kind of custom part by its declaration's name ("Dice"):
/// `make` makes one each time a window with such a part opens.
pub fn register_part(kind: &str, make: Make) -> Result<(), String> {
    let callbacks = CustomCallbacks {
        user: make as *mut c_void,
        make: Some(part_make),
        set: Some(part_set),
        hear: Some(part_hear),
        draw: Some(part_draw),
        start: Some(part_start),
        focusable: Some(part_focusable),
        pointer_pressed: Some(part_pressed),
        pointer_moved: Some(part_moved),
        pointer_released: Some(part_released),
        click: Some(part_click),
        wheel: Some(part_wheel),
        key: Some(part_key),
        text: Some(part_text),
        focus_changed: Some(part_focus_changed),
        wake: Some(part_wake),
        stop: Some(part_stop),
        tooltip_at: Some(part_tooltip),
        destroy: Some(part_destroy),
    };
    checked(unsafe { clarity_custom_register(kind.as_ptr() as C, kind.len(), &callbacks) }, "clarity_custom_register")
}

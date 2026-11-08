//! The desktop through Clarity.UI's ready-made library (DESIGN 7.9): its C
//! interface (`clarity_ui.h`), as Go, C and C++ programs use it, so a Rust
//! program need not build Clarity.UI and Skia itself. The program answers the
//! library's commands through `wire::answer`, the same answers the web wire
//! gives. The generated program's build script links the library from the
//! folder CLARITY_UI names; a project that writes `ui crate` builds the
//! clarity-ui crate instead and never uses this module.

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
pub fn run(program: impl Program, app: AppFiles) -> ExitCode {
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

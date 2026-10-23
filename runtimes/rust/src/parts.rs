//! Custom parts in the shell (DESIGN 6.5): the native parts of a program
//! are in `libclarity_parts` beside it, which the shell loads when it is
//! there and asks to place, set and tell its parts. A program without
//! native parts has no such library, and nothing here does anything.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::OnceLock;

/// What a part told: its window's number, the part, the tell and its values as JSON.
pub type Told = Box<dyn Fn(u64, &str, &str, &str) + Send + Sync>;

type TellFn = extern "C" fn(u64, *const c_char, *const c_char, *const c_char, *mut c_void);

struct Library {
    start: extern "C" fn(TellFn, *mut c_void) -> bool,
    problem: extern "C" fn() -> *const c_char,
    stop: extern "C" fn(),
    place: extern "C" fn(u64, *const c_char, *const c_char, usize, c_int, c_int, c_int, c_int, bool),
    set: extern "C" fn(u64, *const c_char, *const c_char, *const c_char),
    hear: extern "C" fn(u64, *const c_char, *const c_char, *const c_char),
    close: extern "C" fn(u64),
}

static LIBRARY: OnceLock<Library> = OnceLock::new();
static TOLD: OnceLock<Told> = OnceLock::new();

#[cfg(unix)]
mod dl {
    use std::ffi::{c_char, c_int, c_void};
    pub const RTLD_NOW: c_int = 2;
    unsafe extern "C" {
        pub fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        pub fn dlerror() -> *const c_char;
    }
}

/// `libclarity_parts` beside the program.
fn beside_program() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(target_os = "macos") { "libclarity_parts.dylib" } else if cfg!(windows) { "clarity_parts.dll" } else { "libclarity_parts.so" };
    Some(exe.parent()?.join(name))
}

/// Whether the program has native parts: `libclarity_parts` beside it.
pub fn present() -> bool {
    beside_program().is_some_and(|p| p.exists())
}

extern "C" fn on_told(window: u64, part: *const c_char, tell: *const c_char, values: *const c_char, _: *mut c_void) {
    let text = |p: *const c_char| if p.is_null() { "" } else { unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("") };
    if let Some(told) = TOLD.get() {
        told(window, text(part), text(tell), text(values));
    }
}

/// Loads and starts them; `told` is called on the parts' own thread.
#[cfg(unix)]
pub fn start(told: Told) -> Result<(), String> {
    if LIBRARY.get().is_some() {
        return Ok(());
    }
    let path = beside_program().ok_or("the program's custom parts cannot be found")?;
    let path = CString::new(path.to_string_lossy().into_owned()).map_err(|e| e.to_string())?;
    let handle = unsafe { dl::dlopen(path.as_ptr(), dl::RTLD_NOW) };
    if handle.is_null() {
        let why = unsafe { CStr::from_ptr(dl::dlerror()) }.to_string_lossy().into_owned();
        return Err(format!("the program's custom parts cannot load: {why}"));
    }
    let find = |name: &str| -> Result<*mut c_void, String> {
        let name = CString::new(name).unwrap_or_default();
        let found = unsafe { dl::dlsym(handle, name.as_ptr()) };
        if found.is_null() { Err("the program's custom parts library is not Clarity's".to_string()) } else { Ok(found) }
    };
    // The functions of clarity_parts.h, with the signatures it declares.
    let library = unsafe {
        Library {
            start: std::mem::transmute::<*mut c_void, extern "C" fn(TellFn, *mut c_void) -> bool>(find("clarity_parts_start")?),
            problem: std::mem::transmute::<*mut c_void, extern "C" fn() -> *const c_char>(find("clarity_parts_problem")?),
            stop: std::mem::transmute::<*mut c_void, extern "C" fn()>(find("clarity_parts_stop")?),
            place: std::mem::transmute::<*mut c_void, extern "C" fn(u64, *const c_char, *const c_char, usize, c_int, c_int, c_int, c_int, bool)>(find("clarity_parts_place")?),
            set: std::mem::transmute::<*mut c_void, extern "C" fn(u64, *const c_char, *const c_char, *const c_char)>(find("clarity_parts_set")?),
            hear: std::mem::transmute::<*mut c_void, extern "C" fn(u64, *const c_char, *const c_char, *const c_char)>(find("clarity_parts_hear")?),
            close: std::mem::transmute::<*mut c_void, extern "C" fn(u64)>(find("clarity_parts_close")?),
        }
    };
    let _ = TOLD.set(told);
    if !(library.start)(on_told, std::ptr::null_mut()) {
        let why = unsafe { CStr::from_ptr((library.problem)()) }.to_string_lossy().into_owned();
        return Err(why);
    }
    let _ = LIBRARY.set(library);
    Ok(())
}

#[cfg(not(unix))]
pub fn start(_told: Told) -> Result<(), String> {
    Err("custom parts drawn by a native library run on Linux for now".into())
}

fn c(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

/// Where a part is, in the native window `parent`, and whether it shows.
#[allow(clippy::too_many_arguments)]
pub fn place(window: u64, part: &str, kind: &str, parent: usize, x: i32, y: i32, width: i32, height: i32, shown: bool) {
    if let Some(l) = LIBRARY.get() {
        (l.place)(window, c(part).as_ptr(), c(kind).as_ptr(), parent, x, y, width, height, shown);
    }
}

/// Sets a property of a part, its value as JSON.
pub fn set(window: u64, part: &str, name: &str, value: &str) {
    if let Some(l) = LIBRARY.get() {
        (l.set)(window, c(part).as_ptr(), c(name).as_ptr(), c(value).as_ptr());
    }
}

/// Gives a part a hear, its values as a JSON array.
pub fn hear(window: u64, part: &str, hear: &str, values: &str) {
    if let Some(l) = LIBRARY.get() {
        (l.hear)(window, c(part).as_ptr(), c(hear).as_ptr(), c(values).as_ptr());
    }
}

/// Ends every part of a window that closed.
pub fn close(window: u64) {
    if let Some(l) = LIBRARY.get() {
        (l.close)(window);
    }
}

/// Ends every part.
pub fn stop() {
    if let Some(l) = LIBRARY.get() {
        (l.stop)();
    }
}

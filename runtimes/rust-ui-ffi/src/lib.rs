//! The C interface of Clarity.UI (DESIGN 7.9, "One Clarity.UI for all five
//! languages"): `extern "C"` functions over plain numbers, UTF-8 text and
//! opaque handles, which C#, Go, C and C++ call. The header that declares
//! them is written by hand in `include/clarity_ui.h`; each function here
//! matches its line there.
//!
//! Every entry runs inside `guard`: a panic never crosses the interface, a
//! failing call returns a status and leaves its message for the thread
//! (`clarity_last_error`).

pub mod canvas;
pub mod custom;
pub mod desktop;
pub mod files;
pub mod host;
pub mod program;
pub mod screen;
// The program's calls as the wire's JSON: clarity-ui's, which the browser host shares.
pub use ui::program_wire::args as wire;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// What a call answers: `CLARITY_OK` or why it failed.
pub type ClarityStatus = i32;

pub const CLARITY_OK: ClarityStatus = 0;
pub const CLARITY_FAILED: ClarityStatus = 1;
pub const CLARITY_PANIC: ClarityStatus = 2;
pub const CLARITY_INVALID: ClarityStatus = 3;
pub const CLARITY_NOT_FOUND: ClarityStatus = 4;

/// The interface version the header names (`CLARITY_UI_INTERFACE`).
pub const INTERFACE: u32 = 1;

/// Why a call failed: its status and message.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub status: ClarityStatus,
    pub message: String,
}

impl Failure {
    pub fn failed(message: impl Into<String>) -> Failure {
        Failure { status: CLARITY_FAILED, message: message.into() }
    }

    pub fn invalid(message: impl Into<String>) -> Failure {
        Failure { status: CLARITY_INVALID, message: message.into() }
    }

    pub fn not_found(message: impl Into<String>) -> Failure {
        Failure { status: CLARITY_NOT_FOUND, message: message.into() }
    }
}

pub type Done = Result<(), Failure>;

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The words of a panic: its message when it has one.
pub fn panic_words(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "the library stopped".to_string())
}

/// Runs an entry: its failure or panic becomes a status and the thread's last error.
pub fn guard(run: impl FnOnce() -> Done) -> ClarityStatus {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(())) => CLARITY_OK,
        Ok(Err(failure)) => {
            LAST_ERROR.with(|e| *e.borrow_mut() = Some(failure.message));
            failure.status
        }
        Err(panic) => {
            LAST_ERROR.with(|e| *e.borrow_mut() = Some(panic_words(panic.as_ref())));
            CLARITY_PANIC
        }
    }
}

/// Runs an entry that answers nothing and may not fail: a panic is let go.
pub fn quietly(run: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(run));
}

/// Text given as a pointer and a length; none for a NULL pointer.
///
/// # Safety
/// `ptr` points at `len` readable bytes, or is NULL.
pub unsafe fn text_or_none<'a>(ptr: *const c_char, len: usize) -> Result<Option<&'a str>, Failure> {
    if ptr.is_null() {
        return Ok(None);
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    std::str::from_utf8(bytes).map(Some).map_err(|_| Failure::invalid("the text is no UTF-8"))
}

/// Text given as a pointer and a length; NULL is the empty text.
///
/// # Safety
/// As `text_or_none`.
pub unsafe fn text<'a>(ptr: *const c_char, len: usize) -> Result<&'a str, Failure> {
    unsafe { text_or_none(ptr, len) }.map(|t| t.unwrap_or(""))
}

/// Bytes given as a pointer and a length; NULL is none.
///
/// # Safety
/// `ptr` points at `len` readable bytes, or is NULL.
pub unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(ptr, len) } }
}

/// JSON given as text.
///
/// # Safety
/// As `text`.
pub unsafe fn json(ptr: *const c_char, len: usize) -> Result<serde_json::Value, Failure> {
    let written = unsafe { text(ptr, len) }?;
    serde_json::from_str(written).map_err(|e| Failure::invalid(format!("the JSON does not read: {e}")))
}

/// A string for the caller, which it frees with `clarity_string_free`.
pub fn give(text: &str) -> *mut c_char {
    // A NUL inside cannot be given in a C string: it ends the text there.
    let cut = text.split('\0').next().unwrap_or_default();
    CString::new(cut).map(CString::into_raw).unwrap_or(std::ptr::null_mut())
}

/// Writes `value` to an out pointer the caller gave.
///
/// # Safety
/// `out` is NULL or points at writable memory for a `T`.
pub unsafe fn put<T>(out: *mut T, value: T) -> Done {
    if out.is_null() {
        return Err(Failure::invalid("the out pointer is NULL"));
    }
    unsafe { out.write(value) };
    Ok(())
}

/// Writes `value` to an out pointer when the caller gave one.
///
/// # Safety
/// `out` is NULL or points at writable memory for a `T`.
pub unsafe fn put_maybe<T>(out: *mut T, value: T) {
    if !out.is_null() {
        unsafe { out.write(value) };
    }
}

/// A handle the caller gave, as a reference; NULL is invalid.
///
/// # Safety
/// `handle` is NULL or a live handle of the library of type `T`.
pub unsafe fn handle<'a, T>(handle: *const T) -> Result<&'a T, Failure> {
    unsafe { handle.as_ref() }.ok_or_else(|| Failure::invalid("the handle is NULL"))
}

/// A handle the caller gave, to change; NULL is invalid.
///
/// # Safety
/// As `handle`, and no other reference to it lives.
pub unsafe fn handle_mut<'a, T>(handle: *mut T) -> Result<&'a mut T, Failure> {
    unsafe { handle.as_mut() }.ok_or_else(|| Failure::invalid("the handle is NULL"))
}

// -- The entries -------------------------------------------------------------

/// The message of the last failing call on this thread; NULL when there is none.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_last_error() -> *mut c_char {
    catch_unwind(|| LAST_ERROR.with(|e| e.borrow().as_deref().map(give).unwrap_or(std::ptr::null_mut()))).unwrap_or(std::ptr::null_mut())
}

/// Frees a string the library gave.
///
/// # Safety
/// `text` is NULL or a string the library gave and not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_string_free(text: *mut c_char) {
    if !text.is_null() {
        quietly(|| drop(unsafe { CString::from_raw(text) }));
    }
}

static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// The library's version, a static string.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_ui_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// The interface version the library has.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_ui_interface() -> u32 {
    INTERFACE
}

/// What a callback answers, set through `clarity_answer_set`.
#[derive(Debug, Default)]
pub struct ClarityAnswer {
    pub data: Option<Vec<u8>>,
}

impl ClarityAnswer {
    /// The answer as text; none when nothing was set or it is no UTF-8.
    pub fn text(self) -> Option<String> {
        self.data.and_then(|d| String::from_utf8(d).ok())
    }

    /// The answer as JSON; none when nothing was set or it does not read.
    pub fn json(self) -> Option<serde_json::Value> {
        self.text().and_then(|t| serde_json::from_str(&t).ok())
    }
}

/// Sets what a callback answers.
///
/// # Safety
/// `answer` is the answer the callback was given; `data` points at `len` bytes or is NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_answer_set(answer: *mut ClarityAnswer, data: *const u8, len: usize) -> ClarityStatus {
    guard(|| {
        let answer = unsafe { handle_mut(answer) }?;
        answer.data = Some(unsafe { bytes(data, len) }.to_vec());
        Ok(())
    })
}

//! The desktop and a driven program: the program's windows in desktop
//! windows until they close (the desktop host of clarity-ui), woken from
//! another thread; or on a screen without a window, step by step (DESIGN
//! 1.6).

use std::ffi::c_char;
use std::sync::Arc;

use clarity_runtime::program::Program;
use ui::headless::Driven;

use crate::files::ClarityFiles;
use crate::program::ClarityProgram;
use crate::{guard, handle, handle_mut, put, ClarityStatus, Failure};

/// Runs the program's windows until they closed.
///
/// # Safety
/// `program` and `files` are live handles; `exit_code` is writable or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_desktop_run(program: *const ClarityProgram, files: *const ClarityFiles, exit_code: *mut i32) -> ClarityStatus {
    guard(|| {
        let program: Arc<dyn Program> = unsafe { handle(program) }?.program.clone();
        let files = unsafe { handle(files) }?.app_files();
        crate::audio::take_on_this_thread();
        let ended = ui::desktop::run_shared(program, files);
        let code = if ended == std::process::ExitCode::SUCCESS { 0 } else { 1 };
        unsafe { crate::put_maybe(exit_code, code) };
        Ok(())
    })
}

/// Wakes the running desktop from any thread.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_desktop_wake() {
    crate::quietly(ui::desktop::wake);
}

/// A program on a screen without a window, and the thread it lives on.
pub struct ClarityDriven {
    driven: Driven,
    thread: std::thread::ThreadId,
}

/// Starts the program on a screen without a window.
///
/// # Safety
/// `program` and `files` are live handles; `out` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_driven_start(program: *const ClarityProgram, files: *const ClarityFiles, out: *mut *mut ClarityDriven) -> ClarityStatus {
    guard(|| {
        let program: Arc<dyn Program> = unsafe { handle(program) }?.program.clone();
        let files = unsafe { handle(files) }?.app_files();
        ui::fonts::carry_fallbacks(files.resources);
        let driven = ClarityDriven { driven: Driven::start(program, files), thread: std::thread::current().id() };
        unsafe { put(out, Box::into_raw(Box::new(driven))) }
    })
}

/// Does one step and answers it as JSON; a step that could not be done answers its `error` as well as failing.
///
/// # Safety
/// `driven` is a live handle; `step` is (pointer, length); `answer` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_driven_step(driven: *mut ClarityDriven, step: *const c_char, step_len: usize, answer: *mut *mut c_char) -> ClarityStatus {
    guard(|| {
        let driven = unsafe { handle_mut(driven) }?;
        if driven.thread != std::thread::current().id() {
            return Err(Failure::invalid("the driven program is used from another thread than the one that made it"));
        }
        let step = unsafe { crate::json(step, step_len) }?;
        let answered = driven.driven.step(&step);
        unsafe { put(answer, crate::give(&answered.to_string())) }?;
        match answered.get("error").and_then(serde_json::Value::as_str) {
            Some(why) => Err(Failure::failed(why)),
            None => Ok(()),
        }
    })
}

/// Ends a driven program: its windows' parts stop.
///
/// # Safety
/// `driven` is NULL or a handle from `clarity_driven_start`, not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_driven_free(driven: *mut ClarityDriven) {
    if !driven.is_null() {
        crate::quietly(|| {
            let mut made = unsafe { Box::from_raw(driven) };
            made.driven.screen().dispose();
        });
    }
}

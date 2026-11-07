//! The C interface of Clarity.Video (include/clarity_video.h): one table of
//! functions a program hands to Clarity.UI with `clarity_ui_use_video`.
//! No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use video::{Frames, Problem};

pub const INTERFACE: u32 = 2;

pub const OK: i32 = 0;
pub const MISSING: i32 = 1;
pub const UNREADABLE: i32 = 2;
pub const NOT_A_VIDEO: i32 = 3;
pub const NO_CODEC: i32 = 4;
pub const CODEC_MISSING: i32 = 5;
pub const UNDECODABLE: i32 = 6;
pub const FAILED: i32 = 7;

pub const NO_PICTURE: i32 = 0;
pub const SAME_PICTURE: i32 = 1;
pub const NEW_PICTURE: i32 = 2;

/// A video's frames, decoding on their own thread.
pub struct ClarityVideoFrames(Frames);

/// The table of include/clarity_video.h, field for field.
#[repr(C)]
pub struct ClarityVideoApi {
    pub interface: u32,
    pub open: extern "C" fn(*const c_char, usize) -> *mut ClarityVideoFrames,
    pub free: extern "C" fn(*mut ClarityVideoFrames),
    pub size: extern "C" fn(*const ClarityVideoFrames, *mut u32, *mut u32),
    pub length_ms: extern "C" fn(*const ClarityVideoFrames) -> u64,
    pub frame_ms: extern "C" fn(*const ClarityVideoFrames) -> u64,
    pub picture_at: extern "C" fn(*const ClarityVideoFrames, u64, u64, *mut u8, usize, *mut u64) -> i32,
    pub ended_at: extern "C" fn(*const ClarityVideoFrames, u64) -> i32,
    pub problem: extern "C" fn(*const ClarityVideoFrames, *mut c_char, usize) -> i32,
    pub add_codec: extern "C" fn(*const c_char, usize) -> i32,
}

static API: ClarityVideoApi = ClarityVideoApi { interface: INTERFACE, open, free, size, length_ms, frame_ms, picture_at, ended_at, problem, add_codec };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_video_api() -> *const ClarityVideoApi {
    &API
}

thread_local! {
    static LAST: RefCell<(i32, String)> = const { RefCell::new((OK, String::new())) };
}

fn code(problem: &Problem) -> i32 {
    match problem {
        Problem::Missing(_) => MISSING,
        Problem::Unreadable(_) => UNREADABLE,
        Problem::NotAVideo(_) => NOT_A_VIDEO,
        Problem::NoCodec(_) => NO_CODEC,
        Problem::CodecMissing(_) => CODEC_MISSING,
        Problem::Undecodable(_) => UNDECODABLE,
    }
}

fn keep(code: i32, detail: &str) {
    LAST.with(|l| *l.borrow_mut() = (code, detail.to_string()));
}

fn guarded<R>(fault: R, act: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(act)).unwrap_or_else(|_| {
        keep(FAILED, "a fault inside Clarity.Video");
        fault
    })
}

fn copy_out(text: &str, into: *mut c_char, size: usize) {
    if !into.is_null() && size > 0 {
        let n = text.len().min(size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
}

fn frames<'a>(frames: *const ClarityVideoFrames) -> Option<&'a Frames> {
    unsafe { frames.as_ref() }.map(|f| &f.0)
}

extern "C" fn open(path: *const c_char, path_len: usize) -> *mut ClarityVideoFrames {
    guarded(std::ptr::null_mut(), || {
        if path.is_null() {
            keep(FAILED, "no path");
            return std::ptr::null_mut();
        }
        let bytes = unsafe { std::slice::from_raw_parts(path.cast::<u8>(), path_len) };
        let Ok(path) = std::str::from_utf8(bytes) else {
            keep(FAILED, "the path is no UTF-8");
            return std::ptr::null_mut();
        };
        match Frames::open(std::path::Path::new(path)) {
            Ok(opened) => Box::into_raw(Box::new(ClarityVideoFrames(opened))),
            Err(problem) => {
                keep(code(&problem), problem.detail());
                std::ptr::null_mut()
            }
        }
    })
}

extern "C" fn free(frames: *mut ClarityVideoFrames) {
    if !frames.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(frames) }));
    }
}

extern "C" fn size(frames_ptr: *const ClarityVideoFrames, width: *mut u32, height: *mut u32) {
    guarded((), || {
        let (w, h) = frames(frames_ptr).map_or((0, 0), Frames::size);
        unsafe {
            if let Some(width) = width.as_mut() {
                *width = w;
            }
            if let Some(height) = height.as_mut() {
                *height = h;
            }
        }
    })
}

extern "C" fn length_ms(frames_ptr: *const ClarityVideoFrames) -> u64 {
    guarded(0, || frames(frames_ptr).map_or(0, |f| f.length().as_millis() as u64))
}

extern "C" fn frame_ms(frames_ptr: *const ClarityVideoFrames) -> u64 {
    guarded(40, || frames(frames_ptr).map_or(40, |f| f.frame_length().as_millis().max(1) as u64))
}

extern "C" fn picture_at(frames_ptr: *const ClarityVideoFrames, at_ms: u64, known_ms: u64, into: *mut u8, size: usize, time_ms: *mut u64) -> i32 {
    guarded(NO_PICTURE, || {
        let Some((time, picture)) = frames(frames_ptr).and_then(|f| f.picture_at(Duration::from_millis(at_ms))) else { return NO_PICTURE };
        let time = time.as_millis() as u64;
        if let Some(out) = unsafe { time_ms.as_mut() } {
            *out = time;
        }
        if time == known_ms {
            return SAME_PICTURE;
        }
        if into.is_null() || size < picture.rgba.len() {
            return NO_PICTURE;
        }
        unsafe { std::ptr::copy_nonoverlapping(picture.rgba.as_ptr(), into, picture.rgba.len()) };
        NEW_PICTURE
    })
}

extern "C" fn ended_at(frames_ptr: *const ClarityVideoFrames, at_ms: u64) -> i32 {
    guarded(0, || frames(frames_ptr).is_some_and(|f| f.ended_at(Duration::from_millis(at_ms))) as i32)
}

extern "C" fn problem(frames_ptr: *const ClarityVideoFrames, into: *mut c_char, size: usize) -> i32 {
    guarded(FAILED, || {
        let (found, detail) = match frames(frames_ptr) {
            Some(f) => f.problem().map_or((OK, String::new()), |p| (code(&p), p.detail().to_string())),
            None => LAST.with(|l| l.borrow().clone()),
        };
        copy_out(&detail, into, size);
        found
    })
}

extern "C" fn add_codec(file: *const c_char, file_len: usize) -> i32 {
    guarded(FAILED, || {
        if file.is_null() {
            keep(FAILED, "no file");
            return FAILED;
        }
        let bytes = unsafe { std::slice::from_raw_parts(file.cast::<u8>(), file_len) };
        let Ok(file) = std::str::from_utf8(bytes) else {
            keep(FAILED, "the file is no UTF-8");
            return FAILED;
        };
        match video::library::add(file) {
            Ok(_) => OK,
            Err(problem) => {
                keep(code(&problem), problem.detail());
                code(&problem)
            }
        }
    })
}

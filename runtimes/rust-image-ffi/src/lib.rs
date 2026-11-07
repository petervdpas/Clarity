//! The C interface of Clarity.Image (include/clarity_image.h): one table of
//! the picture functions. No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use pictures::Problem;

pub const INTERFACE: u32 = 1;

pub const OK: i32 = 0;
pub const NOT_A_PICTURE: i32 = 1;
pub const BAD_SIZE: i32 = 2;
pub const FAILED: i32 = 3;

type Out = *mut *mut u8;

/// The table of include/clarity_image.h, field for field.
#[repr(C)]
pub struct ClarityImageApi {
    pub interface: u32,
    pub resize: extern "C" fn(*const u8, usize, i64, i64, Out, *mut usize) -> i32,
    pub crop: extern "C" fn(*const u8, usize, i64, i64, i64, i64, Out, *mut usize) -> i32,
    pub rotate: extern "C" fn(*const u8, usize, f64, Out, *mut usize) -> i32,
    pub flip: extern "C" fn(*const u8, usize, i32, Out, *mut usize) -> i32,
    pub blend: extern "C" fn(*const u8, usize, *const u8, usize, i64, i64, i64, Out, *mut usize) -> i32,
    pub adjust: extern "C" fn(*const u8, usize, f64, f64, f64, f64, Out, *mut usize) -> i32,
    pub size: extern "C" fn(*const u8, usize, *mut u32, *mut u32) -> i32,
    pub free: extern "C" fn(*mut u8, usize),
    pub problem: extern "C" fn(*mut c_char, usize) -> i32,
}

static API: ClarityImageApi = ClarityImageApi { interface: INTERFACE, resize, crop, rotate, flip, blend, adjust, size, free, problem };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_image_api() -> *const ClarityImageApi {
    &API
}

thread_local! {
    static LAST: RefCell<(i32, String)> = const { RefCell::new((OK, String::new())) };
}

fn keep(code: i32, detail: &str) {
    LAST.with(|l| *l.borrow_mut() = (code, detail.to_string()));
}

fn bytes<'a>(at: *const u8, len: usize) -> &'a [u8] {
    if at.is_null() || len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(at, len) } }
}

/// Runs a function that gives a picture: its PNG goes to `out`, or the problem is kept and its code given.
fn giving(out: Out, out_len: *mut usize, act: impl FnOnce() -> Result<Vec<u8>, Problem>) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(act));
    let made = match result {
        Ok(Ok(made)) => made,
        Ok(Err(problem)) => {
            let code = match problem {
                Problem::NotAPicture(_) => NOT_A_PICTURE,
                Problem::BadSize => BAD_SIZE,
            };
            keep(code, &problem.detail());
            return code;
        }
        Err(_) => {
            keep(FAILED, "a fault inside Clarity.Image");
            return FAILED;
        }
    };
    if out.is_null() || out_len.is_null() {
        keep(FAILED, "nowhere to give the picture");
        return FAILED;
    }
    let boxed = made.into_boxed_slice();
    let len = boxed.len();
    unsafe {
        *out = Box::into_raw(boxed).cast::<u8>();
        *out_len = len;
    }
    OK
}

extern "C" fn resize(p: *const u8, len: usize, width: i64, height: i64, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::resize(bytes(p, len), width, height))
}

extern "C" fn crop(p: *const u8, len: usize, x: i64, y: i64, width: i64, height: i64, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::crop(bytes(p, len), x, y, width, height))
}

extern "C" fn rotate(p: *const u8, len: usize, degrees: f64, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::rotate(bytes(p, len), degrees))
}

extern "C" fn flip(p: *const u8, len: usize, horizontal: i32, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::flip(bytes(p, len), horizontal != 0))
}

extern "C" fn blend(base: *const u8, base_len: usize, over: *const u8, over_len: usize, x: i64, y: i64, opacity: i64, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::blend(bytes(base, base_len), bytes(over, over_len), x, y, opacity))
}

extern "C" fn adjust(p: *const u8, len: usize, hue: f64, saturation: f64, brightness: f64, contrast: f64, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || pictures::adjust(bytes(p, len), hue, saturation, brightness, contrast))
}

extern "C" fn size(p: *const u8, len: usize, width: *mut u32, height: *mut u32) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| pictures::size(bytes(p, len))));
    match result {
        Ok(Ok((w, h))) => {
            unsafe {
                if let Some(width) = width.as_mut() {
                    *width = w;
                }
                if let Some(height) = height.as_mut() {
                    *height = h;
                }
            }
            OK
        }
        Ok(Err(problem)) => {
            keep(NOT_A_PICTURE, &problem.detail());
            NOT_A_PICTURE
        }
        Err(_) => {
            keep(FAILED, "a fault inside Clarity.Image");
            FAILED
        }
    }
}

extern "C" fn free(at: *mut u8, len: usize) {
    if !at.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(at, len)) })));
    }
}

extern "C" fn problem(into: *mut c_char, size: usize) -> i32 {
    let (code, detail) = LAST.with(|l| l.borrow().clone());
    if !into.is_null() && size > 0 {
        let n = detail.len().min(size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(detail.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
    code
}

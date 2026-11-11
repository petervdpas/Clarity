//! The C interface of Clarity.Auth (include/clarity_auth.h): one table with
//! one operation call, the arguments and result as JSON. No panic crosses
//! the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use auth::ProblemKind;

pub const INTERFACE: u32 = 1;

pub const OK: i32 = 0;
pub const UNKNOWN_STRATEGY: i32 = 1;
pub const BAD_INPUT: i32 = 2;
pub const FAILED: i32 = 3;

type Out = *mut *mut u8;

/// The table of include/clarity_auth.h, field for field.
#[repr(C)]
pub struct ClarityAuthApi {
    pub interface: u32,
    pub call: extern "C" fn(*const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub free: extern "C" fn(*mut u8, usize),
    pub problem: extern "C" fn(*mut c_char, usize) -> i32,
}

static API: ClarityAuthApi = ClarityAuthApi { interface: INTERFACE, call, free, problem };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_auth_api() -> *const ClarityAuthApi {
    &API
}

thread_local! {
    static LAST: RefCell<(i32, String)> = const { RefCell::new((OK, String::new())) };
}

fn keep(code: i32, detail: &str) -> i32 {
    LAST.with(|l| *l.borrow_mut() = (code, detail.to_string()));
    code
}

fn text<'a>(at: *const u8, len: usize) -> Option<&'a str> {
    let bytes = if at.is_null() || len == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(at, len) } };
    std::str::from_utf8(bytes).ok()
}

extern "C" fn call(op: *const u8, op_len: usize, args: *const u8, args_len: usize, out: Out, out_len: *mut usize) -> i32 {
    let (Some(op), Some(args)) = (text(op, op_len), text(args, args_len)) else {
        return keep(BAD_INPUT, "an operation or its arguments are no UTF-8");
    };
    let made = match catch_unwind(AssertUnwindSafe(|| auth::call(op, args))) {
        Ok(Ok(made)) => made,
        Ok(Err(problem)) => {
            let code = match problem.kind {
                ProblemKind::UnknownStrategy => UNKNOWN_STRATEGY,
                ProblemKind::BadInput => BAD_INPUT,
                ProblemKind::Failed => FAILED,
            };
            return keep(code, &problem.text);
        }
        Err(_) => return keep(FAILED, "a fault inside Clarity.Auth"),
    };
    if out.is_null() || out_len.is_null() {
        return keep(FAILED, "nowhere to give the result");
    }
    let boxed = made.into_bytes().into_boxed_slice();
    let len = boxed.len();
    unsafe {
        *out = Box::into_raw(boxed).cast::<u8>();
        *out_len = len;
    }
    OK
}

extern "C" fn free(at: *mut u8, len: usize) {
    if !at.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(at, len)) })));
    }
}

extern "C" fn problem(into: *mut c_char, size: usize) -> i32 {
    let (code, detail) = LAST.with(|l| l.borrow().clone());
    if !into.is_null() && size > 0 {
        // Cut on a character's boundary, so what is copied stays UTF-8.
        let mut n = detail.len().min(size - 1);
        while n > 0 && !detail.is_char_boundary(n) {
            n -= 1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(detail.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(op: &str, args: &str) -> (i32, String) {
        let api = unsafe { &*clarity_auth_api() };
        let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
        let code = (api.call)(op.as_ptr(), op.len(), args.as_ptr(), args.len(), &mut out, &mut len);
        if code == OK {
            let got = String::from_utf8(unsafe { std::slice::from_raw_parts(out, len) }.to_vec()).unwrap();
            (api.free)(out, len);
            return (code, got);
        }
        let mut buf = vec![0 as c_char; 256];
        (api.problem)(buf.as_mut_ptr(), buf.len());
        (code, unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
    }

    #[test]
    fn the_table_calls_by_name_and_says_why_not() {
        assert_eq!(run("hash", r#"{"method":"sha256","text":"a"}"#).0, OK);
        assert_eq!(run("find", r#"{"strategy":"pigeon","inputs":{}}"#).0, UNKNOWN_STRATEGY);
        assert_eq!(run("find", r#"{"strategy":"token","inputs":{}}"#), (BAD_INPUT, "the token strategy needs the input 'token'".into()));
        assert_eq!(run("shout", "{}").0, FAILED);
        assert_eq!(run("sessionFind", r#"{"token":"x"}"#), (OK, "0".into()));
    }

    #[test]
    fn the_table_is_the_same_for_the_library_s_whole_life() {
        assert_eq!(unsafe { (*clarity_auth_api()).interface }, INTERFACE);
        assert!(std::ptr::eq(clarity_auth_api(), clarity_auth_api()));
    }
}

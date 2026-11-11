//! The C interface of Clarity.Formats (include/clarity_formats.h): one table
//! of the format functions. Names, input and results cross as UTF-8 (and
//! bytes), the kind and options as JSON. No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use formats::{Problem, ProblemKind};

pub const INTERFACE: u32 = 2;

pub const OK: i32 = 0;
pub const NOT_FITTING: i32 = 1;
pub const UNKNOWN_FORMAT: i32 = 2;
pub const BAD_OPTIONS: i32 = 3;
pub const FAILED: i32 = 4;

type Out = *mut *mut u8;

/// The table of include/clarity_formats.h, field for field.
#[repr(C)]
pub struct ClarityFormatsApi {
    pub interface: u32,
    pub read: extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub write: extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub catalog: extern "C" fn(Out, *mut usize) -> i32,
    pub free: extern "C" fn(*mut u8, usize),
    pub problem: extern "C" fn(*mut c_char, usize) -> i32,
    pub read_table: extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    #[allow(clippy::type_complexity)]
    pub write_table: extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
}

static API: ClarityFormatsApi = ClarityFormatsApi { interface: INTERFACE, read, write, catalog, free, problem, read_table, write_table };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_formats_api() -> *const ClarityFormatsApi {
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

fn text<'a>(at: *const u8, len: usize) -> Result<&'a str, Problem> {
    std::str::from_utf8(bytes(at, len)).map_err(|_| Problem { kind: ProblemKind::Failed, text: "a name, kind or option given is no UTF-8".into() })
}

fn code(kind: ProblemKind) -> i32 {
    match kind {
        ProblemKind::NotFitting => NOT_FITTING,
        ProblemKind::UnknownFormat => UNKNOWN_FORMAT,
        ProblemKind::BadOptions => BAD_OPTIONS,
        ProblemKind::Failed => FAILED,
    }
}

/// Runs a call that gives bytes: they go to `out`, or the problem is kept and its code given.
fn giving(out: Out, out_len: *mut usize, act: impl FnOnce() -> Result<Vec<u8>, Problem>) -> i32 {
    let made = match catch_unwind(AssertUnwindSafe(act)) {
        Ok(Ok(made)) => made,
        Ok(Err(problem)) => {
            let code = code(problem.kind);
            keep(code, &problem.text);
            return code;
        }
        Err(_) => {
            keep(FAILED, "a fault inside Clarity.Formats");
            return FAILED;
        }
    };
    if out.is_null() || out_len.is_null() {
        keep(FAILED, "nowhere to give the result");
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

#[allow(clippy::too_many_arguments)]
extern "C" fn read(format: *const u8, format_len: usize, input: *const u8, input_len: usize, options: *const u8, options_len: usize, kind: *const u8, kind_len: usize, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || {
        let fitted = formats::read_json(text(format, format_len)?, bytes(input, input_len), text(options, options_len)?, text(kind, kind_len)?)?;
        Ok(fitted.into_bytes())
    })
}

#[allow(clippy::too_many_arguments)]
extern "C" fn write(format: *const u8, format_len: usize, value: *const u8, value_len: usize, options: *const u8, options_len: usize, kind: *const u8, kind_len: usize, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || formats::write_json(text(format, format_len)?, text(value, value_len)?, text(options, options_len)?, text(kind, kind_len)?))
}

#[allow(clippy::too_many_arguments)]
extern "C" fn read_table(format: *const u8, format_len: usize, input: *const u8, input_len: usize, options: *const u8, options_len: usize, fields: *const u8, fields_len: usize, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || Ok(formats::read_table(text(format, format_len)?, bytes(input, input_len), text(options, options_len)?, text(fields, fields_len)?)?.into_bytes()))
}

#[allow(clippy::too_many_arguments)]
extern "C" fn write_table(format: *const u8, format_len: usize, original: *const u8, original_len: usize, table: *const u8, table_len: usize, options: *const u8, options_len: usize, fields: *const u8, fields_len: usize, out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || formats::write_table(text(format, format_len)?, bytes(original, original_len), text(table, table_len)?, text(options, options_len)?, text(fields, fields_len)?))
}

extern "C" fn catalog(out: Out, out_len: *mut usize) -> i32 {
    giving(out, out_len, || Ok(formats::catalog_json().into_bytes()))
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

    fn call(format: &str, input: &str, options: &str, kind: &str) -> (i32, String) {
        let api = unsafe { &*clarity_formats_api() };
        let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
        let code = (api.read)(format.as_ptr(), format.len(), input.as_ptr(), input.len(), options.as_ptr(), options.len(), kind.as_ptr(), kind.len(), &mut out, &mut len);
        if code == OK {
            let got = String::from_utf8(bytes(out, len).to_vec()).unwrap();
            (api.free)(out, len);
            return (code, got);
        }
        let mut buf = vec![0 as c_char; 256];
        (api.problem)(buf.as_mut_ptr(), buf.len());
        (code, unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
    }

    #[test]
    fn the_table_reads_fits_and_says_why_not() {
        let kind = r#"{"kind":"record","entity":"Order","fields":[{"name":"total","kind":"decimal"}],"base":{"id":0,"total":"0"}}"#;
        assert_eq!(call("json", r#"{"Total": 12.50}"#, "", kind), (OK, r#"{"id":0,"total":"12.50"}"#.into()));
        assert_eq!(call("json", r#"{"total": true}"#, "", kind), (NOT_FITTING, "field 'total' of Order is a decimal, not true".into()));
        assert_eq!(call("toml", "x", "", kind).0, UNKNOWN_FORMAT);
        assert_eq!(call("json", "{}", r#"{"sep":","}"#, kind).0, BAD_OPTIONS);
        assert_eq!(call("json", "{}", "", "{}").0, FAILED);
    }

    #[test]
    fn the_table_is_the_same_for_the_library_s_whole_life() {
        assert_eq!(unsafe { (*clarity_formats_api()).interface }, INTERFACE);
        assert!(std::ptr::eq(clarity_formats_api(), clarity_formats_api()));
    }
}

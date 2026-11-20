//! The C interface of Clarity.Pdf (include/clarity_pdf.h): one table that
//! makes a PDF. No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use pdf::Problem;

pub const INTERFACE: u32 = 2;

pub const OK: i32 = 0;
pub const STYLESHEET: i32 = 1;
pub const THEME: i32 = 2;
pub const LAYOUT: i32 = 3;
pub const FAILED: i32 = 4;
pub const REPORT: i32 = 5;

type Out = *mut *mut u8;

/// The table of include/clarity_pdf.h, field for field.
#[repr(C)]
pub struct ClarityPdfApi {
    pub interface: u32,
    pub pdf: extern "C" fn(*const c_char, usize, *const c_char, usize, *const c_char, usize, Out, *mut usize) -> i32,
    pub free: extern "C" fn(*mut u8, usize),
    pub problem: extern "C" fn(*mut c_char, usize) -> i32,
    /// Interface 2.
    pub report: extern "C" fn(*const c_char, usize, *const c_char, usize, *const c_char, usize, *const c_char, usize, *const c_char, usize, Out, *mut usize) -> i32,
}

static API: ClarityPdfApi = ClarityPdfApi { interface: INTERFACE, pdf: make, free, problem, report };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_pdf_api() -> *const ClarityPdfApi {
    &API
}

thread_local! {
    static LAST: RefCell<(i32, String)> = const { RefCell::new((OK, String::new())) };
}

fn keep(code: i32, detail: &str) -> i32 {
    LAST.with(|l| *l.borrow_mut() = (code, detail.to_string()));
    code
}

fn text<'a>(at: *const c_char, len: usize) -> Result<&'a str, i32> {
    let bytes = if at.is_null() || len == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(at.cast::<u8>(), len) } };
    std::str::from_utf8(bytes).map_err(|_| keep(FAILED, "a text given to Clarity.Pdf is no UTF-8"))
}

#[allow(clippy::too_many_arguments)]
extern "C" fn make(t: *const c_char, tn: usize, s: *const c_char, sn: usize, th: *const c_char, thn: usize, out: Out, out_len: *mut usize) -> i32 {
    let (markdown, sheet, theme) = match (text(t, tn), text(s, sn), text(th, thn)) {
        (Ok(markdown), Ok(sheet), Ok(theme)) => (markdown, sheet, theme),
        _ => return FAILED,
    };
    giving(out, out_len, || pdf::made(markdown, sheet, theme))
}

#[allow(clippy::too_many_arguments)]
extern "C" fn report(r: *const c_char, rn: usize, d: *const c_char, dn: usize, t: *const c_char, tn: usize, s: *const c_char, sn: usize, th: *const c_char, thn: usize, out: Out, out_len: *mut usize) -> i32 {
    let (report, data, texts, sheet, theme) = match (text(r, rn), text(d, dn), text(t, tn), text(s, sn), text(th, thn)) {
        (Ok(report), Ok(data), Ok(texts), Ok(sheet), Ok(theme)) => (report, data, texts, sheet, theme),
        _ => return FAILED,
    };
    giving(out, out_len, || pdf::report(report, data, texts, sheet, theme))
}

/// Runs what makes a PDF: the file goes to `out`, or the problem is kept and its code given.
fn giving(out: Out, out_len: *mut usize, act: impl FnOnce() -> Result<Vec<u8>, Problem>) -> i32 {
    let made = match catch_unwind(AssertUnwindSafe(act)) {
        Ok(Ok(made)) => made,
        Ok(Err(problem)) => {
            let code = match problem {
                Problem::Stylesheet(_) => STYLESHEET,
                Problem::Theme(_) => THEME,
                Problem::Layout(_) => LAYOUT,
                Problem::Report(_) => REPORT,
            };
            return keep(code, &problem.to_string());
        }
        Err(_) => return keep(FAILED, "a fault inside Clarity.Pdf"),
    };
    if out.is_null() || out_len.is_null() {
        return keep(FAILED, "nowhere to give the PDF");
    }
    let boxed = made.into_boxed_slice();
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
        // Cut on a character's edge, so what is copied stays UTF-8.
        let mut n = detail.len().min(size - 1);
        while !detail.is_char_boundary(n) {
            n -= 1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(detail.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
    code
}

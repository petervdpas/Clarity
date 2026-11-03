//! setjmp and longjmp for Skia's prebuilt binaries. Their PNG and JPEG
//! decoders leave a picture that does not decode with longjmp, built for
//! Emscripten's JavaScript way of it: each call that may jump goes through an
//! `invoke_<signature>` function, and `emscripten_longjmp` jumps. Rust
//! programs link with WebAssembly exceptions, with which Emscripten offers
//! neither, so they are here: the jump unwinds as a Rust panic of its own
//! (`LongJump`) to the `invoke_` function below the setjmp, which takes it
//! and returns, leaving the jump in `__THREW__` for the caller to follow, as
//! Emscripten's own JavaScript does. The other panics go on unwinding.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

unsafe extern "C" {
    /// Emscripten's: keeps where a jump goes and its value until the caller looks.
    fn setThrew(threw: usize, value: i32);
}

/// A jump on its way to its setjmp.
struct LongJump;

/// Jumps to `env` with `value` (never 0, as C says).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn emscripten_longjmp(env: usize, value: i32) -> ! {
    unsafe { setThrew(env, if value == 0 { 1 } else { value }) };
    resume_unwind(Box::new(LongJump))
}

/// Runs a call that may jump: a jump ends it, said through `__THREW__`.
fn invoked<R: Default>(call: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(done) => done,
        Err(payload) if payload.is::<LongJump>() => {
            unsafe { setThrew(1, 0) };
            R::default()
        }
        Err(payload) => resume_unwind(payload),
    }
}

/// `invoke_<signature>`: `v` returns nothing, `i` is a 32-bit number or pointer.
macro_rules! invoke {
    ($name:ident, ($($arg:ident),*)) => {
        #[unsafe(no_mangle)]
        pub extern "C-unwind" fn $name(f: extern "C-unwind" fn($($arg: i32),*), $($arg: i32),*) {
            invoked(|| f($($arg),*))
        }
    };
    ($name:ident, ($($arg:ident),*) -> i32) => {
        #[unsafe(no_mangle)]
        pub extern "C-unwind" fn $name(f: extern "C-unwind" fn($($arg: i32),*) -> i32, $($arg: i32),*) -> i32 {
            invoked(|| f($($arg),*))
        }
    };
}

invoke!(invoke_v, ());
invoke!(invoke_vi, (a));
invoke!(invoke_vii, (a, b));
invoke!(invoke_viii, (a, b, c));
invoke!(invoke_viiii, (a, b, c, d));
invoke!(invoke_viiiii, (a, b, c, d, e));
invoke!(invoke_viiiiii, (a, b, c, d, e, f2));
invoke!(invoke_viiiiiiiii, (a, b, c, d, e, f2, g, h, k));
invoke!(invoke_ii, (a) -> i32);
invoke!(invoke_iii, (a, b) -> i32);
invoke!(invoke_iiii, (a, b, c) -> i32);
invoke!(invoke_iiiii, (a, b, c, d) -> i32);
invoke!(invoke_iiiiiii, (a, b, c, d, e, f2) -> i32);
invoke!(invoke_iiiiiiii, (a, b, c, d, e, f2, g) -> i32);
invoke!(invoke_iiiiiiiiii, (a, b, c, d, e, f2, g, h, k) -> i32);

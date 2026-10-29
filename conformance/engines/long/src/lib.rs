//! The engine every host is tested with for progress and Stop
//! (conformance/progress.json): a long call that says how far it is, which
//! the screen shows as a bar, and which the person can stop.

use serde_json::{Value, json};

// The one service it uses (`uses progress`).
#[link(wasm_import_module = "clarity")]
unsafe extern "C" {
    /// How far the call is: `done` of `total`.
    fn progress(done: i64, total: i64);
}

/// Room in the engine's memory for the host to write into.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_alloc(len: i32) -> i32 {
    let mut room = Vec::<u8>::with_capacity(len.max(0) as usize);
    let at = room.as_mut_ptr();
    std::mem::forget(room);
    at as i32
}

/// Gives back room the engine handed out, once the host has read it.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_free(at: i32, len: i32) {
    if at != 0 {
        unsafe { drop(Vec::from_raw_parts(at as *mut u8, 0, len.max(0) as usize)) };
    }
}

/// Runs `function` on the arguments at `at`, which it takes over, and hands
/// its answer to the host.
fn answer(at: i32, len: i32, function: fn(&[Value]) -> Result<Value, String>) -> i64 {
    let bytes = unsafe { Vec::from_raw_parts(at as *mut u8, len as usize, len as usize) };
    let reply = match serde_json::from_slice::<Vec<Value>>(&bytes) {
        Ok(args) => match function(&args) {
            Ok(value) => json!({ "value": value }),
            Err(error) => json!({ "error": error }),
        },
        Err(error) => json!({ "error": format!("the arguments are no JSON list: {error}") }),
    };
    let mut out = serde_json::to_vec(&reply).unwrap_or_default().into_boxed_slice();
    let (place, size) = (out.as_mut_ptr() as i64, out.len() as i64);
    std::mem::forget(out);
    (place << 32) | size
}

/// work(steps: integer) -> integer: a step at a time, saying after each how
/// far it is; gives the number of steps done.
#[unsafe(no_mangle)]
pub extern "C" fn work(at: i32, len: i32) -> i64 {
    answer(at, len, |args| {
        let steps = args.first().and_then(Value::as_i64).ok_or("steps is no integer")?;
        let mut sum: u64 = 0;
        for step in 1..=steps {
            // A little work per step, so a long call takes its time.
            for turn in 0..1000u64 {
                sum = std::hint::black_box(sum.wrapping_add(turn));
            }
            unsafe { progress(step, steps) };
        }
        Ok(json!(steps))
    })
}

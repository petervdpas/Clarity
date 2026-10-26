//! The engine beside the screen every renderer is tested with
//! (conformance/screen-engines.json). It speaks Clarity's engine interface
//! (DESIGN 5.11) and its frame loop (DESIGN 5.12): at every frame the
//! renderer calls `clarity_tick` with the milliseconds since the last
//! frame, then `clarity_draw`, which answers the canvas's shapes.
//!
//! It keeps the time it has run. Its drawing is a dot that moves 1 unit
//! every 10 ms across a canvas of 160 units, and that time as text. A tick
//! of less than no time runs forever, for the renderers' time limit; a
//! tick of more than ten seconds is refused.
//!
//! Pointer and keys: the dot goes where the pointer goes down and follows
//! it while it is down, and the text says which keys are down.

use std::cell::{Cell, RefCell};

use serde_json::{Value, json};

thread_local! {
    static ELAPSED: Cell<i64> = const { Cell::new(0) };
    static OPENED: Cell<bool> = const { Cell::new(false) };
    /// Where the pointer put the dot, once it went down.
    static DOT: Cell<Option<(i64, i64)>> = const { Cell::new(None) };
    static PRESSED: Cell<bool> = const { Cell::new(false) };
    static HELD: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
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

/// `on open`: a new instance starts at no time, opened.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_open(at: i32, len: i32) -> i64 {
    answer(at, len, |_| {
        ELAPSED.set(0);
        OPENED.set(true);
        Ok(Value::Null)
    })
}

/// One frame's time, in milliseconds since the last frame.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_tick(at: i32, len: i32) -> i64 {
    answer(at, len, |args| {
        let ms = args.first().and_then(Value::as_f64).ok_or("a tick gives the milliseconds since the last frame")?;
        if ms < 0.0 {
            // Runs until the renderer stops it.
            let mut spin = 0u64;
            loop {
                spin = std::hint::black_box(spin.wrapping_add(1));
            }
        }
        if ms > 10_000.0 {
            return Err("a frame of more than ten seconds".into());
        }
        ELAPSED.set(ELAPSED.get() + ms as i64);
        Ok(Value::Null)
    })
}

/// The pointer on the canvas: `[kind, x, y]` in its units.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_pointer(at: i32, len: i32) -> i64 {
    answer(at, len, |args| {
        let kind = args.first().and_then(Value::as_str).ok_or("a pointer gives its kind first")?;
        let x = args.get(1).and_then(Value::as_i64).ok_or("a pointer gives x")?;
        let y = args.get(2).and_then(Value::as_i64).ok_or("a pointer gives y")?;
        match kind {
            "down" => PRESSED.set(true),
            "move" if !PRESSED.get() => return Ok(Value::Null),
            "move" => {}
            "up" if !PRESSED.get() => return Ok(Value::Null),
            "up" => PRESSED.set(false),
            other => return Err(format!("no pointer kind {other}")),
        }
        DOT.set(Some((x, y)));
        Ok(Value::Null)
    })
}

/// A key on the canvas: `[kind, key]`.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_key(at: i32, len: i32) -> i64 {
    answer(at, len, |args| {
        let kind = args.first().and_then(Value::as_str).ok_or("a key gives its kind first")?;
        let key = args.get(1).and_then(Value::as_str).ok_or("a key gives its name")?.to_string();
        HELD.with_borrow_mut(|held| match kind {
            "down" => {
                if !held.contains(&key) {
                    held.push(key);
                }
                Ok(Value::Null)
            }
            "up" => {
                held.retain(|k| *k != key);
                Ok(Value::Null)
            }
            other => Err(format!("no key kind {other}")),
        })
    })
}

/// The canvas's shapes now, in its units (160 x 100).
#[unsafe(no_mangle)]
pub extern "C" fn clarity_draw(at: i32, len: i32) -> i64 {
    answer(at, len, |_| {
        let elapsed = ELAPSED.get();
        let mut said = if OPENED.get() { format!("{elapsed} ms") } else { format!("{elapsed} ms, not opened") };
        HELD.with_borrow(|held| {
            if !held.is_empty() {
                said = format!("{said}, holding {}", held.join(" "));
            }
        });
        let (x, y) = DOT.get().unwrap_or(((elapsed / 10) % 160, 50));
        Ok(json!([
            { "op": "clear", "color": "background" },
            { "op": "circle", "x": x, "y": y, "r": 5, "color": "accent" },
            { "op": "text", "x": 80, "y": 90, "text": said, "color": "text", "size": 8 },
        ]))
    })
}

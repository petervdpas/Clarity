//! Custom parts of the program as a table of callbacks (DESIGN 5.19, 7.9):
//! the caller registers a kind with its functions, and each `<Custom
//! part="Gauge">` of a window gets a part of its own made by `make`. What
//! the part says to its window (a change, a tell, a wake) goes back through
//! the `ClarityCustomCx` it is given; it draws by commands on the canvas.

use std::ffi::{c_char, c_void};
use std::sync::Arc;
use std::time::Duration;

use ui::geometry::Rect;
use ui::input::{Key, KeyInput, Modifiers, PointerInput};
use ui::painter::Painter;
use ui::parts::custom::{CustomCx, CustomPart};
use serde_json::Value;

use crate::canvas::ClarityCanvas;
use crate::screen::{bits, key_name};
use crate::{guard, handle, handle_mut, put, text, ClarityAnswer, ClarityStatus, Failure};

type CText = *const c_char;
type Cx = *mut ClarityCustomCx;

/// What a custom part reaches of its window during a callback (a `CustomCx`).
pub struct ClarityCustomCx {
    _private: [u8; 0],
}

/// Where the pointer is, in the screen's units, with the keys held and the clicks.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClarityPointer {
    pub x: f64,
    pub y: f64,
    pub modifiers: u32,
    pub clicks: u32,
}

/// A kind of custom part as the caller gives it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClarityCustomCallbacks {
    pub user: *mut c_void,
    pub make: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    pub set: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, Cx)>,
    pub hear: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, Cx)>,
    pub draw: Option<unsafe extern "C" fn(*mut c_void, *mut ClarityCanvas, f64, f64, f64, f64)>,
    pub start: Option<unsafe extern "C" fn(*mut c_void, Cx)>,
    pub focusable: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    pub pointer_pressed: Option<unsafe extern "C" fn(*mut c_void, *const ClarityPointer, Cx)>,
    pub pointer_moved: Option<unsafe extern "C" fn(*mut c_void, *const ClarityPointer, Cx)>,
    pub pointer_released: Option<unsafe extern "C" fn(*mut c_void, *const ClarityPointer, Cx)>,
    pub click: Option<unsafe extern "C" fn(*mut c_void, Cx)>,
    pub wheel: Option<unsafe extern "C" fn(*mut c_void, f64, f64, u32, Cx) -> i32>,
    pub key: Option<unsafe extern "C" fn(*mut c_void, CText, usize, u32, Cx) -> i32>,
    pub text: Option<unsafe extern "C" fn(*mut c_void, CText, usize, Cx)>,
    pub focus_changed: Option<unsafe extern "C" fn(*mut c_void, i32, Cx)>,
    pub wake: Option<unsafe extern "C" fn(*mut c_void, Cx)>,
    pub stop: Option<unsafe extern "C" fn(*mut c_void)>,
    pub tooltip_at: Option<unsafe extern "C" fn(*mut c_void, f64, f64, *mut ClarityAnswer)>,
    pub destroy: Option<unsafe extern "C" fn(*mut c_void)>,
}

/// A kind's table, shared by its parts.
struct Kind(ClarityCustomCallbacks);

// The header keeps every callback on the screen's thread; `make` is called there too.
unsafe impl Send for Kind {}
unsafe impl Sync for Kind {}

/// A part of a kind the caller registered.
pub struct CallbackPart {
    kind: Arc<Kind>,
    part: *mut c_void,
}

impl Drop for CallbackPart {
    fn drop(&mut self) {
        if let Some(destroy) = self.kind.0.destroy {
            crate::quietly(|| unsafe { destroy(self.part) });
        }
    }
}

fn cx_of(cx: &mut CustomCx) -> Cx {
    cx as *mut CustomCx as *mut ClarityCustomCx
}

fn pointer_of(at: PointerInput) -> ClarityPointer {
    ClarityPointer { x: at.x, y: at.y, modifiers: bits(at.modifiers), clicks: at.clicks }
}

/// A key's name as a custom part gets it: the header's names, else the host's own or the character typed.
fn key_input_name(key: &KeyInput) -> String {
    match key.key {
        Key::Other => key.name.map(str::to_string).or_else(|| key.typed.map(String::from)).unwrap_or_default(),
        other => key_name(other),
    }
}

impl CallbackPart {
    fn calls(&self) -> &ClarityCustomCallbacks {
        &self.kind.0
    }

    fn pointer(&self, f: Option<unsafe extern "C" fn(*mut c_void, *const ClarityPointer, Cx)>, at: PointerInput, cx: &mut CustomCx) {
        if let Some(f) = f {
            let at = pointer_of(at);
            unsafe { f(self.part, &at, cx_of(cx)) };
        }
    }
}

impl CustomPart for CallbackPart {
    fn set(&mut self, name: &str, value: &Value, cx: &mut CustomCx) {
        if let Some(f) = self.calls().set {
            let value = value.to_string();
            unsafe { f(self.part, name.as_ptr() as CText, name.len(), value.as_ptr() as CText, value.len(), cx_of(cx)) };
        }
    }

    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut CustomCx) {
        if let Some(f) = self.calls().hear {
            let values = Value::Array(values.to_vec()).to_string();
            unsafe { f(self.part, hear.as_ptr() as CText, hear.len(), values.as_ptr() as CText, values.len(), cx_of(cx)) };
        }
    }

    fn draw(&self, painter: &Painter, area: Rect) {
        if let Some(f) = self.calls().draw {
            let mut canvas = ClarityCanvas::new(painter);
            unsafe { f(self.part, &mut canvas, area.x, area.y, area.width, area.height) };
            canvas.restore_all();
        }
    }

    fn start(&mut self, cx: &mut CustomCx) {
        if let Some(f) = self.calls().start {
            unsafe { f(self.part, cx_of(cx)) };
        }
    }

    fn focusable(&self) -> bool {
        self.calls().focusable.is_some_and(|f| unsafe { f(self.part) } != 0)
    }

    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.pointer(self.calls().pointer_pressed, at, cx);
    }

    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.pointer(self.calls().pointer_moved, at, cx);
    }

    fn pointer_released(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.pointer(self.calls().pointer_released, at, cx);
    }

    fn click(&mut self, cx: &mut CustomCx) {
        if let Some(f) = self.calls().click {
            unsafe { f(self.part, cx_of(cx)) };
        }
    }

    fn wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {
        self.calls().wheel.is_some_and(|f| unsafe { f(self.part, dx, dy, bits(held), cx_of(cx)) } != 0)
    }

    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {
        let Some(f) = self.calls().key else { return false };
        let name = key_input_name(&key);
        unsafe { f(self.part, name.as_ptr() as CText, name.len(), bits(key.modifiers), cx_of(cx)) != 0 }
    }

    fn text(&mut self, text: &str, cx: &mut CustomCx) {
        if let Some(f) = self.calls().text {
            unsafe { f(self.part, text.as_ptr() as CText, text.len(), cx_of(cx)) };
        }
    }

    fn focus_changed(&mut self, focused: bool, cx: &mut CustomCx) {
        if let Some(f) = self.calls().focus_changed {
            unsafe { f(self.part, focused as i32, cx_of(cx)) };
        }
    }

    fn wake(&mut self, cx: &mut CustomCx) {
        if let Some(f) = self.calls().wake {
            unsafe { f(self.part, cx_of(cx)) };
        }
    }

    fn stop(&mut self) {
        if let Some(f) = self.calls().stop {
            unsafe { f(self.part) };
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let f = self.calls().tooltip_at?;
        let mut answer = ClarityAnswer::default();
        unsafe { f(self.part, x, y, &mut answer) };
        answer.text().filter(|t| !t.is_empty())
    }
}

/// Adds a custom part of `kind`.
///
/// # Safety
/// `kind` is (pointer, length); `callbacks` points at a filled table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_register(kind: *const c_char, kind_len: usize, callbacks: *const ClarityCustomCallbacks) -> ClarityStatus {
    guard(|| {
        let name = unsafe { text(kind, kind_len) }?;
        if name.is_empty() {
            return Err(Failure::invalid("the kind has no name"));
        }
        let callbacks = *unsafe { handle(callbacks) }?;
        for (field, given) in [("make", callbacks.make.is_some()), ("set", callbacks.set.is_some()), ("hear", callbacks.hear.is_some()), ("draw", callbacks.draw.is_some())] {
            if !given {
                return Err(Failure::invalid(format!("the custom part has no {field}")));
            }
        }
        let kind = Arc::new(Kind(callbacks));
        ui::parts::custom::register(name, move || {
            let make = kind.0.make.expect("a custom part's make, checked as it was registered");
            let part = unsafe { make(kind.0.user) };
            if part.is_null() {
                panic!("the custom part's make gave nothing");
            }
            Box::new(CallbackPart { kind: kind.clone(), part }) as Box<dyn CustomPart>
        });
        Ok(())
    })
}

/// The cx behind a pointer.
unsafe fn cx<'a>(cx: *mut ClarityCustomCx) -> Result<&'a mut CustomCx<'a>, Failure> {
    unsafe { handle_mut(cx as *mut CustomCx<'a>) }
}

/// # Safety
/// `cx` is the cx of a running callback; the out pointers are writable or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_bounds(cx_: *const ClarityCustomCx, x: *mut f64, y: *mut f64, width: *mut f64, height: *mut f64) -> ClarityStatus {
    guard(|| {
        let b = unsafe { cx(cx_ as *mut ClarityCustomCx) }?.bounds();
        unsafe {
            crate::put_maybe(x, b.x);
            crate::put_maybe(y, b.y);
            crate::put_maybe(width, b.width);
            crate::put_maybe(height, b.height);
        }
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback; `enabled` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_enabled(cx_: *const ClarityCustomCx, enabled: *mut i32) -> ClarityStatus {
    guard(|| {
        let on = unsafe { cx(cx_ as *mut ClarityCustomCx) }?.enabled();
        unsafe { put(enabled, on as i32) }
    })
}

/// # Safety
/// `cx` is the cx of a running callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_invalidate(cx_: *mut ClarityCustomCx) -> ClarityStatus {
    guard(|| {
        unsafe { cx(cx_) }?.invalidate();
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_wake_in(cx_: *mut ClarityCustomCx, ms: f64) -> ClarityStatus {
    guard(|| {
        if !ms.is_finite() || ms < 0.0 {
            return Err(Failure::invalid("the time is no time"));
        }
        let cx = unsafe { cx(cx_) }?;
        let at = cx.now() + Duration::from_secs_f64(ms / 1000.0);
        cx.wake(at);
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_animate(cx_: *mut ClarityCustomCx, on: i32) -> ClarityStatus {
    guard(|| {
        unsafe { cx(cx_) }?.animate(on != 0);
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_focus(cx_: *mut ClarityCustomCx) -> ClarityStatus {
    guard(|| {
        unsafe { cx(cx_) }?.focus();
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback; `value` is (pointer, length) of JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_change(cx_: *mut ClarityCustomCx, value: *const c_char, value_len: usize) -> ClarityStatus {
    guard(|| {
        let value = unsafe { crate::json(value, value_len) }?;
        unsafe { cx(cx_) }?.change(value);
        Ok(())
    })
}

/// # Safety
/// `cx` is the cx of a running callback; the texts are (pointer, length), `values` a JSON array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_custom_tell(cx_: *mut ClarityCustomCx, tell: *const c_char, tell_len: usize, values: *const c_char, values_len: usize) -> ClarityStatus {
    guard(|| {
        let tell = unsafe { text(tell, tell_len) }?;
        let values = match unsafe { crate::text_or_none(values, values_len) }? {
            None => Vec::new(),
            Some(written) => match serde_json::from_str::<Value>(written) {
                Ok(Value::Array(values)) => values,
                _ => return Err(Failure::invalid("the values are no JSON array")),
            },
        };
        unsafe { cx(cx_) }?.tell(tell, values);
        Ok(())
    })
}

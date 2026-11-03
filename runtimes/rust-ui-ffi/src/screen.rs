//! A screen without a window: a window made from its markup with the
//! caller's part host behind its parts, driven as a person uses it and
//! drawn into the caller's pixels. The C# layer's tests and any host of its
//! own drive it. It lives on the thread that made it.

use std::ffi::c_char;
use std::rc::Rc;

use ui::headless::{key_named, Driven};
use ui::input::{Key, KeyInput, Modifiers, PointerButton};
use ui::parts::PartHost;
use ui::theme::ThemeLook;
use ui::window::view::WindowView;
use ui::Screen;
use serde_json::{json, Value};

use crate::host::{CallbackHost, ClarityPartHostCallbacks};
use crate::{guard, handle_mut, put, put_maybe, text, text_or_none, ClarityStatus, Failure};

/// The size a screen starts at.
const START_SIZE: (f64, f64) = (1280.0, 800.0);

/// A window on a screen of its own, and its driver.
pub struct ClarityScreen {
    driven: Driven,
    /// The thread that made it: every call comes from there.
    thread: std::thread::ThreadId,
}

impl ClarityScreen {
    fn screen(&mut self) -> &mut Screen {
        self.driven.screen()
    }

    /// Does a step of the driver; its error as a failure.
    pub fn step(&mut self, step: Value) -> Result<Value, Failure> {
        let answer = self.driven.step(&step);
        match answer.get("error").and_then(Value::as_str) {
            Some(why) if why.starts_with("no part") => Err(Failure::not_found(why)),
            Some(why) => Err(Failure::failed(why)),
            None => Ok(answer),
        }
    }
}

/// The screen behind a handle, on its own thread.
unsafe fn screen<'a>(screen: *mut ClarityScreen) -> Result<&'a mut ClarityScreen, Failure> {
    let screen = unsafe { handle_mut(screen) }?;
    if screen.thread != std::thread::current().id() {
        return Err(Failure::invalid("the screen is used from another thread than the one that made it"));
    }
    Ok(screen)
}

/// The keys held, from the header's bits.
pub fn modifiers(bits: u32) -> Modifiers {
    let mut held = Modifiers::NONE;
    for (bit, key) in [(1, Modifiers::SHIFT), (2, Modifiers::CONTROL), (4, Modifiers::ALT), (8, Modifiers::META)] {
        if bits & bit != 0 {
            held |= key;
        }
    }
    held
}

/// The header's bits of the keys held.
pub fn bits(held: Modifiers) -> u32 {
    [(1, Modifiers::SHIFT), (2, Modifiers::CONTROL), (4, Modifiers::ALT), (8, Modifiers::META)].into_iter().filter(|(_, k)| held.has(*k)).map(|(b, _)| b).sum()
}

fn button(button: i32) -> Result<PointerButton, Failure> {
    match button {
        0 => Ok(PointerButton::Left),
        1 => Ok(PointerButton::Middle),
        2 => Ok(PointerButton::Right),
        other => Err(Failure::invalid(format!("there is no pointer button {other}"))),
    }
}

/// A key by its name, as the header names keys; Shift, Control and Alt by theirs too.
pub fn key(name: &str) -> Result<Key, Failure> {
    match name {
        "Shift" => Ok(Key::Shift),
        "Control" => Ok(Key::Control),
        "Alt" => Ok(Key::Alt),
        _ => key_named(name).ok_or_else(|| Failure::invalid(format!("there is no key {name}"))),
    }
}

/// The name of a key, as the header names keys: `Enter`, `A`, `7`, `F2`.
pub fn key_name(key: Key) -> String {
    if let Some(i) = Key::LETTERS.iter().position(|k| *k == key) {
        return ((b'A' + i as u8) as char).to_string();
    }
    if let Some(i) = Key::DIGITS.iter().position(|k| *k == key) {
        return ((b'0' + i as u8) as char).to_string();
    }
    match key {
        Key::Plus => "+".to_string(),
        Key::Minus => "-".to_string(),
        other => format!("{other:?}"),
    }
}

fn place(value: &Value) -> Option<(f64, f64, f64, f64)> {
    let at = value.as_array()?;
    Some((at.first()?.as_f64()?, at.get(1)?.as_f64()?, at.get(2)?.as_f64()?, at.get(3)?.as_f64()?))
}

/// A window from its markup over the caller's host.
///
/// # Safety
/// The texts are (pointer, length) pairs (`app` may be NULL); `host` points at a filled table; `out` at writable memory for a handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_new(markup: *const c_char, markup_len: usize, app: *const c_char, app_len: usize, host: *const ClarityPartHostCallbacks, out: *mut *mut ClarityScreen) -> ClarityStatus {
    guard(|| {
        let callbacks = *unsafe { crate::handle(host) }?;
        if let Some(name) = crate::host::missing(&callbacks) {
            return Err(Failure::invalid(format!("the part host has no {name}")));
        }
        let markup = unsafe { text(markup, markup_len) }?;
        let app = unsafe { text_or_none(app, app_len) }?;
        let spec = ui::markup::read(markup).map_err(|e| Failure::invalid(format!("the window's markup does not read: {}", e.0)))?;
        let frame = match app {
            Some(app) => Some(ui::markup::read_app(app).map_err(|e| Failure::invalid(format!("the app's markup does not read: {}", e.0)))?),
            None => None,
        };
        let host: Rc<dyn PartHost> = Rc::new(CallbackHost::new(callbacks));
        let view = WindowView::new(&spec, host).with_frame(frame);
        let mut screen = Screen::new(Box::new(view), ThemeLook::clarity(false));
        screen.resize(START_SIZE.0, START_SIZE.1, 1.0);
        let made = ClarityScreen { driven: Driven::over(screen), thread: std::thread::current().id() };
        unsafe { put(out, Box::into_raw(Box::new(made))) }
    })
}

/// Frees a screen: its parts stop and its host is let go.
///
/// # Safety
/// `screen` is NULL or a handle from `clarity_screen_new`, not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_free(screen: *mut ClarityScreen) {
    if !screen.is_null() {
        crate::quietly(|| {
            let mut made = unsafe { Box::from_raw(screen) };
            made.screen().dispose();
        });
    }
}

/// Runs an act on a screen.
unsafe fn act(handle: *mut ClarityScreen, run: impl FnOnce(&mut ClarityScreen) -> crate::Done) -> ClarityStatus {
    guard(|| run(unsafe { screen(handle) }?))
}

/// # Safety
/// `screen` is a live handle; `theme` is (pointer, length) or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_look(screen: *mut ClarityScreen, theme: *const c_char, theme_len: usize, dark: i32) -> ClarityStatus {
    let theme = unsafe { text_or_none(theme, theme_len) };
    unsafe {
        act(screen, |s| {
            let clarity = ThemeLook::clarity(dark != 0);
            let look = match theme? {
                Some(own) => ThemeLook::over(own, clarity),
                None => clarity,
            };
            s.screen().show(look);
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_resize(screen: *mut ClarityScreen, width: f64, height: f64, scale: f32) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            if !(width.is_finite() && height.is_finite() && scale.is_finite()) || width <= 0.0 || height <= 0.0 || scale <= 0.0 {
                return Err(Failure::invalid("the size is no size"));
            }
            s.screen().resize(width, height, scale);
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_pointer_move(screen: *mut ClarityScreen, x: f64, y: f64) -> ClarityStatus {
    unsafe { act(screen, |s| Ok(s.screen().pointer_moved(x, y))) }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_pointer_leave(screen: *mut ClarityScreen) -> ClarityStatus {
    unsafe { act(screen, |s| Ok(s.screen().pointer_left())) }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_pointer_press(screen: *mut ClarityScreen, x: f64, y: f64, which: i32, clicks: u32, held: u32) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            let which = button(which)?;
            s.screen().pointer_pressed_with(x, y, which, clicks.max(1), modifiers(held));
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_pointer_release(screen: *mut ClarityScreen, x: f64, y: f64, which: i32) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            let which = button(which)?;
            s.screen().pointer_released(x, y, which);
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_click(screen: *mut ClarityScreen, x: f64, y: f64) -> ClarityStatus {
    unsafe { act(screen, |s| Ok(s.screen().click(x, y))) }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_wheel(screen: *mut ClarityScreen, x: f64, y: f64, dx: f64, dy: f64, held: u32) -> ClarityStatus {
    unsafe { act(screen, |s| Ok(s.screen().wheel(x, y, dx, dy, modifiers(held)))) }
}

/// # Safety
/// `screen` is a live handle; `name` is (pointer, length); `taken` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_key(screen: *mut ClarityScreen, name: *const c_char, name_len: usize, held: u32, taken: *mut i32) -> ClarityStatus {
    let name = unsafe { text(name, name_len) };
    unsafe {
        act(screen, |s| {
            let key = key(name?)?;
            let took = s.screen().key_pressed(KeyInput::new(key, modifiers(held)));
            put_maybe(taken, took as i32);
            Ok(())
        })
    }
}

/// # Safety
/// As `clarity_screen_key`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_key_release(screen: *mut ClarityScreen, name: *const c_char, name_len: usize, held: u32) -> ClarityStatus {
    let name = unsafe { text(name, name_len) };
    unsafe {
        act(screen, |s| {
            let key = key(name?)?;
            s.screen().key_released(KeyInput::new(key, modifiers(held)));
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle; `typed` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_type(screen: *mut ClarityScreen, typed: *const c_char, typed_len: usize) -> ClarityStatus {
    let typed = unsafe { text(typed, typed_len) };
    unsafe {
        act(screen, |s| {
            s.screen().text_typed(typed?);
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_settle(screen: *mut ClarityScreen, ms: f64) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            if !ms.is_finite() || ms < 0.0 {
                return Err(Failure::invalid("the time is no time"));
            }
            s.step(json!({ "do": "settle", "ms": ms }))?;
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_changed(screen: *mut ClarityScreen) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            s.screen().invalidate();
            s.step(json!({ "do": "settle", "ms": 0 }))?;
            Ok(())
        })
    }
}

/// Draws the screen into the caller's pixels.
///
/// # Safety
/// `screen` is a live handle; `rgba` points at `stride * height` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_draw(screen: *mut ClarityScreen, rgba: *mut u8, width: i32, height: i32, stride: usize) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            if rgba.is_null() || width <= 0 || height <= 0 || stride < width as usize * 4 {
                return Err(Failure::invalid("the pixels are no picture of that size"));
            }
            let pixels = std::slice::from_raw_parts_mut(rgba, stride * height as usize);
            let info = ui::skia_safe::ImageInfo::new((width, height), ui::skia_safe::ColorType::RGBA8888, ui::skia_safe::AlphaType::Premul, None);
            let mut surface = ui::skia_safe::surfaces::wrap_pixels(&info, pixels, stride, None).ok_or_else(|| Failure::failed("Skia could not draw into the pixels"))?;
            s.screen().draw(surface.canvas());
            Ok(())
        })
    }
}

/// The place of a part by its name.
///
/// # Safety
/// `screen` is a live handle; `name` is (pointer, length); the out pointers are writable or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_part_place(screen: *mut ClarityScreen, name: *const c_char, name_len: usize, x: *mut f64, y: *mut f64, width: *mut f64, height: *mut f64) -> ClarityStatus {
    let name = unsafe { text(name, name_len) };
    unsafe {
        act(screen, |s| {
            let name = name?;
            let read = s.step(json!({ "do": "read" }))?;
            let (px, py, pw, ph) = read.get("shown").and_then(|shown| shown.get(name)).and_then(place).ok_or_else(|| Failure::not_found(format!("no part {name} shows")))?;
            put_maybe(x, px);
            put_maybe(y, py);
            put_maybe(width, pw);
            put_maybe(height, ph);
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle; `name` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_focus(screen: *mut ClarityScreen, name: *const c_char, name_len: usize) -> ClarityStatus {
    let name = unsafe { text(name, name_len) };
    unsafe {
        act(screen, |s| {
            s.step(json!({ "do": "focus", "part": name? }))?;
            Ok(())
        })
    }
}

/// # Safety
/// `screen` is a live handle; `out` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_focused(screen: *mut ClarityScreen, out: *mut *mut c_char) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            let read = s.step(json!({ "do": "read" }))?;
            let focused = read.get("focused").and_then(Value::as_str).map(crate::give).unwrap_or(std::ptr::null_mut());
            put(out, focused)
        })
    }
}

/// # Safety
/// `screen` is a live handle; `count` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_overlays(screen: *mut ClarityScreen, count: *mut i32) -> ClarityStatus {
    unsafe { act(screen, |s| put(count, s.screen().overlays().len() as i32)) }
}

/// # Safety
/// `screen` is a live handle; `out` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_parts_json(screen: *mut ClarityScreen, out: *mut *mut c_char) -> ClarityStatus {
    unsafe {
        act(screen, |s| {
            let read = s.step(json!({ "do": "read" }))?;
            let parts: Vec<Value> = read
                .get("shown")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(name, at)| place(at).map(|(x, y, w, h)| json!({ "name": name, "x": x, "y": y, "width": w, "height": h })))
                .collect();
            put(out, crate::give(&Value::Array(parts).to_string()))
        })
    }
}

/// Does one step of the driver (ui::headless) and answers it as JSON.
///
/// # Safety
/// `screen` is a live handle; `step` is (pointer, length); `out` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_screen_step(screen: *mut ClarityScreen, step: *const c_char, step_len: usize, out: *mut *mut c_char) -> ClarityStatus {
    let step = unsafe { crate::json(step, step_len) };
    unsafe {
        act(screen, |s| {
            let answer = s.step(step?)?;
            put(out, crate::give(&answer.to_string()))
        })
    }
}

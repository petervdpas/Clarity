//! Custom parts as a table of callbacks: made per place in a window, given
//! their values, drawn by commands on the library's canvas, and speaking to
//! their window through their cx.

use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::ptr::null_mut;

use serde_json::json;

use super::{given, host_of, last_error, ok, t, window, Model};
use crate::canvas::*;
use crate::custom::*;
use crate::screen::*;
use crate::{ClarityAnswer, CLARITY_INVALID};

/// A gauge as a C program writes one: what it was given and did.
#[derive(Default)]
struct Gauge {
    log: RefCell<Vec<String>>,
}

thread_local! {
    /// The gauges made on this thread, to look at after the screen used them.
    static MADE: RefCell<Vec<*mut Gauge>> = const { RefCell::new(Vec::new()) };
}

fn gauge<'a>(part: *mut c_void) -> &'a Gauge {
    unsafe { &*(part as *const Gauge) }
}

unsafe extern "C" fn make(_user: *mut c_void) -> *mut c_void {
    let made = Box::into_raw(Box::<Gauge>::default());
    MADE.with(|m| m.borrow_mut().push(made));
    made as *mut c_void
}

unsafe extern "C" fn set(part: *mut c_void, name: *const c_char, name_len: usize, value: *const c_char, value_len: usize, _cx: *mut ClarityCustomCx) {
    let (name, value) = unsafe { (given(name, name_len).unwrap(), given(value, value_len).unwrap()) };
    gauge(part).log.borrow_mut().push(format!("set {name} {value}"));
}

unsafe extern "C" fn hear(part: *mut c_void, hear: *const c_char, hear_len: usize, _values: *const c_char, _values_len: usize, _cx: *mut ClarityCustomCx) {
    gauge(part).log.borrow_mut().push(format!("hear {}", unsafe { given(hear, hear_len) }.unwrap()));
}

const RED: u32 = 0xFFFF0000;

unsafe extern "C" fn draw(part: *mut c_void, canvas: *mut ClarityCanvas, x: f64, y: f64, width: f64, height: f64) {
    let mut accent = 0;
    let (role, role_len) = t("accent");
    let found = unsafe { clarity_canvas_look_color(canvas, role, role_len, &mut accent) };
    let (words, words_len) = t("88");
    let mut wide = 0.0;
    let measured = unsafe { clarity_canvas_measure(canvas, words, words_len, 0.0, 0, &mut wide) };
    unsafe {
        clarity_canvas_clip(canvas, x, y, width, height);
        clarity_canvas_fill(canvas, x, y, width, height, RED, 0.0);
        clarity_canvas_line(canvas, x, y, x + width, y + height, accent, 2.0);
        clarity_canvas_oval(canvas, x, y, 10.0, 10.0, accent, 1, 0.0);
        clarity_canvas_text(canvas, words, words_len, x, y, width, 20.0, 0xFF000000, 0.0, 1, 1);
    }
    let (svg, svg_len) = t("M0 0 L10 10 Z");
    let path = unsafe { clarity_canvas_path(canvas, svg, svg_len, accent, 0, 1.0) };
    let (bad, bad_len) = t("not a path");
    let bad = unsafe { clarity_canvas_path(canvas, bad, bad_len, accent, 0, 1.0) };
    // The clip is left open: the library closes it after the draw.
    gauge(part).log.borrow_mut().push(format!("draw {found} {measured} {} {path} {bad} {}", wide > 0.0, accent != 0));
}

unsafe extern "C" fn click(part: *mut c_void, cx: *mut ClarityCustomCx) {
    let (mut x, mut w) = (0.0, 0.0);
    unsafe { clarity_custom_bounds(cx, &mut x, null_mut(), &mut w, null_mut()) };
    let mut enabled = 0;
    unsafe { clarity_custom_enabled(cx, &mut enabled) };
    gauge(part).log.borrow_mut().push(format!("click {} {enabled}", w > 0.0));
    let (value, value_len) = t("5");
    ok(unsafe { clarity_custom_change(cx, value, value_len) });
    let ((tell, tell_len), (values, values_len)) = (t("reached"), t("[5]"));
    ok(unsafe { clarity_custom_tell(cx, tell, tell_len, values, values_len) });
    let (bad, bad_len) = t("{");
    assert_eq!(unsafe { clarity_custom_tell(cx, tell, tell_len, bad, bad_len) }, CLARITY_INVALID);
    ok(unsafe { clarity_custom_invalidate(cx) });
    ok(unsafe { clarity_custom_wake_in(cx, 5.0) });
}

unsafe extern "C" fn focusable(_part: *mut c_void) -> i32 {
    1
}

unsafe extern "C" fn key(part: *mut c_void, name: *const c_char, name_len: usize, held: u32, _cx: *mut ClarityCustomCx) -> i32 {
    gauge(part).log.borrow_mut().push(format!("key {} {held}", unsafe { given(name, name_len) }.unwrap()));
    1
}

unsafe extern "C" fn tooltip_at(_part: *mut c_void, _x: f64, _y: f64, tip: *mut ClarityAnswer) {
    unsafe { super::answer(tip, "the speed") };
}

unsafe extern "C" fn destroy(part: *mut c_void) {
    gauge(part).log.borrow_mut().push("destroy".into());
}

fn callbacks() -> ClarityCustomCallbacks {
    ClarityCustomCallbacks {
        user: null_mut(),
        make: Some(make),
        set: Some(set),
        hear: Some(hear),
        draw: Some(draw),
        start: None,
        focusable: Some(focusable),
        pointer_pressed: None,
        pointer_moved: None,
        pointer_released: None,
        click: Some(click),
        wheel: None,
        key: Some(key),
        text: None,
        focus_changed: None,
        wake: None,
        stop: None,
        tooltip_at: Some(tooltip_at),
        destroy: Some(destroy),
    }
}

const LINE: &str = r#"<Custom name="gauge" at="1,1" span="4" rows="3" part="FfiGauge" bind="speed" tells='[{"tell":"reached","handler":"gaugeReached"}]' />"#;

#[test]
fn a_custom_part_draws_by_commands_and_speaks_to_its_window() {
    let (kind, kind_len) = t("FfiGauge");
    ok(unsafe { clarity_custom_register(kind, kind_len, &callbacks()) });
    let model = Model::with(json!({ "speed": 3 }));
    let markup = window(LINE);
    let (m, ml) = t(&markup);
    let host = host_of(&model);
    let mut screen = null_mut();
    ok(unsafe { clarity_screen_new(m, ml, std::ptr::null(), 0, &host, &mut screen) });
    let made = MADE.with(|m| *m.borrow().last().unwrap());
    let part = unsafe { &*made };
    assert!(part.log.borrow().contains(&"set value 3".to_string()), "{:?}", part.log.borrow());

    // It draws red where it is, and the rest of the drawing goes on.
    ok(unsafe { clarity_screen_resize(screen, 400.0, 300.0, 1.0) });
    let (w, h) = (400usize, 300usize);
    let mut pixels = vec![0u8; w * h * 4];
    ok(unsafe { clarity_screen_draw(screen, pixels.as_mut_ptr(), w as i32, h as i32, w * 4) });
    let (n, nl) = t("gauge");
    let (mut x, mut y, mut pw, mut ph) = (0.0, 0.0, 0.0, 0.0);
    ok(unsafe { clarity_screen_part_place(screen, n, nl, &mut x, &mut y, &mut pw, &mut ph) });
    let at = ((y + ph - 3.0) as usize * w + (x + 3.0) as usize) * 4;
    assert_eq!(&pixels[at..at + 4], &[255, 0, 0, 255], "red at the gauge's corner");
    assert!(part.log.borrow().contains(&"draw 0 0 true 0 3 true".to_string()), "{:?}", part.log.borrow());

    // A click changes its value (the host is written, the tell runs).
    model.logged();
    ok(unsafe { clarity_screen_click(screen, x + pw / 2.0, y + ph / 2.0) });
    assert!(part.log.borrow().contains(&"click true 1".to_string()), "{:?}", part.log.borrow());
    let logged = model.logged();
    assert!(logged.contains(&"write gauge speed 5".to_string()), "{logged:?}");
    assert!(logged.contains(&"told gauge reached [5]".to_string()), "{logged:?}");

    // It took the focus with the click and hears keys by name.
    let (k, kl) = t("Up");
    let mut taken = 0;
    ok(unsafe { clarity_screen_key(screen, k, kl, 2, &mut taken) });
    assert_eq!(taken, 1);
    assert!(part.log.borrow().contains(&"key Up 2".to_string()), "{:?}", part.log.borrow());

    unsafe { clarity_screen_free(screen) };
    assert_eq!(part.log.borrow().last().map(String::as_str), Some("destroy"));
    MADE.with(|m| m.borrow_mut().retain(|p| *p != made));
    drop(unsafe { Box::from_raw(made) });
}

#[test]
fn a_kind_without_its_draw_is_invalid() {
    let mut lacking = callbacks();
    lacking.draw = None;
    let (kind, kind_len) = t("Lacking");
    assert_eq!(unsafe { clarity_custom_register(kind, kind_len, &lacking) }, CLARITY_INVALID);
    assert_eq!(last_error(), "the custom part has no draw");
}

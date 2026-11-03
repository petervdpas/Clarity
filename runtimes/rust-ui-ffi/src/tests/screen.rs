//! A screen without a window over the caller's part host: the parts read
//! and change the caller's window model through the callbacks, input goes
//! in by the screen's functions, and it draws into the caller's pixels.

use std::ffi::c_char;
use std::ptr::null_mut;

use serde_json::{json, Value};

use super::{host_of, last_error, ok, t, taken, window, Model};
use crate::screen::*;
use crate::{CLARITY_INVALID, CLARITY_NOT_FOUND};

const PARTS: &str = r#"<Textbox name="nameBox" at="1,1" span="4" bind="name" /><Button name="save" at="1,2" span="3" caption="@saveCaption" onclick="save" />"#;

fn screen_of(model: &Model, parts: &str) -> *mut ClarityScreen {
    let markup = window(parts);
    let (m, ml) = t(&markup);
    let host = host_of(model);
    let mut screen = null_mut();
    ok(unsafe { clarity_screen_new(m, ml, std::ptr::null(), 0, &host, &mut screen) });
    screen
}

fn place(screen: *mut ClarityScreen, name: &str) -> Result<(f64, f64, f64, f64), i32> {
    let (n, nl) = t(name);
    let (mut x, mut y, mut w, mut h) = (0.0, 0.0, 0.0, 0.0);
    match unsafe { clarity_screen_part_place(screen, n, nl, &mut x, &mut y, &mut w, &mut h) } {
        0 => Ok((x, y, w, h)),
        status => Err(status),
    }
}

fn click_part(screen: *mut ClarityScreen, name: &str) {
    let (x, y, w, h) = place(screen, name).unwrap();
    ok(unsafe { clarity_screen_click(screen, x + w / 2.0, y + h / 2.0) });
}

fn key(screen: *mut ClarityScreen, name: &str) {
    let (n, nl) = t(name);
    ok(unsafe { clarity_screen_key(screen, n, nl, 0, null_mut()) });
}

#[test]
fn a_click_runs_the_handler_through_the_host() {
    let model = Model::with(json!({ "name": "Ann" }));
    let screen = screen_of(&model, PARTS);
    model.logged();
    click_part(screen, "save");
    assert_eq!(model.logged(), ["run save click save"]);
    unsafe { clarity_screen_free(screen) };
    assert!(*model.released.borrow(), "the host is let go with its screen");
}

#[test]
fn typing_writes_the_value_to_the_host() {
    let model = Model::with(json!({ "name": "Ann" }));
    let screen = screen_of(&model, PARTS);
    let (n, nl) = t("nameBox");
    ok(unsafe { clarity_screen_focus(screen, n, nl) });
    let mut focused = null_mut();
    ok(unsafe { clarity_screen_focused(screen, &mut focused) });
    assert_eq!(unsafe { taken(focused) }.as_deref(), Some("nameBox"));
    key(screen, "End");
    let (b, bl) = t(" B");
    ok(unsafe { clarity_screen_type(screen, b, bl) });
    key(screen, "Tab");
    assert_eq!(model.values.borrow()["name"], json!("Ann B"));
    assert!(model.logged().iter().any(|l| l == r#"write nameBox name "Ann B""#));
    unsafe { clarity_screen_free(screen) };
}

#[test]
fn the_hosts_texts_and_states_show() {
    let model = Model::with(json!({ "name": "Ann" }));
    let screen = screen_of(&model, PARTS);
    let read = json!({ "do": "read" }).to_string();
    let mut out = null_mut();
    ok(unsafe { clarity_screen_step(screen, read.as_ptr() as *const c_char, read.len(), &mut out) });
    let shown: Value = serde_json::from_str(&unsafe { taken(out) }.unwrap()).unwrap();
    assert!(shown["buttons"].as_array().unwrap().iter().any(|b| b["text"] == "Save it"), "{shown}");
    // Code hides the button: its state says so, and the parts read it again.
    model.states.borrow_mut().insert("save".into(), json!({ "visible": false }));
    *model.changes.borrow_mut() += 1;
    ok(unsafe { clarity_screen_changed(screen) });
    assert_eq!(place(screen, "save"), Err(CLARITY_NOT_FOUND));
    assert!(last_error().contains("save"));
    assert!(place(screen, "nameBox").is_ok());
    let mut out = null_mut();
    ok(unsafe { clarity_screen_parts_json(screen, &mut out) });
    let parts: Value = serde_json::from_str(&unsafe { taken(out) }.unwrap()).unwrap();
    assert_eq!(parts.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["nameBox"]);
    unsafe { clarity_screen_free(screen) };
}

#[test]
fn it_draws_into_the_callers_pixels() {
    let model = Model::with(json!({ "name": "Ann" }));
    let screen = screen_of(&model, PARTS);
    ok(unsafe { clarity_screen_resize(screen, 400.0, 300.0, 1.0) });
    let (w, h) = (400, 300);
    let mut pixels = vec![0u8; w * h * 4];
    ok(unsafe { clarity_screen_draw(screen, pixels.as_mut_ptr(), w as i32, h as i32, w * 4) });
    // The window's background is opaque everywhere; the parts draw something other than it.
    assert_eq!(pixels[3], 255);
    let first = pixels[0..4].to_vec();
    assert!(pixels.chunks(4).any(|p| p != first.as_slice()), "the parts drew nothing");
    assert_eq!(unsafe { clarity_screen_draw(screen, pixels.as_mut_ptr(), w as i32, h as i32, 8) }, CLARITY_INVALID);
    // A dark look draws another background.
    ok(unsafe { clarity_screen_look(screen, std::ptr::null(), 0, 1) });
    let mut dark = vec![0u8; w * h * 4];
    ok(unsafe { clarity_screen_draw(screen, dark.as_mut_ptr(), w as i32, h as i32, w * 4) });
    assert_ne!(&dark[0..4], first.as_slice());
    unsafe { clarity_screen_free(screen) };
}

#[test]
fn input_by_the_screens_functions() {
    let model = Model::with(json!({ "name": "Ann" }));
    let screen = screen_of(&model, PARTS);
    let (x, y, w, h) = place(screen, "save").unwrap();
    model.logged();
    ok(unsafe { clarity_screen_pointer_move(screen, x + w / 2.0, y + h / 2.0) });
    ok(unsafe { clarity_screen_pointer_press(screen, x + w / 2.0, y + h / 2.0, 0, 1, 0) });
    ok(unsafe { clarity_screen_pointer_release(screen, x + w / 2.0, y + h / 2.0, 0) });
    assert_eq!(model.logged(), ["run save click save"]);
    ok(unsafe { clarity_screen_pointer_leave(screen) });
    ok(unsafe { clarity_screen_wheel(screen, 10.0, 10.0, 0.0, -1.0, 0) });
    ok(unsafe { clarity_screen_settle(screen, 0.0) });
    let mut count = -1;
    ok(unsafe { clarity_screen_overlays(screen, &mut count) });
    assert_eq!(count, 0);
    assert_eq!(unsafe { clarity_screen_pointer_press(screen, 1.0, 1.0, 7, 1, 0) }, CLARITY_INVALID);
    let (n, nl) = t("NoSuchKey");
    assert_eq!(unsafe { clarity_screen_key(screen, n, nl, 0, null_mut()) }, CLARITY_INVALID);
    unsafe { clarity_screen_free(screen) };
}

#[test]
fn markup_that_does_not_read_or_a_host_without_read_is_invalid() {
    let model = Model::default();
    let host = host_of(&model);
    let (m, ml) = t("<Window name=\"Main\"><Button");
    let mut screen = null_mut();
    assert_eq!(unsafe { clarity_screen_new(m, ml, std::ptr::null(), 0, &host, &mut screen) }, CLARITY_INVALID);
    assert!(last_error().starts_with("the window's markup does not read"));
    let mut lacking = host_of(&model);
    lacking.read = None;
    lacking.release = None;
    let markup = window("");
    let (m, ml) = t(&markup);
    assert_eq!(unsafe { clarity_screen_new(m, ml, std::ptr::null(), 0, &lacking, &mut screen) }, CLARITY_INVALID);
    assert_eq!(last_error(), "the part host has no read");
}

#[test]
fn a_screen_is_used_on_the_thread_that_made_it() {
    let model = Model::default();
    let screen = screen_of(&model, PARTS);
    struct Sent(*mut ClarityScreen);
    unsafe impl Send for Sent {}
    let sent = Sent(screen);
    let (status, error) = std::thread::spawn(move || {
        let sent = sent;
        (unsafe { clarity_screen_resize(sent.0, 10.0, 10.0, 1.0) }, last_error())
    })
    .join()
    .unwrap();
    assert_eq!(status, CLARITY_INVALID);
    assert!(error.contains("another thread"));
    unsafe { clarity_screen_free(screen) };
}

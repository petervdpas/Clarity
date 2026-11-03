//! The library itself: its version, errors that cross as a status and a
//! message, strings and answers, the program's files and the person's choices.

use std::ffi::{c_char, CStr};
use std::ptr::null_mut;

use super::{last_error, ok, t, taken};
use crate::files::*;
use crate::*;

#[test]
fn the_version_and_interface_are_the_headers() {
    let version = unsafe { CStr::from_ptr(clarity_ui_version()) }.to_str().unwrap();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert_eq!(clarity_ui_interface(), 1);
    let header = include_str!("../../include/clarity_ui.h");
    assert!(header.contains("#define CLARITY_UI_INTERFACE 1"));
}

#[test]
fn a_panic_becomes_a_status_and_its_message() {
    assert_eq!(guard(|| panic!("the part broke")), CLARITY_PANIC);
    assert_eq!(last_error(), "the part broke");
    assert_eq!(guard(|| Err(Failure::not_found("no part save shows"))), CLARITY_NOT_FOUND);
    assert_eq!(last_error(), "no part save shows");
    assert_eq!(guard(|| Ok(())), CLARITY_OK);
}

#[test]
fn a_null_handle_or_text_that_is_no_utf8_is_invalid() {
    let (name, len) = t("Main");
    assert_eq!(unsafe { clarity_files_window(null_mut(), name, len, name, len) }, CLARITY_INVALID);
    assert_eq!(last_error(), "the handle is NULL");
    let mut files = null_mut();
    ok(unsafe { clarity_files_new(&mut files) });
    let bad = [0xff_u8, 0xfe];
    assert_eq!(unsafe { clarity_files_window(files, bad.as_ptr() as *const c_char, 2, name, len) }, CLARITY_INVALID);
    assert_eq!(last_error(), "the text is no UTF-8");
    let (model, model_len) = t("{ not json");
    assert_eq!(unsafe { clarity_files_model(files, model, model_len) }, CLARITY_INVALID);
    assert!(last_error().starts_with("the model does not read"));
    unsafe { clarity_files_free(files) };
}

#[test]
fn freeing_null_does_nothing() {
    unsafe {
        clarity_string_free(null_mut());
        clarity_files_free(null_mut());
        crate::program::clarity_program_free(null_mut());
        crate::screen::clarity_screen_free(null_mut());
        crate::desktop::clarity_driven_free(null_mut());
    }
    // Waking while no desktop runs does nothing.
    crate::desktop::clarity_desktop_wake();
}

#[test]
fn an_answer_keeps_a_copy_of_what_was_set() {
    let mut answer = ClarityAnswer::default();
    let mut bytes = b"[1,2]".to_vec();
    ok(unsafe { clarity_answer_set(&mut answer, bytes.as_ptr(), bytes.len()) });
    bytes[1] = b'9';
    assert_eq!(answer.json(), Some(serde_json::json!([1, 2])));
    assert_eq!(unsafe { clarity_answer_set(null_mut(), bytes.as_ptr(), bytes.len()) }, CLARITY_INVALID);
}

#[test]
fn files_become_the_programs_files_and_change_when_set_again() {
    let mut files = null_mut();
    ok(unsafe { clarity_files_new(&mut files) });
    let set = |name: &str, markup: &str| {
        let ((n, nl), (m, ml)) = (t(name), t(markup));
        ok(unsafe { clarity_files_window(files, n, nl, m, ml) });
    };
    set("Main", "<Window name=\"Main\" />");
    let (title, title_len) = t("Orders");
    ok(unsafe { clarity_files_title(files, title, title_len) });
    ok(unsafe { clarity_files_size(files, 900.0, 600.0) });
    let (logo, logo_len) = t("logo.png");
    ok(unsafe { clarity_files_resource(files, logo, logo_len, [1u8, 2, 3].as_ptr(), 3) });
    let made = unsafe { &*files }.app_files();
    assert_eq!(made.window("Main"), Some("<Window name=\"Main\" />"));
    assert_eq!(made.resource("logo.png"), Some(&[1u8, 2, 3][..]));
    assert_eq!((made.title, made.width, made.height), ("Orders", 900.0, 600.0));
    set("Main", "<Window name=\"Main\" title=\"Again\" />");
    assert_eq!(unsafe { &*files }.app_files().window("Main"), Some("<Window name=\"Main\" title=\"Again\" />"));
    assert_eq!(unsafe { clarity_files_size(files, f64::NAN, 1.0) }, CLARITY_INVALID);
    unsafe { clarity_files_free(files) };
}

const MODEL: &str = r#"{"project":{"locale":{"language":"en"},"translations":{"nl":{"saveCaption":"Opslaan"}}}}"#;

#[test]
fn choices_come_from_the_model_and_a_language_it_lacks_is_not_found() {
    let _one = super::GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let mut files = null_mut();
    ok(unsafe { clarity_files_new(&mut files) });
    let (model, model_len) = t(MODEL);
    ok(unsafe { clarity_files_model(files, model, model_len) });
    ok(unsafe { clarity_choices_start(files, std::ptr::null(), 0) });
    let mut out = null_mut();
    ok(unsafe { clarity_choices_json(&mut out) });
    let chosen: serde_json::Value = serde_json::from_str(&unsafe { taken(out) }.unwrap()).unwrap();
    assert_eq!(chosen["language"], "en");
    let offered: Vec<&str> = chosen["languages"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
    assert_eq!(offered, ["en", "nl"]);
    let (de, de_len) = t("de");
    assert_eq!(unsafe { clarity_language_choose(de, de_len) }, CLARITY_NOT_FOUND);
    let (nl, nl_len) = t("nl");
    ok(unsafe { clarity_language_choose(nl, nl_len) });
    let ((key, key_len), (english, english_len)) = (t("saveCaption"), t("Save"));
    let mut out = null_mut();
    ok(unsafe { clarity_translate(key, key_len, english, english_len, &mut out) });
    assert_eq!(unsafe { taken(out) }.unwrap(), "Opslaan");
    let (en, en_len) = t("en");
    ok(unsafe { clarity_language_choose(en, en_len) });
    unsafe { clarity_files_free(files) };
}

//! The language changed (C# WindowStack.Remake): the program hears it, and
//! every open window is made again, so its texts show in it. On its own,
//! since the language chosen is the program's, not one test's.

use super::core_windows_fake;

use clarity_ui::parts::label::LabelPart;
use core_windows_fake::*;

fn greeting(screen: &clarity_ui::Screen) -> String {
    let id = named(screen, "greeting");
    screen.element(id).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text()
}

#[test]
fn another_language_makes_every_window_again_in_it() {
    let _alone = super::one_at_a_time();
    clarity_ui::choices::start(&FILES, None);
    let (mut screen, fake) = started();
    assert_eq!("Hello", greeting(&screen));
    // A window below the one on top is made again too.
    click(&mut screen, "openPick");
    screen.key_pressed(clarity_ui::KeyInput::plain(clarity_ui::Key::Escape));
    fake.calls();
    clarity_ui::choices::choose_language("nl");
    screen.act_root::<clarity_ui::window::WindowStack, _>(|_, _| ());
    assert_eq!(vec!["language Some(\"nl\")"], fake.calls());
    assert_eq!("Hallo", greeting(&screen));
    // Back to the language the texts are written in: none for the program.
    clarity_ui::choices::choose_language("en");
    screen.act_root::<clarity_ui::window::WindowStack, _>(|_, _| ());
    assert_eq!(vec!["language None"], fake.calls());
    assert_eq!("Hello", greeting(&screen));
}

/// A language kept from an earlier run is told to the program at the start,
/// before anything else: a server answers each browser in its own.
#[test]
fn a_kept_language_is_told_to_the_program_at_the_start() {
    let _alone = super::one_at_a_time();
    let kept = std::collections::BTreeMap::from([("language".to_string(), "nl".to_string())]);
    clarity_ui::choices::start_kept(&FILES, kept, std::sync::Arc::new(|_: &std::collections::BTreeMap<String, String>| ()));
    let (screen, fake) = started();
    assert_eq!(Some("language Some(\"nl\")"), fake.calls().first().map(String::as_str));
    assert_eq!("Hallo", greeting(&screen));
    clarity_ui::choices::start(&FILES, None);
    clarity_runtime::i18n::set_language(None);
}

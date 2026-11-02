//! The language changed (C# WindowStack.Remake): the program hears it, and
//! every open window is made again, so its texts show in it. On its own,
//! since the language chosen is the program's, not one test's.

mod core_windows_fake;

use clarity_ui::parts::label::LabelPart;
use core_windows_fake::*;

fn greeting(screen: &clarity_ui::Screen) -> String {
    let id = named(screen, "greeting");
    screen.element(id).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text()
}

#[test]
fn another_language_makes_every_window_again_in_it() {
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

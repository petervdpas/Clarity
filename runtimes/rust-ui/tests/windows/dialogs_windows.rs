//! Dialog windows and the dialog that says a warning or an error, as the C#
//! tests have them (DialogWindowTests, SaidDialogTests and ExampleRunTests'
//! dialog sizes): on the desktop a dialog opens in a window of its own,
//! sized to it, as does a window marked `in new window`; where the host
//! opens no window of its own a dialog shows over the window, its keys its
//! own. Closing it tells the window that opened it, and a dialog asked for
//! gives what it closes with to the window that asked. A dialog is as wide
//! as its model says or as its captions need (560 at least), and as high as
//! its rows. The Escape cases are in looks_escape_dialog.

use super::dialogs_fake;

use clarity_runtime::ui::{Level, Message, Text};
use clarity_ui::dialogs::over::{dialog_size, DIALOG_WIDTH};
use clarity_ui::dialogs::DialogOverlay;
use clarity_ui::grid::{GAP, ROW_HEIGHT};
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::testing::TestHost;
use clarity_ui::window::view::{WindowView, CONTENT_PADDING};
use clarity_ui::window::{Apart, SaidDialog, WindowStack};
use clarity_ui::{markup, Element, PartHost, Screen, ThemeLook};
use dialogs_fake::*;
use std::rc::Rc;

fn view(markup_written: &str) -> WindowView {
    let host: Rc<dyn PartHost> = TestHost::new();
    WindowView::new(&markup::read(markup_written).expect("the markup reads"), host)
}

const LONG_CAPTION: &str = "a long caption a long caption a long caption a long caption";

#[test]
fn without_windows_of_its_own_a_dialog_shows_over_the_window_sized_to_it() {
    let (mut screen, _) = started();
    click(&mut screen, "pick");
    assert_eq!(1, screen.overlays().len());
    assert!(screen.overlays()[0].modal);
    shot(&mut screen, "dialog-over");
    let dialog = screen.find::<DialogOverlay>().expect("the dialog shows over the window");
    assert_eq!("Pick a color", dialog.title);
    // The window below stays the one in place; the dialog is the last of the windows.
    assert_eq!("Main", stack(&screen).current().unwrap().window());
    assert_eq!(vec!["Main", "Pick"], stack(&screen).windows());
    let (width, _) = dialog_size(dialog.view(), None);
    assert_eq!(width, dialog.panel().width);
    assert!(dialog.panel().height < 500.0 && dialog.panel().height >= 2.0 * ROW_HEIGHT, "{:?}", dialog.panel());
    assert_eq!(3, dialog.view().parts().len());
}

#[test]
fn a_dialog_asked_for_gives_what_it_closes_with_to_the_window_that_asked() {
    let (mut screen, fake) = started();
    click(&mut screen, "pick");
    fake.calls();
    click(&mut screen, "choose");
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["dispatch choose", "close Pick", "return Main", r#"answer picked ["x","red"]"#], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
}

#[test]
fn escape_presses_the_dialog_s_cancel_button_and_runs_nothing() {
    let (mut screen, fake) = started();
    click(&mut screen, "pick");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    // Its cancel button's handler runs, the window under it is told, and the question gets no answer.
    assert_eq!(vec!["dispatch done", "close Pick", "return Main"], fake.calls());
}

#[test]
fn a_host_with_windows_of_its_own_opens_dialogs_and_new_windows_there() {
    let (mut screen, _) = started();
    stack(&screen).running().set_apart(true);
    let main = stack(&screen).current().unwrap().instance();
    click(&mut screen, "pick");
    click(&mut screen, "openConnectionApart");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let asked: Vec<(String, bool, u64)> = aparts
        .into_iter()
        .map(|a| match a {
            Apart::Window(w) => (w.window, w.dialog, w.opener),
            Apart::Keys => panic!("no keys window was asked for"),
            Apart::Main(_) | Apart::Signin(_) => panic!("no main or sign-in window was asked for"),
        })
        .collect();
    assert_eq!(vec![("Pick".to_string(), true, main), ("Connection".to_string(), false, main)], asked);
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["Main"], stack(&screen).windows());
    // In place is in place, whatever the host.
    click(&mut screen, "openConnection");
    assert_eq!(vec!["Main", "Connection"], stack(&screen).windows());
}

#[test]
fn without_windows_of_its_own_a_new_window_opens_in_place() {
    let (mut screen, _) = started();
    click(&mut screen, "openConnectionApart");
    assert_eq!(vec!["Main", "Connection"], stack(&screen).windows());
    assert!(screen.overlays().is_empty());
}

#[test]
fn a_stack_of_its_own_tells_the_window_that_opened_it_when_it_ends() {
    let (mut screen, fake) = started();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "openPick");
    let Some(Apart::Window(request)) = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap().into_iter().next() else {
        panic!("a window of its own is asked for")
    };
    let running = stack(&screen).running().clone();
    let mut own = Screen::new(Box::new(WindowStack::apart(running, request)), FILES.look(false));
    own.resize(560.0, 160.0, 1.0);
    fake.calls();
    click(&mut own, "cancelPick");
    let (ended, closed) = own.act_root::<WindowStack, _>(|s, _| (s.ended(), s.take_closed())).unwrap();
    assert!(ended);
    let closed = closed.expect("the window that opened it is told");
    screen.act_root::<WindowStack, _>(move |s, cx| s.apart_closed(closed, cx));
    assert_eq!(vec!["dispatch done", "close Pick", "return Main"], fake.calls());
}

#[test]
fn a_dialog_is_as_wide_as_it_says_or_the_standard_and_as_high_as_its_rows() {
    let dialog = view(
        r#"<Window name="PickFriend">
          <Label name="who" at="1,1" span="12" caption="Who?" />
          <Button name="cancelButton" at="1,2" span="6" caption="Cancel" cancel="true" onclick="close" />
        </Window>"#,
    );
    let (width, height) = dialog_size(&dialog, None);
    assert_eq!(DIALOG_WIDTH, width);
    assert_eq!(2.0 * ROW_HEIGHT + GAP + 2.0 * CONTENT_PADDING, height);
    // The size its model names is the size.
    assert_eq!((300.0, 200.0), dialog_size(&dialog, Some((300.0, 200.0))));
}

#[test]
fn a_dialog_is_as_wide_as_the_labels_of_its_form_need() {
    let form = view(&format!(
        r#"<Window name="D">
          <Form name="settingsForm" at="1,1" span="12" rows="2" labels="3">
            <Label name="colorLabel" at="1,1" span="3" caption="{LONG_CAPTION}" for="colorBox" />
            <Textbox name="colorBox" at="4,1" span="6" />
          </Form>
        </Window>"#
    ));
    let flat = view(&format!(r#"<Window name="D"><Label at="1,1" span="3" caption="{LONG_CAPTION}" /></Window>"#));
    let (width, _) = dialog_size(&form, None);
    assert!(width > DIALOG_WIDTH, "{width}");
    assert!((dialog_size(&flat, None).0 - width).abs() <= 1.0);
}

#[test]
fn a_dialog_is_as_wide_as_a_tab_that_is_not_open_needs() {
    let tabs = view(&format!(
        r#"<Window name="D">
          <Tabs name="pages" at="1,1" span="12" rows="3">
            <Tab title="One"><Label at="1,1" span="3" caption="Short" /></Tab>
            <Tab title="Two"><Label at="1,1" span="3" caption="{LONG_CAPTION}" /></Tab>
          </Tabs>
        </Window>"#
    ));
    assert!(dialog_size(&tabs, None).0 > DIALOG_WIDTH);
}

#[test]
fn it_says_its_text_and_ok_without_a_heading() {
    let dialog = SaidDialog::new(Message { level: Level::Error, text: Text::plain("The game is over"), toast: None });
    let mut screen = Screen::new(Box::new(dialog), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    let mut picture = clarity_ui::testing::Shot::of(&mut screen, 800, 400, 1.0);
    picture.save(format!("{SHOTS}/r1-dialogs-said.png")).unwrap();
    // Its padding, one line of text, its padding, OK and its padding.
    let said = screen.root().as_any().downcast_ref::<SaidDialog>().unwrap();
    assert_eq!(20.0 + 20.0 + 20.0 + ROW_HEIGHT + 20.0, said.panel().height);
}

/// Its OK button is a row of the project's height, as the window's buttons are.
#[test]
fn its_ok_is_as_high_as_the_project_s_rows() {
    clarity_ui::grid::set_row_height(30.0);
    let dialog = SaidDialog::new(Message { level: Level::Error, text: Text::plain("The game is over"), toast: None });
    let mut screen = Screen::new(Box::new(dialog), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    clarity_ui::testing::Shot::of(&mut screen, 800, 400, 1.0);
    let said = screen.root().as_any().downcast_ref::<SaidDialog>().unwrap();
    assert_eq!(20.0 + 20.0 + 20.0 + 30.0 + 20.0, said.panel().height);
    clarity_ui::grid::set_row_height(ROW_HEIGHT);
}

#[test]
fn a_note_s_x_says_dismiss_and_shows_a_hand() {
    let (mut screen, _) = started();
    screen.act_root::<WindowStack, _>(|s, cx| s.say(Level::Info, Text::plain("Saved"), cx));
    // Drawn once, so the note has its place.
    shot(&mut screen, "note");
    let start = std::time::Instant::now();
    let now = Rc::new(std::cell::Cell::new(start));
    let clock = now.clone();
    screen.now = Box::new(move || clock.get());
    // The x sits at the top right of the first note.
    let (x, y) = (800.0 - CONTENT_PADDING - 18.0, CONTENT_PADDING + 18.0);
    screen.pointer_moved(x, y);
    now.set(start + clarity_ui::screen::TIP_PAUSE);
    screen.wake_due(now.get());
    assert_eq!(Some("Dismiss"), screen.tip().map(|t| t.text.as_str()));
    assert_eq!(clarity_ui::input::Cursor::Hand, screen.cursor());
}

#[test]
fn a_dialog_is_as_wide_as_its_captions_need_or_the_standard() {
    let narrow = view(r#"<Window name="D"><Label at="1,1" span="6" caption="Hi" /></Window>"#);
    assert_eq!(DIALOG_WIDTH, dialog_size(&narrow, None).0);
    let caption = vec!["a long caption"; 8].join(" ");
    let wide = view(&format!(r#"<Window name="D"><Label at="1,1" span="3" caption="{caption}" /></Window>"#));
    assert!(dialog_size(&wide, None).0 > DIALOG_WIDTH);
}

#[test]
fn a_note_is_420_wide_at_the_top_right() {
    let (mut screen, _) = started();
    screen.act_root::<WindowStack, _>(|s, cx| s.say(Level::Info, Text::plain("Saved"), cx));
    // Drawn once, so the note has its place.
    shot(&mut screen, "note");
    let notes = screen.find::<clarity_ui::window::ToastList>().expect("the notes");
    let b = notes.node().bounds;
    assert_eq!(420.0, b.width);
    assert_eq!(800.0 - CONTENT_PADDING, b.right());
    assert_eq!(CONTENT_PADDING, b.y);
}

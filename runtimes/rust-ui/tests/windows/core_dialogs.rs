//! Clarity's own dialogs over a program's windows, as the C# tests have them
//! (AskDialogTests, DialogWindowTests, KeysWindowTests, LongCallBarTests,
//! UnconnectedWindowTests): a dialog window over the window that opened it,
//! a dialog that answers the handler that asked for it, questions asked one
//! at a time, the keys window and the person's keys, the long-call bar with
//! Stop, and the window that keeps the connection.

use super::core_windows_fake;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use clarity_runtime::program::Unconnected;
use clarity_runtime::ui::{Level, Text};
use clarity_ui::dialogs::keys::{written, Role};
use clarity_ui::dialogs::{AskDialog, DialogOverlay, KeysWindow, LongCallBar};
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::window::{Apart, WindowStack};
use clarity_ui::{Element, Screen};
use core_windows_fake::*;
use serde_json::json;

fn keys_window(screen: &Screen) -> &KeysWindow {
    screen.find::<KeysWindow>().expect("the keys window is open")
}

fn press_keys_button(screen: &mut Screen, role: Role, command: Option<&str>) {
    let id = keys_window(screen).button(role, command).node().id();
    click_at(screen, id);
}

#[test]
fn a_dialog_shows_over_the_window_and_closing_it_tells_the_window_below() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "openPick");
    assert_eq!(vec!["open Pick"], fake.calls());
    assert_eq!(vec!["Main", "Pick"], stack(&screen).windows());
    let dialog = screen.find::<DialogOverlay>().expect("the dialog shows over the window");
    assert_eq!("Pick a color", dialog.title);
    // Sized to the dialog: the standard width, its rows high with its title.
    assert_eq!(clarity_ui::dialogs::over::DIALOG_WIDTH, dialog.panel().width);
    assert!(dialog.panel().height < 200.0);
    shot(&mut screen, "dialog-overlay");
    // Its keys are its own: Escape presses its cancel button.
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(vec!["dispatch done", "close Pick", "return Main"], fake.calls());
    assert!(screen.find::<DialogOverlay>().is_none());
    assert_eq!(vec!["Main"], stack(&screen).windows());
}

#[test]
fn what_a_dialog_says_shows_in_the_window_under_it() {
    let (mut screen, _) = started();
    click(&mut screen, "openPick");
    let host = stack(&screen).dialog_host().unwrap().clone();
    use clarity_ui::PartHost;
    host.say(Level::Warning, Text::plain("From the dialog"));
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!("From the dialog", stack(&screen).said()[0].text.show());
}

#[test]
fn a_dialog_asked_for_answers_the_handler_with_what_it_closes_with() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "pick");
    assert_eq!(vec!["dispatch pick", "open Pick"], fake.calls());
    click(&mut screen, "choose");
    assert_eq!(vec!["dispatch choose", "close Pick", "return Main", r#"answer picked ["x","red"]"#], fake.calls());
    // Closed any other way, it answers nothing.
    click(&mut screen, "pick");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(vec!["dispatch done", "close Pick", "return Main"], fake.calls());
}

#[test]
fn where_the_host_gives_windows_of_their_own_a_dialog_opens_in_one_and_answers_on_closing() {
    let (mut screen, fake) = started();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "pick");
    assert!(screen.find::<DialogOverlay>().is_none());
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    assert_eq!(("Pick", true), (request.window.as_str(), request.dialog));
    let running = stack(&screen).running().clone();
    let apart = WindowStack::apart(running, request);
    // Sized to the dialog: the standard width and as high as its rows.
    let (width, height) = apart.wanted_size().unwrap();
    assert_eq!(clarity_ui::dialogs::over::DIALOG_WIDTH, width);
    assert!(height > 40.0 && height < 200.0, "{height}");
    assert_eq!("Pick a color", apart.title());
    let mut own = Screen::new(Box::new(apart), FILES.look(false));
    own.resize(width, height, 1.0);
    fake.calls();
    click(&mut own, "choose");
    let closed = own.act_root::<WindowStack, _>(|s, _| (s.ended(), s.take_closed())).unwrap();
    assert!(closed.0);
    let closed = closed.1.expect("the window that opened it is told");
    assert_eq!(Some(json!("red")), closed.result);
    screen.act_root::<WindowStack, _>(move |s, cx| s.apart_closed(closed, cx));
    assert_eq!(vec!["dispatch choose", "close Pick", "return Main", r#"answer picked ["x","red"]"#], fake.calls());
}

#[test]
fn a_question_asks_yes_or_no_in_its_own_words_and_enter_answers() {
    let (mut screen, fake) = started();
    click(&mut screen, "ask");
    let ask = screen.find::<AskDialog>().expect("the question shows");
    assert_eq!(("Delete", "Keep"), (ask.ok().text.as_str(), ask.cancel().text.as_str()));
    assert!(ask.input().is_none());
    shot(&mut screen, "ask-dialog");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(vec!["answer confirmed [1]"], fake.calls());
    assert!(screen.find::<AskDialog>().is_none());
    // Cancelling runs nothing.
    click(&mut screen, "ask");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(fake.calls().is_empty());
    assert!(screen.overlays().is_empty());
}

#[test]
fn a_question_for_a_number_reads_what_is_typed() {
    let (mut screen, fake) = started();
    click(&mut screen, "askNumber");
    let ask = screen.find::<AskDialog>().unwrap();
    assert!(ask.input().is_some());
    // The input has the focus; nothing typed is no answer.
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(fake.calls().is_empty());
    screen.text_typed("42");
    shot(&mut screen, "ask-number");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    let calls = fake.calls();
    assert_eq!(1, calls.len(), "{calls:?}");
    assert!(calls[0].starts_with("answer numbered [") && calls[0].contains("42"), "{calls:?}");
}

#[test]
fn a_question_for_an_enum_offers_only_what_it_allows() {
    let (mut screen, fake) = started();
    click(&mut screen, "askColor");
    let ask = screen.find::<AskDialog>().unwrap();
    // It starts at the first value allowed.
    assert_eq!(Some(json!("green")), ask.value());
    let ok = ask.ok().node().id();
    fake.calls();
    click_at(&mut screen, ok);
    assert_eq!(vec![r#"answer colored ["green"]"#], fake.calls());
}

#[test]
fn questions_are_asked_one_at_a_time() {
    let (mut screen, fake) = started();
    click(&mut screen, "ask");
    // The window under the question takes nothing; a second comes once the first is answered.
    let host = stack(&screen).current().unwrap().clone();
    host.answered(Ok({
        let mut r = reply(host.state_now());
        r.asks.push(clarity_runtime::ui::Ask { question: Text::plain("Second?"), form: clarity_runtime::ui::AskForm::Confirm { yes: None, no: None }, handler: "second".into(), values: vec![] });
        r
    }));
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(1, screen.overlays().len());
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(vec!["answer confirmed [1]"], fake.calls());
    let second = screen.find::<AskDialog>().expect("the second question shows");
    assert_eq!(("OK", "Cancel"), (second.ok().text.as_str(), second.cancel().text.as_str()));
}

#[test]
fn the_keys_window_lists_the_commands_and_sets_the_person_s_keys() {
    let (mut screen, fake) = started();
    click(&mut screen, "keys");
    let says = keys_window(&screen).says();
    assert_eq!(vec!["Keys", "Change gives a command a new key; Reset gives it its own key back", "Everywhere", "Quit", "Ctrl+Q", "Orders", "Save", "Ctrl+S", "Warn", "Ctrl+W"], says);
    shot(&mut screen, "keys-window");
    // Change, then the new key.
    press_keys_button(&mut screen, Role::Change, Some("Main.save"));
    assert!(keys_window(&screen).capturing());
    // A key the inputs need is refused, saying why.
    screen.key_pressed(KeyInput::plain(Key::A));
    assert!(keys_window(&screen).says()[1].contains("cannot be used"), "{:?}", keys_window(&screen).says());
    fake.calls();
    screen.key_pressed(KeyInput::new(Key::K, Modifiers::CONTROL));
    assert_eq!(vec!["Ctrl+K".to_string()], keys_window(&screen).person().of("Main.save", Some("Ctrl+S")));
    // Kept with the settings.
    assert_eq!(vec![r#"keep {"keys":{"Main.save":["Ctrl+K"]}}"#], fake.calls());
    // A key another command of the same place has offers a swap.
    press_keys_button(&mut screen, Role::Change, Some("Main.save"));
    screen.key_pressed(KeyInput::new(Key::W, Modifiers::CONTROL));
    assert!(keys_window(&screen).says()[1].contains("Swap them?"));
    shot(&mut screen, "keys-swap");
    press_keys_button(&mut screen, Role::Swap, None);
    let person = keys_window(&screen).person();
    assert_eq!((vec!["Ctrl+W".to_string()], vec!["Ctrl+K".to_string()]), (person.of("Main.save", None), person.of("Main.warn", None)));
    // Escape closes it; the window answers the person's keys now.
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.find::<KeysWindow>().is_none());
    fake.calls();
    screen.key_pressed(KeyInput::new(Key::W, Modifiers::CONTROL));
    assert_eq!(vec!["dispatch save"], fake.calls());
    // Reset all puts the design's back.
    click(&mut screen, "keys");
    press_keys_button(&mut screen, Role::ResetAll, None);
    assert!(keys_window(&screen).person().choices.is_empty());
}

#[test]
fn keys_are_written_as_clarity_writes_them() {
    assert_eq!(Some("Ctrl+Shift+N".to_string()), written(KeyInput::new(Key::N, Modifiers::CONTROL | Modifiers::SHIFT)));
    assert_eq!(Some("Alt+F4".to_string()), written(KeyInput::new(Key::F4, Modifiers::ALT)));
    assert_eq!(Some("Ctrl++".to_string()), written(KeyInput::new(Key::Plus, Modifiers::CONTROL)));
    assert_eq!(None, written(KeyInput::plain(Key::Shift)));
}

fn wait_for(what: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !what() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_long_call_runs_off_the_screen_s_thread_with_its_bar_and_stop() {
    let (mut screen, fake) = started_with(Fake { watches: true, ..Fake::default() });
    fake.calls();
    click(&mut screen, "long");
    assert!(stack(&screen).long_call());
    wait_for(|| fake.long_reached.load(Ordering::SeqCst));
    let bar = screen.find::<LongCallBar>().expect("the bar lies over the window");
    assert!(bar.says());
    assert_eq!("25%", bar.said_text());
    shot(&mut screen, "long-call-bar");
    // Nothing under it is pressed, and no other handler runs.
    click(&mut screen, "save");
    assert_eq!(vec!["dispatch long"], fake.calls());
    // Stop ends the call at its next look.
    let stop = screen.find::<LongCallBar>().unwrap().stop_button().node().id();
    click_at(&mut screen, stop);
    screen.finish_later();
    assert!(!stack(&screen).long_call());
    assert!(screen.find::<LongCallBar>().is_none());
    assert_eq!(json!(true), stack(&screen).current().unwrap().state_now().data["stopped"]);
    // Handlers run again.
    click(&mut screen, "save");
    screen.finish_later();
    assert_eq!(vec!["dispatch save"], fake.calls());
}

fn unconnected(handler: Option<&str>) -> Fake {
    Fake { unconnected: Some(Unconnected { handler: handler.map(str::to_string), window: Some("Connection".into()), problem: Text::plain("The database could not be opened") }), ..Fake::default() }
}

#[test]
fn unconnected_the_window_that_keeps_the_connection_shows_first_saying_why() {
    let (mut screen, fake) = started_with(unconnected(None));
    assert_eq!(vec!["open Connection"], fake.calls());
    assert_eq!(vec!["Connection"], stack(&screen).windows());
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!("The database could not be opened", stack(&screen).said()[0].text.show());
    shot(&mut screen, "unconnected");
    // Closed without a change, the program ends.
    screen.key_pressed(KeyInput::plain(Key::Enter));
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(stack(&screen).ended());
    assert!(!fake.calls().contains(&"connect".to_string()));
}

#[test]
fn unconnected_a_changed_setting_connects_again_and_the_main_window_opens() {
    let (mut screen, fake) = started_with(Fake { connects: true, ..unconnected(None) });
    screen.act_root::<WindowStack, _>(|_, _| ());
    screen.key_pressed(KeyInput::plain(Key::Enter));
    click(&mut screen, "keep");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(vec!["dispatch done", "close Connection", "connect", "start", "open Main"], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert!(!stack(&screen).ended());
}

#[test]
fn unconnected_the_app_s_handler_runs_with_why_and_what_it_opens_shows() {
    let (screen, fake) = started_with(unconnected(Some("unconnected")));
    let calls = fake.calls();
    assert_eq!(r#"answer unconnected ["The database could not be opened"]"#, calls[0]);
    assert_eq!(vec!["Connection"], stack(&screen).windows());
}

#[test]
fn quit_ends_the_program() {
    let (mut screen, _) = started();
    click(&mut screen, "quit");
    assert!(stack(&screen).quitting());
    assert!(stack(&screen).ended());
}

#[test]
fn ctrl_z_in_a_dialog_takes_back_the_dialog_s_own_edit() {
    // The dialog's part with the focus gives the scope, on the dialog's window, not the one under it.
    let (mut screen, fake) = started();
    click(&mut screen, "openPick");
    fake.calls();
    let choose = named(&screen, "choose");
    screen.focus(Some(choose));
    screen.key_pressed(KeyInput::new(Key::Z, Modifiers::CONTROL));
    screen.key_pressed(KeyInput::new(Key::Y, Modifiers::CONTROL));
    assert_eq!(vec!["undo choose true", "undo choose false"], fake.calls());
    assert_eq!(vec!["Main", "Pick"], stack(&screen).windows());
}

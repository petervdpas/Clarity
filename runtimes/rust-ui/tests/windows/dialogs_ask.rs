//! A question a handler asks (DESIGN 5.14), as the C# tests have it
//! (AskDialogTests, AskParityTests, AskSizeTests and CheckFixWindowTests'
//! yes or no question): asked once the handler has finished, in Clarity's
//! own dialog over the window; the input its form needs, OK and Cancel; the
//! answer runs the handler it goes to, cancelling runs nothing; a day or a
//! moment gets a date input, an empty text answers an empty text, and the
//! dialog is as wide as its question between 320 and 520.

use super::dialogs_fake;

use clarity_runtime::ui::{Ask, AskForm, Text};
use clarity_ui::dialogs::AskDialog;
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::parts::inputs::date::DatePart;
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::number::NumberPart;
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::Choice;
use clarity_ui::theme::ThemeLook;
use clarity_ui::{Element, Screen};
use dialogs_fake::*;
use serde_json::{json, Value};

fn asking(screen: &Screen) -> &AskDialog {
    screen.find::<AskDialog>().expect("the question shows")
}

fn question(text: &str, form: AskForm) -> Ask {
    Ask { question: Text::plain(text), form, handler: "answered".into(), values: Vec::new() }
}

fn value_form(ty: &str, default: Option<Value>) -> AskForm {
    AskForm::Value { ty: ty.into(), of: None, only: None, default }
}

fn input_is<T: 'static>(ask: &AskDialog) -> bool {
    ask.input().is_some_and(|i| i.as_any().downcast_ref::<T>().is_some())
}

#[test]
fn a_question_shows_in_clarity_s_dialog_once_the_handler_has_finished() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "ask");
    // The handler ran to its end first; the question waits for the person.
    assert_eq!(vec!["dispatch ask"], fake.calls());
    let ask = asking(&screen);
    assert!(ask.input().is_none());
    assert_eq!(("Delete", "Keep"), (ask.ok().text.as_str(), ask.cancel().text.as_str()));
    assert!(ask.panel().width < 800.0 && ask.panel().x > 0.0);
    // OK answers: the dialog goes and the handler the answer goes to runs.
    let ok = ask.ok().node().id();
    click_at(&mut screen, ok);
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["answer confirmed [1]"], fake.calls());
}

#[test]
fn cancel_or_escape_runs_nothing() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "ask");
    let cancel = asking(&screen).cancel().node().id();
    click_at(&mut screen, cancel);
    assert!(screen.overlays().is_empty());
    click(&mut screen, "ask");
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["dispatch ask", "dispatch ask"], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
}

#[test]
fn a_value_is_typed_in_its_format_and_enter_answers() {
    let (mut screen, fake) = started();
    click(&mut screen, "askCopies");
    let ask = asking(&screen);
    assert!(input_is::<NumberPart>(ask));
    assert_eq!(Some(json!(2)), ask.value());
    // The input has the focus, so the person types at once.
    assert_eq!(Some(ask.first()), screen.focused());
    screen.key_pressed(KeyInput::plain(Key::End));
    screen.key_pressed(KeyInput::plain(Key::Backspace));
    screen.text_typed("3");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
    let calls = fake.calls();
    assert_eq!(1, calls.len(), "{calls:?}");
    assert!(calls[0].starts_with("answer copies [") && calls[0].contains('3'), "{calls:?}");
}

#[test]
fn an_answer_the_handler_refuses_is_an_error_message() {
    let (mut screen, _) = started();
    click(&mut screen, "askCopies");
    screen.key_pressed(KeyInput::plain(Key::End));
    screen.key_pressed(KeyInput::plain(Key::Backspace));
    screen.text_typed("0");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    screen.act_root::<clarity_ui::window::WindowStack, _>(|_, _| ());
    assert_eq!("At least one copy", stack(&screen).said().last().expect("the refusal is said").text.show());
}

#[test]
fn text_and_choices_get_their_inputs() {
    let text = AskDialog::new(question("Name?", value_form("text", Some(json!("Blue")))), None);
    assert!(input_is::<TextBoxPart>(&text));
    assert_eq!(Some(json!("Blue")), text.value());

    let friends = vec![Choice { value: json!(3), label: "Ada".into(), cells: None }, Choice { value: json!(5), label: "Grace".into(), cells: None }];
    let choice = AskDialog::new(question("Lend it to", value_form("record", None)), Some(friends.clone()));
    assert!(input_is::<DropdownPart>(&choice));
    assert_eq!(Some(json!(3)), choice.value());
    let keep = AskDialog::new(question("Lend it to", value_form("record", Some(json!(5)))), Some(friends));
    assert_eq!(Some(json!(5)), keep.value());

    let words = AskDialog::new(question("Sure?", AskForm::Confirm { yes: Some(Text::plain("Yes")), no: Some(Text::plain("Keep")) }), None);
    assert_eq!(("Yes", "Keep"), (words.ok().text.as_str(), words.cancel().text.as_str()));
}

#[test]
fn a_typed_text_is_the_answer() {
    let (mut screen, fake) = started();
    click(&mut screen, "askName");
    screen.key_pressed(KeyInput::plain(Key::End));
    screen.text_typed("s");
    fake.calls();
    let ok = asking(&screen).ok().node().id();
    click_at(&mut screen, ok);
    assert_eq!(vec![r#"answer named ["Blues"]"#], fake.calls());
}

#[test]
fn a_date_question_gets_a_date_input_and_answers_a_day() {
    let ask = AskDialog::new(question("When?", value_form("date", Some(json!("2026-11-02")))), None);
    let input = ask.input().and_then(|i| i.as_any().downcast_ref::<DatePart>()).expect("a date input");
    assert!(!input.is_moment());
    assert_eq!(Some(json!("2026-11-02")), ask.value());
}

#[test]
fn a_moment_question_gets_a_date_and_time_input_and_answers_a_moment() {
    let ask = AskDialog::new(question("When exactly?", value_form("datetime", Some(json!("2026-11-02T14:30:00")))), None);
    let input = ask.input().and_then(|i| i.as_any().downcast_ref::<DatePart>()).expect("a date input");
    assert!(input.is_moment());
    assert_eq!(Some(json!("2026-11-02T14:30:00")), ask.value());
}

#[test]
fn an_empty_text_answers_an_empty_text() {
    let ask = AskDialog::new(question("Name?", value_form("text", None)), None);
    assert_eq!(Some(json!("")), ask.value());
    // OK with nothing typed answers the empty text too.
    let (mut screen, fake) = started();
    click(&mut screen, "askEmpty");
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(vec![r#"answer named [""]"#], fake.calls());
}

#[test]
fn an_ask_dialog_is_as_wide_as_its_question_between_320_and_520() {
    fn width(text: &str) -> f64 {
        let ask = AskDialog::new(question(text, AskForm::Confirm { yes: None, no: None }), None);
        let mut screen = Screen::new(Box::new(ask), ThemeLook::clarity(false));
        screen.resize(1200.0, 800.0, 1.0);
        screen.root().as_any().downcast_ref::<AskDialog>().unwrap().panel().width
    }
    assert_eq!(320.0, width("Sure?"));
    assert_eq!(520.0, width(&vec!["a long question"; 20].join(" ")));
}

#[test]
fn a_yes_or_no_question_focuses_ok_and_enter_answers() {
    let (mut screen, fake) = started();
    click(&mut screen, "ask");
    let ask = asking(&screen);
    assert!(ask.input().is_none());
    let ok = ask.ok().node().id();
    assert_eq!(ok, ask.first());
    assert_eq!(Some(ok), screen.focused());
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(vec!["answer confirmed [1]"], fake.calls());
}

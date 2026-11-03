//! A language chosen while windows are open (C# LanguageRemakeTests,
//! LanguageFocusTests, CheckFixWindowTests.Language): every window shown is
//! made again in it and the title follows; the part with the focus keeps
//! it, so the next Tab goes on from it; a stack whose desktop window closed
//! no longer hears it. The language is the program's, so these run one at a time.

use super::core_windows_fake;

use std::sync::{Arc, Mutex};

use clarity_runtime::program::Program;
use clarity_ui::parts::label::LabelPart;
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Key, KeyInput, Screen};
use core_windows_fake::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const GREET: &str = r#"<Window name="Main">
  <Label name="greeting" at="1,1" span="6" caption="@hello" />
  <Textbox name="first" at="1,2" span="6" />
  <Textbox name="second" at="1,3" span="6" />
  <Button name="go" at="1,4" span="6" caption="Go" />
  <Button name="openOther" at="7,4" span="6" caption="Other" opens="Other" />
  <Button name="openAsk" at="1,5" span="6" caption="Ask" opens="Ask" />
</Window>"#;

const OTHER: &str = r#"<Window name="Other"><Label name="greeting" at="1,1" span="6" caption="@hello" /><Button name="back" at="1,2" span="3" caption="Back" onclick="done" /></Window>"#;

const ASK: &str = r#"<Window name="Ask"><Label name="askGreeting" at="1,1" span="6" caption="@hello" /></Window>"#;

const GREET_MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "translations": {"en": {"hello": "Hello", "greetTitle": "Greeting"}, "nl": {"hello": "Hallo", "greetTitle": "Groet"}},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "@greetTitle"},
      {"name": {"name": "Other"}, "kind": "window", "title": "@greetTitle"},
      {"name": {"name": "Ask"}, "kind": "dialog", "title": "@greetTitle"}
    ]}]
  }
}"#;

const GREET_FILES: AppFiles = AppFiles { windows: &[("Main", GREET), ("Other", OTHER), ("Ask", ASK)], apps: &[], themes: &[], model: GREET_MODEL, resources: &[], title: "Greet", width: 800.0, height: 400.0 };

fn greet() -> (Screen, Arc<Fake>) {
    clarity_ui::choices::start(&GREET_FILES, None);
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, GREET_FILES)), GREET_FILES.look(false));
    screen.resize(800.0, 400.0, 1.0);
    (screen, fake)
}

fn greeting(screen: &Screen) -> String {
    screen.element(named(screen, "greeting")).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text()
}

/// The language changes and the screen's next event shows it, drawn.
fn choose(screen: &mut Screen, language: &str) {
    clarity_ui::choices::choose_language(language);
    screen.act_root::<WindowStack, _>(|_, _| ());
    clarity_ui::testing::Shot::of(screen, 800, 400, 1.0);
}

/// The view shown now, by its element's id: a view made again is another.
fn view_of(screen: &Screen) -> clarity_ui::element::ElementId {
    clarity_ui::Element::node(stack(screen).view().unwrap()).id()
}

/// The name of the part the focus is in.
fn focused_part(screen: &Screen) -> Option<String> {
    let focused = screen.focused()?;
    let chain = clarity_ui::element::chain(screen.root(), focused)?;
    chain.iter().rev().filter_map(|id| screen.element(*id)).find_map(|e| e.as_part().and_then(|p| p.base().spec.name().map(str::to_string)))
}

#[test]
fn a_language_change_rebuilds_the_views_of_every_window_and_the_title_follows() {
    let _alone = super::one_at_a_time();
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (mut screen, _) = greet();
    assert_eq!(("Hello", "Greeting"), (greeting(&screen).as_str(), stack(&screen).title().as_str()));
    click(&mut screen, "openOther");
    choose(&mut screen, "nl");
    assert_eq!(("Hallo", "Groet"), (greeting(&screen).as_str(), stack(&screen).title().as_str()));
    // The window below is made again too: it shows in Dutch once it is on top.
    click(&mut screen, "back");
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert_eq!("Hallo", greeting(&screen));
    choose(&mut screen, "en");
}

#[test]
fn a_language_change_remakes_a_dialog_over_the_window_too() {
    let _alone = super::one_at_a_time();
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (mut screen, _) = greet();
    click(&mut screen, "openAsk");
    assert_eq!(1, screen.overlays().len());
    let label = |screen: &Screen| screen.element(named(screen, "askGreeting")).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text();
    assert_eq!("Hello", label(&screen));
    choose(&mut screen, "nl");
    assert_eq!("Hallo", label(&screen));
    assert_eq!("Hallo", greeting(&screen));
    choose(&mut screen, "en");
}

#[test]
fn a_language_change_keeps_the_focus_on_its_part() {
    let _alone = super::one_at_a_time();
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (mut screen, _) = greet();
    screen.key_pressed(KeyInput::plain(Key::Tab));
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some("second".to_string()), focused_part(&screen));
    choose(&mut screen, "nl");
    assert_eq!(Some("second".to_string()), focused_part(&screen));
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some("go".to_string()), focused_part(&screen));
    choose(&mut screen, "en");
}

#[test]
fn a_closed_window_no_longer_hears_language_changes() {
    let _alone = super::one_at_a_time();
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (mut closed, closed_fake) = greet();
    closed.act_root::<WindowStack, _>(|stack, cx| stack.closing(cx));
    assert!(stack(&closed).ended());
    closed_fake.calls();
    let (mut open, open_fake) = greet();
    open_fake.calls();
    let (closed_view, open_view) = (view_of(&closed), view_of(&open));
    choose(&mut closed, "nl");
    choose(&mut open, "nl");
    // The closed one stays as it was and tells the program nothing; the open one is made again.
    assert_eq!(closed_view, view_of(&closed));
    assert!(closed_fake.calls().is_empty());
    assert_ne!(open_view, view_of(&open));
    assert_eq!("Hallo", greeting(&open));
    assert_eq!(vec![r#"language Some("nl")"#], open_fake.calls());
    choose(&mut open, "en");
}

#[test]
fn a_value_kept_under_a_key_comes_back_and_can_be_forgotten() {
    let _alone = super::one_at_a_time();
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("clarity-kept-{}", std::process::id()));
    clarity_ui::choices::start(&GREET_FILES, Some(folder.clone()));
    clarity_ui::choices::keep_value("columns.Main.grid", r#"{"title":220}"#);
    assert_eq!(Some(r#"{"title":220}"#.to_string()), clarity_ui::choices::kept("columns.Main.grid"));
    // The next start reads it back, beside the other choices.
    clarity_ui::choices::choose_language("nl");
    clarity_ui::choices::start(&GREET_FILES, Some(folder.clone()));
    assert_eq!(Some(r#"{"title":220}"#.to_string()), clarity_ui::choices::kept("columns.Main.grid"));
    assert_eq!("nl", clarity_ui::choices::language());
    clarity_ui::choices::forget("columns.Main.grid");
    clarity_ui::choices::start(&GREET_FILES, Some(folder.clone()));
    assert_eq!(None, clarity_ui::choices::kept("columns.Main.grid"));
    clarity_ui::choices::choose_language("en");
    let _ = std::fs::remove_dir_all(folder);
}

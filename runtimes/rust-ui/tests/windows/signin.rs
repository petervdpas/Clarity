//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4): the
//! sign-in window shows before the main window when the start says so, a
//! `signin <strategy>` button signs in with the window's parts named as the
//! strategy's inputs, a refusal shows as refusals do, and `signout` closes
//! every window and shows the sign-in window again.

use super::core_windows_fake;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Screen};
use core_windows_fake::*;

const SIGN_IN: &str = r#"<Window name="SignIn">
  <Textbox name="name" at="1,1" span="12" />
  <Textbox name="password" at="1,2" span="12" />
  <Button name="go" at="1,3" span="6" caption="Sign in" onclick="signin password" />
</Window>
"#;

const HOME: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Orders" />
  <Button name="openPick" at="1,2" span="6" caption="Pick" opens="Pick" />
  <Button name="out" at="7,2" span="6" caption="Sign out" onclick="signout" />
</Window>
"#;

const MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "signin": {"entity": "Member", "targets": ["desktop"], "strategies": [{"name": "password", "fields": ["email", "passwordHash"]}], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "Pick"}, "kind": "dialog", "title": "Pick a color"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"}
    ]}]
  }
}"#;

const SIGNIN_FILES: AppFiles = AppFiles { windows: &[("Main", HOME), ("Pick", PICK), ("SignIn", SIGN_IN)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 500.0 };

fn signing_in() -> (Screen, Arc<Fake>) {
    let fake = Arc::new(Fake { signin: Some("SignIn".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, SIGNIN_FILES)), SIGNIN_FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    (screen, fake)
}

fn type_in(screen: &mut Screen, part: &str, text: &str) {
    click(screen, part);
    screen.text_typed(text);
}

#[test]
fn the_sign_in_window_shows_before_the_app_starts() {
    let (screen, fake) = signing_in();
    assert_eq!(vec!["open SignIn"], fake.calls());
    assert_eq!(vec!["SignIn"], stack(&screen).windows());
}

#[test]
fn a_refused_sign_in_says_why_and_stays_on_the_sign_in_window() {
    let (mut screen, fake) = signing_in();
    fake.calls();
    type_in(&mut screen, "name", "ann");
    type_in(&mut screen, "password", "wrong");
    click(&mut screen, "go");
    let calls = fake.calls();
    assert!(calls.contains(&r#"signin password {"name":"ann","password":"wrong"}"#.to_string()), "{calls:?}");
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(stack(&screen).said()[0].text.key.as_deref(), Some("signinRefused"));
    assert_eq!(vec!["SignIn"], stack(&screen).windows());
    assert!(!stack(&screen).ended());
}

#[test]
fn signing_in_opens_the_main_window_and_signing_out_shows_the_sign_in_window_again() {
    let (mut screen, fake) = signing_in();
    type_in(&mut screen, "name", "ann");
    type_in(&mut screen, "password", "right");
    fake.calls();
    click(&mut screen, "go");
    let calls = fake.calls();
    assert_eq!(calls.iter().filter(|c| !c.starts_with("edit")).cloned().collect::<Vec<_>>(), vec![r#"signin password {"name":"ann","password":"right"}"#, "close SignIn", "start", "open Main"]);
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert!(fake.signed_in.load(Ordering::SeqCst));
    // With a dialog open too: signing out closes every window. (Laid out as a host does before it draws.)
    screen.resize(800.0, 500.0, 1.0);
    click(&mut screen, "openPick");
    assert_eq!(vec!["Main", "Pick"], stack(&screen).windows());
    fake.calls();
    screen.act_root::<WindowStack, _>(|stack, cx| {
        let host = stack.current().unwrap().clone();
        use clarity_ui::PartHost;
        host.run(None, &clarity_ui::markup::HandlerCall::new("click", "signout", Vec::new()));
        let _ = cx;
    });
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec!["signout", "close Pick", "close Main", "open SignIn"], fake.calls());
    assert_eq!(vec!["SignIn"], stack(&screen).windows());
    assert!(!stack(&screen).ended());
}

#[test]
fn a_program_whose_people_need_not_sign_in_starts_as_it_did() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let screen = Screen::new(Box::new(WindowStack::start(program, SIGNIN_FILES)), SIGNIN_FILES.look(false));
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert_eq!(vec!["start", "open Main"], fake.calls());
}

// The splash doubles as the sign-in window ("One window does both").

const WELCOME: &str = r#"<Window name="Welcome" width="480" height="280" decorations="none" frame="none">
  <Label name="status" at="1,1" span="12" caption="Starting" />
  <Textbox name="name" at="1,2" span="12" />
  <Textbox name="password" at="1,3" span="12" />
  <Button name="go" at="1,4" span="6" caption="Sign in" onclick="signin password" />
</Window>"#;

const QUICK_APP: &str = r#"<App name="Shop" splash="Welcome"></App>"#;
const TIMED_APP: &str = r#"<App name="Shop" splash="Welcome" splashFor="3"></App>"#;

const SPLASH_SIGNS_IN: AppFiles = AppFiles { windows: &[("Main", HOME), ("Pick", PICK), ("Welcome", WELCOME)], apps: &[("Shop", QUICK_APP)], ..SIGNIN_FILES };

fn windows_of(driven: &mut clarity_ui::headless::Driven) -> Vec<String> {
    driven.screen().root().as_any().downcast_ref::<WindowStack>().unwrap().windows()
}

fn settle(driven: &mut clarity_ui::headless::Driven, times: usize) {
    for _ in 0..times {
        driven.step(&serde_json::json!({ "do": "settle", "ms": 20 }));
    }
}

fn starts(calls: &[String]) -> usize {
    calls.iter().filter(|c| *c == "start").count()
}

fn sign_in_on(driven: &mut clarity_ui::headless::Driven) {
    driven.screen().resize(480.0, 280.0, 1.0);
    type_in(driven.screen(), "name", "ann");
    type_in(driven.screen(), "password", "right");
    click(driven.screen(), "go");
}

#[test]
fn a_splash_that_signs_in_stays_after_on_start_and_its_sign_in_opens_the_main_window() {
    let fake = Arc::new(Fake { signin: Some("Welcome".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let mut driven = clarity_ui::headless::Driven::start_splashed(program, SPLASH_SIGNS_IN, None);
    settle(&mut driven, 20);
    assert_eq!(windows_of(&mut driven), vec!["Welcome"], "on start ran, the splash waits for a sign-in");
    let mut calls = fake.calls();
    assert_eq!(starts(&calls), 1, "{calls:?}");
    sign_in_on(&mut driven);
    settle(&mut driven, 3);
    assert_eq!(windows_of(&mut driven), vec!["Main"]);
    calls.extend(fake.calls());
    assert_eq!(starts(&calls), 1, "on start runs once: {calls:?}");
    // Signing out shows the splash as the sign-in window: no start, and it stays.
    driven.screen().act_root::<WindowStack, _>(|stack, _| {
        use clarity_ui::PartHost;
        stack.current().unwrap().clone().run(None, &clarity_ui::markup::HandlerCall::new("click", "signout", Vec::new()));
    });
    settle(&mut driven, 20);
    assert_eq!(windows_of(&mut driven), vec!["Welcome"]);
    assert_eq!(starts(&fake.calls()), 0);
}

#[test]
fn signed_in_while_the_splash_shows_its_time_the_main_window_follows_once_on_start_ran() {
    let fake = Arc::new(Fake { signin: Some("Welcome".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let files = AppFiles { apps: &[("Shop", TIMED_APP)], ..SPLASH_SIGNS_IN };
    let mut driven = clarity_ui::headless::Driven::start_splashed(program, files, None);
    settle(&mut driven, 1);
    sign_in_on(&mut driven);
    settle(&mut driven, 1);
    assert_eq!(windows_of(&mut driven), vec!["Welcome"], "the splash shows its time");
    let mut shown = Vec::new();
    for _ in 0..400 {
        let windows = windows_of(&mut driven);
        if windows == vec!["Main".to_string()] {
            shown = windows;
            break;
        }
        settle(&mut driven, 1);
    }
    assert_eq!(shown, vec!["Main"]);
    assert_eq!(starts(&fake.calls()), 1);
}

#[test]
fn a_sign_in_page_other_than_the_splash_skips_the_splash() {
    let fake = Arc::new(Fake { signin: Some("SignIn".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let files = AppFiles { windows: &[("Main", HOME), ("Welcome", WELCOME), ("SignIn", SIGN_IN)], ..SPLASH_SIGNS_IN };
    let mut driven = clarity_ui::headless::Driven::start_splashed(program, files, None);
    settle(&mut driven, 5);
    assert_eq!(windows_of(&mut driven), vec!["SignIn"]);
    assert_eq!(starts(&fake.calls()), 0, "on start runs after the sign-in");
}

/// On the desktop the splash is a desktop window of its own: the host runs
/// `on start` and hands its answer to the splash, which stays; after the
/// sign-in the main window opens in a desktop window of its own and the splash ends.
#[test]
fn on_the_desktop_the_signed_in_splash_asks_for_the_main_window_and_ends() {
    let fake = Arc::new(Fake { signin: Some("Welcome".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let running = WindowStack::prepare(program.clone(), SPLASH_SIGNS_IN);
    running.set_apart(true);
    let splash = WindowStack::splash_signing_in(running, "Welcome");
    let mut screen = Screen::new(Box::new(splash), SPLASH_SIGNS_IN.look(false));
    screen.resize(480.0, 280.0, 1.0);
    let reply = program.start_app();
    screen.act_root::<WindowStack, _>(move |stack, cx| stack.splash_started(reply, cx));
    assert_eq!(stack(&screen).windows(), vec!["Welcome"]);
    assert!(!stack(&screen).ended());
    type_in(&mut screen, "name", "ann");
    type_in(&mut screen, "password", "right");
    click(&mut screen, "go");
    let asked = screen.act_root::<WindowStack, _>(|stack, _| (stack.take_aparts(), stack.ended())).unwrap();
    assert!(matches!(asked.0.as_slice(), [clarity_ui::window::Apart::Main(Ok(_))]), "{:?}", asked.0);
    assert!(asked.1, "the splash ends once the main window is asked for");
    assert_eq!(starts(&fake.calls()), 1);
}

/// On the desktop, signing out shows the splash that is the sign-in window
/// as at the start: in a borderless desktop window of its own at its size
/// (the host opens it, then closes the main window's). No start runs and it
/// does not close by itself; a sign-in on it asks for the main window again.
#[test]
fn on_the_desktop_signing_out_shows_the_splash_in_its_own_window_again() {
    let fake = Arc::new(Fake { signin: Some("Welcome".into()), ..Fake::default() });
    fake.signed_in.store(true, Ordering::SeqCst);
    let program: Arc<dyn Program> = fake.clone();
    let running = WindowStack::prepare(program.clone(), SPLASH_SIGNS_IN);
    running.set_apart(true);
    let mut screen = Screen::new(Box::new(WindowStack::started(running.clone(), program.start_app())), SPLASH_SIGNS_IN.look(false));
    screen.resize(800.0, 500.0, 1.0);
    fake.calls();
    screen.act_root::<WindowStack, _>(|stack, _| {
        use clarity_ui::PartHost;
        stack.current().unwrap().clone().run(None, &clarity_ui::markup::HandlerCall::new("click", "signout", Vec::new()));
    });
    let asked = screen.act_root::<WindowStack, _>(|stack, _| (stack.take_aparts(), stack.ended())).unwrap();
    assert_eq!(asked, (vec![clarity_ui::window::Apart::Signin("Welcome".into())], true));
    assert_eq!(fake.calls(), vec!["signout", "close Main"]);
    // What the host opens for it: the splash, borderless at its size, nothing started.
    let again = WindowStack::splash_again(running, "Welcome");
    assert_eq!(again.windows(), vec!["Welcome"]);
    assert_eq!(again.wanted_size(), Some((480.0, 280.0)));
    assert!(!again.window_look().decorations);
    let mut screen = Screen::new(Box::new(again), SPLASH_SIGNS_IN.look(false));
    screen.resize(480.0, 280.0, 1.0);
    for _ in 0..3 {
        screen.act_root::<WindowStack, _>(|_, _| ());
    }
    assert_eq!(stack(&screen).windows(), vec!["Welcome"]);
    assert_eq!(fake.calls(), vec!["open Welcome"], "no start, no closing by itself");
    type_in(&mut screen, "name", "ann");
    type_in(&mut screen, "password", "right");
    click(&mut screen, "go");
    let asked = screen.act_root::<WindowStack, _>(|stack, _| (stack.take_aparts(), stack.ended())).unwrap();
    assert!(matches!(asked.0.as_slice(), [clarity_ui::window::Apart::Main(Ok(_))]), "{:?}", asked.0);
    assert!(asked.1);
    assert_eq!(starts(&fake.calls()), 1, "the new sign-in starts the app again");
}

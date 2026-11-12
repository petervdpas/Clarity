//! The sign-in window designed as views (docs/APP-CAPABILITIES-PLAN.md,
//! "The sign-in window is designed, and guided"): its content shows the
//! sign-in views one at a time; a sign-in button signs in with the fields of
//! its own view and "keep me signed in" there; a refusal shows in the view's
//! refusal; an "other way" shows another view; the language dropdown lists
//! the program's languages; every word is Clarity's own unless a caption
//! says others.

use super::core_windows_fake;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::parts::signin_view::RefusalPart;
use clarity_ui::window::WindowStack;
use clarity_ui::{element, AppFiles, ElementId, Screen};
use core_windows_fake::*;

const HOME: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Orders" />
</Window>
"#;

const SIGN_IN: &str = r#"<SignIn name="SignIn" width="420" height="500" startup="centerScreen" resizable="false" frame="none">
  <Content name="ways" at="1,1" span="12" rows="12">
    <View name="PasswordView" columns="12">
      <SigninLanguage name="language" at="1,1" span="12" rows="2" />
      <SigninField name="nameField" at="1,3" span="12" rows="2" input="name" />
      <SigninField name="passwordField" at="1,5" span="12" rows="2" input="password" />
      <SigninRemember name="remember" at="1,7" span="12" />
      <SigninButton name="go" at="1,8" span="12" strategy="password" />
      <SigninButton name="account" at="1,9" span="12" strategy="account" />
      <SigninOther name="useToken" at="1,10" span="12" shows="TokenView" />
      <SigninRefusal name="refusal" at="1,11" span="12" />
    </View>
    <View name="TokenView" columns="12">
      <SigninField name="tokenField" at="1,1" span="12" rows="2" input="token" caption="Your key" />
      <SigninButton name="tokenGo" at="1,3" span="12" strategy="token" />
      <SigninOther name="back" at="1,4" span="12" shows="PasswordView" />
      <SigninRefusal name="tokenRefusal" at="1,5" span="12" />
    </View>
  </Content>
</SignIn>
"#;

const MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en", "languages": ["en", "nl"]},
    "signin": {"entity": "Member", "targets": ["desktop"], "rememberAsks": true, "languagePicker": true, "strategies": [
      {"name": "password", "fields": ["email", "passwordHash"], "inputs": ["name", "password"], "targets": ["desktop", "web", "cli"]},
      {"name": "account", "fields": ["login"], "inputs": [], "targets": ["desktop", "cli"]},
      {"name": "token", "fields": ["apiToken"], "inputs": ["token"], "targets": ["desktop", "web", "cli"]}
    ], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"},
      {"name": {"name": "PasswordView"}, "kind": "view"},
      {"name": {"name": "TokenView"}, "kind": "view"}
    ]}]
  }
}"#;

const FILES: AppFiles = AppFiles { windows: &[("Main", HOME), ("SignIn", SIGN_IN)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 600.0 };

fn signing_in() -> (Screen, Arc<Fake>) {
    let fake = Arc::new(Fake { signin: Some("SignIn".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, FILES)), FILES.look(false));
    screen.resize(420.0, 500.0, 1.0);
    fake.calls();
    (screen, fake)
}

/// The element of the part named `name` that shows, in the window.
fn shown_named(screen: &Screen, name: &str) -> Option<ElementId> {
    fn walk(e: &dyn clarity_ui::Element, name: &str) -> Option<ElementId> {
        if !e.is_shown() {
            return None;
        }
        if e.as_part().and_then(|p| p.base().spec.name()) == Some(name) {
            return Some(e.node().id());
        }
        e.node().children().iter().find_map(|c| walk(c.as_ref(), name))
    }
    walk(screen.root(), name)
}

fn press(screen: &mut Screen, name: &str) {
    let id = shown_named(screen, name).unwrap_or_else(|| panic!("{name} does not show"));
    if name.ends_with(".link") {
        // A link is pressed on its words, at the left.
        let b = screen.element(id).unwrap().node().bounds;
        screen.click(b.x + 8.0, b.y + b.height / 2.0);
        return;
    }
    click_at(screen, id);
}

fn type_in(screen: &mut Screen, name: &str, text: &str) {
    press(screen, name);
    screen.text_typed(text);
}

fn sign_ins(calls: Vec<String>) -> Vec<String> {
    calls.into_iter().filter(|c| c.starts_with("signin")).collect()
}

fn caption(screen: &Screen, name: &str) -> String {
    let id = shown_named(screen, name).unwrap_or_else(|| panic!("{name} does not show"));
    let part = screen.element(id).and_then(|e| e.as_part()).unwrap();
    part.base().caption().unwrap_or_default()
}

#[test]
fn the_first_view_shows_and_its_words_are_clarity_s_own() {
    let (screen, _) = signing_in();
    assert!(shown_named(&screen, "passwordField.field").is_some());
    assert!(shown_named(&screen, "tokenField.field").is_none(), "one view at a time");
    assert_eq!(caption(&screen, "nameField.label"), "Name");
    assert_eq!(caption(&screen, "passwordField.label"), "Password");
    assert_eq!(caption(&screen, "remember.box"), "Keep me signed in");
    assert_eq!(caption(&screen, "go.button"), "Sign in");
    assert_eq!(caption(&screen, "account.button"), "Use this computer's account");
    // The other way is named after the way it shows.
    assert_eq!(caption(&screen, "useToken.link"), "Use a token");
    assert_eq!(caption(&screen, "language.label"), "Language");
}

#[test]
fn a_sign_in_button_signs_in_with_the_fields_of_its_view() {
    let (mut screen, fake) = signing_in();
    type_in(&mut screen, "nameField.field", "ann");
    type_in(&mut screen, "passwordField.field", "right");
    press(&mut screen, "remember.box");
    press(&mut screen, "go.button");
    assert_eq!(sign_ins(fake.calls()), vec![r#"signin password {"name":"ann","password":"right","remember":"true"}"#]);
    assert!(fake.signed_in.load(Ordering::SeqCst));
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(stack(&screen).windows(), vec!["Main"]);
}

#[test]
fn a_refusal_shows_in_the_view_s_refusal_and_the_window_stays() {
    let (mut screen, fake) = signing_in();
    type_in(&mut screen, "nameField.field", "ann");
    type_in(&mut screen, "passwordField.field", "wrong");
    press(&mut screen, "go.button");
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(stack(&screen).windows(), vec!["SignIn"]);
    let refusal = element::find_type::<RefusalPart>(screen.root(), |_| true).unwrap();
    assert_eq!(refusal.text().as_deref(), Some("Signing in did not work. Check what you typed and try again."));
    assert!(stack(&screen).said().is_empty(), "said in the view, not as a message");
    assert!(!fake.signed_in.load(Ordering::SeqCst));
}

#[test]
fn another_way_shows_its_view_and_signs_in_with_that_view_s_fields() {
    let (mut screen, fake) = signing_in();
    press(&mut screen, "useToken.link");
    // Laid out as a host does before it draws.
    screen.resize(420.0, 500.0, 1.0);
    assert!(shown_named(&screen, "passwordField.field").is_none());
    assert_eq!(caption(&screen, "tokenField.label"), "Your key", "a caption of its own");
    assert_eq!(caption(&screen, "back.link"), "Sign in with a password");
    type_in(&mut screen, "tokenField.field", "abc");
    press(&mut screen, "tokenGo.button");
    assert_eq!(sign_ins(fake.calls()), vec![r#"signin token {"token":"abc"}"#]);
    press(&mut screen, "back.link");
    screen.resize(420.0, 500.0, 1.0);
    assert!(shown_named(&screen, "passwordField.field").is_some());
}

#[test]
fn the_computer_s_account_signs_in_with_nothing_typed() {
    let (mut screen, fake) = signing_in();
    press(&mut screen, "account.button");
    assert_eq!(sign_ins(fake.calls()), vec![r#"signin account {"remember":"false"}"#]);
}



/// A provider's button asks for the browser's way to the provider, with the
/// settings the browser part needs and never the secret; what the browser
/// brings back signs in through the button's view, and a failure shows in
/// its refusal. (The browser itself is Clarity.Auth's, tested there.)
#[test]
fn a_provider_s_button_signs_in_with_what_the_browser_brings_back() {
    const GH_MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "signin": {"entity": "Member", "targets": ["desktop"], "strategies": [
      {"name": "github", "fields": ["email"], "inputs": [], "targets": ["desktop", "web"], "provider": {"client": "Ov23li", "secret": "setting:githubSecret", "register": false}}
    ], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"},
      {"name": {"name": "ProviderView"}, "kind": "view"}
    ]}]
  }
}"#;
    const GH_SIGN_IN: &str = r#"<SignIn name="SignIn" width="420" height="300" frame="none">
  <Content name="ways" at="1,1" span="12" rows="4">
    <View name="ProviderView" columns="12">
      <SigninButton name="github" at="1,1" span="12" strategy="github" />
      <SigninRefusal name="refusal" at="1,2" span="12" />
    </View>
  </Content>
</SignIn>
"#;
    let files = AppFiles { windows: &[("Main", HOME), ("SignIn", GH_SIGN_IN)], model: GH_MODEL, ..FILES };
    let fake = Arc::new(Fake { signin: Some("SignIn".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let screen = Screen::new(Box::new(WindowStack::start(program, files)), files.look(false));
    let host = stack(&screen).current().unwrap().clone();
    use clarity_ui::PartHost;
    fake.calls();
    assert!(host.run(Some("github.button"), &clarity_ui::markup::HandlerCall::new("click", "signin", vec!["github".into()])));
    let effects = host.take_effects();
    let [clarity_ui::window::host::Effect::Provider(signin)] = effects.as_slice() else { panic!("{effects:?}") };
    assert_eq!((signin.button.as_str(), signin.strategy.as_str()), ("github", "github"));
    assert_eq!(signin.options.get("client"), Some(&serde_json::json!("Ov23li")));
    assert!(!signin.options.contains_key("secret"), "the secret stays with the program");
    assert!(sign_ins(fake.calls()).is_empty(), "nothing signs in before the browser came back");
    // The browser came back: the view signs in with what it brought.
    let brought: serde_json::Map<String, serde_json::Value> = serde_json::from_value(serde_json::json!({"code": "abc", "verifier": "v", "redirect": "http://127.0.0.1:5/"})).unwrap();
    host.provider_answered(signin, Ok(brought));
    assert_eq!(sign_ins(fake.calls()), vec![r#"signin github {"code":"abc","redirect":"http://127.0.0.1:5/","verifier":"v"}"#]);
    // A browser that did not come back says why in the view's refusal.
    host.provider_answered(signin, Err("the sign-in took too long; sign in again to try once more".into()));
    assert_eq!(host.signin_refusal().map(|t| t.show()).as_deref(), Some("the sign-in took too long; sign in again to try once more"));
}

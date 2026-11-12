//! The sign-in part (docs/APP-CAPABILITIES-PLAN.md step 4, "The sign-in
//! part"): it draws a labelled field per input of each strategy of the
//! project's `signin` block that works here and a button per strategy, the
//! block's `shows` orders and limits them, "Keep me signed in" sends
//! `remember` when the block asks, its buttons and Enter sign in with what was
//! typed in its own fields, a refusal shows inside it, and the recovery code
//! swaps in and Back swaps out.

use super::core_windows_fake;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::parts::signin::SigninPart;
use clarity_ui::window::WindowStack;
use clarity_ui::{element, AppFiles, Element, Screen};
use core_windows_fake::*;

const HOME: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Orders" />
</Window>
"#;

const SIGN_IN: &str = r#"<Window name="SignIn">
  <Signin name="signin" at="1,1" span="12" />
</Window>
"#;

const MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "signin": {"entity": "Member", "targets": ["desktop"], "strategies": [
      {"name": "password", "fields": ["email", "passwordHash"], "inputs": ["name", "password"], "targets": ["desktop", "web", "cli"]},
      {"name": "account", "fields": ["login"], "inputs": [], "targets": ["desktop", "cli"]},
      {"name": "token", "fields": ["apiToken"]},
      {"name": "recovery", "fields": ["email", "codes"], "inputs": ["name", "code"], "targets": ["desktop", "web", "cli"]},
      {"name": "webOnly", "fields": ["x"], "inputs": ["name"], "targets": ["web"]}
    ], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"}
    ]}]
  }
}"#;

const FILES: AppFiles = AppFiles { windows: &[("Main", HOME), ("SignIn", SIGN_IN)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 600.0 };

/// The block shows token, then password, and asks to keep the person signed in.
const SHOWN_MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "signin": {"entity": "Member", "targets": ["desktop"], "shows": ["token", "password", "webOnly"], "rememberAsks": true, "strategies": [
      {"name": "password", "fields": ["email", "passwordHash"], "minimum": 12, "inputs": ["name", "password"], "targets": ["desktop", "web", "cli"]},
      {"name": "token", "fields": ["apiToken"], "inputs": ["token"], "targets": ["desktop", "web", "cli"]},
      {"name": "webOnly", "fields": ["x"], "inputs": ["name"], "targets": ["web"]}
    ], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"}
    ]}]
  }
}"#;

const SHOWN: AppFiles = AppFiles { model: SHOWN_MODEL, ..FILES };

fn signing_in(files: AppFiles) -> (Screen, Arc<Fake>) {
    let fake = Arc::new(Fake { signin: Some("SignIn".into()), ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, files)), files.look(false));
    screen.resize(800.0, 600.0, 1.0);
    fake.calls();
    (screen, fake)
}

fn part(screen: &Screen) -> &SigninPart {
    element::find_type::<SigninPart>(screen.root(), |_| true).expect("the window holds the sign-in part")
}

/// The names of the parts inside the sign-in part that show, in order.
fn shown(screen: &Screen) -> Vec<String> {
    fn walk(e: &dyn Element, out: &mut Vec<String>) {
        if !e.is_shown() {
            return;
        }
        if let Some(name) = e.as_part().and_then(|p| p.base().spec.name()) {
            out.push(name.to_string());
        }
        for child in e.node().children() {
            walk(child.as_ref(), out);
        }
    }
    let mut out = Vec::new();
    for child in part(screen).node().children() {
        walk(child.as_ref(), &mut out);
    }
    out
}

fn type_in(screen: &mut Screen, part: &str, text: &str) {
    click(screen, part);
    screen.text_typed(text);
}

fn without_edits(calls: Vec<String>) -> Vec<String> {
    calls.into_iter().filter(|c| !c.starts_with("edit")).collect()
}

#[test]
fn it_shows_a_field_per_input_and_a_button_per_strategy_that_works_here() {
    let (screen, _) = signing_in(FILES);
    assert_eq!(part(&screen).strategies(), vec!["password", "account", "token", "recovery"], "webOnly works on the web only");
    assert_eq!(
        shown(&screen),
        vec![
            "signin.password.name.label",
            "signin.password.name",
            "signin.password.password.label",
            "signin.password.password",
            "signin.password.button",
            "signin.account.button",
            "signin.token.token.label",
            "signin.token.token",
            "signin.token.button",
            "signin.useCode.button",
        ]
    );
    let kind = |name: &str| element::in_order(screen.root()).into_iter().find_map(|e| e.as_part().filter(|p| p.base().spec.name() == Some(name)).map(|p| p.base().spec.kind.clone())).unwrap();
    assert_eq!(kind("signin.password.password"), "password");
    assert_eq!(kind("signin.password.name"), "textbox");
    assert_eq!(kind("signin.token.token"), "textbox");
}

#[test]
fn its_words_are_clarity_s_own() {
    let (screen, _) = signing_in(FILES);
    let caption = |name: &str| element::in_order(screen.root()).into_iter().find_map(|e| e.as_part().filter(|p| p.base().spec.name() == Some(name)).map(|p| p.base().caption().unwrap_or_default())).unwrap();
    assert_eq!(caption("signin.password.name.label"), "Name");
    assert_eq!(caption("signin.password.password.label"), "Password");
    assert_eq!(caption("signin.password.button"), "Sign in");
    assert_eq!(caption("signin.account.button"), "Use this computer's account");
    assert_eq!(caption("signin.token.token.label"), "Token");
    assert_eq!(caption("signin.useCode.button"), "Use a recovery code");
}

#[test]
fn the_block_s_shows_orders_and_limits_what_it_offers_and_the_first_is_the_default() {
    let (screen, _) = signing_in(SHOWN);
    assert_eq!(part(&screen).strategies(), vec!["token", "password"], "webOnly works on the web only");
    assert_eq!(
        shown(&screen),
        vec!["signin.token.token.label", "signin.token.token", "signin.remember", "signin.token.button", "signin.password.name.label", "signin.password.name", "signin.password.password.label", "signin.password.password", "signin.password.button"]
    );
    let default = |name: &str| element::in_order(screen.root()).into_iter().find_map(|e| e.as_part().filter(|p| p.base().spec.name() == Some(name)).map(|p| p.base().spec.flag("default"))).unwrap();
    assert!(default("signin.token.button"));
    assert!(!default("signin.password.button"));
}

#[test]
fn keep_me_signed_in_goes_with_the_sign_in_as_remember() {
    let (mut screen, fake) = signing_in(SHOWN);
    type_in(&mut screen, "signin.password.name", "ann");
    type_in(&mut screen, "signin.password.password", "wrong");
    click(&mut screen, "signin.password.button");
    assert_eq!(without_edits(fake.calls()), vec![r#"signin password {"name":"ann","password":"wrong","remember":"false"}"#]);
    click(&mut screen, "signin.remember");
    click(&mut screen, "signin.password.button");
    assert_eq!(without_edits(fake.calls()), vec![r#"signin password {"name":"ann","password":"wrong","remember":"true"}"#]);
}

#[test]
fn without_remember_asks_there_is_no_check_box_and_no_remember_input() {
    let (mut screen, fake) = signing_in(FILES);
    assert!(!shown(&screen).iter().any(|n| n == "signin.remember"));
    click(&mut screen, "signin.account.button");
    assert_eq!(without_edits(fake.calls()), vec!["signin account {}"]);
}

#[test]
fn its_button_signs_in_with_its_own_fields_and_the_main_window_opens() {
    let (mut screen, fake) = signing_in(FILES);
    type_in(&mut screen, "signin.password.name", "ann");
    type_in(&mut screen, "signin.password.password", "right");
    click(&mut screen, "signin.password.button");
    let calls = without_edits(fake.calls());
    assert_eq!(calls, vec![r#"signin password {"name":"ann","password":"right"}"#, "close SignIn", "start", "open Main"]);
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert!(fake.signed_in.load(Ordering::SeqCst));
}

#[test]
fn enter_in_a_field_signs_in_with_that_field_s_strategy() {
    let (mut screen, fake) = signing_in(FILES);
    type_in(&mut screen, "signin.token.token", "abc");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    let calls = without_edits(fake.calls());
    assert_eq!(calls, vec![r#"signin token {"token":"abc"}"#]);
    type_in(&mut screen, "signin.password.name", "ann");
    type_in(&mut screen, "signin.password.password", "right");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(without_edits(fake.calls())[0], r#"signin password {"name":"ann","password":"right"}"#);
    assert_eq!(vec!["Main"], stack(&screen).windows());
}

#[test]
fn the_account_button_signs_in_with_no_inputs() {
    let (mut screen, fake) = signing_in(FILES);
    click(&mut screen, "signin.account.button");
    assert_eq!(without_edits(fake.calls()), vec!["signin account {}"]);
}

#[test]
fn a_refusal_shows_inside_the_part_and_the_window_stays() {
    let (mut screen, fake) = signing_in(FILES);
    type_in(&mut screen, "signin.password.name", "ann");
    type_in(&mut screen, "signin.password.password", "wrong");
    click(&mut screen, "signin.password.button");
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert!(fake.calls().iter().any(|c| c == r#"signin password {"name":"ann","password":"wrong"}"#));
    assert_eq!(part(&screen).own().refusal().as_deref(), Some("Signing in did not work. Check what you typed and try again."));
    assert!(stack(&screen).said().is_empty(), "the refusal shows in the part, not among the window's messages");
    assert_eq!(vec!["SignIn"], stack(&screen).windows());
    // Drawn with the refusal on its line, in the error color, under the fields.
    let line = part(&screen).node().bounds;
    let error = clarity_ui::ThemeLook::clarity(false).color("error");
    let mut shot = clarity_ui::testing::Shot::of(&mut screen, 800, 600, 1.0);
    let (x, y) = (line.x as i32, (line.bottom() - 26.0) as i32);
    let found = (y..y + 26).any(|y| (x..x + 200).any(|x| clarity_ui::testing::near(shot.pixel(x, y), error, 40)));
    assert!(found, "the refusal is drawn on the part's last line");
}

#[test]
fn the_recovery_code_swaps_in_with_the_name_typed_and_back_swaps_out() {
    let (mut screen, fake) = signing_in(FILES);
    type_in(&mut screen, "signin.password.name", "ann");
    click(&mut screen, "signin.useCode.button");
    screen.resize(800.0, 600.0, 1.0);
    assert_eq!(shown(&screen), vec!["signin.recovery.name.label", "signin.recovery.name", "signin.recovery.code.label", "signin.recovery.code", "signin.recovery.button", "signin.back.button"]);
    assert_eq!(screen.focused(), Some(named(&screen, "signin.recovery.name")), "the first field has the focus");
    type_in(&mut screen, "signin.recovery.code", "k3y");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(without_edits(fake.calls()), vec![r#"signin recovery {"code":"k3y","name":"ann"}"#]);
    assert!(part(&screen).own().refusal().is_some());
    click(&mut screen, "signin.back.button");
    screen.resize(800.0, 600.0, 1.0);
    assert!(part(&screen).own().refusal().is_none());
    assert_eq!(shown(&screen)[0], "signin.password.name.label");
}

#[test]
fn tab_goes_through_its_fields_and_buttons_in_order() {
    let (mut screen, _) = signing_in(FILES);
    let mut reached = Vec::new();
    for _ in 0..7 {
        screen.key_pressed(KeyInput::plain(Key::Tab));
        let focused = screen.focused().unwrap();
        reached.push(element::find(screen.root(), focused).and_then(|e| e.as_part()).and_then(|p| p.base().spec.name().map(str::to_string)).unwrap_or_default());
    }
    assert_eq!(reached, vec!["signin.password.name", "signin.password.password", "signin.password.button", "signin.account.button", "signin.token.token", "signin.token.button", "signin.useCode.button"]);
}

#[test]
fn its_fields_have_accessible_names() {
    let (screen, _) = signing_in(FILES);
    let tree = clarity_ui::access::tree(&screen);
    let name_of = |id| tree.nodes.iter().find(|n| n.id == id).map(|n| n.name.clone()).unwrap_or_default();
    assert_eq!(name_of(named(&screen, "signin.password.name")), "Name");
    assert_eq!(name_of(named(&screen, "signin.password.password")), "Password");
}

/// In the IDE's designer the part is a placeholder: no fields or buttons,
/// only a frame naming what it offers; the project's sign-in setup decides
/// the rest, and the running program draws it.
#[test]
fn in_the_designer_it_is_a_placeholder_naming_what_it_offers() {
    const DESIGNED: &str = r#"<Window name="SignIn" designItem="SignIn.cxaml">
  <Signin name="signin" at="1,1" span="12" rows="3" designItem="SignIn.cxaml:signin" />
</Window>
"#;
    let files = AppFiles { windows: &[("Main", HOME), ("SignIn", DESIGNED)], ..FILES };
    let (screen, _) = signing_in(files);
    assert!(shown(&screen).is_empty(), "{:?}", shown(&screen));
    assert_eq!(part(&screen).placeholder().as_deref(), Some("Sign in: password, account, token, recovery"));
    assert_eq!(part(&screen).around(), 0.0, "a placeholder asks no room beyond its rows");
}

/// With `language picker` the part offers the program's languages above its
/// fields, each by its own name, the one shown chosen; without it, none.
/// (Choosing one is the frame picker's `choose_language`, tested there.)
#[test]
fn its_language_picker_is_there_only_when_the_block_turns_it_on() {
    const PICKING: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "translations": {"en": {}, "nl": {}},
    "signin": {"entity": "Member", "targets": ["desktop"], "languagePicker": true, "strategies": [
      {"name": "password", "fields": ["email", "passwordHash"], "inputs": ["name", "password"], "targets": ["desktop", "web", "cli"]}
    ], "sessionMinutes": 480, "tries": 5, "triesMinutes": 15},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
      {"name": {"name": "SignIn"}, "kind": "signin", "title": "Sign in"}
    ]}]
  }
}"#;
    let (screen, _) = signing_in(AppFiles { model: PICKING, ..FILES });
    assert_eq!(shown(&screen)[..3], ["signin.language.label", "signin.language", "signin.password.name.label"]);
    let own = part(&screen).own().clone();
    use clarity_ui::parts::PartHost;
    let offered: Vec<(String, String)> = own.choices("signin.language").into_iter().map(|c| (c.value.as_str().unwrap_or_default().to_string(), c.label)).collect();
    // The languages the program offers, each by its own name, as the frame's picker offers them.
    let languages: Vec<(String, String)> = clarity_ui::choices::languages().into_iter().map(|o| (o.name, o.label)).collect();
    assert!(!languages.is_empty());
    assert_eq!(offered, languages);
    assert_eq!(own.read("signin.language"), Some(serde_json::Value::from(clarity_ui::choices::language())));
    let (plain, _) = signing_in(FILES);
    assert!(!shown(&plain).iter().any(|n| n.starts_with("signin.language")), "{:?}", shown(&plain));
}

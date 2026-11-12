//! The sign-in window is designed, and guided (docs/APP-CAPABILITIES-PLAN.md
//! step 4): window, content, views. Its content shows the sign-in views; a
//! sign-in view holds only the sign-in parts (fields, buttons, keep me
//! signed in, language, other way, refusal) and a heading, label, picture,
//! line or link; the window holds its content and that decoration. Each
//! sign-in part fits the project's signin block.

use clarity_spec::signin::{Signin, SigninStrategy};
use clarity_spec::{Project, Severity};

const CLUB: &str = "module club

entity Member
    email: email, unique, required
    passwordHash: text, hashed
    apiToken: text, hashed
    codes: text, hashed

app Club
    title \"Club\"
    main Main

page Main: window
    label \"Hello\" name hello at 1,1 span 4

page SignIn: signin
    heading \"Club\" name title at 1,1 span 12
    content name ways at 1,2 span 12 rows 10
        view PasswordView
        view CodeView

page PasswordView: view
    signinLanguage name language at 1,1 span 12 rows 2
    signinField name nameField at 1,3 span 12 rows 2 input name
    signinField name passwordField at 1,5 span 12 rows 2 input password
    signinRemember name remember at 1,7 span 12
    signinButton name signInButton at 1,8 span 12 strategy password
    signinOther name useCode at 1,9 span 12 shows CodeView
    signinRefusal name refusal at 1,10 span 12

page CodeView: view
    signinField name codeName at 1,1 span 12 rows 2 input name
    signinField name code at 1,3 span 12 rows 2 input code
    signinButton name codeButton at 1,5 span 12 strategy recovery
    signinOther name back at 1,6 span 12 shows PasswordView
    signinRefusal name codeRefusal at 1,7 span 12
";

fn signin() -> Signin {
    let mut signin = Signin::new("Member", vec!["desktop".into()]);
    signin.strategies.push(SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: None, provider: None });
    signin.strategies.push(SigninStrategy { name: "recovery".into(), fields: vec!["email".into(), "codes".into()], minimum: None, provider: None });
    signin.strategies.push(SigninStrategy { name: "token".into(), fields: vec!["apiToken".into()], minimum: None, provider: None });
    signin.remember_days = Some(30);
    signin.remember_asks = true;
    signin.language_picker = true;
    signin.line = 3;
    signin
}

fn load_with(source: &str, signin: &Signin) -> Project {
    let sources = clarity_spec::converted([("club.clarity", source)]);
    clarity_spec::load_sources_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), Some(signin))
}

fn errors_with(source: &str, signin: &Signin) -> Vec<String> {
    load_with(source, signin).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn errors(source: &str) -> Vec<String> {
    errors_with(source, &signin())
}

fn with(old: &str, new: &str) -> String {
    assert!(CLUB.contains(old), "{old:?}");
    CLUB.replacen(old, new, 1)
}

fn has(found: &[String], words: &str) {
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn a_sign_in_window_of_views_with_sign_in_parts_is_clean() {
    assert_eq!(errors(CLUB), Vec::<String>::new());
}

#[test]
fn the_catalog_says_where_each_part_may_stand() {
    use clarity_spec::widgets::{self, Category, WidgetKind};
    let field = widgets::info(WidgetKind::SigninField);
    assert_eq!(field.category, Category::Signin);
    assert!(field.signin_view && !field.signin_window);
    let heading = widgets::info(WidgetKind::Heading);
    assert!(heading.signin_view && heading.signin_window);
    let content = widgets::info(WidgetKind::Content);
    assert!(!content.signin_view && content.signin_window);
    let textbox = widgets::info(WidgetKind::Textbox);
    assert!(!textbox.signin_view && !textbox.signin_window);
}

#[test]
fn a_sign_in_view_holds_only_sign_in_parts_and_decoration() {
    let found = errors(&with("    signinRefusal name codeRefusal at 1,7 span 12\n", "    textbox name note at 1,7 span 12\n"));
    has(&found, "a sign-in view holds the sign-in parts and a heading, label, picture, line or link; a text box has no place on CodeView");
    let decorated = with("    signinRefusal name codeRefusal at 1,7 span 12\n", "    signinRefusal name codeRefusal at 1,7 span 12\n    separator name line at 1,8 span 12\n    link \"Help\" name help at 1,9 span 4 to \"https://example.com\"\n");
    assert_eq!(errors(&decorated), Vec::<String>::new());
}

#[test]
fn the_sign_in_window_holds_its_content_and_decoration() {
    let found = errors(&with("        view CodeView\n", "        view CodeView\n    textbox name note at 1,12 span 4\n"));
    has(&found, "the sign-in window SignIn holds its content, which shows the sign-in views, and a heading, label, picture, line or link; put the text box on a sign-in view");
    let part = errors(&with("        view CodeView\n", "        view CodeView\n    signin name signIn at 1,12 span 12 rows 6\n"));
    has(&part, "a signin part stands on the app's splash; the sign-in window is designed from its views");
}

#[test]
fn a_sign_in_part_belongs_on_a_sign_in_view() {
    let found = errors(&with("    label \"Hello\" name hello at 1,1 span 4\n", "    label \"Hello\" name hello at 1,1 span 4\n    signinRefusal name refusal at 1,2 span 4\n"));
    has(&found, "a refusal belongs on a sign-in view, a view the sign-in window's content shows");
}

#[test]
fn each_part_fits_the_signin_block() {
    has(&errors(&with("input password", "input pin")), "the project's strategies take name, password, code, token; not pin");
    has(&errors(&with("strategy recovery", "strategy pigeon")), "the project signs in with password, recovery, token; not pigeon");
    has(&errors(&with(" input password", "")), "passwordField says which input it takes: input name, password, code or token");
    has(&errors(&with(" strategy recovery", "")), "codeButton says which strategy it signs in with: strategy password, recovery or token");
    has(&errors(&with(" shows CodeView", "")), "useCode says which sign-in view it shows: shows CodeView");
    let mut plain = signin();
    plain.remember_asks = false;
    plain.language_picker = false;
    let found = errors_with(CLUB, &plain);
    has(&found, "keep me signed in asks whether to remember, which needs remember 30 days when asked in the signin block");
    has(&found, "the language dropdown needs language picker in the signin block");
}

#[test]
fn a_sign_in_button_needs_the_fields_of_its_strategy_on_its_view() {
    let found = errors(&with("    signinField name passwordField at 1,5 span 12 rows 2 input password\n", ""));
    has(&found, "signInButton signs in with password, which takes the password from a field on PasswordView (input password); it has none");
}

#[test]
fn other_way_shows_another_sign_in_view() {
    has(&errors(&with("shows CodeView", "shows Main")), "useCode shows a sign-in view, one the content of SignIn shows: PasswordView, CodeView; not Main");
    has(&errors(&with("shows CodeView", "shows PasswordView")), "useCode shows PasswordView, the view it stands on");
}

#[test]
fn a_sign_in_view_has_each_part_once() {
    let twice = with("    signinRefusal name refusal at 1,10 span 12\n", "    signinRefusal name refusal at 1,10 span 12\n    signinRefusal name again at 1,11 span 12\n");
    has(&errors(&twice), "PasswordView has a refusal already, refusal; a sign-in view has one");
    let fields = with("    signinRemember name remember at 1,7 span 12\n", "    signinField name nameAgain at 1,7 span 12 rows 2 input name\n    signinRemember name remember at 1,9 span 12\n");
    has(&errors(&fields), "PasswordView has a field for name already, nameField; a sign-in view has one");
}

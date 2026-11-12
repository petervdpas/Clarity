//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4), the
//! language side: the project file's `signin` block checked against the
//! entity of people, the field modifier `hashed`, `me`, `newToken()`, `allow
//! when` on actions, commands, windows and frame commands, an entity's
//! `readable when` and `writable when`, the window kind `signin` and the
//! click targets `signin <strategy>` and `signout`.

use clarity_spec::code::{ExprKind, FunctionKind};
use clarity_spec::model::PageKind;
use clarity_spec::signin::{Signin, SigninStrategy};
use clarity_spec::{Project, Severity};

const CLUB: &str = "module club

enum Role
    Admin
    Plain

entity Member
    name: text(80)
    email: email, unique, required
    passwordHash: text, hashed
    apiToken: text, hashed
    role: Role, default Plain
    readable when me.role == Admin or member.id == me.id
    writable when me.role == Admin

action promote(member: Member)
    allow when me.role == Admin
    effects db.write
    member.role = Admin
    save member

action newKey(member: Member) -> text
    effects db.write
    let token = newToken()
    member.apiToken = token
    save member
    return token

cli Club
    help \"The club\"

command members
    allow when me.role == Admin
    help \"Lists the members\"
    print \"{me.name}\"

app Club
    title \"Club\"
    main Main

page Main: window
    allow when me.role == Admin or me.role == Plain
    menu name mainMenu
        item \"Members\" name membersItem on click -> Main allow when me.role == Admin
        item \"Sign out\" name signOutItem on click -> signout
    label \"Hello\" name hello at 1,1 span 4
    button \"Greet\" name greet at 1,2 span 2 on click: greetClicked

page SignIn: signin
    content name ways at 1,1 span 12 rows 10
        view PasswordView
        view TokenView

page PasswordView: view
    signinField name nameField at 1,1 span 12 rows 2 input name
    signinField name passwordField at 1,3 span 12 rows 2 input password
    signinButton name signInButton at 1,5 span 12 strategy password
    signinOther name useToken at 1,6 span 12 shows TokenView

page TokenView: view
    signinField name tokenField at 1,1 span 12 rows 2 input token
    signinButton name tokenButton at 1,3 span 12 strategy token
    signinOther name back at 1,4 span 12 shows PasswordView

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

/// The club's sign-in window and its two sign-in views, as written in CLUB.
const GUIDED: &str = "page SignIn: signin
    content name ways at 1,1 span 12 rows 10
        view PasswordView
        view TokenView

page PasswordView: view
    signinField name nameField at 1,1 span 12 rows 2 input name
    signinField name passwordField at 1,3 span 12 rows 2 input password
    signinButton name signInButton at 1,5 span 12 strategy password
    signinOther name useToken at 1,6 span 12 shows TokenView

page TokenView: view
    signinField name tokenField at 1,1 span 12 rows 2 input token
    signinButton name tokenButton at 1,3 span 12 strategy token
    signinOther name back at 1,4 span 12 shows PasswordView
";

/// The hand-built sign-in: parts named after the inputs and a `signin password` button, which the splash may hold.
const HAND_BUILT: &str = "    textbox name name at 1,1 span 4\n    password name password at 1,2 span 4\n    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password\n";

fn signin() -> Signin {
    let mut signin = Signin::new("Member", vec!["desktop".into()]);
    signin.strategies.push(SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: None, provider: None });
    signin.strategies.push(SigninStrategy { name: "token".into(), fields: vec!["apiToken".into()], minimum: None, provider: None });
    signin.line = 3;
    signin
}

fn load_with(source: &str, signin: Option<&Signin>) -> Project {
    let sources = clarity_spec::converted([("club.clarity", source)]);
    clarity_spec::load_sources_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), signin)
}

fn load(source: &str) -> Project {
    load_with(source, Some(&signin()))
}

fn errors_of(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn errors(source: &str) -> Vec<String> {
    errors_of(&load(source))
}

fn with(old: &str, new: &str) -> String {
    assert!(CLUB.contains(old), "{old:?}");
    CLUB.replacen(old, new, 1)
}

fn has(found: &[String], words: &str) {
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn a_club_with_signin_is_clean() {
    assert_eq!(errors(CLUB), Vec::<String>::new());
}

#[test]
fn the_model_carries_what_the_generators_need() {
    let project = load(CLUB);
    let member = project.entity("Member").unwrap();
    assert!(member.people);
    assert!(member.field("passwordHash").unwrap().modifiers.hashed);
    assert!(!member.field("email").unwrap().modifiers.hashed);
    assert!(member.readable.is_some() && member.writable.is_some());
    let promote = project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == "promote").unwrap();
    assert_eq!(promote.kind, FunctionKind::Action);
    assert!(matches!(promote.allow.as_ref().map(|e| &e.kind), Some(ExprKind::Binary { .. })));
    let command = project.files.iter().flat_map(|f| &f.commands).next().unwrap();
    assert!(command.allow.is_some());
    let main = project.pages().find(|p| p.name.name == "Main").unwrap();
    assert_eq!(main.allow.as_deref(), Some("me.role == Admin or me.role == Plain"));
    assert!(main.allow_condition.is_some());
    let items = &main.frame.menu.as_ref().unwrap().items;
    assert_eq!(items[0].props.allow.as_deref(), Some("me.role == Admin"));
    assert!(items[0].props.allow_condition.is_some());
    assert_eq!(items[1].props.on_click.as_deref(), Some("signout"));
    let page = project.pages().find(|p| p.name.name == "SignIn").unwrap();
    assert_eq!(page.kind, PageKind::Signin);
    assert_eq!(page.kind.word(), "signin");
    assert_eq!(clarity_spec::signin::views_of(page), vec!["PasswordView", "TokenView"]);
    let view = project.pages().find(|p| p.name.name == "PasswordView").unwrap();
    let button = view.widgets.iter().find(|w| w.props.name.as_deref() == Some("signInButton")).unwrap();
    assert_eq!(button.props.strategy.as_deref(), Some("password"));
    let field = view.widgets.iter().find(|w| w.props.name.as_deref() == Some("passwordField")).unwrap();
    assert_eq!(field.props.input.as_deref(), Some("password"));
    assert_eq!(project.signin, Some(signin()));
}

#[test]
fn the_page_file_writes_and_reads_allow_and_the_signin_kind() {
    let sources = clarity_spec::converted([("club.clarity", CLUB)]);
    let main = &sources.iter().find(|(p, _)| p.ends_with("Main.cxaml")).unwrap().1;
    assert!(main.contains("<Window class=\"club.Main\" allow=\"me.role == Admin or me.role == Plain\""), "{main}");
    assert!(main.contains("opens=\"Main\" allow=\"me.role == Admin\""), "{main}");
    let page = &sources.iter().find(|(p, _)| p.ends_with("SignIn.cxaml")).unwrap().1;
    assert!(page.contains("<SignIn class=\"club.SignIn\""), "{page}");
    assert!(page.contains("<View name=\"PasswordView\" />"), "{page}");
    let view = &sources.iter().find(|(p, _)| p.ends_with("PasswordView.cxaml")).unwrap().1;
    assert!(view.contains("<SigninButton name=\"signInButton\" at=\"1,5\" span=\"12\" strategy=\"password\" />"), "{view}");
    assert!(view.contains("<SigninField name=\"passwordField\" at=\"1,3\" span=\"12\" rows=\"2\" input=\"password\" />"), "{view}");
    // Read back, the parts are the same.
    let project = clarity_spec::load_sources_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), Some(&signin()));
    assert_eq!(errors_of(&project), Vec::<String>::new());
}

#[test]
fn the_signin_block_fits_the_entity() {
    let mut other = signin();
    other.entity = "Person".into();
    let found = errors_of(&load_with(CLUB, Some(&other)));
    has(&found, "there is no entity Person for 'signin Person'");
    let found = errors_of(&load_with(&CLUB.replace("    email: email, unique, required\n", "    email: integer, unique, required\n"), Some(&signin())));
    has(&found, "email is integer, but the password strategy matches text or email");
    let found = errors(&with("passwordHash: text, hashed", "passwordHash: text"));
    has(&found, "passwordHash keeps the password strategy's secret, so it is hashed: passwordHash: text, hashed");
    let mut missing = signin();
    missing.strategies[0].fields[1] = "secret".into();
    has(&errors_of(&load_with(CLUB, Some(&missing))), "Member has no field secret for the password strategy");
    // The first account is made from the strategy's inputs, the other fields at their defaults.
    let found = errors(&with("name: text(80)", "name: text(80), required"));
    has(&found, "Member's name is required and has no default, so the password strategy could not make the first account: give name a default, or take 'required' away");
    // A salted hash is never equal to another.
    has(&errors(&with("passwordHash: text, hashed", "passwordHash: text, hashed, unique")), "passwordHash is hashed with a salt, so no two are equal: take 'unique' away");
    assert!(errors(&with("apiToken: text, hashed", "apiToken: text, hashed, unique")).is_empty(), "a token's hash is unique when the token is");
}

#[test]
fn a_hashed_field_is_text_set_by_code_and_never_read() {
    has(&errors(&with("apiToken: text, hashed", "apiToken: integer, hashed")), "only text is hashed, not integer");
    has(&errors(&with("apiToken: text, hashed", "apiToken: text(40), hashed")), "a hash is longer than 40 characters");
    has(&errors(&with("apiToken: text, hashed", "apiToken: text, hashed, search")), "a hashed field is not searched");
    has(&errors(&with("apiToken: text, hashed", "apiToken: text, hashed, default \"x\"")), "a hashed field has no default");
    let found = errors(&with("    return token\n", "    return member.apiToken\n"));
    has(&found, "apiToken is hashed: code sets it (member.apiToken = typed) and never reads it");
    let bound = with("    label \"Hello\" name hello at 1,1 span 4\n", "    data member: Member\n    textbox name hello at 1,1 span 4 bind member.passwordHash\n");
    has(&errors(&bound), "passwordHash is hashed, so no part shows it");
    // A record keeps values, never stored: nothing to hash.
    let record = with("enum Role\n", "record Shape\n    secret: text, hashed\n\nenum Role\n");
    has(&errors(&record), "only a stored entity's field is hashed");
}

#[test]
fn me_is_the_signed_in_record() {
    let found = errors(&with("hello.caption = \"Hello \" + me.name", "hello.caption = \"Hello \" + me.nickname"));
    has(&found, "Member has no field 'nickname'");
    has(&errors(&with("hello.caption = \"Hello \" + me.name", "me = none")), "me is the person signed in and cannot be set");
    // Without sign-in, me says what to write.
    let found = errors_of(&load_with(CLUB, None));
    has(&found, "me is who signed in: write signin and the entity of people in the project file, like: signin Member");
}

#[test]
fn me_is_not_in_a_function() {
    let source = with("cli Club\n", "function who() -> text\n    return me.name\n\ncli Club\n");
    has(&errors(&source), "'me' is not allowed in a function; make it an action");
}

#[test]
fn new_token_gives_text() {
    has(&errors(&with("let token = newToken()", "let token: integer = newToken()")), "cannot");
    has(&errors(&with("let token = newToken()", "let token = newToken(3)")), "'newToken' takes no arguments");
}

#[test]
fn allow_is_a_condition_that_needs_signin() {
    has(&errors(&with("    allow when me.role == Admin\n    effects", "    allow when me.name\n    effects")), "a condition must be bool, not text");
    has(&errors(&with("    allow when me.role == Admin\n    help", "    allow when 3\n    help")), "a condition must be bool, not integer");
    has(&errors(&with("allow when me.role == Admin or me.role == Plain", "allow when me.name")), "a condition must be bool, not text");
    has(&errors(&with("on click -> Main allow when me.role == Admin", "on click -> Main allow when me.nickname == \"x\"")), "Member has no field 'nickname'");
    has(&errors(&with("    save member\n\naction newKey", "    save member\n    allow when true\n\naction newKey")), "'allow' goes before the statements");
    has(&errors(&with("    allow when me.role == Admin\n    effects", "    allow when me.role == Admin\n    allow when true\n    effects")), "'allow when' is written once; join the conditions with 'or'");
    let found = errors_of(&load_with(&CLUB.replace("me.", "member."), None));
    has(&found, "'allow when' says who may: write signin and the entity of people in the project file, like: signin Member");
}

#[test]
fn readable_and_writable_name_the_record() {
    has(&errors(&with("readable when me.role == Admin or member.id == me.id", "readable when person.id == me.id")), "unknown name 'person'");
    has(&errors(&with("writable when me.role == Admin", "writable when member.name")), "a condition must be bool, not text");
    has(&errors(&with("    writable when me.role == Admin\n", "    writable when me.role == Admin\n    writable when true\n")), "'writable when' is written twice");
    let record = with("enum Role\n", "record Shape\n    side: integer\n    readable when true\n\nenum Role\n");
    has(&errors(&record), "a record is never stored, so it has no 'readable when'");
}

#[test]
fn the_signin_window() {
    // One at most.
    let two = format!("{CLUB}\npage Other: signin\n    label \"x\" name x at 1,1 span 2\n");
    has(&errors(&two), "there is already a sign-in window, SignIn");
    // Needed where people sign in on the desktop or the web.
    let none = CLUB.replace(GUIDED, "");
    has(&errors(&none), "people sign in on desktop, so the project needs a sign-in window: add a window of the kind signin");
    let mut cli = signin();
    cli.targets = vec!["cli".into()];
    cli.strategies.retain(|s| s.name == "token");
    assert!(!errors_of(&load_with(&none, Some(&cli))).iter().any(|m| m.contains("sign-in window")));
    // Not without sign-in.
    let plain = errors_of(&load_with(&CLUB.replace("me.", "member.").replace("allow when", "// allow when"), None));
    has(&plain, "SignIn is a sign-in window, which needs signin in the project file, like: signin Member");
    // Nothing opens it, and it has no menus.
    has(&errors(&with("on click -> Main allow", "on click -> SignIn allow")), "SignIn is the sign-in window, which the program shows itself; nothing opens it");
    let menu = with("page SignIn: signin\n", "page SignIn: signin\n    menu\n        item \"x\" name xItem on click -> quit\n");
    has(&errors(&menu), "only windows have menus, toolbars, ribbons, sidebars and footers; a sign-in window has none");
}

#[test]
fn the_click_targets_signin_and_signout() {
    has(&errors(&on_splash().replace("on click -> signin password", "on click -> signin pigeon")), "the project signs in with password, token, not pigeon");
    let missing = on_splash().replace("    password name password at 1,2 span 4\n", "    password name secret at 1,2 span 4\n");
    has(&errors(&missing), "signin password takes the password from a part named password; Welcome has none");
    let elsewhere = with("button \"Greet\" name greet at 1,2 span 2 on click: greetClicked", "button \"Greet\" name greet at 1,2 span 2 on click -> signin password");
    has(&errors(&elsewhere), "signin password signs in from a button on the app's splash; the sign-in window signs in with its sign-in buttons");
    let plain = errors_of(&load_with(&on_splash().replace("me.", "member.").replace("allow when", "// allow when"), None));
    has(&plain, "signout signs out who signed in, which needs signin in the project file");
    has(&plain, "signin password signs in, which needs signin in the project file");
    has(&errors(&with("item \"Sign out\" name signOutItem on click -> signout", "item \"Sign out\" name signOutItem on click -> signout in new window")), "only a window opens in a new window");
}

#[test]
fn signin_words_are_used_only_with_signin() {
    let keys = |project: &Project| clarity_spec::translations::keys(project).into_iter().map(|k| k.key).collect::<Vec<_>>();
    let with_signin = keys(&load(CLUB));
    for key in ["signinRefused", "signinTooManyTries", "notAllowed", "notYours"] {
        assert!(with_signin.iter().any(|k| k == key), "{key}: {with_signin:?}");
    }
    let without = keys(&load_with("module m\n\nentity Note\n    text: text\n", None));
    assert!(!without.iter().any(|k| k == "signinRefused"), "{without:?}");
}

#[test]
fn the_signin_template_makes_a_window_of_sign_in_views() {
    use clarity_spec::design_edit::Designed;
    use clarity_spec::templates::{PageTemplate, new_window};
    use clarity_spec::widgets::WidgetKind;
    // The computer's account makes the first member from the name alone: the e-mail is not required.
    let none = CLUB.replace(GUIDED, "").replace("email: email, unique, required", "email: email, unique");
    let mut block = signin();
    block.strategies.push(SigninStrategy { name: "account".into(), fields: vec!["name".into()], minimum: None, provider: None });
    block.remember_days = Some(30);
    block.remember_asks = true;
    block.language_picker = true;
    let project = load_with(&none, Some(&block));
    let made = new_window(&project, "SignIn", &PageTemplate::SignIn).unwrap();
    let pages: Vec<_> = made.designed.iter().map(|d| match d { Designed::Page(p) => p, Designed::App(_) => panic!("pages only") }).collect();
    let names: Vec<&str> = pages.iter().map(|p| p.name.name.as_str()).collect();
    assert_eq!(names, vec!["SignIn", "PasswordView", "TokenView"]);
    assert_eq!(pages[0].kind, PageKind::Signin);
    assert_eq!(clarity_spec::signin::views_of(pages[0]), vec!["PasswordView", "TokenView"]);
    // The password view: language, name, password, keep me signed in, its button, the account's, the other way, the refusal.
    let kinds = |page: &clarity_spec::model::Page| page.widgets.iter().map(|w| (w.props.kind, w.props.name.clone().unwrap())).collect::<Vec<_>>();
    assert_eq!(kinds(pages[1]), vec![
        (WidgetKind::SigninLanguage, "passwordLanguage".to_string()),
        (WidgetKind::SigninField, "passwordNameField".to_string()),
        (WidgetKind::SigninField, "passwordPasswordField".to_string()),
        (WidgetKind::SigninRemember, "passwordRemember".to_string()),
        (WidgetKind::SigninButton, "passwordButton".to_string()),
        (WidgetKind::SigninButton, "accountButton".to_string()),
        (WidgetKind::SigninOther, "passwordToToken".to_string()),
        (WidgetKind::SigninRefusal, "passwordRefusal".to_string()),
    ]);
    assert_eq!(pages[1].widgets[6].props.shows.as_deref(), Some("TokenView"));
    // Written into the project, it is the sign-in window the project needed, and clean.
    let mut sources = clarity_spec::converted([("club.clarity", none.as_str())]);
    for page in &pages {
        sources.push((format!("design/{}.cxaml", page.name.name), clarity_spec::cxaml::page_source(page, "club")));
    }
    let project = clarity_spec::load_sources_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), Some(&block));
    assert_eq!(errors_of(&project), Vec::<String>::new());
}

#[test]
fn new_recovery_codes_gives_text_lines() {
    let source = CLUB.replace("    apiToken: text, hashed\n", "    apiToken: text, hashed\n    codes: text, hashed\n").replace("    return token\n", "    member.codes = newRecoveryCodes(10)\n    return token\n");
    assert_eq!(errors(&source), Vec::<String>::new());
    has(&errors(&source.replace("newRecoveryCodes(10)", "newRecoveryCodes(\"ten\")")), "'newRecoveryCodes' takes how many codes, a whole number, not text");
    has(&errors(&source.replace("newRecoveryCodes(10)", "newRecoveryCodes()")), "'newRecoveryCodes' takes how many codes: newRecoveryCodes(10)");
}

#[test]
fn a_recovery_button_takes_a_name_and_a_code() {
    let source = on_splash().replace("    apiToken: text, hashed\n", "    apiToken: text, hashed\n    codes: text, hashed\n").replace(
        "on click -> signin password\n",
        "on click -> signin password\n    button \"Use a code\" name useCode at 3,3 span 2 on click -> signin recovery\n",
    );
    let mut with_recovery = signin();
    with_recovery.strategies.push(SigninStrategy { name: "recovery".into(), fields: vec!["email".into(), "codes".into()], minimum: None, provider: None });
    has(&errors_of(&load_with(&source, Some(&with_recovery))), "signin recovery takes the code from a part named code; Welcome has none");
    let fixed = source.replace("    password name password at 1,2 span 4\n", "    password name password at 1,2 span 4\n    textbox name code at 5,2 span 4\n");
    assert_eq!(errors_of(&load_with(&fixed, Some(&with_recovery))), Vec::<String>::new());
}

#[test]
fn rules_and_the_command_line_bring_their_words() {
    let keys = |signin: &Signin| clarity_spec::translations::keys(&load_with(CLUB, Some(signin))).into_iter().map(|k| k.key).collect::<Vec<_>>();
    let plain = keys(&signin());
    assert!(!plain.iter().any(|k| k == "secretTooShort" || k == "cliPassword"), "{plain:?}");
    let mut ruled = signin();
    ruled.strategies[0].minimum = Some(12);
    ruled.targets.push("cli".into());
    let found = keys(&ruled);
    assert!(found.iter().any(|k| k == "secretTooShort") && found.iter().any(|k| k == "cliPassword"), "{found:?}");
}

/// Clarity's words of signing in are one text everywhere: the English every
/// runtime refuses with (conformance/signin.json) is the spec's own.
#[test]
fn the_runtimes_refuse_in_clarity_s_own_words() {
    let file: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/signin.json")).unwrap();
    let refusals = file["refusals"].as_object().unwrap();
    let own: Vec<(&str, &str)> = clarity_spec::translations::SIGNIN_KEYS.iter().chain(clarity_spec::translations::SECRET_RULE_KEYS.iter()).copied().collect();
    assert_eq!(refusals.len(), own.len(), "{refusals:?}");
    for (key, english) in own {
        assert_eq!(refusals.get(key).and_then(|v| v.as_str()), Some(english), "{key}");
    }
}

/// The club with its sign-in parts on the app's splash, not on a window of their own.
fn on_splash() -> String {
    CLUB.replace("    main Main\n", "    main Main\n    splash Welcome\n").replace(GUIDED, &format!("page Welcome: splash\n{HAND_BUILT}"))
}

#[test]
fn the_app_s_splash_doubles_as_the_signin_window() {
    assert_eq!(errors(&on_splash()), Vec::<String>::new());
    let missing = on_splash().replace("    password name password at 1,2 span 4\n", "");
    has(&errors(&missing), "signin password takes the password from a part named password; Welcome has none");
    // Both is one too many.
    let both = format!("{}\n{GUIDED}", on_splash());
    has(&errors(&both), "both SignIn and the splash Welcome sign in; keep one");
    // A splash the app does not show signs no one in.
    let other = on_splash().replace("    splash Welcome\n", "");
    has(&errors(&other), "signin password signs in from a button on the app's splash");
    // It is still a splash: nothing opens it.
    has(&errors(&on_splash().replace("on click -> Main allow", "on click -> Welcome allow")), "Welcome is the splash screen");
}

/// The club's splash holding the sign-in part instead of the sign-in window.
fn with_part(part: &str) -> String {
    CLUB.replace("    main Main\n", "    main Main\n    splash Welcome\n").replace(GUIDED, &format!("page Welcome: splash\n    {part}\n"))
}

#[test]
fn the_signin_part_signs_in_on_its_own() {
    let source = with_part("signin name signIn at 1,1 span 12 rows 6");
    let project = load(&source);
    assert_eq!(errors_of(&project), Vec::<String>::new());
    let page = project.pages().find(|p| p.name.name == "Welcome").unwrap();
    let part = &page.widgets[0];
    assert_eq!(part.props.kind, clarity_spec::widgets::WidgetKind::Signin);
    assert!(clarity_spec::frame::page_signs_in(page));
    // The part keeps only its place and look; the project's block says what it offers.
    has(&errors(&with_part("signin name signIn at 1,1 span 12 rows 6 strategies password")), "strategies");
    let sources = clarity_spec::converted([("club.clarity", source.as_str())]);
    let markup = &sources.iter().find(|(p, _)| p.ends_with("Welcome.cxaml")).unwrap().1;
    assert!(markup.contains("<Signin name=\"signIn\" at=\"1,1\" span=\"12\" rows=\"6\" />"), "{markup}");
}

#[test]
fn the_signin_part_follows_the_rules_of_signing_in() {
    // Only on the app's splash.
    let elsewhere = with_part("signin name signIn at 1,1 span 12 rows 6").replace("    label \"Hello\" name hello at 1,1 span 4\n", "    label \"Hello\" name hello at 1,1 span 4\n    signin name other at 1,4 span 12 rows 6\n");
    has(&errors(&elsewhere), "a signin part stands on the app's splash; the sign-in window is designed from its views");
    // One in a project.
    has(&errors(&elsewhere), "there is already a signin part, on Main; a project has one");
    // Not without sign-in.
    let plain = errors_of(&load_with(&with_part("signin name signIn at 1,1 span 12 rows 6").replace("me.", "member.").replace("allow when", "// allow when"), None));
    has(&plain, "a signin part signs in, which needs signin in the project file, like: signin Member");
}

#[test]
fn the_signin_part_on_the_splash_makes_it_the_signin_window() {
    let splash = on_splash().replace(HAND_BUILT, "    signin name signIn at 1,1 span 12 rows 6\n");
    assert_eq!(errors(&splash), Vec::<String>::new());
    let both = format!("{splash}\npage SignIn: signin\n    label \"x\" name x at 1,1 span 2\n");
    has(&errors(&both), "both SignIn and the splash Welcome sign in; keep one");
}

#[test]
fn the_model_gives_each_strategy_its_inputs_and_targets() {
    let mut block = signin();
    block.shows = vec!["token".into(), "password".into()];
    block.remember_days = Some(30);
    block.remember_asks = true;
    let project = load_with(&with_part("signin name signIn at 1,1 span 12 rows 6"), Some(&block));
    let json = serde_json::to_value(&project).unwrap();
    assert_eq!(json["signin"]["shows"], serde_json::json!(["token", "password"]));
    assert_eq!((json["signin"]["rememberDays"].clone(), json["signin"]["rememberAsks"].clone()), (serde_json::json!(30), serde_json::json!(true)));
    let strategies = json["signin"]["strategies"].as_array().unwrap();
    assert_eq!(strategies[0]["name"], "password");
    assert_eq!(strategies[0]["inputs"], serde_json::json!(["name", "password"]));
    assert_eq!(strategies[0]["targets"], serde_json::json!(["desktop", "web", "cli"]));
    assert_eq!(strategies[1]["inputs"], serde_json::json!(["token"]));
    // Read back, the block is the same.
    let back: Signin = serde_json::from_value(json["signin"].clone()).unwrap();
    assert_eq!(back, block);
    let page = json["files"].as_array().unwrap().iter().flat_map(|f| f["pages"].as_array().unwrap()).find(|p| p["name"]["name"] == "Welcome").unwrap();
    assert_eq!(page["widgets"][0]["props"]["kind"], "signin");
    assert!(page["widgets"][0]["props"].get("strategies").is_none());
}

#[test]
fn the_signin_part_brings_its_words() {
    let keys = |source: &str| clarity_spec::translations::keys(&load(source)).into_iter().map(|k| k.key).collect::<Vec<_>>();
    let part = keys(&with_part("signin name signIn at 1,1 span 12 rows 6"));
    for (key, _) in clarity_spec::translations::SIGNIN_PART_KEYS {
        assert!(part.iter().any(|k| k == key), "{key}: {part:?}");
    }
    assert!(!part.iter().any(|k| k == "signinRemember"), "{part:?}");
    let hand = keys(&on_splash());
    assert!(!hand.iter().any(|k| k == "signinButton"), "{hand:?}");
    // The keep-me-signed-in box comes with `remember 30 days when asked`.
    let mut asks = signin();
    asks.remember_days = Some(30);
    asks.remember_asks = true;
    let found = clarity_spec::translations::keys(&load_with(&with_part("signin name signIn at 1,1 span 12 rows 6"), Some(&asks))).into_iter().map(|k| k.key).collect::<Vec<_>>();
    assert!(found.iter().any(|k| k == "signinRemember"), "{found:?}");
    // Its language picker, when the block turns it on, brings the picker's word.
    assert!(!part.iter().any(|k| k == "languagePicker"), "{part:?}");
    let mut picks = signin();
    picks.language_picker = true;
    let found = clarity_spec::translations::keys(&load_with(&with_part("signin name signIn at 1,1 span 12 rows 6"), Some(&picks))).into_iter().map(|k| k.key).collect::<Vec<_>>();
    assert!(found.iter().any(|k| k == "languagePicker"), "{found:?}");
}

/// A provider needs what it signs in with: its client, an OpenID
/// provider its issuer, and GitHub the app's secret to trade the code.
#[test]
fn a_provider_says_what_it_needs() {
    use clarity_spec::signin::ProviderSettings;
    let with = |name: &str, settings: Option<ProviderSettings>| {
        let mut block = signin();
        block.strategies.push(SigninStrategy { name: name.into(), fields: vec!["email".into()], minimum: None, provider: settings });
        errors_of(&load_with(CLUB, Some(&block)))
    };
    has(&with("microsoft", None), "the microsoft strategy needs its client, the app's id at Microsoft: write client \"...\" indented under its line");
    has(&with("openid", Some(ProviderSettings { client: "app".into(), ..Default::default() })), "the openid strategy needs its issuer, the provider's address: write issuer \"https://...\" indented under its line");
    has(&with("github", Some(ProviderSettings { client: "app".into(), ..Default::default() })), "GitHub trades the code for a token only with the app's secret: write secret \"setting:githubSecret\" indented under its line");
    assert_eq!(with("microsoft", Some(ProviderSettings { client: "app".into(), ..Default::default() })), Vec::<String>::new());
    has(&with("github", Some(ProviderSettings { client: "app".into(), secret: Some("setting:githubSecret".into()), ..Default::default() })), "the github strategy's secret setting:githubSecret is no secret setting of the project: add githubSecret: secret to the settings");
    // Declared as a secret setting, it is kept in the program's protected store.
    let mut block = signin();
    block.strategies.push(SigninStrategy { name: "github".into(), fields: vec!["email".into()], minimum: None, provider: Some(ProviderSettings { client: "app".into(), secret: Some("setting:githubSecret".into()), ..Default::default() }) });
    let kept = CLUB.replace("cli Club\n", "settings\n    githubSecret: secret\n\ncli Club\n");
    assert_eq!(errors_of(&load_with(&kept, Some(&block))), Vec::<String>::new());
}

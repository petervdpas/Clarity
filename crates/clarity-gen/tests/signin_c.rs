//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4) in C:
//! the program carries Clarity.Auth only when its project signs people in,
//! names its people after the databases open (and again on `connect`),
//! reads `me` through the runtime, refuses what `allow when`, `readable
//! when` and `writable when` do not let through, hashes text assigned to a
//! hashed field, and its command line signs in before a command runs.

use clarity_spec::signin::{Signin, SigninStrategy};

const CLUB: &str = "module club

enum Role
    Admin
    Plain

entity Member
    name: text(80)
    email: email, unique, required
    passwordHash: text, hashed
    apiToken: text, hashed
    recoveryCodes: text, hashed
    role: Role, default Plain
    photo: image
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

action newCodes(member: Member) -> text
    effects db.write
    let codes = newRecoveryCodes(8)
    member.recoveryCodes = codes
    save member
    return codes

action setPassword(member: Member, typed: text)
    effects db.write
    member.passwordHash = typed
    save member

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
        item \"Greet all\" name greetItem on click: greetClicked allow when me.role == Admin
        item \"About\" name aboutItem on click: aboutClicked allow when me.role == Plain
        item \"Sign out\" name signOutItem on click -> signout
    label \"Hello\" name hello at 1,1 span 4
    button \"Greet\" name greet at 1,2 span 2 on click: greetClicked

page SignIn: signin
    content name ways at 1,1 span 12 rows 6
        view PasswordView

page PasswordView: view
    signinField name nameField at 1,1 span 12 rows 2 input name
    signinField name passwordField at 1,3 span 12 rows 2 input password
    signinButton name signInButton at 1,5 span 12 strategy password

code Club
    handler aboutClicked
        message \"Club\"

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

const PLAIN: &str = "module notes\n\nentity Note\n    text: text(200)\n\npage Notes: window\n    data notes: list<Note>\n    table name list at 1,1 span 4 bind notes\n";

fn signin() -> Signin {
    let mut signin = Signin::new("Member", vec!["desktop".into(), "cli".into()]);
    signin.strategies.push(SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: Some(12), provider: None });
    signin.strategies.push(SigninStrategy { name: "token".into(), fields: vec!["apiToken".into()], minimum: None, provider: None });
    signin.strategies.push(SigninStrategy { name: "recovery".into(), fields: vec!["email".into(), "recoveryCodes".into()], minimum: None, provider: None });
    signin.session_minutes = 120;
    signin
}

const CLUB_STEP: &str = "migration \"Members\"\n    add entity Member\n        name: text(80)\n        email: email, unique, required\n        passwordHash: text\n        apiToken: text\n        recoveryCodes: text\n        role: Role, default Plain\n        photo: image\n";
const PLAIN_STEP: &str = "migration \"Notes\"\n    add entity Note\n        text: text(200)\n";

fn files(source: &str, signs_in: bool, cli: bool) -> Vec<clarity_gen::OutFile> {
    files_with(source, signs_in.then(signin), cli)
}

fn files_with(source: &str, signin: Option<Signin>, cli: bool) -> Vec<clarity_gen::OutFile> {
    let signs_in = signin.is_some();
    let signin = signin.as_ref();
    let sources = clarity_spec::converted([("club/club.clarity", source)]);
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", if signs_in { CLUB_STEP } else { PLAIN_STEP })], &[], signin);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let target = if cli { "cli" } else { "desktop" };
    let mut settings = clarity_project::parse(&format!("project Club\n    stack c {target}\n    datasource main sqlite \"data/club.db\"\n")).unwrap();
    settings.signin = signin.cloned();
    if cli { clarity_gen::generate_cli(&project, &settings) } else { clarity_gen::generate(&project, &settings) }.unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

#[test]
fn only_a_program_whose_people_sign_in_carries_clarity_auth() {
    for cli in [false, true] {
        let signs = files(CLUB, true, cli);
        has(file(&signs, "CMakeLists.txt"), "clarity_use_auth(");
        assert!(signs.iter().any(|f| f.path == "clarity-runtime/auth/auth.c"), "cli {cli}: auth.c");
        has(file(&signs, "src/records.h"), "#include \"auth/auth.h\"");
        // The library folder is fetched when CLARITY_UI names none, the command line's too.
        assert!(signs.iter().any(|f| f.path == "clarity-runtime/clarity_ui.cmake"), "cli {cli}: clarity_ui.cmake");
        let plain = files(PLAIN, false, cli);
        assert!(!file(&plain, "CMakeLists.txt").contains("clarity_use_auth"), "cli {cli}");
        assert!(!plain.iter().any(|f| f.path.starts_with("clarity-runtime/auth/")), "cli {cli}");
        // The core's side of people is always there: the database and the service call it.
        assert!(plain.iter().any(|f| f.path == "clarity-runtime/clarity/people.c"), "cli {cli}: people.c");
    }
}

#[test]
fn the_people_are_named_after_the_databases_open() {
    let desktop = files(CLUB, true, false);
    let main = file(&desktop, "src/main.c");
    has(main, "static const char* const signin_targets[] = {\"desktop\", \"cli\"};");
    has(main, "static const char* const signin_password[] = {\"email\", \"password_hash\"};");
    has(main, "{\"recovery\", signin_recovery, 2}");
    has(main, ".table = \"member\", .key = \"id\", .page = \"SignIn\"");
    has(main, ".session_seconds = 7200, .tries = 5, .tries_seconds = 900, .program = \"club\", .remember_seconds = 0}");
    // Inside open_databases, which `connect` calls again.
    let opened = &main[main.find("static bool open_databases").unwrap()..];
    let opened = &opened[..opened.find("*out = db;").unwrap()];
    has(opened, "if (!clarity_db_add_signin(db, &signin_source)) return closed(db);");
    let cli = files(CLUB, true, true);
    has(file(&cli, "src/main.c"), "if (!clarity_db_add_signin(db, &signin_source)) return failed(db);");
}

#[test]
fn a_provider_strategy_carries_its_settings() {
    let mut with = signin();
    let provider = clarity_spec::signin::ProviderSettings { client: "app".into(), secret: Some("setting:githubSecret".into()), ..Default::default() };
    with.strategies.push(SigninStrategy { name: "github".into(), fields: vec!["email".into()], minimum: None, provider: Some(provider) });
    // The app's secret is kept under a secret setting the project declares.
    let kept = format!("{CLUB}\nsettings\n    githubSecret: secret\n");
    let desktop = files_with(&kept, Some(with), false);
    let main = file(&desktop, "src/main.c");
    has(main, "static const char* const signin_github_options[] = {\"client\", \"app\", \"secret\", \"setting:githubSecret\", \"claim\", \"email\"};");
    has(main, "{\"github\", signin_github, 1, signin_github_options, 3}");
    // A strategy without settings has no options.
    has(main, "{\"password\", signin_password, 2}");
}

#[test]
fn me_allow_and_hashing_go_through_the_runtime() {
    let desktop = files(CLUB, true, false);
    let club = file(&desktop, "src/modules/club.c");
    has(club, "clarity_me(db, &member_entity, &");
    has(club, "if (!clarity_allow(");
    has(club, "clarity_new_token(&");
    has(club, "clarity_new_recovery_codes(8, &");
    has(club, "clarity_hashed(\"argon2id\", typed, 12, &");
    has(club, "clarity_hashed(\"sha256\", ");
    has(club, "clarity_hashed(\"sha256-lines\", ");
    // The window's allow is decided when it opens, before its data loads.
    let load = &club[club.find("bool main_window_load(").unwrap()..];
    has(&load[..load.find("\n}\n").unwrap()], "if (!clarity_allow(");
    // The menu item that opens Main adds its allow to Main's own.
    assert_eq!(load[..load.find("\n}\n").unwrap()].matches("clarity_allow(").count(), 2);
    // A frame command's allow is checked when its handler runs, an app handler's too.
    let about = &club[club.find("bool club_app_about_clicked(").unwrap()..];
    has(&about[..about.find("\n}\n").unwrap()], "if (!clarity_allow(");
    let greet = &club[club.find("bool main_window_greet_clicked(").unwrap()..];
    has(&greet[..greet.find("\n}\n").unwrap()], "if (!clarity_allow(");
}

#[test]
fn readable_and_writable_are_checked_on_every_find_and_save() {
    let desktop = files(CLUB, true, false);
    let records = file(&desktop, "src/records.c");
    has(records, "bool member_readable(ClarityDb* db, const Member* member)");
    has(records, "return clarity_yours(");
    has(records, "bool member_save(ClarityDb* db, Member* record) { return member_writable(db, record) && clarity_save(db, &member_entity, record); }");
    has(records, "member_writable(db, record) && clarity_delete(");
    has(records, "if (!member_readable(db, &out->items[i])) return false;");
    has(records, "member_readable(db, out)");
    // A picture served from a record is read through the readable check.
    has(file(&desktop, "src/main.c"), "if (!member_get(db, id, &record)) return false;");
}

#[test]
fn the_command_line_signs_in_before_a_command_runs() {
    let cli = files(CLUB, true, true);
    let main = file(&cli, "src/main.c");
    has(main, "ClarityCliSignin signin = clarity_cli_signin_args(argc, argv);");
    has(main, "clarity_signin_cli(db, &signin, clarity_read_password, ");
    has(main, "\"cliPassword\"");
    let signs = main.find("clarity_signin_cli(").unwrap();
    assert!(main.find("CLARITY_CLI_HELP").unwrap() < signs && signs < main.find("switch (parsed.command)").unwrap(), "after the help, before the command");
    has(file(&cli, "src/modules/club.c"), "if (!clarity_allow(");
    let plain = files(PLAIN, false, true);
    assert!(!file(&plain, "src/main.c").contains("clarity_signin_cli"));
}

/// The club builds with CMake on the runtime and Clarity.Auth's library this
/// repository built (`cargo build --release -p clarity-ui-ffi -p
/// clarity-auth-ffi`), the desktop program and the command line, with no
/// warning in the program's own files.
#[test]
#[ignore]
fn the_club_builds_in_c() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = clarity_gen::clarity_ui::repository_library(&root, std::env::consts::OS).expect("Clarity.UI's and Clarity.Auth's libraries in target/release");
    for cli in [false, true] {
        let out = root.join("target/generated").join(format!("clarity-gen-c-signin-{}-{cli}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        // Remembered for 30 days, so the desktop program carries the protected store too.
        let mut remembered = signin();
        remembered.remember_days = Some(30);
        // A provider too, so its settings are built.
        let provider = clarity_spec::signin::ProviderSettings { client: "app".into(), secret: Some("setting:githubSecret".into()), ..Default::default() };
        remembered.strategies.push(SigninStrategy { name: "github".into(), fields: vec!["email".into()], minimum: None, provider: Some(provider) });
        clarity_gen::write_out(&out, &files_with(CLUB, Some(remembered), cli), &["build", "data"]).unwrap();
        let built = std::process::Command::new("cmake").args(["--workflow", "--preset", "debug"]).env("CLARITY_UI", &folder).current_dir(&out).output().unwrap();
        let said = format!("{}{}", String::from_utf8_lossy(&built.stdout), String::from_utf8_lossy(&built.stderr));
        assert!(built.status.success(), "cli {cli}:\n{said}");
        let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
        assert!(own.is_empty(), "cli {cli}, warnings:\n{}", own.join("\n"));
        std::fs::remove_dir_all(&out).unwrap();
    }
}

#[test]
fn a_remembered_desktop_sign_in_keeps_it_in_the_protected_store() {
    let mut remembered = signin();
    remembered.remember_days = Some(30);
    let desktop = files_with(CLUB, Some(remembered), false);
    has(file(&desktop, "src/main.c"), ".remember_seconds = 2592000}");
    has(file(&desktop, "src/main.c"), "clarity_secrets_use();");
    assert!(desktop.iter().any(|f| f.path == "clarity-runtime/secrets/secrets.c"));
    assert!(!file(&files(CLUB, true, false), "src/main.c").contains("clarity_secrets_use"));
    // `when asked`: the runtime remembers only a ticked "keep me signed in" box.
    let mut asking = signin();
    asking.remember_days = Some(30);
    asking.remember_asks = true;
    has(file(&files_with(CLUB, Some(asking), false), "src/main.c"), ".remember_seconds = 2592000, .remember_asks = true}");
}

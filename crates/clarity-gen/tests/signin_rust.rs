//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4) on the
//! Rust stack: the program carries Clarity.Auth, names its people after
//! the databases open, starts with the sign-in window, answers signing in
//! and out, reads `me`, checks `allow when`, `readable when` and
//! `writable when`, hashes what code assigns to a hashed field, and the
//! command line signs in before a command runs.

use clarity_project::Backend;

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

entity Task
    title: text(80)
    owner: ref Member
    readable when task.owner.id == me.id
    writable when task.owner.id == me.id

action promote(member: Member)
    allow when me.role == Admin
    effects db.write
    member.role = Admin
    save member

action newKey(member: Member) -> text
    effects db.write
    let token = newToken()
    member.apiToken = token
    member.recoveryCodes = newRecoveryCodes(10)
    save member
    return token

action setPassword(member: Member, typed: text)
    effects db.write
    member.passwordHash = typed
    save member

action mine() -> list<Task>
    return find t in Task where t.owner == me

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
        item \"Greet\" name greetItem on click: greetClicked allow when me.role == Admin
        item \"Tasks\" name tasksItem on click -> Tasks allow when me.role == Admin
        item \"Sign out\" name signOutItem on click -> signout
    label \"Hello\" name hello at 1,1 span 4
    button \"Greet\" name greet at 1,2 span 2 on click: greetClicked

page Tasks: window
    data tasks: list<Task>
    table name list at 1,1 span 4 bind tasks

page SignIn: signin
    textbox name name at 1,1 span 4
    password name password at 1,2 span 4
    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

const STEP: &str = "migration \"Club\"
    add entity Member
        name: text(80)
        email: email, unique, required
        passwordHash: text
        apiToken: text
        recoveryCodes: text
        role: Role, default Plain
        photo: image
    add entity Task
        title: text(80)
        owner: ref Member
";

const PROJECT: &str = "project Club
    stack rust desktop cli
    datasource main sqlite \"data/club.db\"
    signin Member on desktop cli
        password email passwordHash
            minimum 12
        token apiToken
        recovery email recoveryCodes
        remember 30 days
";

fn files(cli: bool, source: &str) -> Vec<clarity_gen::OutFile> {
    files_in(PROJECT, cli, source)
}

fn files_in(project_file: &str, cli: bool, source: &str) -> Vec<clarity_gen::OutFile> {
    let settings = clarity_project::parse(project_file).unwrap();
    assert_eq!(settings.stack.backend, Backend::Rust);
    let sources = clarity_spec::converted([("club/club.clarity", source)]);
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", STEP)], &[], settings.signin.as_ref());
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    if cli { clarity_gen::generate_cli(&project, &settings) } else { clarity_gen::generate(&project, &settings) }.unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

#[test]
fn the_program_carries_clarity_auth_only_when_it_signs_people_in() {
    for cli in [false, true] {
        has(file(&files(cli, CLUB), "Cargo.toml"), "\"auth\"");
    }
    // A sign-in remembered between runs is kept in the system's protected store.
    has(file(&files(false, CLUB), "Cargo.toml"), "\"secrets\"");
    let settings = clarity_project::parse("project Plain\n    stack rust desktop\n    datasource main sqlite \"data/plain.db\"\n").unwrap();
    let sources = clarity_spec::converted([("plain/main.clarity", "module main\n\npage Main: window\n    label \"Hi\" name hi at 1,1\n")]);
    let project = clarity_spec::load_in(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &[]);
    let plain = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?} {:#?}", project.diagnostics));
    assert!(!file(&plain, "Cargo.toml").contains("\"auth\""));
    assert!(!file(&plain, "src/main.rs").contains("add_signin"));
}

#[test]
fn remember_when_asked_has_the_runtime_remember_only_a_ticked_box() {
    let asking = PROJECT.replace("        remember 30 days\n", "        remember 30 days when asked\n");
    has(file(&files_in(&asking, false, CLUB), "src/main.rs"), "remember_seconds: Some(2592000), remember_asks: true });");
}

#[test]
fn the_people_are_named_after_the_databases_open() {
    for cli in [false, true] {
        let files = files(cli, CLUB);
        let main = file(&files, "src/main.rs");
        has(main, "db.add_signin(clarity_runtime::signin::SigninSource { source: \"main\".into(), table: \"member\".into(), key: \"id\".into(), page: \"SignIn\".into(), targets: vec![\"desktop\".into(), \"cli\".into()], strategies: vec![");
        has(main, "clarity_runtime::signin::SigninStrategy { name: \"password\".into(), columns: vec![\"email\".into(), \"password_hash\".into()] }");
        has(main, "session_seconds: 28800, tries: 5, tries_seconds: 900, program: PROGRAM.into(), remember_seconds: Some(2592000), remember_asks: false });");
    }
}

#[test]
fn the_start_says_the_signin_window_and_the_program_signs_in_and_out() {
    let files = files(false, CLUB);
    let main = file(&files, "src/main.rs");
    has(main, "signin: self.0.lock().ok().and_then(|db| db.signin_first())");
    has(main, "fn signin(&self, strategy: &str, inputs: Map<String, Value>) -> Answer<()> {\n        locked(self, |db| db.signin(strategy, &inputs).map(|_| ()))");
    has(main, "fn signout(&self) -> Answer<()> {\n        locked(self, |db| {\n            db.signout();\n            Ok(())\n        })");
}

#[test]
fn me_allow_and_secrets_in_code() {
    let files = files(false, CLUB);
    let club = file(&files, "src/modules/club.rs");
    // `allow when` refuses at the start of the action.
    has(club, "pub fn promote(db: &Db, member: &mut Member) -> Result<()> {\n    clarity_runtime::signin::allow(");
    has(club, "db.me::<Member>()?.role");
    // A hashed field keeps the hash, by its strategy's method and minimum.
    has(club, "member.passwordHash = clarity_runtime::signin::hashed(\"argon2id\", &typed.clone(), Some(12))?;");
    has(club, "member.recoveryCodes = clarity_runtime::signin::hashed(\"sha256-lines\", &clarity_runtime::signin::new_recovery_codes(10i64)?, None)?;");
    has(club, "let mut token = clarity_runtime::signin::new_token()?;");
    // `me` compared with a reference is its id, refused when nobody signed in.
    has(club, "Some(db.me_id().ok_or_else(|| Error::Refused(clarity_runtime::signin::not_allowed()))?)");
    // The handler reads `me`.
    has(club, "db.me::<Member>()?.name");
}

#[test]
fn readable_and_writable_are_checked() {
    let files = files(false, CLUB);
    let club = file(&files, "src/modules/club.rs");
    has(club, "impl Task {\n    /// `readable when task.owner.id == me.id`: refused with notYours when it does not hold.\n    pub fn readable(db: &Db, task: Task) -> Result<Task> {");
    has(club, "pub fn writable(db: &Db, task: &Task) -> Result<()> {");
    // A find gives back only records that are readable, or refuses.
    has(club, "Task::readable_all(db, db.find::<Task>(");
    // A save and a delete check writable first, on the record as held and as stored.
    has(club, "const EVENTS: bool = true;");
    has(club, "fn before_save(&mut self, db: &Db) -> Result<()> {\n        Task::writable(db, self)");
    has(club, "fn before_delete(&mut self, db: &Db) -> Result<()> {\n        Task::writable(db, self)");
    // A window's list data, the dropdowns' records and a record's picture are checked too.
    has(club, "tasks: Task::readable_all(db, db.all::<Task>()?)?,");
    let main = file(&files, "src/main.rs");
    has(main, "\"Task\" => modules::club::Task::readable_all(db, db\n            .all::<modules::club::Task>()?)?");
    has(main, "modules::club::Member::readable(db, db.get::<modules::club::Member>(id)?)?;\n            db.picture::<modules::club::Member>(id, \"photo\")");
}

#[test]
fn windows_and_frame_commands_check_allow() {
    let files = files(false, CLUB);
    let club = file(&files, "src/modules/club.rs");
    has(club, "impl MainWindow {\n    /// `allow when me.role == Admin or me.role == Plain`: checked before the window opens.\n    pub fn allowed(db: &Db) -> Result<()> {");
    let main = file(&files, "src/main.rs");
    has(main, "modules::club::MainWindow::allowed(db)?;");
    // The menu item's allow is checked when its handler runs.
    has(club, "pub fn main__greet_clicked__allowed(db: &Db) -> Result<()> {");
    has(main, "modules::club::main__greet_clicked__allowed(db)?;");
    // A command that opens a window: its allow is checked when that window opens.
    has(club, "pub fn main__opens__tasks__allowed(db: &Db) -> Result<()> {");
    has(main, "\"Tasks\" => {\n            modules::club::main__opens__tasks__allowed(db)?;\n            to_data(&modules::club::TasksWindow::load(db)?)?\n        },");
}

#[test]
fn the_command_line_signs_in_before_a_command() {
    let files = files(true, CLUB);
    let main = file(&files, "src/main.rs");
    has(main, "db.signin_cli(&clarity_runtime::signin::CliSignin::from_args(&std::env::args().skip(1).collect::<Vec<String>>()), &mut || clarity_runtime::cli::read_password(&format!(\"{}: \", Text::keyed(\"cliPassword\", \"Password\").show())))?;");
    // Every command takes --user and --token.
    has(main, "Param { name: \"user\".to_string()");
    has(main, "Param { name: \"token\".to_string()");
    let club = file(&files, "src/modules/club.rs");
    has(club, "pub fn command_members(db: &Db, cli: &mut Session, w: &mut crate::CommandData) -> Result<i32> {\n    clarity_runtime::signin::allow(");
}

/// The club builds on the runtime in this repository, the command line and
/// the desktop program (on Clarity.UI's library in CLARITY_UI, else the one
/// target/clarity-ui holds).
#[test]
#[ignore]
fn the_club_builds_on_the_runtime() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = root.join("runtimes/rust").canonicalize().unwrap();
    let target = root.join("target/generated");
    let ui = std::env::var_os("CLARITY_UI").map(std::path::PathBuf::from).unwrap_or_else(|| root.join("target/clarity-ui"));
    for (cli, command) in [(true, "build"), (false, "build")] {
        let out = target.join(format!("clarity-gen-signin-rust-{}-{}", if cli { "cli" } else { "desktop" }, std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files(cli, CLUB), &["target"]).unwrap();
        std::fs::create_dir_all(out.join(".cargo")).unwrap();
        std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
        let status = std::process::Command::new("cargo").args([command, "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", &target).env("CLARITY_UI", &ui).status().unwrap();
        assert!(status.success(), "the generated club does not {command}; it is in {}", out.display());
        std::fs::remove_dir_all(&out).unwrap();
    }
}

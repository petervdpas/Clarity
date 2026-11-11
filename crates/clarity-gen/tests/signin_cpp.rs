//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4) in C++:
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
        item \"Sign out\" name signOutItem on click -> signout
    label \"Hello\" name hello at 1,1 span 4
    button \"Greet\" name greet at 1,2 span 2 on click: greetClicked

page SignIn: signin
    textbox name name at 1,1 span 4
    password name password at 1,2 span 4
    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

const PLAIN: &str = "module notes\n\nentity Note\n    text: text(200)\n\npage Notes: window\n    data notes: list<Note>\n    table name list at 1,1 span 4 bind notes\n";

fn signin() -> Signin {
    let mut signin = Signin::new("Member", vec!["desktop".into(), "cli".into()]);
    signin.strategies.push(SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: Some(12) });
    signin.strategies.push(SigninStrategy { name: "token".into(), fields: vec!["apiToken".into()], minimum: None });
    signin.strategies.push(SigninStrategy { name: "recovery".into(), fields: vec!["email".into(), "recoveryCodes".into()], minimum: None });
    signin.session_minutes = 120;
    signin
}

fn files(source: &str, signs_in: bool, cli: bool) -> Vec<clarity_gen::OutFile> {
    let signin = signin();
    let signin = signs_in.then_some(&signin);
    let sources = clarity_spec::converted([("club/club.clarity", source)]);
    // The people's table, as its migration makes it.
    let step = "migration \"Members\"\n    add entity Member\n        name: text(80)\n        email: email, unique, required\n        passwordHash: text, hashed\n        apiToken: text, hashed\n        recoveryCodes: text, hashed\n        role: Role, default Plain\n";
    let notes = "migration \"Notes\"\n    add entity Note\n        text: text(200)\n";
    let steps = vec![("migrations/main/0001.clarity", if source.contains("entity Member") { step } else { notes })];
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps, &[clarity_spec::SourceRef::sql("main")], signin);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let target = if cli { "cli" } else { "desktop" };
    let mut settings = clarity_project::parse(&format!("project Club\n    stack cpp {target}\n    datasource main sqlite \"data/club.db\"\n")).unwrap();
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
        assert!(signs.iter().any(|f| f.path == "clarity-runtime/auth/auth.cpp"), "cli {cli}: auth.cpp");
        // The library's folder is fetched by its version where no CLARITY_UI names it.
        assert!(signs.iter().any(|f| f.path == "clarity-runtime/clarity_ui.cmake"), "cli {cli}: clarity_ui.cmake");
        has(file(&signs, "src/records.hpp"), "#include \"auth/auth.hpp\"");
        let plain = files(PLAIN, false, cli);
        assert!(!file(&plain, "CMakeLists.txt").contains("clarity_use_auth"), "cli {cli}");
        assert!(!plain.iter().any(|f| f.path.starts_with("clarity-runtime/auth/")), "cli {cli}");
        assert!(!file(&plain, "src/records.hpp").contains("auth/auth.hpp"), "cli {cli}");
        // The core's side of people is always there: the database and the service call it.
        assert!(plain.iter().any(|f| f.path == "clarity-runtime/clarity/people.cpp"), "cli {cli}: people.cpp");
        assert!(plain.iter().any(|f| f.path == "clarity-runtime/clarity/people.hpp"), "cli {cli}: people.hpp");
    }
}

#[test]
fn the_people_are_named_after_the_databases_open() {
    let desktop = files(CLUB, true, false);
    let main = file(&desktop, "src/main.cpp");
    has(main, ".source = \"\", .table = \"member\", .key = \"id\", .page = \"SignIn\"");
    has(main, ".targets = {\"desktop\", \"cli\"}");
    has(main, "{\"password\", {\"email\", \"password_hash\"}}");
    has(main, "{\"recovery\", {\"email\", \"recovery_codes\"}}");
    has(main, ".session_seconds = 7200, .tries = 5, .tries_seconds = 900");
    let migrated = main.find("db.migrate_source(").unwrap();
    let added = main.find("db.add_signin(club::signin_source);").unwrap();
    assert!(migrated < added, "after the databases open");
    let cli = files(CLUB, true, true);
    has(file(&cli, "src/main.cpp"), "db.add_signin(club::signin_source);");
    assert!(!file(&files(PLAIN, false, false), "src/main.cpp").contains("add_signin"));
}

#[test]
fn a_program_that_connects_again_names_its_people_again() {
    let unconnected = CLUB.replace("    main Main\n", "    main Main\n    on unconnected: notConnected\n") + "\ncode Club\n    handler notConnected\n        open Main\n";
    let desktop = files(&unconnected, true, false);
    let main = file(&desktop, "src/main.cpp");
    let opened = &main[main.find("std::unique_ptr<clarity::Db> open_databases()").unwrap()..];
    has(&opened[..opened.find("return db;").unwrap()], "db->add_signin(signin_source);");
    has(main, "options.connect = club::open_databases;");
}

#[test]
fn me_allow_and_hashing_go_through_the_runtime() {
    let desktop = files(CLUB, true, false);
    let club = file(&desktop, "src/modules/club.cpp");
    has(club, "db.me<Member>()");
    has(club, "clarity::allow(");
    has(club, "clarity::new_token()");
    has(club, "clarity::new_recovery_codes(int64_t{8})");
    has(club, "member.passwordHash = clarity::hashed(\"argon2id\", typed, int64_t{12});");
    has(club, "member.apiToken = clarity::hashed(\"sha256\", token, std::nullopt);");
    has(club, "member.recoveryCodes = clarity::hashed(\"sha256-lines\", codes, std::nullopt);");
    // An action's allow refuses before anything else runs.
    let promote = &club[club.find("void promote(").unwrap()..];
    let first = promote.lines().nth(1).unwrap().trim();
    assert!(first.starts_with("clarity::allow("), "{first}");
    // The window's allow is decided when it opens, before its data loads.
    let load = &club[club.find("MainWindow MainWindow::load(").unwrap()..];
    has(&load[..load.find("\n}\n").unwrap()], "clarity::allow(");
    // A frame command's allow holds for the window it opens too: Main checks its own and the menu's.
    assert_eq!(load[..load.find("\n}\n").unwrap()].matches("clarity::allow(").count(), 2, "{load}");
    // A frame command's allow is checked when its handler runs.
    let greet = &club[club.find("void MainWindow::greet_clicked(").unwrap()..];
    has(&greet[..greet.find("\n}\n").unwrap()], "clarity::allow(");
}

#[test]
fn readable_and_writable_are_checked_on_every_find_and_save() {
    let desktop = files(CLUB, true, false);
    let records = file(&desktop, "src/records.hpp");
    has(records, "void member_readable(clarity::Db& db, const Member& member);");
    has(records, "void member_writable(clarity::Db& db, const Member& member);");
    has(records, "    void before_save(clarity::Db& db);");
    has(records, "    void before_delete(clarity::Db& db);");
    let club = file(&desktop, "src/modules/club.cpp");
    has(club, "void member_readable([[maybe_unused]] clarity::Db& db, [[maybe_unused]] const Member& member) {\n    clarity::yours(");
    has(club, "void Member::before_save(clarity::Db& db) {\n    member_writable(db, *this);\n}");
    has(club, "void Member::before_delete(clarity::Db& db) {\n    member_writable(db, *this);\n}");
    // The service hands records out as dropdown options too: each is checked.
    has(file(&desktop, "src/main.cpp"), "for (const auto& r : found) member_readable(db, r);");
    // An entity without the lines has none of it.
    let plain = files(PLAIN, false, false);
    assert!(!file(&plain, "src/records.hpp").contains("_readable"));
}

#[test]
fn a_find_gives_only_records_that_are_readable() {
    let source = CLUB.replace("action promote(member: Member)", "action admins() -> list<Member>\n    return find m in Member where m.role == Admin\n\naction promote(member: Member)");
    let desktop = files(&source, true, false);
    let club = file(&desktop, "src/modules/club.cpp");
    has(club, "for (const auto& r : found) member_readable(db, r);");
}

#[test]
fn the_command_line_signs_in_before_a_command_runs() {
    let cli = files(CLUB, true, true);
    let main = file(&cli, "src/main.cpp");
    has(main, "clarity::cli_signin(db, clarity::cli_signin_of(std::vector<std::string>(argv + 1, argv + argc)), terminal, ");
    has(main, "clarity::Text::keyed(\"cliPassword\", \"Password\").show() + \": \"");
    // --user and --token are options of every command, after its own.
    has(main, "clarity::cli::Param{\"user\", clarity::cli::Kind::text");
    has(main, "clarity::cli::Param{\"token\", clarity::cli::Kind::text");
    let signs = main.find("clarity::cli_signin(").unwrap();
    assert!(main.find("clarity::cli::run(").unwrap() < signs && signs < main.find("switch (command)").unwrap(), "inside the run, before the command");
    has(file(&cli, "src/modules/club.cpp"), "clarity::allow(");
    let plain = files(PLAIN, false, true);
    assert!(!file(&plain, "src/main.cpp").contains("cli_signin"));
    assert!(!file(&plain, "src/main.cpp").contains("Param{\"user\""));
}

/// The club built as a C++ developer would, the desktop program and the
/// command line, on the runtime and Clarity.Auth's library this repository
/// built (`cargo build --release -p clarity-ui-ffi -p clarity-auth-ffi`), and
/// each runs with `--check`. Slow, so it only runs when asked:
/// `cargo test -p clarity-gen --test signin_cpp -- --ignored`.
#[test]
#[ignore]
fn the_club_builds_and_runs_in_cpp() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = std::env::var_os("CLARITY_UI").filter(|f| !f.is_empty()).map(std::path::PathBuf::from).or_else(|| clarity_gen::clarity_ui::repository_library(&root, std::env::consts::OS)).expect("a Clarity.UI folder with Clarity.Auth beside it");
    for cli in [false, true] {
        let out = root.join("target/generated").join(format!("clarity-gen-signin-cpp-{}-{}", if cli { "cli" } else { "desktop" }, std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files(CLUB, true, cli), &["build", "data"]).unwrap();
        let run = |tool: &str, args: &[&str]| {
            let done = std::process::Command::new(tool).env("CLARITY_UI", &folder).args(args).current_dir(&out).output().unwrap_or_else(|e| panic!("{tool}: {e}"));
            let said = format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr));
            assert!(done.status.success(), "{tool} {args:?} failed in {}:\n{said}", out.display());
            said
        };
        let said = run("cmake", &["--workflow", "--preset", "debug"]);
        let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
        assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
        if cli {
            run(&out.join("build/debug/club").display().to_string(), &["--help"]);
        } else {
            let said = run(&out.join("build/debug/club").display().to_string(), &["--check"]);
            assert!(said.contains("Club: the databases are ready."), "{said}");
        }
        let _ = std::fs::remove_dir_all(&out);
    }
}

#[test]
fn a_desktop_program_that_remembers_a_sign_in_keeps_it_in_the_protected_store() {
    let desktop = files(CLUB, true, false);
    let main = file(&desktop, "src/main.cpp");
    has(main, ".remember_seconds = 0}");
    assert!(!main.contains("secrets_use()"));
    let mut signin = signin();
    signin.remember_days = Some(30);
    let sources = clarity_spec::converted([("club/club.clarity", CLUB)]);
    let step = "migration \"Members\"\n    add entity Member\n        name: text(80)\n        email: email, unique, required\n        passwordHash: text, hashed\n        apiToken: text, hashed\n        recoveryCodes: text, hashed\n        role: Role, default Plain\n";
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", step)], &[clarity_spec::SourceRef::sql("main")], Some(&signin));
    let mut settings = clarity_project::parse("project Club\n    stack cpp desktop\n    datasource main sqlite \"data/club.db\"\n").unwrap();
    settings.signin = Some(signin);
    let remembers = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let main = file(&remembers, "src/main.cpp");
    has(main, ".program = \"club\", .remember_seconds = 2592000}");
    has(main, "clarity::secrets_use();");
    has(file(&remembers, "CMakeLists.txt"), "clarity_secrets");
    assert!(remembers.iter().any(|f| f.path == "clarity-runtime/secrets/secrets.cpp"));
    // `when asked`: the runtime remembers only a ticked "keep me signed in" box.
    let mut asking = settings.signin.clone().unwrap();
    asking.remember_asks = true;
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", step)], &[clarity_spec::SourceRef::sql("main")], Some(&asking));
    settings.signin = Some(asking);
    let asks = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&asks, "src/main.cpp"), ".program = \"club\", .remember_seconds = 2592000, .remember_asks = true}");
}

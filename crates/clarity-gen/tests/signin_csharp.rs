//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4) on the
//! C# stack: the program carries Clarity.Auth, names its people after the
//! databases open, reads `me` from the database handle, checks `allow when`,
//! `readable when` and `writable when`, hashes text given to a hashed field,
//! and its command line signs in before a command runs.

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
    readable when me.role == Admin or member.id == me.id
    writable when me.role == Admin

entity Note
    text: text(200)

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

function admins() -> list<Member>
    return find m in Member where m.role == Admin

code Note
    before save
        note.text = me.name

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
    textbox name name at 1,1 span 4
    password name password at 1,2 span 4
    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

const PROJECT: &str = "project Club
    stack csharp desktop cli
    datasource main sqlite \"data/club.db\"
    signin Member on desktop cli
        password email passwordHash
            minimum 12
        token apiToken
        recovery email recoveryCodes
        session 2 hours
        tries 3 per 10 minutes
";

const STEP: &str = "migration \"Club\"
    add entity Member
        name: text(80)
        email: email, unique, required
        passwordHash: text
        apiToken: text
        recoveryCodes: text
        role: Role, default Plain
    add entity Note
        text: text(200)
";

fn files(project_file: &str, source: &str, cli: bool) -> Vec<clarity_gen::OutFile> {
    let mut settings = clarity_project::parse(project_file).unwrap();
    settings.stack.backend = Backend::Csharp;
    let converted = clarity_spec::converted([("club/club.clarity", source)]);
    let project = clarity_spec::load_in_signed(converted.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", STEP)], &[clarity_spec::SourceRef::sql("main")], settings.signin.as_ref());
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    if cli { clarity_gen::generate_cli(&project, &settings) } else { clarity_gen::generate(&project, &settings) }.unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}: {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn desktop() -> Vec<clarity_gen::OutFile> {
    files(PROJECT, CLUB, false)
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted} in\n{text}");
}

#[test]
fn the_program_carries_clarity_auth_only_when_it_signs_people_in() {
    has(file(&desktop(), "Club.csproj"), "<PackageReference Include=\"Clarity.Auth\"");
    has(file(&files(PROJECT, CLUB, true), "Club.csproj"), "<PackageReference Include=\"Clarity.Auth\"");
    let plain = PROJECT.split("    signin").next().unwrap().to_string();
    let source = "module club\n\nentity Note\n    text: text(200)\n\napp Club\n    title \"Club\"\n    main Main\n\npage Main: window\n    label \"Hello\" name hello at 1,1 span 4\n";
    for cli in [false, true] {
        let mut settings = clarity_project::parse(&plain).unwrap();
        settings.stack.backend = Backend::Csharp;
        let converted = clarity_spec::converted([("club/club.clarity", source)]);
        let project = clarity_spec::load_in(converted.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", "migration \"Notes\"\n    add entity Note\n        text: text(200)\n")], &[clarity_spec::SourceRef::sql("main")]);
        let out = if cli { clarity_gen::generate_cli(&project, &settings) } else { clarity_gen::generate(&project, &settings) }.unwrap();
        let text: String = out.iter().map(|f| f.text.as_str()).collect();
        assert!(!text.contains("Clarity.Auth"), "cli {cli}: carries Clarity.Auth without signin");
    }
}

#[test]
fn the_databases_name_their_people_after_they_open() {
    let wanted = "Clarity.Auth.Signins.AddSignin(db, new Clarity.Auth.SigninSource(\"member\", \"SignIn\") { Source = \"main\", Key = \"id\", Targets = [\"desktop\", \"cli\"], Strategies = [new Clarity.Auth.SigninStrategy(\"password\", [\"email\", \"password_hash\"]), new Clarity.Auth.SigninStrategy(\"token\", [\"api_token\"]), new Clarity.Auth.SigninStrategy(\"recovery\", [\"email\", \"recovery_codes\"])], SessionSeconds = 7200, Tries = 3, TriesSeconds = 600, Program = \"club\" });";
    let program = file(&desktop(), "Program.cs").to_string();
    has(&program, wanted);
    // After the databases connect and migrate, before the handle is given out.
    assert!(program.find("db.MigrateSource").unwrap() < program.find(wanted).unwrap() && program.find(wanted).unwrap() < program.find("return db;").unwrap());
    has(file(&files(PROJECT, CLUB, true), "Program.cs"), wanted);
    // `remember N days` keeps the desktop's sign-in that long.
    let remembering = PROJECT.replace("        session 2 hours\n", "        session 2 hours\n        remember 30 days\n");
    has(file(&files(&remembering, CLUB, false), "Program.cs"), "TriesSeconds = 600, Program = \"club\", RememberSeconds = 2592000 });");
    // `when asked`: the runtime remembers only a ticked "keep me signed in" box.
    let asking = PROJECT.replace("        session 2 hours\n", "        session 2 hours\n        remember 30 days when asked\n");
    has(file(&files(&asking, CLUB, false), "Program.cs"), "TriesSeconds = 600, Program = \"club\", RememberSeconds = 2592000, RememberAsks = true });");
}

#[test]
fn me_is_the_handle_s_person_in_handlers_actions_events_and_commands() {
    let out = desktop();
    has(file(&out, "Windows/MainViewModel.cs"), "Db.Me<Member>().Name");
    let module = file(&out, "Modules/Club.cs");
    // The data event of a note.
    has(module, "note.Text = db.Me<Member>().Name;");
    let cli = files(PROJECT, CLUB, true);
    has(file(&cli, "Modules/Club.cs"), "db.Me<Member>().Name");
}

#[test]
fn allow_when_refuses_actions_and_commands_at_their_start() {
    let module = desktop().iter().find(|f| f.path == "Modules/Club.cs").unwrap().text.clone();
    has(&module, "public static void Promote(Db db, Member member)\n    {\n        Clarity.Runtime.Signin.Allow((db.Me<Member>().Role == Role.Admin));\n");
    let cli = files(PROJECT, CLUB, true);
    has(file(&cli, "Modules/Club.cs"), "public static int CommandMembers(Db db, CliSession cli)\n    {\n        Clarity.Runtime.Signin.Allow((db.Me<Member>().Role == Role.Admin));\n");
}

#[test]
fn allow_when_on_a_window_is_checked_before_it_opens_and_on_a_frame_command_when_it_runs() {
    let out = desktop();
    let main = file(&out, "Windows/MainViewModel.cs");
    has(main, "public override void Allow() => Clarity.Runtime.Signin.Allow(((Db.Me<Member>().Role == Role.Admin) || (Db.Me<Member>().Role == Role.Plain)));");
    has(main, "void Open_Main() => Handle(() =>\n    {\n        Clarity.Runtime.Signin.Allow((Db.Me<Member>().Role == Role.Admin));\n        Open(new MainViewModel(Db));\n    });");
    // The sign-in window has no condition and signin and signout are Clarity.UI's own.
    let signin = file(&out, "Windows/SignInViewModel.cs");
    assert!(!signin.contains("Allow()"), "{signin}");
    assert!(!main.contains("signout") && !main.contains("Signout"), "{main}");
}

#[test]
fn readable_when_checks_each_record_found_and_writable_when_each_save_and_delete() {
    let module = desktop().iter().find(|f| f.path == "Modules/Club.cs").unwrap().text.clone();
    has(&module, "public static bool ReadableMember(Db db, Member member)\n    {\n        return ((db.Me<Member>().Role == Role.Admin) || (member.Id == db.Me<Member>().Id));\n    }");
    has(&module, "public static bool WritableMember(Db db, Member member)\n    {\n        return (db.Me<Member>().Role == Role.Admin);\n    }");
    has(&module, "Member.Readable_(db, db.Find<Member>(");
    has(&module, "public static List<Member> Readable_(Db db, List<Member> found)\n    {\n        foreach (var record in found)\n            Clarity.Runtime.Signin.Yours(Logic.ReadableMember(db, record));\n        return found;\n    }");
    has(&module, "public static bool HasEvents => true;");
    has(&module, "public void BeforeSave(Db db) => Clarity.Runtime.Signin.Yours(Logic.WritableMember(db, this));");
    has(&module, "public void BeforeDelete(Db db) => Clarity.Runtime.Signin.Yours(Logic.WritableMember(db, this));");
    // A note's own data event stays as it was.
    has(&module, "public void BeforeSave(Db db) => Logic.BeforeSaveNote(db, this);");
}

/// A club whose members are listed, picked, referenced, reloaded and shown with their pictures.
const ROSTER: &str = "module club

enum Role
    Admin
    Plain

entity Member
    name: text(80)
    email: email, unique, required
    passwordHash: text, hashed
    photo: image
    role: Role, default Plain
    readable when me.role == Admin or member.id == me.id

entity Duty
    title: text(80)
    member: ref Member

function dutyOwner(duty: Duty) -> text
    return duty.member.name

app Club
    title \"Club\"
    main Main

page Main: window
    data members: list<Member>
    data member: Member
    data duty: Duty
    table name list at 1,1 span 4 bind members
    dropdown name owner at 1,6 span 4 bind duty.member
    image name photo at 1,7 span 4 bind member.photo
    button \"Again\" name again at 1,12 span 2 on click: againClicked

page SignIn: signin
    textbox name name at 1,1 span 4
    password name password at 1,2 span 4
    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password

code Main
    handler againClicked
        reload members
        reload member
";

const ROSTER_STEP: &str = "migration \"Club\"
    add entity Member
        name: text(80)
        email: email, unique, required
        passwordHash: text
        photo: image
        role: Role, default Plain
    add entity Duty
        title: text(80)
        member: ref Member
";

fn roster() -> Vec<clarity_gen::OutFile> {
    let mut settings = clarity_project::parse("project Club\n    stack csharp desktop\n    datasource main sqlite \"data/club.db\"\n    signin Member on desktop\n        password email passwordHash\n").unwrap();
    settings.stack.backend = Backend::Csharp;
    let converted = clarity_spec::converted([("club/club.clarity", ROSTER)]);
    let project = clarity_spec::load_in_signed(converted.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", ROSTER_STEP)], &[clarity_spec::SourceRef::sql("main")], settings.signin.as_ref());
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

#[test]
fn readable_when_checks_every_path_that_hands_a_record_out() {
    let out = roster();
    let main = file(&out, "Windows/MainViewModel.cs");
    // The window's own list, the dropdown's choices, the picture, and reloads.
    has(main, "_members = Member.Readable_(Db, Db.All<Member>());");
    has(main, "MemberChoices = Choice.Of(Member.Readable_(Db, Db.All<Member>()), r => ");
    has(main, "Db.PictureOf<Member>(Member.Readable_(Db, Member).Id, \"photo\")");
    has(main, "Members = Member.Readable_(Db, Db.All<Member>());");
    has(main, "Member = Member.Readable_(Db, Db.Get<Member>(Member.Id));");
    // A reference read in code.
    has(file(&out, "Modules/Club.cs"), "Member.Readable_(db, db.Load<Member>(duty.Member)).Name");
    has(file(&out, "Modules/Club.cs"), "public static Member Readable_(Db db, Member record)\n    {\n        Clarity.Runtime.Signin.Yours(Logic.ReadableMember(db, record));\n        return record;\n    }");
}

#[test]
fn text_given_to_a_hashed_field_is_kept_as_its_hash() {
    let module = desktop().iter().find(|f| f.path == "Modules/Club.cs").unwrap().text.clone();
    has(&module, "member.PasswordHash = Clarity.Auth.Signins.Hashed(\"argon2id\", typed, 12);");
    has(&module, "member.ApiToken = Clarity.Auth.Signins.Hashed(\"sha256\", token, null);");
    has(&module, "member.RecoveryCodes = Clarity.Auth.Signins.Hashed(\"sha256-lines\", codes, null);");
    has(&module, "var token = Clarity.Auth.Signins.NewToken();");
    has(&module, "var codes = Clarity.Auth.Signins.NewRecoveryCodes(8L);");
}

#[test]
fn the_command_line_signs_in_before_a_command_runs() {
    let cli = files(PROJECT, CLUB, true);
    let program = file(&cli, "Program.cs");
    has(program, "var signin = CliSignin.FromArgs(args);");
    has(program, "args = WithoutSignin(args);");
    has(program, "db.SigninCli(signin, () => Clarity.Runtime.Signin.ReadPassword(CliLanguage.Translate(\"cliPassword\", \"Password\") + \": \"));");
    // Without cli among the targets, nothing signs in.
    let desktop_only = PROJECT.replace("on desktop cli", "on desktop");
    let program = files(&desktop_only, CLUB, true).into_iter().find(|f| f.path == "Program.cs").unwrap().text;
    assert!(!program.contains("SigninCli"), "{program}");
}

/// Builds the club as the desktop program and the command line on the
/// runtime's own projects (runtimes/csharp), so the generated code compiles
/// against Clarity.Auth and Clarity.Runtime, with `remember`; and the roster,
/// whose every read is checked by `readable when`. Slow; only when asked:
/// `cargo test -p clarity-gen --test signin_csharp -- --ignored --exact builds_on_the_runtime`.
#[test]
#[ignore]
fn builds_on_the_runtime() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/signin-csharp");
    let remembering = PROJECT.replace("        session 2 hours\n", "        session 2 hours\n        remember 30 days\n");
    for (name, generated) in [("desktop", files(&remembering, CLUB, false)), ("cli", files(&remembering, CLUB, true)), ("roster", roster())] {
        let out = root.join(name);
        let _ = std::fs::remove_dir_all(&out);
        for f in generated {
            let path = out.join(&f.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            match &f.bytes {
                Some(bytes) => std::fs::write(&path, bytes).unwrap(),
                None => std::fs::write(&path, &f.text).unwrap(),
            }
        }
        let built = std::process::Command::new("dotnet")
            .args(["build", "-nologo", "-v", "q"])
            .current_dir(&out)
            .env("CLARITY_RUNTIME", std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp"))
            .env("MSBUILDDISABLENODEREUSE", "1")
            .output()
            .expect("dotnet runs");
        let said = String::from_utf8_lossy(&built.stdout).to_string() + &String::from_utf8_lossy(&built.stderr);
        assert!(built.status.success(), "{name}: {said}");
        let _ = std::fs::remove_dir_all(&out);
    }
}

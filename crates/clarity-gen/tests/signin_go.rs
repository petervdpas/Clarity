//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4) on the
//! Go stack: the program carries Clarity.Auth's binding only when it signs
//! people in, names its people after the databases open, and writes `me`,
//! `allow when`, `readable when`, `writable when`, hashed fields,
//! newToken() and newRecoveryCodes() through the Go runtime's helpers; the
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
    codes: text, hashed
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
    member.codes = codes
    save member
    return codes

action setPassword(email: text, typed: text)
    effects db.write
    let found = find m in Member where m.email == email
    for m in found
        m.passwordHash = typed
        save m

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
        item \"Members\" name membersItem on click -> Members allow when me.role == Admin
        item \"Greet\" name greetItem on click: greetClicked allow when me.role == Plain
        item \"Sign out\" name signOutItem on click -> signout
    label \"Hello\" name hello at 1,1 span 4
    button \"Greet\" name greet at 1,2 span 2 on click: greetClicked

page Members: window
    label \"Members\" name title at 1,1 span 4

page SignIn: signin
    textbox name name at 1,1 span 4
    password name password at 1,2 span 4
    button \"Sign in\" name signIn at 1,3 span 2 on click -> signin password

code Main
    handler greetClicked
        hello.caption = \"Hello \" + me.name
";

const PROJECT: &str = "project Club
    stack go desktop cli
    datasource main sqlite \"data/club.db\"
    signin Member on desktop cli
        password email passwordHash
            minimum 12
        token apiToken
        recovery email codes
        session 2 hours
";

fn project(source: &str, signed: bool) -> (clarity_spec::Project, clarity_project::Settings) {
    let mut settings = clarity_project::parse(PROJECT).unwrap();
    settings.stack.backend = Backend::Go;
    if !signed {
        settings.signin = None;
    }
    let sources = clarity_spec::converted([("club/club.clarity", source)]);
    let step = "migration \"Members\"\n    add entity Member\n        name: text(80)\n        email: email, unique, required\n        passwordHash: text, hashed\n        apiToken: text, hashed\n        codes: text, hashed\n        role: Role, default Plain\n        photo: image\n";
    let steps = if source.contains("entity Member") { vec![("migrations/main/0001.clarity", step)] } else { Vec::new() };
    let project = clarity_spec::load_in_signed(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps, &[clarity_spec::SourceRef::sql("main")], settings.signin.as_ref());
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.iter().all(|e| e.contains("label") || e.contains("translation")), "{errors:#?}");
    (project, settings)
}

fn desktop(source: &str, signed: bool) -> Vec<clarity_gen::OutFile> {
    let (project, settings) = project(source, signed);
    clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn cli(source: &str) -> Vec<clarity_gen::OutFile> {
    let (project, settings) = project(source, true);
    clarity_gen::generate_cli(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in\n{text}");
}

#[test]
fn the_program_carries_clarity_auth_only_when_it_signs_people_in() {
    for files in [desktop(CLUB, true), cli(CLUB)] {
        let auth = file(&files, "auth.go");
        has(auth, "#cgo LDFLAGS: -lclarity_auth");
        has(auth, "func authCall(op, args string) (string, error) {");
        has(auth, "clarity_auth_api()->call(");
    }
    let plain = CLUB.split("\nenum Role").next().unwrap().to_string() + "\n\npage Main: window\n    label \"Hello\" name hello at 1,1 span 4\n";
    let files = desktop(&plain, false);
    assert!(files.iter().all(|f| f.path != "auth.go"), "auth.go without signin");
}

const SOURCE: &str = "clarity.SigninSource{Program: \"club\", Table: \"member\", Key: \"id\", Page: \"SignIn\", Targets: []string{\"desktop\", \"cli\"}, Strategies: []clarity.SigninStrategy{{Name: \"password\", Columns: []string{\"email\", \"password_hash\"}}, {Name: \"token\", Columns: []string{\"api_token\"}}, {Name: \"recovery\", Columns: []string{\"email\", \"codes\"}}}, SessionSeconds: 7200, Tries: 5, TriesSeconds: 900}";

#[test]
fn the_people_are_named_after_the_databases_open() {
    for files in [desktop(CLUB, true), cli(CLUB)] {
        let main = file(&files, "main.go");
        has(main, &format!("\tif err := db.AddSignin({SOURCE}, authCall); err != nil {{\n\t\tdb.Close()\n\t\treturn nil, err\n\t}}\n"));
        // Inside openDatabases, which the unconnected program's Connect opens again.
        let open = main.split("func openDatabases()").nth(1).unwrap().split("\n}\n").next().unwrap();
        has(open, "db.AddSignin(");
    }
    // `remember N days` keeps a desktop sign-in that long.
    let (mut remembered, settings) = project(CLUB, true);
    remembered.signin.as_mut().unwrap().remember_days = Some(30);
    let files = clarity_gen::generate(&remembered, &settings).unwrap();
    let main = file(&files, "main.go");
    has(main, "TriesSeconds: 900, RememberSeconds: 2592000}, authCall)");
    // `when asked`: the runtime remembers only a ticked "keep me signed in" box.
    let (mut asked, asked_settings) = project(CLUB, true);
    asked.signin.as_mut().unwrap().remember_days = Some(30);
    asked.signin.as_mut().unwrap().remember_asks = true;
    has(file(&clarity_gen::generate(&asked, &asked_settings).unwrap(), "main.go"), "TriesSeconds: 900, RememberSeconds: 2592000, RememberAsks: true}, authCall)");
    // The remembered sign-in is kept in the system's protected store.
    has(main, "\tclarity.Secrets = secrets.System{}\n");
    has(main, "\t\"github.com/petervdpas/Clarity/runtimes/go/secrets\"\n");
    has(file(&files, "go.mod"), "github.com/zalando/go-keyring");
    // Without `remember`, no secret store.
    let files = desktop(CLUB, true);
    assert!(!file(&files, "main.go").contains("clarity.Secrets"), "secrets without remember");
}

#[test]
fn me_is_the_runtime_s_signed_in_record() {
    let files = desktop(CLUB, true);
    let module = file(&files, "club_module.go");
    has(module, "clarity.Me[Member](db)");
    let cli = cli(CLUB);
    has(file(&cli, "club_module.go"), "clarity.Me[Member](db)");
}

#[test]
fn allow_when_refuses_actions_commands_windows_and_frame_commands() {
    let files = desktop(CLUB, true);
    let module = file(&files, "club_module.go");
    let promote = module.split("func Promote(").nth(1).unwrap();
    let first = promote.lines().skip(1).take(8).collect::<Vec<_>>().join("\n");
    has(&first, "clarity.Allow(");
    // A window's allow is decided before it opens; a frame command's when it runs.
    has(module, "func allowMain(db *clarity.Db) error {");
    has(module, "func allowMainOpensMembers(db *clarity.Db) error {");
    let main = file(&files, "main.go");
    has(main, "\tcase \"Main\":\n\t\tif err := allowMain(db); err != nil {\n\t\t\treturn clarity.WindowState{}, err\n\t\t}\n");
    has(main, "\tcase \"Members\":\n\t\tif err := allowMainOpensMembers(db); err != nil {");
    has(module, "func allowMainGreetClicked(db *clarity.Db) error {");
    has(main, "if err := allowMainGreetClicked(db); err != nil {");
    let cli = cli(CLUB);
    let command = file(&cli, "club_module.go").split("func commandMembers(").nth(1).unwrap().to_string();
    has(&command.lines().skip(1).take(8).collect::<Vec<_>>().join("\n"), "clarity.Allow(");
}

#[test]
fn readable_and_writable_are_checked_on_find_and_save() {
    let files = desktop(CLUB, true);
    let module = file(&files, "club_module.go");
    has(module, "func readableMember(db *clarity.Db, member *Member) error {");
    has(module, "func writableMember(db *clarity.Db, member *Member) error {");
    has(module, "return clarity.Yours(");
    // Every record a find gives code is checked.
    let set = module.split("func SetPassword(").nth(1).unwrap();
    has(set, "if err := readableMember(db, &v");
    // So is every record a dropdown offers and every picture handed out.
    let main = file(&files, "main.go");
    has(main, "\tcase \"Member\":\n\t\trows, err := clarity.All[Member](db)\n\t\tif err != nil {\n\t\t\treturn nil, err\n\t\t}\n\t\tfor i := range rows {\n\t\t\tif err := readableMember(db, &rows[i]); err != nil {\n\t\t\t\treturn nil, err\n\t\t\t}\n\t\t}\n");
    has(main, "\tcase \"Member.photo\":\n\t\tr, err := clarity.Get[Member](db, id)\n\t\tif err != nil {\n\t\t\treturn nil, err\n\t\t}\n\t\tif err := readableMember(db, &r); err != nil {\n\t\t\treturn nil, err\n\t\t}\n\t\treturn clarity.PictureOf[Member]");
    // A save and a delete, by code or by a window, check writable first.
    has(module, "func (r *Member) BeforeSave(db *clarity.Db) error {\n\treturn writableMember(db, r)\n}");
    has(module, "func (r *Member) BeforeDelete(db *clarity.Db) error {\n\treturn writableMember(db, r)\n}");
}

#[test]
fn a_hashed_field_keeps_the_hash_and_the_helpers_make_tokens_and_codes() {
    let files = desktop(CLUB, true);
    let module = file(&files, "club_module.go");
    has(module, "clarity.Hashed(authCall, \"argon2id\", typed, 12)");
    has(module, "clarity.Hashed(authCall, \"sha256\", token)");
    has(module, "clarity.Hashed(authCall, \"sha256-lines\", codes)");
    has(module, "clarity.NewToken(authCall)");
    has(module, "clarity.NewRecoveryCodes(authCall, 8)");
}

#[test]
fn the_command_line_signs_in_before_a_command_runs() {
    let files = cli(CLUB);
    let main = file(&files, "main.go");
    has(main, "\tsignin, args := clarity.CliSigninFlags(os.Args[1:])\n\tos.Args = append(os.Args[:1], args...)\n");
    has(main, "\t\tif _, err := cli.Signin(db, signin); err != nil {\n\t\t\treturn 1, err\n\t\t}\n");
    // Without cli among the targets, the command line signs nobody in.
    let (mut project, mut settings) = project(CLUB, true);
    let mut signin = settings.signin.clone().unwrap();
    signin.targets = vec!["desktop".into()];
    settings.signin = Some(signin.clone());
    project.signin = Some(signin);
    let files = clarity_gen::generate_cli(&project, &settings).unwrap();
    let main = file(&files, "main.go");
    assert!(!main.contains("CliSigninFlags"), "{main}");
}

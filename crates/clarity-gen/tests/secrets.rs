//! Secret settings (DESIGN 5.27) on every stack: kept in the system's
//! protected store per program, never in the settings file. A web API signs
//! with one and a database connects with one, each named in the project file
//! as `setting:name` and in the program as `setting:<program>/name`.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};

const SHOP: &str = "module shop

settings
    shopDb: secret

entity Customer
    name: text(80)

page Main: window
    password name connection at 1,1 span 6 bind settings.shopDb
";

fn generated(backend: Backend, ui: Ui, connection: &str) -> String {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    // A SQLite file is no secret; a server database's connection may be.
    if connection != "data/shop.db" {
        settings.datasources[0].kind = SourceKind::Postgres;
        settings.datasources[0].url = connection.into();
    }
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

const STACKS: [(Backend, Ui); 5] = [(Backend::Rust, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::Csharp, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity)];

/// C and C++ store records in SQLite only, for now, so they have no
/// connection to keep secret yet.
#[test]
fn a_database_connects_with_the_secret_setting_of_its_program() {
    for (backend, ui) in &STACKS[..3] {
        let (backend, ui) = (*backend, *ui);
        let code = generated(backend, ui, "setting:shopDb");
        assert!(code.contains("/shopDb"), "{backend:?} names the setting of its program");
        assert!(!code.contains("\"setting:shopDb\""), "{backend:?} keeps the project file's name");
    }
}

#[test]
fn the_secrets_are_named_to_the_settings() {
    let rust = generated(Backend::Rust, Ui::Clarity, "data/shop.db");
    assert!(rust.contains("const SECRETS: &[&str] = &[\"shopDb\"];"));
    assert!(rust.contains("clarity_runtime::settings::read_with(PROGRAM, SECRETS)"));
    let go = generated(Backend::Go, Ui::Clarity, "data/shop.db");
    assert!(go.contains("clarity.Secrets = secrets.System{}"));
    // The desktop's service keeps them in the store when the library keeps the settings.
    assert!(go.contains(", Secrets: []string{\"shopDb\"}}"), "Go names its secrets to the service");
    let csharp = generated(Backend::Csharp, Ui::Clarity, "data/shop.db");
    assert!(csharp.contains("KeptSettings.Read(ProgramName, Secrets)"));
    for backend in [Backend::C, Backend::Cpp] {
        let code = generated(backend, clarity_project::usual_ui(backend), "data/shop.db");
        assert!(code.contains("secrets_use();"), "{backend:?} turns the store on");
        assert!(code.contains("clarity_secrets"), "{backend:?} links the store");
    }
    let cpp = generated(Backend::Cpp, Ui::Clarity, "data/shop.db");
    assert!(cpp.contains("options.secrets = nlohmann::json::array({\"shopDb\"});"), "C++ names its secrets to the service");
}

/// The app's `on unconnected` (DESIGN 5.27) takes over from the window that
/// keeps the connection: the start says it, with why, and it opens what shows first.
const HANDLED: &str = "
app Shop
    main Main
    on unconnected: notConnected

code Shop
    handler notConnected(reason: text)
        message reason
        open Main
";

fn handled(backend: Backend, ui: Ui) -> String {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.datasources[0].kind = SourceKind::Postgres;
    settings.datasources[0].url = "setting:shopDb".into();
    let source = format!("{SHOP}{HANDLED}");
    let project = clarity_spec::load_in([("shop.clarity", source.as_str())], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

#[test]
fn the_app_handles_its_databases_not_opening() {
    let rust = handled(Backend::Rust, Ui::Clarity);
    assert!(rust.contains("const UNCONNECTED_HANDLER: Option<&str> = Some(\"notConnected\");"));
    assert!(rust.contains("const CONNECTION_WINDOW: Option<&str> = Some(\"Main\");"));
    assert!(rust.contains("clarity_runtime::ui::answer(value(0)?)?"));
    // Why, in Clarity's own words the project translates.
    assert!(rust.contains("Some(clarity_runtime::unconnected_reason(&err))"));
    // The program opens them again when the person has kept their connection.
    assert!(rust.contains("    fn connect(&self) -> Answer<()> {\n        let db = open_databases().map_err(|err| clarity_runtime::unconnected_reason(&err))?;"));
    // Clarity.UI shows what the handler opens first, or the window that keeps the connection (DESIGN 7.9).
    assert!(rust.contains("program::Unconnected { handler: UNCONNECTED_HANDLER.map(String::from), window: CONNECTION_WINDOW.map(String::from), problem }"));
    let go = handled(Backend::Go, Ui::Clarity);
    assert!(go.contains("unconnected = &shell.Unconnected{Handler: \"notConnected\", Window: \"Main\", Problem: clarity.UnconnectedReason(err)}"));
    assert!(go.contains("reason, err := clarity.Answer[string](values[0])"));
    let csharp = handled(Backend::Csharp, Ui::Clarity);
    assert!(csharp.contains("RunUnconnected(unconnecting, reason => OnNotConnected(reason));"));
    // Clarity.UI shows what the handler opens first, or the window that keeps the connection (DESIGN 7.9).
    assert!(csharp.contains("Handler = (db, unconnecting) => new AppUnconnectedViewModel(db).Unconnected(unconnecting),"));
    assert!(csharp.contains("}, Unconnected.Reason(e));"));
}

/// C and C++ keep their records in SQLite: a file that does not open runs
/// the app's `on unconnected` too.
#[test]
fn c_and_cpp_handle_their_database_not_opening() {
    for backend in [Backend::C, Backend::Cpp] {
        let mut settings: Settings = starter("Shop", "shop");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let source = format!("{SHOP}{HANDLED}");
        let project = clarity_spec::load_in([("shop.clarity", source.as_str())], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
        let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        let code: String = files.iter().map(|f| f.text.clone()).collect();
        assert!(code.contains("options.unconnected_handler = \"notConnected\";"), "{backend:?} names the handler to the screen");
        assert!(code.contains("options.connect = "), "{backend:?} connects again");
        // The shell says why in Clarity's own words, from the error kept.
        let kept = if backend == Backend::C { "unconnected_error = *clarity_error();" } else { "unconnected = clarity::unconnected_reason(e);" };
        assert!(code.contains(kept), "{backend:?} keeps why");
    }
}

/// An application with `on unconnected`, written out to be built.
fn written(backend: Backend, ui: Ui) -> std::path::PathBuf {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    // A server database whose connection is a secret setting, where the stack has one;
    // C and C++ have SQLite only, whose file may not open.
    if !matches!(backend, Backend::C | Backend::Cpp) {
        settings.datasources[0].kind = SourceKind::Postgres;
        settings.datasources[0].url = "setting:shopDb".into();
    }
    let source = format!("{SHOP}{HANDLED}");
    let project = clarity_spec::load_in([("shop.clarity", source.as_str())], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(format!("unconnected-{backend:?}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["build", "data", "target", "bin", "obj"]).unwrap();
    out
}

fn run(dir: &std::path::Path, command: &mut std::process::Command) -> String {
    let out = command.current_dir(dir).output().unwrap_or_else(|e| panic!("{command:?}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{command:?} failed in {}:\n{said}", dir.display());
    said
}

#[test]
#[ignore = "builds the Rust program with cargo (Tauri)"]
fn an_unconnected_app_builds_in_rust() {
    let out = written(Backend::Rust, Ui::Clarity);
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    run(&out, std::process::Command::new("cargo").args(["build", "--quiet"]).env("CARGO_TARGET_DIR", &target));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the Go program"]
fn an_unconnected_app_builds_in_go() {
    let out = written(Backend::Go, Ui::Clarity);
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
    let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
    std::fs::write(out.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", clarity_project::slashed(&runtime))).unwrap();
    assert_eq!(run(&out, std::process::Command::new("gofmt").args(["-l", "."])), "", "gofmt would change these files");
    run(&out, std::process::Command::new("go").args(["vet", "./..."]));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C# program with dotnet"]
fn an_unconnected_app_builds_in_csharp() {
    let out = written(Backend::Csharp, Ui::Clarity);
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    run(&out, std::process::Command::new("dotnet").args(["build", "-warnaserror"]).env("CLARITY_RUNTIME", runtime));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C and C++ programs with CMake"]
fn an_unconnected_app_builds_in_c_and_cpp() {
    for backend in [Backend::C, Backend::Cpp] {
        let out = written(backend, clarity_project::usual_ui(backend));
        let said = run(&out, std::process::Command::new("cmake").args(["--workflow", "--preset", "debug"]));
        let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
        assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
        std::fs::remove_dir_all(&out).unwrap();
    }
}

/// Every stack keeps the settings and the secrets under one name, the
/// project's in snake case (conformance/secret-store.json): the desktop
/// program and the command-line program alike, so what one kept another finds.
#[test]
fn every_stack_keeps_under_the_one_program_name() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/secret-store.json")).unwrap();
    for case in cases["programs"].as_array().unwrap() {
        let (name, program) = (case["project"].as_str().unwrap(), case["program"].as_str().unwrap());
        for (backend, ui) in STACKS {
            for cli in [false, true] {
                let mut settings: Settings = starter(name, "shop");
                settings.stack.backend = backend;
                settings.stack.ui = ui;
                // A server database connects with the secret where the stack has one.
                let server = !matches!(backend, Backend::C | Backend::Cpp);
                if server {
                    settings.datasources[0].kind = SourceKind::Postgres;
                    settings.datasources[0].url = "setting:shopDb".into();
                }
                let source = if server && !cli { format!("{SHOP}{HANDLED}").replace("app Shop", &format!("app {name}")).replace("code Shop", &format!("code {name}")) } else { SHOP.to_string() };
                let project = clarity_spec::load_in([("shop.clarity", source.as_str())], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
                let files = if cli { clarity_gen::generate_cli(&project, &settings) } else { generate(&project, &settings) }.unwrap_or_else(|e| panic!("{name} {backend:?} cli={cli}: {e:#?}"));
                let code: String = files.iter().map(|f| f.text.clone()).collect();
                let kept_as = match (backend, cli) {
                    (Backend::Rust, _) => format!("const PROGRAM: &str = \"{program}\";"),
                    (Backend::Go, false) => format!("shell.Options{{Name: \"{program}\","),
                    (Backend::Go, true) => format!("const program = \"{program}\""),
                    (Backend::Csharp, _) => format!("const string ProgramName = \"{program}\";"),
                    (Backend::C, false) => format!("ClarityServiceOptions options = {{\"{program}\","),
                    (Backend::C, true) => format!("#define PROGRAM \"{program}\""),
                    (Backend::Cpp, false) => format!("clarity::shell::Options options{{\"{program}\","),
                    (Backend::Cpp, true) => format!("const std::string PROGRAM = \"{program}\";"),
                };
                assert!(code.contains(&kept_as), "{name} on {backend:?} (cli: {cli}) keeps its settings as {program}: wanted {kept_as}");
                if server {
                    assert!(code.contains(&format!("setting:{program}/shopDb")), "{name} on {backend:?} (cli: {cli}) finds its secret as {program}");
                }
            }
        }
    }
}

/// A Rust program on Clarity.UI driven without a window (CLARITY_HEADLESS)
/// keeps its secrets in memory for the run, apart from the person's keyring.
#[test]
fn a_driven_rust_program_keeps_its_secrets_in_memory() {
    let text = generated(Backend::Rust, Ui::Clarity, "data/shop.db");
    assert!(text.contains("    if std::env::var_os(\"CLARITY_HEADLESS\").is_some_and(|v| !v.is_empty()) {\n        clarity_runtime::secrets::use_memory();\n    }\n"), "{text}");
}

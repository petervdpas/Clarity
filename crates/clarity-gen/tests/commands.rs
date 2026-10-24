//! Command-line programs (DESIGN 5.20) as generated code: a program of its
//! own beside the desktop one, over the same records and actions. The
//! slow tests build it and run its commands; they run only when asked:
//! `cargo test -p clarity-gen --test commands -- --ignored --exact <name>`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clarity_project::{Backend, Target};

const FIXTURE: &str = include_str!("fixtures/commands.clarity");

fn settings(backend: Backend) -> clarity_project::Settings {
    let mut settings = clarity_project::parse("project Shop\n    stack rust cli\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    settings.stack.backend = backend;
    settings
}

fn project() -> clarity_spec::Project {
    let project = clarity_spec::load_sources([("shop/shop.clarity", FIXTURE)]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

#[test]
fn a_cli_project_is_one_program_in_out_cli() {
    let programs = clarity_gen::programs(&project(), &settings(Backend::Rust)).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(programs.iter().map(|p| p.target).collect::<Vec<_>>(), [Target::Cli]);
}

#[test]
fn rust_writes_a_program_without_a_window() {
    let files = clarity_gen::generate_cli(&project(), &settings(Backend::Rust)).unwrap_or_else(|e| panic!("{e:#?}"));
    let cargo = file(&files, "Cargo.toml");
    assert!(cargo.contains("name = \"shop\""), "{cargo}");
    assert!(!cargo.contains("tauri"), "{cargo}");
    let main = file(&files, "src/main.rs");
    assert!(main.contains("clarity_runtime::cli::run_program"), "{main}");
    assert!(main.contains("line: vec![\"customer\".into(), \"add\".into()]"), "{main}");
    let module = file(&files, "src/modules/shop.rs");
    assert!(module.contains("pub fn command_customer_add(db: &Db, cli: &mut Session, w: &mut crate::CommandData, name: String, email: String, vip: bool, limit: Decimal, since: Option<NaiveDate>) -> Result<i32>"), "{module}");
    assert!(module.contains("cli.print("), "{module}");
    assert!(module.contains("cli.confirm("), "{module}");
    assert!(module.contains("cli.print_table("), "{module}");
    assert!(module.contains("return Ok((3i64) as i32);"), "{module}");
    // No windows: the module has its records, actions and commands only.
    assert!(!module.contains("ui: &mut Ui"), "{module}");
}

/// Where builds go: target/generated in the repository, on disk.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What a run of the program gave: its exit code, standard output and standard error.
fn run(program: &Path, dir: &Path, args: &[&str], input: &str) -> (i32, String, String) {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("LANG", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
}

/// The commands of a built program, run as a person or a script runs them.
fn the_commands_run(program: &Path, dir: &Path) {
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let (code, out, _) = run(program, dir, &["--help"], "");
    assert_eq!(code, 0);
    assert!(out.starts_with("The shop from the command line\n\nUsage: shop <command> [options]\n"), "{out}");
    assert!(out.contains("  customer add     Adds a customer\n"), "{out}");

    let (code, out, err) = run(program, dir, &["customer", "add", "Ada", "ada@example.org", "--vip", "--limit", "500"], "");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "Added Ada\n", ""));
    assert_eq!(run(program, dir, &["customer", "add", "Bob", "bob@example.org"], "").0, 0);
    let (code, out, _) = run(program, dir, &["customer", "list"], "");
    assert_eq!(code, 0);
    assert_eq!(out, "Name  Email            Kind      Credit limit\nAda   ada@example.org  Business           500\nBob   bob@example.org  Person            1000\n");

    // Wrong command lines end with 2, refusals with 1.
    let (code, _, err) = run(program, dir, &["customer", "add", "Ada", "a@b.c", "--limit", "9000"], "");
    assert_eq!((code, err.lines().next()), (2, Some("--limit is at most 5000")));
    let (code, _, err) = run(program, dir, &["custmer"], "");
    assert_eq!((code, err.lines().next()), (2, Some("There is no command custmer")));
    let (code, _, err) = run(program, dir, &["customer", "remove", "Zed"], "");
    assert_eq!((code, err.as_str()), (1, "There is no customer Zed\n"));
    // Piped, nobody answers: a question without a default ends the program.
    let (code, _, err) = run(program, dir, &["customer", "remove", "Bob"], "Remove\n");
    assert_eq!((code, err.as_str()), (2, "There is nobody to answer: Remove Bob?\n"));

    // A setting set by a command is kept for the next one, and exit gives its code.
    assert_eq!(run(program, dir, &["page", "--size", "50"], "").0, 3);
    let (code, out, _) = run(program, dir, &["page"], "");
    assert_eq!((code, out.as_str()), (0, "50\n"));
}

#[test]
#[ignore = "builds the Rust program with cargo"]
fn the_rust_program_builds_and_runs_its_commands() {
    let files = clarity_gen::generate_cli(&project(), &settings(Backend::Rust)).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch("commands-rust");
    clarity_gen::write_out(&out, &files, &["target", "data", ".cargo"]).unwrap();
    // On the runtime in this repository.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
    let status = Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_BUILD_JOBS", "2").status().unwrap();
    assert!(status.success());
    the_commands_run(&out.join("target/debug/shop"), &out);
}

#[test]
#[ignore = "builds the Go program with go"]
fn the_go_program_builds_and_runs_its_commands() {
    let files = clarity_gen::generate_cli(&project(), &settings(Backend::Go)).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch("commands-go");
    clarity_gen::write_out(&out, &files, &["bin", "data", "go.work"]).unwrap();
    // On the runtime in this repository, through a workspace.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
    let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
    std::fs::write(out.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", runtime.display())).unwrap();
    let gofmt = Command::new("gofmt").args(["-l", "."]).current_dir(&out).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&gofmt.stdout), "", "gofmt would change these files");
    let vet = Command::new("go").args(["vet", "./..."]).current_dir(&out).output().unwrap();
    assert!(vet.status.success(), "{}", String::from_utf8_lossy(&vet.stderr));
    let build = Command::new("go").args(["build", "-o", "bin/shop", "."]).current_dir(&out).output().unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    the_commands_run(&out.join("bin/shop"), &out);
}

#[test]
#[ignore = "builds the C# program with dotnet"]
fn the_csharp_program_builds_and_runs_its_commands() {
    let mut settings = settings(Backend::Csharp);
    settings.stack.ui = clarity_project::Ui::Avalonia;
    let files = clarity_gen::generate_cli(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch("commands-csharp");
    clarity_gen::write_out(&out, &files, &["bin", "obj", "data"]).unwrap();
    // On the runtime projects in this repository.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp").canonicalize().unwrap();
    let build = Command::new("dotnet")
        .args(["build", "--nologo", "-tl:off", "-v", "quiet", &format!("-p:ClarityRuntime={}", runtime.display())])
        .current_dir(&out)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stdout));
    the_commands_run(&out.join("bin/Debug/net10.0/shop"), &out);
}

/// Builds a generated CMake project in `out` and gives its program.
fn cmake_build(out: &Path, program: &str) -> PathBuf {
    let configure = Command::new("cmake").args(["-S", ".", "-B", "build/debug", "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Debug"]).current_dir(out).output().unwrap();
    assert!(configure.status.success(), "{}{}", String::from_utf8_lossy(&configure.stdout), String::from_utf8_lossy(&configure.stderr));
    let build = Command::new("cmake").args(["--build", "build/debug", "-j2"]).current_dir(out).output().unwrap();
    assert!(build.status.success(), "{}{}", String::from_utf8_lossy(&build.stdout), String::from_utf8_lossy(&build.stderr));
    out.join("build/debug").join(program)
}

#[test]
#[ignore = "builds the C program with CMake"]
fn the_c_program_builds_and_runs_its_commands() {
    let files = clarity_gen::generate_cli(&project(), &settings(Backend::C)).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch("commands-c");
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    let program = cmake_build(&out, "shop");
    the_commands_run(&program, &out);
}

#[test]
#[ignore = "builds the C++ program with CMake"]
fn the_cpp_program_builds_and_runs_its_commands() {
    let files = clarity_gen::generate_cli(&project(), &settings(Backend::Cpp)).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch("commands-cpp");
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    let program = cmake_build(&out, "shop");
    the_commands_run(&program, &out);
}

#[test]
fn a_command_line_typed_in_clarity_is_split_into_words() {
    assert_eq!(clarity_gen::command_words("customer add Ada ada@example.org"), ["customer", "add", "Ada", "ada@example.org"]);
    assert_eq!(clarity_gen::command_words("  customer   remove \"Ada Lovelace\" --reason 'moved away' "), ["customer", "remove", "Ada Lovelace", "--reason", "moved away"]);
    assert_eq!(clarity_gen::command_words("add \"\" x"), ["add", "", "x"]);
    assert!(clarity_gen::command_words("").is_empty());
}

/// The programs of an example as its project file names them, from its folder.
fn example_programs(name: &str) -> (clarity_project::Settings, Vec<clarity_gen::Program>) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}.clarity"));
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let settings = clarity_project::load(&file).settings.unwrap();
    let programs = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    (settings, programs)
}

#[test]
#[ignore = "builds the ledger example with dotnet"]
fn the_ledger_example_runs_on_its_own_stack() {
    let (settings, programs) = example_programs("ledger");
    assert_eq!(settings.stack.backend, Backend::Csharp);
    assert_eq!(programs.iter().map(|p| p.target).collect::<Vec<_>>(), [Target::Cli]);
    let out = scratch("example-ledger");
    clarity_gen::write_programs(&out, &programs, Backend::Csharp, &[]).unwrap();
    let cli = out.join("cli");
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp").canonicalize().unwrap();
    let build = Command::new("dotnet").args(["build", "--nologo", "-tl:off", "-v", "quiet", &format!("-p:ClarityRuntime={}", runtime.display())]).current_dir(&cli).output().unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stdout));
    std::fs::create_dir_all(cli.join("data")).unwrap();
    let program = cli.join("bin/Debug/net10.0/ledger");
    assert_eq!(run(&program, &cli, &["account", "add", "Bank"], "").1, "Added the account Bank\n");
    assert_eq!(run(&program, &cli, &["account", "use", "Bank"], "").0, 0);
    assert_eq!(run(&program, &cli, &["book", "80.5", "--expense"], "").1, "Booked 80.5 on Bank\n");
    let (code, out, err) = run(&program, &cli, &["report"], "");
    assert_eq!((code, out.as_str(), err.as_str()), (3, "Bank: -80.5\n", "An account is below zero\n"));
}

#[test]
#[ignore = "builds the stock example with go"]
fn the_stock_example_runs_on_its_own_stack() {
    let (settings, programs) = example_programs("stock");
    assert_eq!(settings.stack.backend, Backend::Go);
    assert_eq!(programs.iter().map(|p| p.target).collect::<Vec<_>>(), [Target::Desktop, Target::Cli]);
    let out = scratch("example-stock");
    clarity_gen::write_programs(&out, &programs, Backend::Go, &[]).unwrap();
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
    let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
    for target in ["desktop", "cli"] {
        let dir = out.join(target);
        std::fs::write(dir.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", runtime.display())).unwrap();
        let build = Command::new("go").args(["build", "-o", "bin/stock", "."]).current_dir(&dir).output().unwrap();
        assert!(build.status.success(), "{target}: {}", String::from_utf8_lossy(&build.stderr));
        std::fs::create_dir_all(dir.join("data")).unwrap();
    }
    let desktop = out.join("desktop");
    let checked = Command::new(desktop.join("bin/stock")).arg("--check").current_dir(&desktop).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&checked.stdout), "Stock: the databases are ready.\n");
    let cli = out.join("cli");
    let program = cli.join("bin/stock");
    assert_eq!(run(&program, &cli, &["add", "Pears", "--count", "3", "--low", "4"], "").0, 0);
    let (code, out, _) = run(&program, &cli, &["check"], "");
    assert_eq!((code, out.as_str()), (3, "Pears\n"));
}

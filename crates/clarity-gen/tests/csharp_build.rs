//! Generates the shop example as C# with Avalonia and builds it with
//! `dotnet build` (the AXAML compiler checks every binding), then runs it with
//! `--check`: the program opens its database, applies the migration steps and
//! stops before showing a window. Slow (NuGet and the compiler), so it only
//! runs when asked, one at a time:
//! `cargo test -p clarity-gen --test csharp_build -- --ignored --test-threads=1 --exact <name>`.
//!
//! The projects build on the runtime's own projects in runtimes/csharp (the
//! CLARITY_RUNTIME switch of the generated project file), so a change to the
//! runtime is tested before it is published; one test builds on the packages
//! on nuget.org, as a project outside this repository does.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, SourceKind, Ui};

mod scratch;
mod served;
use scratch::empty;

/// Where builds go: target/generated in the repository, on disk; never the
/// system's temporary folder, which may live in memory.
fn scratch() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
}

/// Removes a finished build. On Windows a file the program or a virus scan
/// still holds for a moment refuses to go, so it tries again for a while.
fn remove(out: &Path) {
    for _ in 0..20 {
        if std::fs::remove_dir_all(out).is_ok() || !out.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    std::fs::remove_dir_all(out).unwrap();
}

/// Which runtime a generated project builds on.
#[derive(Clone, Copy)]
enum Runtime {
    /// The runtime's projects in this repository.
    Local,
    /// The published packages.
    Packages,
}

/// `dotnet` with `args` in `out`, on `runtime`.
fn dotnet(out: &Path, args: &[&str], runtime: Runtime) -> Command {
    let mut command = Command::new("dotnet");
    command.args(args).current_dir(out);
    match runtime {
        Runtime::Local => command.env("CLARITY_RUNTIME", Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp")),
        Runtime::Packages => command.env_remove("CLARITY_RUNTIME"),
    };
    command
}

/// Generates the C# example `name` (examples/<name>.clarity) on its own
/// stack, builds it with warnings as errors and runs `--check`; with
/// `translated`, it has a translation into that language, whose satellite
/// assembly the build makes.
fn builds_and_runs_in_csharp(name: &str, program: &str, translated: Option<&str>, runtime: Runtime) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    // As Run loads it: with the project file's language, currency and themes.
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    // Its translations, whose satellite assembly the build must make.
    assert_eq!(!project.translations.is_empty(), translated.is_some(), "the example's translations");
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    // An example builds on the stack its project file names, and no other.
    assert_eq!((settings.stack.backend, settings.stack.ui), (Backend::Csharp, Ui::Avalonia), "{name} is not a C# example");
    let files = clarity_gen::generate(&project, &settings).unwrap();
    if let Some(translated) = translated {
        assert!(files.iter().any(|f| f.path == format!("Texts.{translated}.resx")));
    }

    let out = scratch().join(format!("clarity-gen-csharp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();

    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], runtime).output().expect("the .NET SDK is installed");
    assert!(
        built.status.success(),
        "the generated project does not build; it is in {}\n{}",
        out.display(),
        String::from_utf8_lossy(&built.stdout)
    );

    let ran = dotnet(&out, &["run", "--no-build", "--", "--check"], runtime).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "{said}{}", String::from_utf8_lossy(&ran.stderr));
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    // The database the project file names, with every step applied.
    // A project with records has its database; one without needs none.
    if project.stored_entities().next().is_some() {
        assert!(out.join(format!("data/{name}.db")).exists());
    }
    // The translated texts as a satellite assembly, as .NET looks for them.
    if let Some(translated) = translated {
        assert!(out.join(format!("bin/Debug/net10.0/{translated}/{program}.resources.dll")).exists());
    }
    remove(&out);
}

/// The todo example on the runtime's packages from nuget.org, at the version
/// the generator names: what a project outside this repository builds on.
#[test]
#[ignore]
fn the_todo_example_builds_and_runs_on_the_published_packages() {
    builds_and_runs_in_csharp("todo", "Todo", Some("nl"), Runtime::Packages);
}

/// The third example: tasks edited in their table, and the people who finish them.
#[test]
#[ignore]
fn the_todo_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("todo", "Todo", Some("nl"), Runtime::Local);
}

/// The todo example on each server database: the project builds with only
/// that driver, and where a scratch database is named (the variables of
/// tests/servers.rs) the program migrates it with `--check`, twice.
#[test]
#[ignore]
fn the_todo_example_builds_and_migrates_on_every_server_database() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/todo.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let tables: Vec<String> = project.entities().map(|e| e.table_name()).collect();
    for (kind, dialect, variable) in [
        (SourceKind::Postgres, clarity_runtime::Dialect::Postgres, "CLARITY_TEST_POSTGRES"),
        (SourceKind::Mysql, clarity_runtime::Dialect::Mysql, "CLARITY_TEST_MYSQL"),
        (SourceKind::Mssql, clarity_runtime::Dialect::Mssql, "CLARITY_TEST_MSSQL"),
    ] {
        let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
        assert_eq!(settings.stack.backend, Backend::Csharp, "todo is a C# example");
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:TODO_DATABASE".into();
        let files = clarity_gen::generate(&project, &settings).unwrap();

        let out = scratch().join(format!("clarity-gen-csharp-{variable}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
        let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
        assert!(built.status.success(), "{variable}: the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));

        if let Ok(at) = std::env::var(variable) {
            empty(&at, dialect, &tables);
            for _ in 0..2 {
                let ran = dotnet(&out, &["run", "--no-build", "--", "--check"], Runtime::Local)
                    .env("TODO_DATABASE", &at)
                    .output()
                    .unwrap();
                let said = String::from_utf8_lossy(&ran.stdout);
                assert!(ran.status.success(), "{variable}: {said}{}", String::from_utf8_lossy(&ran.stderr));
                assert!(said.contains("Todo: the databases are ready."), "{said}");
            }
            let db = clarity_runtime::Db::connect(&[("main", dialect, &at)]).unwrap();
            let steps = project.migrations.as_ref().map_or(0, |m| m.get("main").map_or(0, Vec::len));
            assert_eq!(db.schema_version().unwrap(), steps, "{variable}: every step once");
            empty(&at, dialect, &tables);
        } else {
            eprintln!("{variable} is not set; built only");
        }
        remove(&out);
    }
}

/// The test engine of conformance/engines.json in a generated application:
/// every function called from Clarity code, its values crossing as JSON.
const ENGINE_SHOP: &str = include_str!("../../../conformance/engines/shop.clarity");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

/// A program beside the generated one that calls its functions.
const ENGINE_CHECK: &str = r#"using Clarity.Runtime;
using Shop;

static void Expect(bool holds, string what)
{
    if (!holds)
        throw new Exception(what);
}

Engines.Check();
Expect(Logic.Answer(null!, 2) == 42, "answer");
Expect(Logic.Greeting(null!, "Grüße") == "Hello, Grüße!", "greeting");
var lines = new List<Line> { new() { Price = 9.95m, Count = 3 }, new() { Price = 0.10m, Count = 1 } };
Expect(Logic.Sum(null!, lines) == 29.95m, "sum");
Expect(Logic.SizeClass(null!, 12) == Size.Large, "size");
var cheaper = Logic.Cheaper(null!, new Order { Id = 7, Number = "ORD-7", Total = 100.05m });
Expect(cheaper.Id == 7 && cheaper.Total == 90.05m, "cheaper");
try
{
    Logic.Refused(null!, "no stock");
    Expect(false, "refused");
}
catch (RefusedException e)
{
    Expect(e.Message == "no stock", "refused in its words");
}
// Bytes cross to an engine and back as they are.
Expect(Logic.Backwards(null!, [1, 2, 3]).SequenceEqual(new byte[] { 3, 2, 1 }), "backwards");
Expect(Logic.Measured(null!, [1, 2, 3]) == 6, "measured");
// An engine with state counts on from what its on open set.
Expect(Logic.Counted(null!) == 11 && Logic.Counted(null!) == 12, "counted");
// Each open window has an instance of its own of an engine with state per
// window, which goes when the window closes.
var first = new MainViewModel(null!);
var second = new MainViewModel(null!);
first.MovedCommand.Execute(null);
second.MovedCommand.Execute(null);
first.MovedCommand.Execute(null);
Expect(first.Moves == 12 && second.Moves == 11, $"moves {first.Moves} and {second.Moves}");
first.Closing();
first.MovedCommand.Execute(null);
second.MovedCommand.Execute(null);
Expect(first.Moves == 11 && second.Moves == 12, $"after closing, moves {first.Moves} and {second.Moves}");
Console.WriteLine("every engine function answered");
"#;

#[test]
#[ignore]
fn an_engine_answers_in_the_generated_application_in_csharp() {
    let mut project = clarity_spec::load_sources([("shop.clarity", ENGINE_SHOP)]);
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let root = scratch().join(format!("clarity-gen-csharp-engine-{}", std::process::id()));
    let (out, check) = (root.join("Shop"), root.join("Check"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));

    std::fs::create_dir_all(&check).unwrap();
    std::fs::write(check.join("Program.cs"), ENGINE_CHECK).unwrap();
    std::fs::write(
        check.join("Check.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <OutputType>Exe</OutputType>\n    <TargetFramework>net10.0</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"../Shop/Shop.csproj\" />\n  </ItemGroup>\n</Project>\n",
    )
    .unwrap();
    let ran = dotnet(&check, &["run", "--nologo"], Runtime::Local).output().unwrap();
    let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
    assert!(ran.status.success() && said.contains("every engine function answered"), "{said}\nin {}", root.display());
    remove(&root);
}

/// Functions named like a type, an enum value, a constructor or a window
/// build beside them, with `-warnaserror` (tests/fixtures/name_clash.clarity).
#[test]
#[ignore]
fn functions_named_like_types_build_in_csharp() {
    let project = clarity_spec::load_sources([("shop.clarity", include_str!("fixtures/name_clash.clarity"))]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-csharp-names-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    remove(&out);
}

/// The fifth example: noughts and crosses, each window's game kept by an engine.
#[test]
#[ignore]
fn the_noughts_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("noughts", "Noughts", Some("nl"), Runtime::Local);
}

/// The noughts example's Board engine, written in Clarity and compiled to
/// WebAssembly, played through the C# application's engine host from a
/// program of its own.
const BOARD_CHECK: &str = r#"using Clarity.Avalonia;
using Clarity.Runtime;
using Noughts;

// The texts in the project's default language, as the application starts
// them; nothing is remembered on disk.
Locale.Start(Noughts.Texts.BaseLanguage, Noughts.Texts.Languages, null, Noughts.Texts.In, Labels.Named);

static void Expect(bool holds, string what)
{
    if (!holds)
        throw new Exception(what);
}

static string Refusal(Action call)
{
    try
    {
        call();
        return "";
    }
    catch (RefusedException e)
    {
        return e.Message;
    }
}

// The engine's functions in a window; what it says is a key, shown in the person's language.
static string Play(long window, long square) => Locale.Translate(Engines.Board.CallIn<string>(window, "play", square));
static long Mark(long window, long square) => Engines.Board.CallIn<long>(window, "mark", square);

var said = Play(1, 1);
Expect(said == "Your move", $"play said {said}");
Expect(Mark(1, 1) == 1, "your cross");
// The computer took the centre.
Expect(Mark(1, 5) == 2, "the centre");
Expect(Refusal(() => Play(1, 5)) == "That square is taken", "taken");
Expect(Refusal(() => Play(1, 10)) == "There are nine squares", "nine squares");
// Another window has a board of its own, and a closed window's board goes.
Expect(Mark(2, 1) == 0, "another window");
OpenWindows.Close(1);
Expect(Mark(1, 1) == 0, "closed");
// The computer blocks your line.
Play(3, 1);
Play(3, 2);
Expect(Mark(3, 3) == 2, "blocked");
Expect(Locale.Translate(Engines.Board.CallIn<string>(3, "restart")) == "Your move", "restart");
Expect(Engines.Board.CallIn<long>(3, "taken") == 0, "an empty board");
Console.WriteLine("the board played");
"#;

#[test]
#[ignore]
fn an_engine_written_in_clarity_runs_in_csharp() {
    board_runs_in_csharp(false);
}

/// The same Board run by the interpreter (`engines interpreted`).
#[test]
#[ignore]
fn an_interpreted_engine_runs_in_csharp() {
    board_runs_in_csharp(true);
}

fn board_runs_in_csharp(interpreted: bool) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/noughts.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.engines_interpreted = interpreted;
    assert_eq!((settings.stack.backend, settings.stack.ui), (Backend::Csharp, Ui::Avalonia), "noughts is a C# example");
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let root = scratch().join(format!("clarity-gen-csharp-board-{}-{}", interpreted, std::process::id()));
    let (out, check) = (root.join("Noughts"), root.join("Check"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    std::fs::create_dir_all(&check).unwrap();
    std::fs::write(check.join("Program.cs"), BOARD_CHECK).unwrap();
    std::fs::write(
        check.join("Check.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <OutputType>Exe</OutputType>\n    <TargetFramework>net10.0</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"../Noughts/Noughts.csproj\" />\n  </ItemGroup>\n</Project>\n",
    )
    .unwrap();
    let ran = dotnet(&check, &["run", "--nologo"], Runtime::Local).output().unwrap();
    let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
    assert!(ran.status.success() && said.contains("the board played"), "{said}\nin {}", root.display());
    remove(&root);
}

/// A window that asks each kind of question (DESIGN 5.14) builds without a
/// warning, every binding checked.
#[test]
#[ignore]
fn questions_and_their_answers_build_in_csharp() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-csharp-asks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    remove(&out);
}

/// Pictures of the resources folder, SVG among them (conformance/drawing.json):
/// an image by name, and a canvas drawing one, build without a warning.
#[test]
#[ignore]
fn pictures_build_in_csharp() {
    let source = "module art\n\napp Art\n    main Gallery\n\npage Gallery: window\n    image name logo at 1,1 span 3 rows 3 source \"logo.svg\"\n    canvas name field at 4,1 span 6 rows 6 units 160 x 100 on draw: drawField\n\ncode Gallery\n    handler drawField\n        field.image(10, 20, 17, 5, \"logo.svg\")\n";
    let mut project = clarity_spec::load_sources([("art.clarity", source)]);
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"red\"/></svg>".to_vec();
    clarity_spec::resources::attach(&mut project, vec![clarity_spec::resources::ResourceFile { name: "logo.svg".into(), bytes: svg }]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = clarity_project::starter("Art", "art");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-csharp-art-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    remove(&out);
}

/// An engine beside the screen (DESIGN 5.12, conformance/engines/play.clarity):
/// the application builds with the engine embedded and a canvas that runs
/// it. With CLARITY_KEEP set the built application stays, to run it.
#[test]
#[ignore]
fn an_engine_beside_the_screen_builds_in_csharp() {
    let mut project = clarity_spec::load_sources([("play.clarity", include_str!("../../../conformance/engines/play.clarity"))]);
    let bytes = include_bytes!("../../../conformance/engines/screen.wasm").to_vec();
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "screen.wasm".into(), bytes }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Play", "play");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join("clarity-gen-csharp-play");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    if std::env::var_os("CLARITY_KEEP").is_none() {
        remove(&out);
    }
}

/// The web target (DESIGN 7.6): the server builds, keeps a session for a
/// browser tab and answers it with the main window's state. The browser
/// program needs .NET's wasm-tools workload; it is left out of the build here.
#[test]
#[ignore]
fn the_web_server_keeps_sessions_in_csharp() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    settings.stack.targets = vec![clarity_project::Target::Web];
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-csharp-web-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "Server", "--nologo", "-v", "quiet", "-warnaserror", "-p:SkipBrowser=true"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the server does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    let address = "127.0.0.1:18735";
    let program = out.join("Server/bin/Debug/net10.0/Music.Server");
    let mut server = Command::new(&program).args(["--address", address]).current_dir(&out).spawn().unwrap();
    let started = served::post(address, "/clarity/session", r#"{"do": "start"}"#);
    let _ = server.kill();
    let _ = server.wait();
    let reply: serde_json::Value = serde_json::from_str(&started).unwrap_or_else(|e| panic!("{e}: {started}"));
    assert!(reply["session"].as_str().is_some_and(|s| !s.is_empty()), "{started}");
    let window = &reply["windows"][0];
    assert!(window["type"].as_str().unwrap_or("").starts_with("Music."), "{started}");
    assert!(window["state"]["parts"].is_object(), "{started}");
    remove(&out);
}

/// The walker example, on its own stack: C# with Avalonia on the web. Its
/// server builds, with the browser program published into it, and a session
/// opens its window.
#[test]
#[ignore]
fn the_walker_example_serves_in_csharp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/walker.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    assert_eq!((settings.stack.backend, settings.stack.ui), (Backend::Csharp, Ui::Avalonia), "walker is not a C# example");
    let out = scratch().join(format!("clarity-gen-csharp-walker-{}", std::process::id()));
    settings.datasources[0].url = out.join("data/walker.db").display().to_string();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
    let built = dotnet(&out, &["build", "Server", "--nologo", "-v", "quiet"], Runtime::Local).output().unwrap();
    assert!(built.status.success(), "the walker does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    assert!(out.join("Server/bin/Debug/net10.0/wwwroot/_framework/dotnet.js").exists(), "the browser program is published into the server");
    let address = "127.0.0.1:18738";
    let mut server = Command::new(out.join("Server/bin/Debug/net10.0/Walker.Server")).args(["--address", address]).current_dir(&out).spawn().unwrap();
    let page = served::post(address, "/clarity/session", r#"{"do": "start"}"#);
    let _ = server.kill();
    let _ = server.wait();
    let reply: serde_json::Value = serde_json::from_str(&page).unwrap_or_else(|e| panic!("{e}: {page}"));
    assert!(reply["windows"][0]["type"].as_str().unwrap_or("").starts_with("Walker.FieldViewModel"), "{page}");
    remove(&out);
}

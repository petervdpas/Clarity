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
use scratch::empty;

/// Where builds go: target/generated in the repository, on disk; never the
/// system's temporary folder, which may live in memory.
fn scratch() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
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
    std::fs::remove_dir_all(&out).unwrap();
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
        std::fs::remove_dir_all(&out).unwrap();
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
    std::fs::remove_dir_all(&root).unwrap();
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
    std::fs::remove_dir_all(&out).unwrap();
}

/// The fifth example: noughts and crosses, each window's game kept by an engine.
#[test]
#[ignore]
fn the_noughts_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("noughts", "Noughts", Some("nl"), Runtime::Local);
}

/// The noughts example's Board engine, written in Clarity, as the C# it
/// becomes, played from a program of its own.
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

var said = BoardEngine.Play(null!, 1, 1);
Expect(said == "Your move", $"play said {said}");
Expect(BoardEngine.Mark(null!, 1, 1) == 1, "your cross");
// The computer took the centre.
Expect(BoardEngine.Mark(null!, 1, 5) == 2, "the centre");
Expect(Refusal(() => BoardEngine.Play(null!, 1, 5)) == "That square is taken", "taken");
Expect(Refusal(() => BoardEngine.Play(null!, 1, 10)) == "There are nine squares", "nine squares");
// Another window has a board of its own, and a closed window's board goes.
Expect(BoardEngine.Mark(null!, 2, 1) == 0, "another window");
OpenWindows.Close(1);
Expect(BoardEngine.Mark(null!, 1, 1) == 0, "closed");
// The computer blocks your line.
BoardEngine.Play(null!, 3, 1);
BoardEngine.Play(null!, 3, 2);
Expect(BoardEngine.Mark(null!, 3, 3) == 2, "blocked");
Expect(BoardEngine.Restart(null!, 3) == "Your move", "restart");
Expect(BoardEngine.Taken(null!, 3) == 0, "an empty board");
Console.WriteLine("the board played");
"#;

#[test]
#[ignore]
fn an_engine_written_in_clarity_runs_in_csharp() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/noughts.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    assert_eq!((settings.stack.backend, settings.stack.ui), (Backend::Csharp, Ui::Avalonia), "noughts is a C# example");
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let root = scratch().join(format!("clarity-gen-csharp-board-{}", std::process::id()));
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
    std::fs::remove_dir_all(&root).unwrap();
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
    std::fs::remove_dir_all(&out).unwrap();
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
        std::fs::remove_dir_all(&out).unwrap();
    }
}

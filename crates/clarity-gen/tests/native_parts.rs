//! Custom parts in the output (DESIGN 5.19 and 6.5): each stack's part is a
//! library in its own language, drawn by Clarity.UI, and code sets, reads
//! and hears it.

use clarity_gen::parts;
use clarity_spec::keyed::load_keyed;
use clarity_spec::native::PartFile;

const GAME: &str = "module game

entity Player
    name: text(80), required
    best: integer

part Arena
    native \"parts/Arena\"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    running: bool
    tells scored(points: integer, by: text)
    hears restart(level: integer)
    hears say(words: text, loud: bool)
    size 8 x 6

page Game: window
    data player: Player
    Arena name arena at 1,1 span 8 rows 6 bind player.best speed 5 on scored: arenaScored

code Game
    handler arenaScored(points: integer, by: text)
        arena.restart(points)
        arena.speed = arena.speed + 1
        arena.title = by
";

fn settings() -> clarity_project::Settings {
    clarity_project::parse("project Game\n    stack cpp desktop\n    datasource main sqlite \"data/game.db\"\n").unwrap()
}

fn generated() -> Vec<clarity_gen::OutFile> {
    clarity_gen::generate(&project(), &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

const TEST_PART_CPP: &str = r#"#include <cstdio>

#include "clarity/Arena.h"

namespace {

class Arena final : public ArenaPart {
public:
    void draw(clarity::ui::Canvas& canvas, double x, double y, double width, double height) override { canvas.fill(x, y, width, height, 0xFF2040A0); }
    void click(clarity::ui::Cx& cx) override { scored(cx, 7, "test"); }
    void show_value(int64_t, clarity::ui::Cx&) override {}
    void show_speed(int64_t speed, clarity::ui::Cx&) override { std::fprintf(stderr, "arena speed %lld\n", (long long)speed); }
    void show_title(const std::string& title, clarity::ui::Cx&) override { std::fprintf(stderr, "arena title %s\n", title.c_str()); }
    void show_running(bool, clarity::ui::Cx&) override {}
    void on_restart(clarity::ui::Cx&, int64_t level) override { std::fprintf(stderr, "arena restart %lld\n", (long long)level); }
    void on_say(clarity::ui::Cx&, const std::string&, bool) override {}
};

}  // namespace

std::unique_ptr<ArenaPart> make_Arena() { return std::make_unique<Arena>(); }
"#;

/// The test's part in Go.
const TEST_PART_GO: &str = r#"package arena

import (
	"fmt"
	"os"
)

type part struct{}

func New() Part { return &part{} }

func (p *part) Draw(canvas Canvas, x, y, width, height float64) {
	canvas.Fill(x, y, width, height, canvas.LookColor("accent"), 0)
}

// Click tells scored, as the C and C++ test parts do.
func (p *part) Click(cx Cx) { Scored(cx, 7, "test") }

func (p *part) SetEnabled(enabled bool, cx Cx) {}
func (p *part) SetValue(value int64, cx Cx)    {}
func (p *part) SetSpeed(speed int64, cx Cx)    { fmt.Fprintf(os.Stderr, "arena speed %d\n", speed) }
func (p *part) SetTitle(title string, cx Cx)   { fmt.Fprintf(os.Stderr, "arena title %s\n", title) }
func (p *part) SetRunning(running bool, cx Cx) {}
func (p *part) Restart(level int64, cx Cx)     { fmt.Fprintf(os.Stderr, "arena restart %d\n", level) }
func (p *part) Say(words string, loud bool, cx Cx) {}
"#;

const TEST_PART_RUST: &str = r#"mod clarity;
pub use clarity::*;

#[derive(Default)]
pub struct Arena;

impl ArenaPart for Arena {
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {
        let accent = canvas.look_color("accent");
        canvas.fill(x, y, width, height, accent, 0.0);
    }
    fn click(&mut self, cx: &mut dyn Cx) {
        cx.scored(7, "test");
    }
    fn show_value(&mut self, _value: i64, _cx: &mut dyn Cx) {}
    fn show_speed(&mut self, speed: i64, _cx: &mut dyn Cx) {
        eprintln!("arena speed {speed}");
    }
    fn show_title(&mut self, title: String, _cx: &mut dyn Cx) {
        eprintln!("arena title {title}");
    }
    fn show_running(&mut self, _running: bool, _cx: &mut dyn Cx) {}
    fn on_restart(&mut self, level: i64, _cx: &mut dyn Cx) {
        eprintln!("arena restart {level}");
    }
    fn on_say(&mut self, _words: String, _loud: bool, _cx: &mut dyn Cx) {}
}
"#;

/// The game, its part written in the language of `backend`.
fn project_for(backend: clarity_project::Backend) -> clarity_spec::Project {
    use clarity_project::Backend;
    let (line, test_file, test_part) = match backend {
        // A Rust part is a control of Clarity.UI, drawn through its library.
        Backend::Rust => ("crate \"parts/Arena\"", "src/lib.rs", TEST_PART_RUST),
        Backend::Go => ("go \"parts/arena\"", "arena.go", TEST_PART_GO),
        Backend::Cpp => ("native \"parts/Arena\"", "Arena.cpp", TEST_PART_CPP),
        // A C part as declaring it in a C project writes it.
        _ => ("native \"parts/Arena\"", "", ""),
    };
    let text = GAME.replace("native \"parts/Arena\"", line);
    let mut p = load_keyed([("game.clarity", text.as_str())], &[], "en");
    // A C or C++ project's part is a control of Clarity.UI.
    let starter = match backend {
        Backend::Cpp => parts::cpp_starter(&p.files[0].parts[0]),
        Backend::C => parts::c_starter(&p.files[0].parts[0]),
        _ => parts::starter_for(&p.files[0].parts[0]),
    };
    let files = starter
        .into_iter()
        .map(|(path, text)| {
            let text = if path == test_file { test_part.to_string() } else { text };
            PartFile { part: "Arena".into(), path, bytes: text.into_bytes() }
        })
        .collect();
    clarity_spec::native::attach(&mut p, files);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn project() -> clarity_spec::Project {
    project_for(clarity_project::Backend::Cpp)
}

#[test]
fn cpp_code_sets_reads_and_tells_a_custom_part() {
    let files = generated();
    let logic = files.iter().filter(|f| f.path.starts_with("src/") && f.path.ends_with(".cpp")).map(|f| f.text.as_str()).collect::<String>();
    assert!(logic.contains("ui.hear(\"arena\", \"restart\", nlohmann::json::array({nlohmann::json(points)}));"), "{logic}");
    assert!(logic.contains("ui.set_part_value(\"arena\", \"speed\", nlohmann::json("), "{logic}");
    assert!(logic.contains("ui.part_value(\"arena\", \"speed\").value_or(nlohmann::json::parse(\"5\")).get<int64_t>()"), "{logic}");
    // The tell's handler runs with the tell's values, as an answer's does.
    assert!(logic.contains("if (handler == \"arenaScored\")"), "{logic}");
    // Each part is built into the program and registered with Clarity.UI by its name.
    assert!(text(&files, "CMakeLists.txt").contains("add_subdirectory(parts/Arena)\ntarget_link_libraries(Arena PUBLIC clarity_desktop)\ntarget_link_libraries(game PRIVATE Arena)"));
    assert!(text(&files, "src/parts.cpp").contains("clarity::ui::register_part(\"Arena\", [] { return std::unique_ptr<clarity::ui::CustomPart>(make_Arena()); });"));
    assert!(!files.iter().any(|f| f.path == "parts/glue.c" || f.path.starts_with("parts/host/")), "no part host of native windows");
    // The header is the class the part implements, typed from the declaration.
    let h = text(&files, "parts/Arena/clarity/Arena.h");
    for line in [
        "class ArenaPart : public clarity::ui::CustomPart {",
        "    virtual void show_value(int64_t value, clarity::ui::Cx& cx) = 0;",
        "    virtual void show_title(const std::string& title, clarity::ui::Cx& cx) = 0;",
        "    virtual void on_say(clarity::ui::Cx& cx, const std::string& words, bool loud) = 0;",
        "        else if (name == \"speed\") show_speed(integer_of(given), cx);",
        "    void scored(clarity::ui::Cx& cx, int64_t points, const std::string& by) { cx.tell(\"scored\", nlohmann::json::array({points, by}).dump()); }",
        "std::unique_ptr<ArenaPart> make_Arena();",
    ] {
        assert!(h.contains(line), "{line}\n{h}");
    }
}

/// Generates the game for `backend` into target/generated, builds it with
/// `build`, and gives the folder.
fn built(backend: clarity_project::Backend, build: &[&str]) -> std::path::PathBuf {
    let mut settings = settings();
    settings.stack.backend = backend;
    if backend == clarity_project::Backend::Csharp {
        settings.stack.ui = clarity_project::Ui::Clarity;
    }
    let files = clarity_gen::generate(&project_for(backend), &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/generated/native-parts-{backend:?}"));
    // A Rust build keeps its target folder between runs; it takes minutes.
    if backend != clarity_project::Backend::Rust {
        let _ = std::fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&dir).unwrap();
    clarity_gen::write_out(&dir, &files, &["build", "data", "target"]).unwrap();
    if backend == clarity_project::Backend::Rust {
        // The runtime in this repository, as a developer of the runtime builds with it.
        let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
        std::fs::create_dir_all(dir.join(".cargo")).unwrap();
        std::fs::write(dir.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
    }
    if backend == clarity_project::Backend::Go {
        // The runtime in this repository, as `go work` points a Go developer at it.
        let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
        let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
        std::fs::write(dir.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", runtime.display())).unwrap();
    }
    // C# builds on the runtime's own projects in this repository.
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    // A Go desktop program links Clarity.UI's library through cgo, as Clarity's build does.
    let cgo = if backend == clarity_project::Backend::Go { clarity_gen::go_cgo_env(std::env::var_os("CLARITY_UI").as_deref().map(std::path::Path::new), std::env::consts::OS) } else { Vec::new() };
    let out = std::process::Command::new(build[0]).args(&build[1..]).current_dir(&dir).env_remove("GOWORK").env("CLARITY_RUNTIME", runtime).envs(cgo).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    dir
}

/// A C++ program on Clarity.UI with its part, driven without a window
/// (CLARITY_HEADLESS): the window's values reach the part, a click on it
/// tells `scored`, and the handler's hear and values come back. Needs
/// CLARITY_UI (the library's folder) for the build.
#[test]
#[ignore = "builds a C++ program with a custom part and drives it; run with --ignored"]
fn a_cpp_program_drives_its_custom_part() {
    use std::io::Write;
    let dir = built(clarity_project::Backend::Cpp, &["cmake", "--workflow", "--preset", "debug"]);
    let mut app = std::process::Command::new(dir.join("build/debug/game"))
        .current_dir(&dir)
        .env("CLARITY_HEADLESS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let steps = "{\"do\":\"click\",\"part\":\"arena\"}\n{\"do\":\"settle\",\"ms\":50}\n{\"do\":\"read\"}\n{\"do\":\"quit\"}\n";
    app.stdin.take().unwrap().write_all(steps.as_bytes()).unwrap();
    let out = app.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    for line in ["arena speed 5", "arena restart 7", "arena speed 6", "arena title test"] {
        assert!(said.contains(line), "{line} in\n{said}\n{}", String::from_utf8_lossy(&out.stdout));
    }
}

/// A Rust program on Clarity.UI's library with its part written in Rust,
/// driven without a window (CLARITY_HEADLESS), as the C++ and Go ones are:
/// the part draws and hears through the runtime's `native_ui`. Needs
/// CLARITY_UI (the library's folder) for the build.
#[test]
#[ignore = "builds a Rust program with a part written in Rust and drives it; slow; run with --ignored"]
fn a_rust_program_drives_its_rust_part() {
    use std::io::Write;
    let dir = built(clarity_project::Backend::Rust, &["cargo", "build"]);
    let mut app = std::process::Command::new(dir.join("target/debug/game"))
        .current_dir(&dir)
        .env("CLARITY_HEADLESS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let steps = "{\"do\":\"click\",\"part\":\"arena\"}\n{\"do\":\"settle\",\"ms\":50}\n{\"do\":\"read\"}\n{\"do\":\"quit\"}\n";
    app.stdin.take().unwrap().write_all(steps.as_bytes()).unwrap();
    let out = app.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    for line in ["arena speed 5", "arena restart 7", "arena speed 6", "arena title test"] {
        assert!(said.contains(line), "{line} in\n{said}\n{}", String::from_utf8_lossy(&out.stdout));
    }
}

/// A Go program on Clarity.UI with its part written in Go, driven without a
/// window (CLARITY_HEADLESS), as the C++ one is. Needs CLARITY_UI (the
/// library's folder) for the build.
#[test]
#[ignore = "builds a Go program with a part written in Go and drives it; slow; run with --ignored"]
fn a_go_program_drives_its_go_part() {
    use std::io::Write;
    let dir = built(clarity_project::Backend::Go, &["go", "build", "-o", "bin/game", "."]);
    let mut app = std::process::Command::new(dir.join("bin/game"))
        .current_dir(&dir)
        .env("CLARITY_HEADLESS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let steps = "{\"do\":\"click\",\"part\":\"arena\"}\n{\"do\":\"settle\",\"ms\":50}\n{\"do\":\"read\"}\n{\"do\":\"quit\"}\n";
    app.stdin.take().unwrap().write_all(steps.as_bytes()).unwrap();
    let out = app.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    for line in ["arena speed 5", "arena restart 7", "arena speed 6", "arena title test"] {
        assert!(said.contains(line), "{line} in\n{said}\n{}", String::from_utf8_lossy(&out.stdout));
    }
}

#[test]
fn the_go_interface_types_the_declaration() {
    use clarity_project::Backend;
    // A Rust part's interface is Clarity.UI's (tests/rust_clarity_parts.rs).
    let go = parts::starter_for(&project_for(Backend::Go).files[0].parts[0]);
    let go = &go.iter().find(|(p, _)| p == "clarity.go").unwrap().1;
    for line in ["package arena", "\tSetSpeed(speed int64, cx Cx)", "\tSay(words string, loud bool, cx Cx)", "func Scored(cx Cx, points int64, by string) { cx.Tell(\"scored\", points, by) }", "\tDraw(canvas Canvas, x, y, width, height float64)", "\tClicker interface {\n\t\tClick(cx Cx)\n\t}"] {
        assert!(go.contains(line), "{line}\n{go}");
    }
}

#[test]
fn a_part_in_another_language_than_the_stacks_is_refused() {
    let mut settings = settings();
    settings.stack.backend = clarity_project::Backend::Rust;
    let refused = clarity_gen::generate(&project(), &settings).unwrap_err();
    assert!(refused.iter().any(|e| e.contains("part Arena is C or C++ (native), but the project is written in Rust")), "{refused:?}");
}

/// A part drawn by a .NET library: its package, and the starter adapter.
const NOTES: &str = "module notes

entity Note
    title: text(80), required
    body: text

part Notes
    nuget \"Newtonsoft.Json\" 13.0.1
    binds text
    wrap: bool, default true
    tells edited(words: integer)
    hears clear
    hears jump(line: integer)
    size 8 x 6

page Writing: window
    data note: Note
    Notes name notes at 1,1 span 8 rows 6 bind note.body wrap false on change: bodyChanged on edited: notesEdited

code Writing
    handler bodyChanged
        save note

    handler notesEdited(words: integer)
        if words > 100
            notes.clear()
        notes.jump(words)
        notes.wrap = true
";

fn notes_project() -> clarity_spec::Project {
    let mut p = load_keyed([("notes.clarity", NOTES)], &[], "en");
    let adapter = parts::dotnet_adapter(&p, "Notes").expect("the part is declared");
    clarity_spec::native::attach(&mut p, vec![PartFile { part: "Notes".into(), path: "Notes.cs".into(), bytes: adapter.into_bytes() }]);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn notes_settings() -> clarity_project::Settings {
    clarity_project::parse("project Notes\n    stack csharp desktop\n    datasource main sqlite \"data/notes.db\"\n").unwrap()
}

/// A .NET custom part is a control of Clarity.UI (DESIGN 7.9): its package
/// in the project file, its adapter, and the half generated from its
/// declaration, which names it for the window that places it.
#[test]
fn a_dotnet_part_is_a_clarity_ui_control_with_its_package_its_adapter_and_its_generated_half() {
    let files = clarity_gen::generate(&notes_project(), &notes_settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let half = text(&files, "parts/Notes/Notes.g.cs");
    for line in [
        "using Clarity.Ui;",
        "namespace ClarityParts;",
        "[PartKind(\"Notes\")]",
        "public partial class Notes : CustomPart",
        "private partial void ShowValue(string value);",
        "private partial void ShowWrap(bool wrap);",
        "private partial void OnClear();",
        "private partial void OnJump(long line);",
        "protected void Edited(long words) => Tell(\"edited\", words);",
        "protected void Changed(string value) => ChangeValue(value);",
        "case \"wrap\":\n                ShowWrap(As<bool>(value));",
        "case \"jump\":\n                OnJump(As<long>(values.ElementAtOrDefault(0)));",
    ] {
        assert!(half.contains(line), "{line}\n{half}");
    }
    assert_eq!(text(&files, "parts/Notes/Notes.cs"), "", "the adapter is copied as it is");
    let adapter = parts::dotnet_adapter(&notes_project(), "Notes").unwrap();
    assert!(files.iter().any(|f| f.path == "parts/Notes/Notes.cs" && f.bytes.as_deref() == Some(adapter.as_bytes())));
    assert!(text(&files, "Notes.csproj").contains("<PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />"));
    // The window places it by its name, with its binding, its handlers and its wired tells.
    let window = text(&files, "Windows/Writing.cxaml");
    assert!(window.contains("<Custom name=\"notes\""), "{window}");
    assert!(window.contains("part=\"Notes\""), "{window}");
    assert!(window.contains("&quot;tell&quot;:&quot;edited&quot;"), "{window}");
    // The view model holds its values and runs its tells.
    let model = files.iter().find(|f| f.path.ends_with("WritingViewModel.cs")).map(|f| f.text.as_str()).unwrap_or_default();
    assert!(model.contains("public void Told_Notes(string tell, System.Text.Json.JsonElement[] values)"), "{model}");
}

/// A part from a DLL is referenced beside its adapter, where the generated
/// project carries the assembly.
#[test]
fn a_dll_part_is_referenced_beside_its_adapter() {
    let text_ = NOTES.replace("nuget \"Newtonsoft.Json\" 13.0.1", "dll \"libs/Notes.Editor.dll\"");
    let mut p = load_keyed([("notes.clarity", text_.as_str())], &[], "en");
    let adapter = parts::dotnet_adapter(&p, "Notes").expect("the part is declared");
    clarity_spec::native::attach(
        &mut p,
        vec![
            PartFile { part: "Notes".into(), path: "Notes.cs".into(), bytes: adapter.into_bytes() },
            PartFile { part: "Notes".into(), path: "Notes.Editor.dll".into(), bytes: vec![0, 1, 2] },
        ],
    );
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    let files = clarity_gen::generate(&p, &notes_settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(text(&files, "Notes.csproj").contains("<Reference Include=\"Notes.Editor\" HintPath=\"parts/Notes/Notes.Editor.dll\" />"));
    assert!(files.iter().any(|f| f.path == "parts/Notes/Notes.Editor.dll"));
}

/// A part without a value, properties or hears gets methods without a
/// switch, which C# would warn of when empty.
#[test]
fn a_dotnet_part_with_nothing_to_set_has_no_empty_switch() {
    let bare = "module bare\n\npart Lamp\n    nuget \"Newtonsoft.Json\" 13.0.1\n    size 2 x 2\n";
    let p = load_keyed([("bare.clarity", bare)], &[], "en");
    let part = &p.files[0].parts[0];
    let half = clarity_gen::parts::dotnet_half(&p, part);
    assert!(!half.contains("switch"), "{half}");
    assert!(half.contains("protected override void Set(string name, object? value)\n    {\n    }"), "{half}");
}

/// Declaring a .NET part writes a starter adapter: a control of Clarity.UI
/// that draws its value in a frame and writes every method the generated
/// half asks for, whatever the value's type.
#[test]
fn a_dotnet_parts_starter_is_a_clarity_ui_control() {
    let starter = clarity_gen::parts::dotnet_adapter(&notes_project(), "Notes").expect("the part is declared");
    for line in [
        "using Clarity.Ui;",
        "public partial class Notes",
        "public override void Draw(Painter painter)",
        "private partial void ShowValue(string value)\n    {\n        shown = $\"{value}\";",
        "private partial void ShowWrap(bool wrap)",
        "private partial void OnJump(long line)",
        "Edited(...)",
    ] {
        assert!(starter.contains(line), "{line}\n{starter}");
    }
    let gauge = NOTES.replace("binds text", "binds integer");
    let p = load_keyed([("notes.clarity", gauge.as_str())], &[], "en");
    let starter = clarity_gen::parts::dotnet_adapter(&p, "Notes").unwrap();
    assert!(starter.contains("private partial void ShowValue(long value)"), "{starter}");
    // Declaring it in a project folder writes it there once, and never over the developer's.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/dotnet-starter");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let written = parts::write_part_folders(&dir, &p).unwrap();
    assert!(written.iter().any(|w| w.ends_with("parts/Notes/Notes.cs")), "{written:?}");
    std::fs::write(dir.join("parts/Notes/Notes.cs"), "// mine\n").unwrap();
    parts::write_part_folders(&dir, &p).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("parts/Notes/Notes.cs")).unwrap(), "// mine\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "builds a C# program with a .NET part; run with --ignored"]
fn a_csharp_program_with_a_dotnet_part_builds() {
    let mut p = load_keyed([("notes.clarity", NOTES)], &[], "en");
    let adapter = clarity_gen::parts::dotnet_adapter(&p, "Notes").expect("the part is declared");
    clarity_spec::native::attach(&mut p, vec![PartFile { part: "Notes".into(), path: "Notes.cs".into(), bytes: adapter.into_bytes() }]);
    let files = clarity_gen::generate(&p, &notes_settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/dotnet-part");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    clarity_gen::write_out(&dir, &files, &["bin", "obj"]).unwrap();
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    let out = std::process::Command::new("dotnet")
        .args(["build", "--nologo", "-v", "quiet", "-warnaserror", "-p:UseSharedCompilation=false", "-nodeReuse:false"])
        .current_dir(&dir)
        .env("CLARITY_RUNTIME", runtime)
        .env("MSBUILDDISABLENODEREUSE", "1")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(dir.join("bin"));
    let _ = std::fs::remove_dir_all(dir.join("obj"));
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_dotnet_part_on_another_stack_is_refused() {
    let mut settings = notes_settings();
    settings.stack.backend = clarity_project::Backend::Go;
    settings.stack.ui = clarity_project::Ui::Clarity;
    let refused = clarity_gen::generate(&notes_project(), &settings).unwrap_err();
    assert!(refused.iter().any(|e| e.contains("part Notes is .NET, but the project is written in Go")), "{refused:?}");
}

/// A custom part draws in a window of the desktop (DESIGN 6.5), which a
/// program served to a browser alone has not: generating it says so, on each
/// stack, and the same project with a desktop window generates.
#[test]
fn a_program_served_to_a_browser_alone_has_no_native_parts() {
    use clarity_project::{Backend, Target};
    // Rust names no UI; its desktop program is drawn by Clarity.UI and its web program stands alone.
    for (backend, web, desktop) in [(Backend::Cpp, "cpp web", "cpp desktop"), (Backend::Rust, "rust web", "rust desktop"), (Backend::Go, "go web", "go desktop")] {
        let project = project_for(backend);
        let web = clarity_project::parse(&format!("project Game\n    stack {web}\n    datasource main sqlite \"data/game.db\"\n")).unwrap();
        assert_eq!(web.stack.targets, [Target::Web]);
        let refused = clarity_gen::generate(&project, &web).expect_err("a part in a browser");
        assert_eq!(refused, ["part Arena draws in a window of the desktop, which a program served to a browser has not; add the desktop target, or leave the part out"]);
        let both = clarity_project::parse(&format!("project Game\n    stack {desktop}\n    datasource main sqlite \"data/game.db\"\n")).unwrap();
        assert!(clarity_gen::generate(&project, &both).is_ok(), "{desktop} with a desktop window");
    }
    // A .NET part too: Clarity.UI runs alone in the browser, for every language (DESIGN 7.9, B2).
    let web = clarity_project::parse("project Notes\n    stack csharp web\n    datasource main sqlite \"data/notes.db\"\n").unwrap();
    let refused = clarity_gen::generate(&notes_project(), &web).expect_err("a .NET part in a browser");
    assert_eq!(refused, ["part Notes draws in a window of the desktop, which a program served to a browser has not; add the desktop target, or leave the part out"]);
    assert!(clarity_gen::generate(&notes_project(), &notes_settings()).is_ok());
}

#[test]
fn a_c_or_cpp_part_built_on_its_own_finds_the_library_header() {
    let p = load_keyed([("game.clarity", GAME)], &[], "en");
    let part = &p.files[0].parts[0];
    for starter in [parts::c_starter(part), parts::cpp_starter(part)] {
        let cmake = starter.iter().find(|(path, _)| path.ends_with("CMakeLists.txt")).map(|(_, text)| text).expect("a CMakeLists.txt");
        assert!(cmake.contains("if(NOT TARGET clarity_ui AND DEFINED ENV{CLARITY_UI})"), "{cmake}");
        assert!(cmake.contains("\"$ENV{CLARITY_UI}/include\""), "{cmake}");
    }
}

// -- Reading a part's code when Clarity generates (DESIGN 5.19) --------------

/// The game with its part on `line` and the part's own files `own`, beside
/// the build file each kind of part has.
fn game_with_part(line: &str, own: &[(&str, &str)]) -> clarity_spec::Project {
    let text = GAME.replace("native \"parts/Arena\"", line);
    let mut p = load_keyed([("game.clarity", text.as_str())], &[], "en");
    let mut files: Vec<PartFile> = own.iter().map(|(path, text)| PartFile { part: "Arena".into(), path: path.to_string(), bytes: text.as_bytes().to_vec() }).collect();
    for manifest in ["Cargo.toml", "go.mod", "CMakeLists.txt"] {
        files.push(PartFile { part: "Arena".into(), path: manifest.into(), bytes: b"\n".to_vec() });
    }
    clarity_spec::native::attach(&mut p, files);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn stack(name: &str) -> clarity_project::Settings {
    clarity_project::parse(&format!("project Game\n    stack {name} desktop\n    datasource main sqlite \"data/game.db\"\n")).unwrap()
}

fn refused(project: &clarity_spec::Project, name: &str) -> String {
    clarity_gen::generate(project, &stack(name)).expect_err("the part's code is out of step").join("\n")
}

#[test]
fn parts_that_fit_their_clarity_file_generate() {
    use clarity_project::Backend;
    for (backend, name) in [(Backend::Rust, "rust"), (Backend::Go, "go"), (Backend::Cpp, "cpp"), (Backend::C, "c")] {
        let done = clarity_gen::generate(&project_for(backend), &stack(name));
        assert!(done.is_ok(), "{name}: {:?}", done.err());
    }
    assert!(clarity_gen::generate(&notes_project(), &notes_settings()).is_ok());
}

#[test]
fn a_rust_part_that_lacks_a_method_or_uses_what_it_no_longer_takes_is_refused() {
    let lib = "mod clarity;
pub use clarity::*;

#[derive(Default)]
pub struct Arena;

impl ArenaPart for Arena {
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {
        fn helper() {}
        let _ = \"fn show_value\";
    }
    // fn show_title is written in a comment only, which does not count.
    fn show_speed(&mut self, speed: i64, cx: &mut dyn Cx) {}
    fn show_running(&mut self, running: bool, cx: &mut dyn Cx) {}
    fn on_restart(&mut self, level: i64, cx: &mut dyn Cx) {}
    fn on_say(&mut self, words: String, loud: bool, cx: &mut dyn Cx) {}
    fn frame(&mut self) {}
}

fn tell(host: &ArenaHost) {}
";
    let said = refused(&game_with_part("crate \"parts/Arena\"", &[("src/lib.rs", lib)]), "rust");
    for line in [
        "custom part Arena: parts/Arena/src/lib.rs:7 impl ArenaPart lacks show_value: write fn show_value(&mut self, value: i64, cx: &mut dyn Cx) { .. }",
        "lacks show_title: write fn show_title(&mut self, title: String, cx: &mut dyn Cx)",
        "parts/Arena/src/lib.rs:17 fn frame is not a method of ArenaPart",
        "parts/Arena/src/lib.rs:20 uses ArenaHost, which Clarity no longer has",
    ] {
        assert!(said.contains(line), "{line}\n{said}");
    }
    assert!(!said.contains("helper") && !said.contains("lacks draw") && !said.contains("lacks show_speed"), "{said}");
    // A crate with no impl of the trait at all.
    let said = refused(&game_with_part("crate \"parts/Arena\"", &[("src/lib.rs", "mod clarity;\nuse clarity_runtime::parts::Host;\nuse clarity_ui::Painter;\n")]), "rust");
    assert!(said.contains("no type of its crate implements ArenaPart (src/clarity.rs): write impl ArenaPart for Arena { .. } in src/lib.rs"), "{said}");
    assert!(said.contains("parts/Arena/src/lib.rs:2 uses clarity_runtime::parts"), "{said}");
    // Clarity.UI's crate is not the part's: it draws on the canvas of its src/clarity.rs.
    assert!(said.contains("parts/Arena/src/lib.rs:3 uses clarity_ui, which a part no longer takes"), "{said}");
    let manifest = game_with_part("crate \"parts/Arena\"", &[("src/lib.rs", "mod clarity;\n"), ("Cargo.toml", "[dependencies]\nclarity-ui = \"0.16.0\"\n")]);
    assert!(refused(&manifest, "rust").contains("custom part Arena names clarity-ui in its Cargo.toml, which a part no longer takes"));
}

#[test]
fn a_go_part_that_lacks_a_method_or_takes_the_old_window_is_refused() {
    let code = TEST_PART_GO
        .replace("func (p *part) SetTitle(title string, cx Cx)   { fmt.Fprintf(os.Stderr, \"arena title %s\\n\", title) }\n", "// SetTitle is gone.\n")
        .replace("func New() Part { return &part{} }", "func New(window Window, host Host) Part { _ = NewHost(nil); return &part{} }");
    let said = refused(&game_with_part("go \"parts/arena\"", &[("arena.go", code.as_str())]), "go");
    for line in [
        "custom part Arena: its package has no method SetTitle, which Part in clarity.go asks: write func (p *part) SetTitle(title string, cx Cx) { .. }",
        "custom part Arena: parts/arena/arena.go:10 func New takes (window Window, host Host), but Clarity.UI makes the part with nothing: write func New() Part",
        "parts/arena/arena.go:10 uses NewHost",
    ] {
        assert!(said.contains(line), "{line}\n{said}");
    }
    assert!(!said.contains("SetSpeed"), "{said}");
}

#[test]
fn a_cpp_part_that_lacks_an_override_or_uses_the_old_header_is_refused() {
    let code = TEST_PART_CPP.replace("    void show_title(const std::string& title, clarity::ui::Cx&) override { std::fprintf(stderr, \"arena title %s\\n\", title.c_str()); }\n", "    // show_title(\n").replace("namespace {\n", "namespace {\nvoid step(ArenaPart* p) { Arena_frame(p); }\n");
    let said = refused(&game_with_part("native \"parts/Arena\"", &[("Arena.cpp", code.as_str())]), "cpp");
    assert!(said.contains("custom part Arena: its class does not write show_title, which ArenaPart asks: write void show_title(const std::string& title, clarity::ui::Cx& cx) override { .. }"), "{said}");
    assert!(said.contains("parts/Arena/Arena.cpp:6 uses Arena_frame, which Clarity no longer has"), "{said}");
    assert!(!said.contains("show_speed") && !said.contains("make_Arena"), "{said}");
    // No class of Clarity.UI's at all.
    let said = refused(&game_with_part("native \"parts/Arena\"", &[("Arena.cpp", "struct Arena {};\nArenaPart* make_Arena();\n")]), "cpp");
    assert!(said.contains("no class of its code derives from ArenaPart (clarity/Arena.h): write class Arena final : public ArenaPart { .. };"), "{said}");
    assert!(said.contains("its code does not define make_Arena"), "{said}");
}

#[test]
fn a_c_part_that_lacks_a_function_or_uses_the_old_header_is_refused() {
    let p = load_keyed([("game.clarity", GAME)], &[], "en");
    let starter = parts::c_starter(&p.files[0].parts[0]);
    let (path, code) = starter.iter().find(|(path, _)| path.ends_with(".c")).expect("a .c file");
    assert!(code.contains("bool Arena_key("), "{code}");
    let code = code.replace("bool Arena_key(", "bool Arena_key_typed(") + "\nvoid Arena_set_speed(ArenaPart* part, int64_t speed) {}\n";
    let said = refused(&game_with_part("native \"parts/Arena\"", &[(path.as_str(), code.as_str())]), "c");
    assert!(said.contains("custom part Arena: none of its .c files defines Arena_key, which clarity/Arena.h asks: write bool Arena_key(ArenaPart* part, const char* name, uint32_t modifiers, ClarityCustomCx* cx) { .. }"), "{said}");
    assert!(said.contains(&format!("parts/Arena/{path}:")) && said.contains("uses Arena_set_speed, which Clarity no longer has: Arena_show_speed"), "{said}");
    assert!(!said.contains("Arena_draw"), "{said}");
}

#[test]
fn a_dotnet_adapter_that_lacks_a_method_or_uses_removed_types_is_refused() {
    let mut p = load_keyed([("notes.clarity", NOTES)], &[], "en");
    let adapter = parts::dotnet_adapter(&p, "Notes").unwrap();
    assert!(adapter.contains("private partial void ShowWrap("), "{adapter}");
    let adapter = format!("using Clarity.Ui.Browser;\n{}", adapter.replace("private partial void ShowWrap(", "private void ShowWrapped(")).replacen("    string shown", "    public override void Arrange(Box box) { }\n    CustomPlace place;\n    string shown", 1);
    clarity_spec::native::attach(&mut p, vec![PartFile { part: "Notes".into(), path: "Notes.cs".into(), bytes: adapter.into_bytes() }]);
    let said = clarity_gen::generate(&p, &notes_settings()).expect_err("out of step").join("\n");
    for line in [
        "custom part Notes: its adapter Notes.cs does not write ShowWrap, which Notes.g.cs asks: write private partial void ShowWrap(bool wrap) { .. }",
        "parts/Notes/Notes.cs:1 names Clarity.Ui.Browser",
        "overrides Arrange, which CustomPart no longer has",
        "uses CustomPlace, which Clarity.UI no longer has",
    ] {
        assert!(said.contains(line), "{line}\n{said}");
    }
    assert!(!said.contains("ShowValue") && !said.contains("OnJump"), "{said}");
}

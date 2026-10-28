//! Native custom parts in the output (DESIGN 5.19 and 6.5): the header each
//! part's library implements, the glue, and the parts folder every stack
//! builds into the shared library clarity_parts.

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
    clarity_project::parse("project Game\n    stack cpp web desktop\n    datasource main sqlite \"data/game.db\"\n").unwrap()
}

fn generated() -> Vec<clarity_gen::OutFile> {
    clarity_gen::generate(&project(), &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

/// The test's own library for the part: it tells `scored` after a few
/// frames, and says on standard error what it is given and hears.
const TEST_PART: &str = r#"#include <cstdio>

#include "clarity/Arena.h"

struct ArenaPart {
    ArenaHost* host;
    int frames = 0;
};

ArenaPart* Arena_create(void*, int, int, ArenaHost* host) { return new ArenaPart{host}; }
void Arena_resize(ArenaPart*, int, int) {}
void Arena_frame(ArenaPart* part) {
    if (++part->frames == 20) Arena_scored(part->host, 7, "test");
}
void Arena_destroy(ArenaPart* part) { delete part; }
void Arena_set_enabled(ArenaPart*, bool) {}
void Arena_set_value(ArenaPart*, int64_t) {}
void Arena_set_speed(ArenaPart*, int64_t speed) { fprintf(stderr, "arena speed %lld\n", (long long)speed); }
void Arena_set_title(ArenaPart*, const char* title) { fprintf(stderr, "arena title %s\n", title); }
void Arena_set_running(ArenaPart*, bool) {}
void Arena_restart(ArenaPart*, int64_t level) { fprintf(stderr, "arena restart %lld\n", (long long)level); }
void Arena_say(ArenaPart*, const char*, bool) {}
"#;

/// The test's part in Rust: as the C++ one, it tells and says what it hears.
const TEST_PART_RUST: &str = r#"mod clarity;
pub use clarity::*;

pub struct Arena {
    host: ArenaHost,
    frames: u32,
}

impl ArenaPart for Arena {
    fn create(_window: Window, _width: u32, _height: u32, host: ArenaHost) -> Self {
        Arena { host, frames: 0 }
    }
    fn frame(&mut self) {
        self.frames += 1;
        if self.frames == 20 {
            self.host.scored(7, "test");
        }
    }
    fn set_value(&mut self, _value: i64) {}
    fn set_speed(&mut self, speed: i64) {
        eprintln!("arena speed {speed}");
    }
    fn set_title(&mut self, title: String) {
        eprintln!("arena title {title}");
    }
    fn set_running(&mut self, _running: bool) {}
    fn restart(&mut self, level: i64) {
        eprintln!("arena restart {level}");
    }
    fn say(&mut self, _words: String, _loud: bool) {}
}
"#;

/// The test's part in Go.
const TEST_PART_GO: &str = r#"package arena

import (
	"fmt"
	"os"
)

type part struct {
	host   Host
	frames int
}

func New(window Window, width, height int, host Host) Part { return &part{host: host} }

func (p *part) Resize(width, height int) {}
func (p *part) Frame() {
	p.frames++
	if p.frames == 20 {
		p.host.Scored(7, "test")
	}
}
func (p *part) SetEnabled(enabled bool)       {}
func (p *part) SetValue(value int64)          {}
func (p *part) SetSpeed(speed int64)          { fmt.Fprintf(os.Stderr, "arena speed %d\n", speed) }
func (p *part) SetTitle(title string)         { fmt.Fprintf(os.Stderr, "arena title %s\n", title) }
func (p *part) SetRunning(running bool)       {}
func (p *part) Restart(level int64)           { fmt.Fprintf(os.Stderr, "arena restart %d\n", level) }
func (p *part) Say(words string, loud bool)   {}
"#;

/// The game, its part written in the language of `backend`.
fn project_for(backend: clarity_project::Backend) -> clarity_spec::Project {
    use clarity_project::Backend;
    let (line, test_file, test_part) = match backend {
        Backend::Rust => ("crate \"parts/Arena\"", "src/lib.rs", TEST_PART_RUST),
        Backend::Go => ("go \"parts/arena\"", "arena.go", TEST_PART_GO),
        _ => ("native \"parts/Arena\"", "Arena.cpp", TEST_PART),
    };
    let text = GAME.replace("native \"parts/Arena\"", line);
    let mut p = load_keyed([("game.clarity", text.as_str())], &[], "en");
    let starter = parts::starter_for(&p.files[0].parts[0]);
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
fn the_header_types_the_declaration() {
    let p = project();
    let h = parts::header(&p.files[0].parts[0]);
    for line in [
        "ArenaPart* Arena_create(void* parent, int width, int height, ArenaHost* host);",
        "void Arena_frame(ArenaPart* part);",
        "void Arena_set_value(ArenaPart* part, int64_t value);",
        "void Arena_set_speed(ArenaPart* part, int64_t speed);",
        "void Arena_set_title(ArenaPart* part, const char* title);",
        "void Arena_set_running(ArenaPart* part, bool running);",
        "void Arena_restart(ArenaPart* part, int64_t level);",
        "void Arena_say(ArenaPart* part, const char* words, bool loud);",
        "void Arena_changed(ArenaHost* host, int64_t value);",
        "void Arena_scored(ArenaHost* host, int64_t points, const char* by);",
    ] {
        assert!(h.contains(line), "{line}\n{h}");
    }
}

#[test]
fn the_parts_folder_has_the_host_the_glue_and_each_part() {
    let p = project();
    let files = parts::out_files(&p, &["../bin"]);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in ["parts/CMakeLists.txt", "parts/glue.c", "parts/host/clarity_parts.c", "parts/host/cJSON.h", "parts/Arena/CMakeLists.txt", "parts/Arena/Arena.cpp", "parts/Arena/clarity/Arena.h"] {
        assert!(paths.contains(&path), "{path} in {paths:?}");
    }
    let cmake = &files.iter().find(|f| f.path == "parts/CMakeLists.txt").unwrap().text;
    assert!(cmake.contains("add_subdirectory(Arena)") && cmake.contains("${CMAKE_CURRENT_SOURCE_DIR}/../bin"), "{cmake}");
}

#[test]
#[ignore = "builds the parts folder with CMake; run with --ignored"]
fn the_parts_folder_builds_into_one_shared_library() {
    let p = project();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/native-parts");
    let _ = std::fs::remove_dir_all(&dir);
    for file in parts::out_files(&p, &["../bin"]) {
        let path = dir.join(&file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        match &file.bytes {
            Some(bytes) => std::fs::write(&path, bytes).unwrap(),
            None => std::fs::write(&path, &file.text).unwrap(),
        }
    }
    for step in parts::build_steps() {
        let status = std::process::Command::new(&step[0]).args(&step[1..]).current_dir(&dir).status().unwrap();
        assert!(status.success(), "{step:?}");
    }
    let library = dir.join(if cfg!(target_os = "linux") { "bin/libclarity_parts.so" } else { "bin/clarity_parts" });
    assert!(library.exists(), "{}", library.display());
    let symbols = std::process::Command::new("nm").arg("-D").arg(&library).output().unwrap();
    let symbols = String::from_utf8_lossy(&symbols.stdout);
    for symbol in ["clarity_parts_start", "clarity_parts_place", "clarity_parts_set", "clarity_parts_hear", "clarity_parts_close"] {
        assert!(symbols.contains(symbol), "{symbol}");
    }
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
    assert!(text(&files, "CMakeLists.txt").contains("add_subdirectory(parts)"));
    assert!(files.iter().any(|f| f.path == "parts/glue.c"));
}

/// Generates the game for `backend` into target/generated, builds it with
/// `build`, and gives the folder.
fn built(backend: clarity_project::Backend, build: &[&str]) -> std::path::PathBuf {
    let mut settings = settings();
    settings.stack.backend = backend;
    if backend == clarity_project::Backend::Csharp {
        settings.stack.ui = clarity_project::Ui::Avalonia;
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
    for step in clarity_gen::parts::steps_before(&dir, backend) {
        let out = std::process::Command::new(&step[0]).args(&step[1..]).current_dir(&dir).output().unwrap();
        assert!(out.status.success(), "{step:?}: {}", String::from_utf8_lossy(&out.stderr));
    }
    // C# builds on the runtime's own projects in this repository.
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    let out = std::process::Command::new(build[0]).args(&build[1..]).current_dir(&dir).env_remove("GOWORK").env("CLARITY_RUNTIME", runtime).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    dir
}

/// Runs `program` in a virtual X server (Xvfb) with the part host tracing,
/// and waits until it placed the game's part: the renderer measured it, the
/// shell gave the webview's window, and the part host made its window.
fn places_its_part(dir: &std::path::Path, program: &str, display: &str) {
    places(dir, program, display, "arena");
}

/// As `places_its_part`, for the part host's name of the part: on Avalonia
/// each part is a window of its own to the part host, named `part`.
fn places(dir: &std::path::Path, program: &str, display: &str, part: &str) {
    let mut xvfb = std::process::Command::new("Xvfb").args([display, "-screen", "0", "1280x800x24"]).spawn().expect("Xvfb is installed");
    std::thread::sleep(std::time::Duration::from_secs(1));
    let log = dir.join("parts.log");
    let mut app = std::process::Command::new(dir.join(program))
        .current_dir(dir)
        .env("DISPLAY", display)
        .env("CLARITY_PARTS_TRACE", "1")
        .stderr(std::fs::File::create(&log).unwrap())
        .spawn()
        .unwrap();
    let mut placed = String::new();
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        placed = std::fs::read_to_string(&log).unwrap_or_default();
        if placed.contains("arena title test") {
            break;
        }
    }
    let _ = app.kill();
    let _ = app.wait();
    let _ = xvfb.kill();
    let _ = xvfb.wait();
    // Placed in the webview's native window, shown: the window's number is the stack's own.
    let line = placed.lines().find(|l| l.starts_with(&format!("clarity parts: place {part} (Arena) of window "))).unwrap_or_else(|| panic!("{placed}"));
    assert!(line.contains(" in 0x") && line.ends_with(" shown"), "{line}");
    // The window's values reach the part; its tell runs the handler, whose hear and values come back.
    for said in ["arena speed 5", "arena restart 7", "arena speed 6", "arena title test"] {
        assert!(placed.contains(said), "{said} in\n{placed}");
    }
}

#[test]
#[ignore = "builds a C++ program with a native part and runs it in Xvfb; run with --ignored"]
fn a_cpp_program_places_its_native_part() {
    let dir = built(clarity_project::Backend::Cpp, &["cmake", "--workflow", "--preset", "debug"]);
    assert!(dir.join("build/debug/libclarity_parts.so").exists());
    places_its_part(&dir, "build/debug/game", ":93");
}

#[test]
#[ignore = "builds a C program with a native part and runs it in Xvfb; run with --ignored"]
fn a_c_program_places_its_native_part() {
    let dir = built(clarity_project::Backend::C, &["cmake", "--workflow", "--preset", "debug"]);
    assert!(dir.join("build/debug/libclarity_parts.so").exists());
    places_its_part(&dir, "build/debug/game", ":94");
}

#[test]
#[ignore = "builds a Tauri program with a part written in Rust and runs it in Xvfb; slow; run with --ignored"]
fn a_rust_program_places_its_rust_part() {
    let dir = built(clarity_project::Backend::Rust, &["cargo", "build", "--color", "never"]);
    places_its_part(&dir, "target/debug/game", ":95");
}

#[test]
#[ignore = "builds a Go program with a part written in Go and runs it in Xvfb; slow; run with --ignored"]
fn a_go_program_places_its_go_part() {
    let dir = built(clarity_project::Backend::Go, &["go", "build", "-o", "bin/game", "."]);
    places_its_part(&dir, "bin/game", ":96");
}

#[test]
fn the_rust_and_go_interfaces_type_the_declaration() {
    use clarity_project::Backend;
    let rust = parts::starter_for(&project_for(Backend::Rust).files[0].parts[0]);
    let rust = &rust.iter().find(|(p, _)| p == "src/clarity.rs").unwrap().1;
    for line in ["pub trait ArenaPart {", "fn set_speed(&mut self, speed: i64);", "fn say(&mut self, words: String, loud: bool);", "pub fn scored(&self, points: i64, by: &str) {"] {
        assert!(rust.contains(line), "{line}\n{rust}");
    }
    let go = parts::starter_for(&project_for(Backend::Go).files[0].parts[0]);
    let go = &go.iter().find(|(p, _)| p == "clarity.go").unwrap().1;
    for line in ["package arena", "\tSetSpeed(speed int64)", "\tSay(words string, loud bool)", "func (h Host) Scored(points int64, by string) {"] {
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
    let adapter = clarity_gen::parts::dotnet_adapter(&p, "Notes").expect("the part is declared");
    clarity_spec::native::attach(&mut p, vec![PartFile { part: "Notes".into(), path: "Notes.cs".into(), bytes: adapter.into_bytes() }]);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn notes_settings() -> clarity_project::Settings {
    clarity_project::parse("project Notes\n    stack csharp avalonia desktop\n    datasource main sqlite \"data/notes.db\"\n").unwrap()
}

#[test]
fn a_dotnet_part_has_its_package_its_adapter_and_its_generated_half() {
    let files = clarity_gen::generate(&notes_project(), &notes_settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let half = text(&files, "parts/Notes/Notes.g.cs");
    for line in [
        "public partial class Notes : DotNetPart",
        "private partial void ShowValue(string value);",
        "private partial void ShowWrap(bool wrap);",
        "private partial void OnClear();",
        "private partial void OnJump(long line);",
        "protected void Edited(long words) => Tell(\"edited\", words);",
    ] {
        assert!(half.contains(line), "{line}\n{half}");
    }
    assert!(files.iter().any(|f| f.path == "parts/Notes/Notes.cs"));
    assert!(text(&files, "Notes.csproj").contains("<PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />"));
    let view = files.iter().find(|f| f.path.ends_with("WritingView.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    assert!(view.contains("<parts:Notes ") && view.contains("xmlns:parts=\"using:ClarityParts\""), "{view}");
}

/// The adapter's starter shows the value as text, whatever its type: a
/// number has no `?.` (found by the every-part project).
#[test]
fn a_dotnet_parts_starter_shows_a_value_of_any_type() {
    let gauge = NOTES.replace("binds text", "binds integer");
    let p = load_keyed([("notes.clarity", gauge.as_str())], &[], "en");
    let starter = clarity_gen::parts::dotnet_adapter(&p, "Notes").expect("the part is declared");
    assert!(starter.contains("private partial void ShowValue(long value) => shown.Text = $\"{value}\";"), "{starter}");
    let text = clarity_gen::parts::dotnet_adapter(&notes_project(), "Notes").unwrap();
    assert!(text.contains("private partial void ShowValue(string value) => shown.Text = $\"{value}\";"), "{text}");
}

#[test]
fn a_dotnet_part_on_another_stack_is_refused() {
    let mut settings = notes_settings();
    settings.stack.backend = clarity_project::Backend::Go;
    settings.stack.ui = clarity_project::Ui::Web;
    let refused = clarity_gen::generate(&notes_project(), &settings).unwrap_err();
    assert!(refused.iter().any(|e| e.contains("part Notes is .NET, but the project is written in Go")), "{refused:?}");
}

#[test]
#[ignore = "builds a C# program with a .NET part; run with --ignored"]
fn a_csharp_program_with_a_dotnet_part_builds() {
    let files = clarity_gen::generate(&notes_project(), &notes_settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/dotnet-part");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    clarity_gen::write_out(&dir, &files, &["bin", "obj"]).unwrap();
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    let out = std::process::Command::new("dotnet").args(["build", "--nologo", "-v", "quiet", "-warnaserror"]).current_dir(&dir).env("CLARITY_RUNTIME", runtime).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

/// A custom part draws in a native window of the desktop (DESIGN 6.5), which a
/// program served to a browser alone has not: generating it says so, on each
/// stack, and the same project with a desktop window generates.
#[test]
fn a_program_served_to_a_browser_alone_has_no_native_parts() {
    use clarity_project::{Backend, Target};
    for (backend, word) in [(Backend::Cpp, "cpp"), (Backend::Rust, "rust"), (Backend::Go, "go")] {
        let project = project_for(backend);
        let web = clarity_project::parse(&format!("project Game\n    stack {word} web web\n    datasource main sqlite \"data/game.db\"\n")).unwrap();
        assert_eq!(web.stack.targets, [Target::Web]);
        let refused = clarity_gen::generate(&project, &web).expect_err("a part in a browser");
        assert_eq!(refused, ["part Arena draws in a window of the desktop, which a program served to a browser has not; add the desktop target, or leave the part out"]);
        let both = clarity_project::parse(&format!("project Game\n    stack {word} web desktop web\n    datasource main sqlite \"data/game.db\"\n")).unwrap();
        assert!(clarity_gen::generate(&project, &both).is_ok(), "{word} with a desktop window");
    }
}

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

fn project() -> clarity_spec::Project {
    let mut p = load_keyed([("game.clarity", GAME)], &[], "en");
    let starter = parts::starter(&p.files[0].parts[0]);
    let files = starter
        .into_iter()
        .map(|(path, text)| {
            let text = if path == "Arena.cpp" { TEST_PART.to_string() } else { text };
            PartFile { part: "Arena".into(), path, bytes: text.into_bytes() }
        })
        .collect();
    clarity_spec::native::attach(&mut p, files);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
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
    let files = clarity_gen::generate(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"));
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
#[ignore = "builds a Tauri program with a native part and runs it in Xvfb; slow; run with --ignored"]
fn a_rust_program_places_its_native_part() {
    let dir = built(clarity_project::Backend::Rust, &["cargo", "build", "--color", "never"]);
    assert!(dir.join("target/debug/libclarity_parts.so").exists());
    places_its_part(&dir, "target/debug/game", ":95");
}

#[test]
#[ignore = "builds a Go program with a native part and runs it in Xvfb; slow; run with --ignored"]
fn a_go_program_places_its_native_part() {
    let dir = built(clarity_project::Backend::Go, &["go", "build", "-o", "bin/game", "."]);
    assert!(dir.join("bin/libclarity_parts.so").exists());
    places_its_part(&dir, "bin/game", ":96");
}

#[test]
#[ignore = "builds a C# program with a native part and runs it in Xvfb; slow; run with --ignored"]
fn a_csharp_program_places_its_native_part() {
    let dir = built(clarity_project::Backend::Csharp, &["dotnet", "build", "--nologo", "-v", "quiet", "-warnaserror"]);
    let program = format!("bin/Debug/{}/Game", "net10.0");
    assert!(dir.join("bin/Debug/net10.0/libclarity_parts.so").exists());
    places(&dir, &program, ":92", "part");
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

#[test]
fn a_dotnet_part_on_another_stack_is_refused() {
    let mut settings = notes_settings();
    settings.stack.backend = clarity_project::Backend::Go;
    settings.stack.ui = clarity_project::Ui::Web;
    let refused = clarity_gen::generate(&notes_project(), &settings).unwrap_err();
    assert!(refused.iter().any(|e| e.contains("only the C# stack shows")), "{refused:?}");
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

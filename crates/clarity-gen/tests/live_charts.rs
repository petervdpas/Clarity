//! Live graphs in the output (DESIGN 5.26): a timer's ticks run its
//! handler, a trend and a chart of numbers draw on every renderer, and code
//! grows a window's list of values with `+` on every stack.

use clarity_project::Backend;

const WATCH: &str = "module watch

page Monitor: window
    data cpu: decimal
    data readings: list<decimal>
    data counts: list<integer>
    timer name clock every 1 on tick: sampled
    trend name cpuTrend at 1,1 span 6 rows 4 bind cpu points 60 max 100
    chart line name history at 1,5 span 6 rows 4 bind readings points 30 max 100
    chart bar name tally at 7,5 span 6 rows 4 bind counts
    button \"Stop\" name stop at 7,1 span 2 on click: stopClicked

code Monitor
    handler sampled
        cpu = cpu + 1
        readings = readings + [cpu]
        counts = counts + [1, 2]
    handler stopClicked
        clock.enabled = false
";

fn project() -> clarity_spec::Project {
    let p = clarity_spec::load_sources([("watch.clarity", WATCH)]);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn settings(backend: Backend) -> clarity_project::Settings {
    let stack = match backend {
        Backend::Csharp => "csharp avalonia desktop",
        Backend::Rust => "rust web desktop",
        Backend::Go => "go web desktop",
        Backend::C => "c web desktop",
        Backend::Cpp => "cpp web desktop",
    };
    clarity_project::parse(&format!("project Watch\n    stack {stack}\n    datasource main sqlite \"data/watch.db\"\n")).unwrap()
}

fn code(backend: Backend) -> String {
    let files = clarity_gen::generate(&project(), &settings(backend)).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn every_stack_generates_a_monitor() {
    for backend in [Backend::Rust, Backend::Go, Backend::Csharp, Backend::C, Backend::Cpp] {
        code(backend);
    }
}

#[test]
fn the_web_stacks_hand_the_renderer_the_parts() {
    let model = clarity_gen::web::model_json(&project());
    for expected in ["\"kind\":\"timer\"", "\"every\":\"1\"", "\"kind\":\"trend\"", "\"points\":60"] {
        assert!(model.contains(expected), "no {expected} in the model");
    }
}

#[test]
fn avalonia_places_a_timer_a_trend_and_a_chart_of_numbers() {
    let code = code(Backend::Csharp);
    for expected in [
        "<c:Timer Every=\"1\" Command=\"{Binding SampledCommand}\" IsEnabled=\"{Binding Clock.IsEnabled}\" />",
        "Grid.RowSpan=\"4\" Value=\"{Binding Cpu}\" Points=\"60\" Max=\"100\"",
        "Numbers=\"{Binding Readings}\" Points=\"30\" Max=\"100\"",
    ] {
        assert!(code.contains(expected), "no {expected:?} in\n{code}");
    }
}

/// Where builds go: target/generated in the repository, never the system's
/// temporary folder, which may live in memory.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(format!("live-charts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
}

fn run(dir: &std::path::Path, command: &mut std::process::Command) -> String {
    let out = command.current_dir(dir).output().unwrap_or_else(|e| panic!("{command:?}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{command:?} failed in {}:\n{said}", dir.display());
    said
}

fn written(backend: Backend, name: &str) -> std::path::PathBuf {
    let out = scratch(name);
    let files = clarity_gen::generate(&project(), &settings(backend)).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data", "target", "bin", "obj"]).unwrap();
    out
}

#[test]
#[ignore = "builds the Rust program with cargo (Tauri)"]
fn a_monitor_builds_in_rust() {
    let out = written(Backend::Rust, "rust");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    run(&out, std::process::Command::new("cargo").args(["build", "--quiet"]).env("CARGO_TARGET_DIR", &target));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the Go program"]
fn a_monitor_builds_in_go() {
    let out = written(Backend::Go, "go");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
    let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
    std::fs::write(out.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", clarity_project::slashed(&runtime))).unwrap();
    assert_eq!(run(&out, std::process::Command::new("gofmt").args(["-l", "."])), "", "gofmt would change these files");
    run(&out, std::process::Command::new("go").args(["vet", "./..."]));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C# program with dotnet"]
fn a_monitor_builds_in_csharp() {
    let out = written(Backend::Csharp, "csharp");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    run(&out, std::process::Command::new("dotnet").args(["build", "-warnaserror"]).env("CLARITY_RUNTIME", runtime));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C program with CMake"]
fn a_monitor_builds_in_c() {
    let out = written(Backend::C, "c");
    let said = run(&out, std::process::Command::new("cmake").args(["--workflow", "--preset", "debug"]));
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C++ program with CMake"]
fn a_monitor_builds_in_cpp() {
    let out = written(Backend::Cpp, "cpp");
    let said = run(&out, std::process::Command::new("cmake").args(["--workflow", "--preset", "debug"]));
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    std::fs::remove_dir_all(&out).unwrap();
}

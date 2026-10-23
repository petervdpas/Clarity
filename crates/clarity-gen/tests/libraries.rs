//! External libraries in the output (DESIGN 5.21): the adapter in the
//! project's language, the file Clarity types its functions in, and the
//! program's calls, on every stack.

use clarity_project::Backend;
use clarity_spec::keyed::load_keyed;
use clarity_spec::native::PartFile;

const SHOP: &str = "module shop

entity Order
    total: money
    words: text

library Words
    LINE
    function spell(n: integer) -> text
    function rounded(amount: money, places: integer) -> money
    function shout(words: text, loud: bool) -> text

page Orders: window
    data order: Order
    button \"Spell\" name spell at 1,1 span 2 on click: spellClicked

code Orders
    handler spellClicked
        order.words = Words.shout(Words.spell(42), true)
        order.total = Words.rounded(order.total, 2)
";

fn line(backend: Backend) -> &'static str {
    match backend {
        Backend::Csharp => "nuget \"Newtonsoft.Json\" 13.0.1",
        Backend::Rust => "crate \"itoa\" \"1\"",
        Backend::Go => "go \"github.com/shopspring/decimal\" v1.4.0",
        Backend::C | Backend::Cpp => "cmake \"Threads\"",
    }
}

fn settings(backend: Backend) -> clarity_project::Settings {
    let stack = match backend {
        Backend::Csharp => "csharp avalonia desktop",
        Backend::Rust => "rust web desktop",
        Backend::Go => "go web desktop",
        Backend::C => "c web desktop",
        Backend::Cpp => "cpp web desktop",
    };
    clarity_project::parse(&format!("project Shop\n    stack {stack}\n    datasource main sqlite \"data/shop.db\"\n")).unwrap()
}

fn project(backend: Backend) -> clarity_spec::Project {
    let text = SHOP.replace("LINE", line(backend));
    let mut p = load_keyed([("shop.clarity", text.as_str())], &[], "en");
    let library = &p.files[0].libraries[0];
    let files: Vec<PartFile> = clarity_gen::libraries::starter(&p, library, backend == Backend::Cpp, clarity_gen::go_runtime_version())
        .into_iter()
        .map(|(path, text)| PartFile { part: "Words".into(), path, bytes: text.into_bytes() })
        .collect();
    clarity_spec::native::attach(&mut p, files);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn generated(backend: Backend) -> Vec<clarity_gen::OutFile> {
    clarity_gen::generate(&project(backend), &settings(backend)).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn all_code(files: &[clarity_gen::OutFile]) -> String {
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn csharp_calls_the_adapter_and_references_the_package() {
    let files = generated(Backend::Csharp);
    let code = all_code(&files);
    assert!(code.contains("ClarityLibraries.Words.Shout(ClarityLibraries.Words.Spell(42L), true)"), "{code}");
    assert!(code.contains("public static partial string Spell(long n);"));
    assert!(code.contains("<PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />"));
    assert!(files.iter().any(|f| f.path == "libraries/Words/Words.cs"));
}

#[test]
fn rust_calls_the_crate_through_its_trait() {
    let code = all_code(&generated(Backend::Rust));
    assert!(code.contains("<words_library::Words as words_library::WordsLibrary>::spell("), "{code}");
    assert!(code.contains("words_library = { path = \"libraries/Words\" }"), "{code}");
}

#[test]
fn go_calls_the_package_and_requires_its_module() {
    let code = all_code(&generated(Backend::Go));
    assert!(code.contains("wordslib.Spell(42)") || code.contains("wordslib.Spell(int64(42))"), "{code}");
    assert!(code.contains("replace clarity.libraries/words => ./libraries/Words"), "{code}");
}

#[test]
fn c_and_cpp_call_the_functions_of_the_header() {
    let code = all_code(&generated(Backend::Cpp));
    assert!(code.contains("::Words::spell(int64_t{42})"), "{code}");
    assert!(code.contains("add_subdirectory(libraries/Words)"), "{code}");
    let code = all_code(&generated(Backend::C));
    assert!(code.contains("Words_spell(42)"), "{code}");
}

#[test]
fn a_library_in_another_language_is_refused() {
    let refused = clarity_gen::generate(&project(Backend::Csharp), &settings(Backend::Go)).unwrap_err();
    assert!(refused.iter().any(|e| e.contains("library Words is .NET, but the project is written in Go")), "{refused:?}");
}

/// Writes the program for `backend` into target/generated and builds it with `build`.
fn builds(backend: Backend, build: &[&str]) {
    let files = generated(backend);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/generated/library-{backend:?}"));
    if backend != Backend::Rust {
        let _ = std::fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&dir).unwrap();
    clarity_gen::write_out(&dir, &files, &["build", "data", "target", "bin", "obj"]).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    if backend == Backend::Rust {
        std::fs::create_dir_all(dir.join(".cargo")).unwrap();
        std::fs::write(dir.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", root.join("runtimes/rust").display().to_string())).unwrap();
    }
    if backend == Backend::Go {
        let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
        std::fs::write(dir.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", root.join("runtimes/go").display())).unwrap();
    }
    let out = std::process::Command::new(build[0])
        .args(&build[1..])
        .current_dir(&dir)
        .env_remove("GOWORK")
        .env("CLARITY_RUNTIME", root.join("runtimes/csharp"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[test]
#[ignore = "builds a C# program with a library; run with --ignored"]
fn a_csharp_program_with_a_library_builds() {
    builds(Backend::Csharp, &["dotnet", "build", "--nologo", "-v", "quiet", "-warnaserror"]);
}

#[test]
#[ignore = "builds a Go program with a library; run with --ignored"]
fn a_go_program_with_a_library_builds() {
    builds(Backend::Go, &["go", "build", "-o", "bin/shop", "."]);
}

#[test]
#[ignore = "builds a C++ program with a library; run with --ignored"]
fn a_cpp_program_with_a_library_builds() {
    builds(Backend::Cpp, &["cmake", "--workflow", "--preset", "debug"]);
}

#[test]
#[ignore = "builds a C program with a library; run with --ignored"]
fn a_c_program_with_a_library_builds() {
    builds(Backend::C, &["cmake", "--workflow", "--preset", "debug"]);
}

#[test]
#[ignore = "builds a Tauri program with a library; slow; run with --ignored"]
fn a_rust_program_with_a_library_builds() {
    builds(Backend::Rust, &["cargo", "build", "--color", "never"]);
}

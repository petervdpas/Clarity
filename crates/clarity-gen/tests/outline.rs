//! Records in app code (DESIGN 5.12) and a tree by key, on every stack: a
//! window holds a list of records built in code and shows it in a tree, a
//! tree table, a table and a list; code reads and selects their rows. The
//! build tests build the same fixture (`fixtures/outline.clarity`).

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const OUTLINE: &str = include_str!("fixtures/outline.clarity");

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("outline.clarity", OUTLINE)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("Outline", "outline");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(code: &str, backend: Backend, expected: &str) {
    assert!(code.contains(expected), "{backend:?} misses {expected:?}");
}

#[test]
fn every_stack_writes_records_and_a_tree_by_key() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let code = generated(backend);
        // The tree's key and parent travel in its markup to Clarity.UI.
        has(&code, backend, "rowKey=\"key\"");
        has(&code, backend, "parentKey=\"parent\"");
    }
}

#[test]
fn the_facts_carry_a_record_s_fields() {
    let rust = generated(Backend::Rust);
    has(&rust, Backend::Rust, "(\"Node.label\", FieldFacts { label: \"Label\"");
    // A decimal without precision and scale shows the places it has.
    has(&rust, Backend::Rust, "(\"Node.weight\", FieldFacts { label: \"Weight\", format: Some(FieldFormat { kind: FormatKind::Decimal, scale: None");
    has(&generated(Backend::Csharp), Backend::Csharp, "[\"Node.label\"] = new(\"Label\"");
}

#[test]
fn a_record_has_no_id_and_no_table() {
    let rust = generated(Backend::Rust);
    has(&rust, Backend::Rust, "pub struct Node {");
    assert!(!rust.contains("impl Record for Node"), "a record is never stored");
    assert!(!rust.contains("CREATE TABLE node"), "a record has no table");
    let go = generated(Backend::Go);
    has(&go, Backend::Go, "type Node struct {");
    has(&go, Backend::Go, "func NewNode() Node {");
    assert!(!go.contains("func (*Node) Table() string"), "a record is never stored");
    let c = generated(Backend::C);
    has(&c, Backend::C, "struct Node {");
    has(&c, Backend::C, "Node node_new(void);");
    assert!(!c.contains("node_entity"), "a record is never stored");
    let cpp = generated(Backend::Cpp);
    has(&cpp, Backend::Cpp, "struct Node {");
    let csharp = generated(Backend::Csharp);
    has(&csharp, Backend::Csharp, "class Node");
}

/// gofmt has nothing to change in the Go code: a sum among a call's values
/// (`node(p.name + "/c", ...)`) is written as gofmt writes it.
#[test]
fn gofmt_is_content_with_the_go_code() {
    let project = clarity_spec::keyed::load_keyed([("outline.clarity", OUTLINE)], &[], "en");
    let mut settings = starter("Outline", "outline");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/generated/clarity-gen-outline-gofmt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    for file in files.iter().filter(|f| f.path.ends_with(".go")) {
        let path = out.join(&file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &file.text).unwrap();
    }
    let gofmt = std::process::Command::new("gofmt").args(["-l", "."]).current_dir(&out).output().expect("gofmt runs");
    let listed = String::from_utf8_lossy(&gofmt.stdout).to_string();
    let diff = std::process::Command::new("gofmt").args(["-d", "."]).current_dir(&out).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    std::fs::remove_dir_all(&out).unwrap();
    assert_eq!(listed, "", "gofmt would change these files:\n{diff}");
}

/// A record is a value on every stack (DESIGN 5.12): C# keeps records as
/// objects, so what keeps or gets one takes a copy of its own, as Rust, Go,
/// C and C++ copy a struct. A record put into a list, assigned, read from a
/// list or a part, or given to a function is copied.
#[test]
fn csharp_copies_a_record_as_a_value() {
    let csharp = generated(Backend::Csharp);
    has(&csharp, Backend::Csharp, "public Node Copy() => new()");
    // Its list of values is copied too.
    has(&csharp, Backend::Csharp, "Tags = [.. Tags],");
    // Put into a list.
    has(&csharp, Backend::Csharp, "new List<Node> { n.Copy(), ");
    has(&csharp, Backend::Csharp, "new List<Node> { fresh.Copy() }");
    // Read from a part.
    has(&csharp, Backend::Csharp, "Picked = Selected<Node>(Nav).Copy();");
    // A record made in place, or given back by a function, is already its own.
    has(&csharp, Backend::Csharp, "var fresh = new Node();");
    has(&csharp, Backend::Csharp, "var n = Node_(db, p.Name, \"land\", p.Name, Kind.Domain);");
}

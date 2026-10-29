// The every-part project (tests/fixtures/everything.clarity): one project
// that uses every part, every field type and every kind of page, so that
// the build tests of each stack meet everything the generator writes and
// no warning hides in a part no example has. A new part or type fails here
// until the project uses it.

use clarity_spec::model::{FieldType, PageKind, Widget};
use clarity_spec::widgets;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/everything.clarity")
}

fn project() -> clarity_spec::model::Project {
    clarity_project::load_modules(&clarity_project::module_dir(&file())).expect("the every-part project loads")
}

fn kinds(widgets: &[Widget], found: &mut BTreeSet<String>) {
    for w in widgets {
        found.insert(widgets::info(w.props.kind).keyword);
        kinds(&w.children, found);
        for page in &w.pages {
            kinds(&page.children, found);
        }
    }
}

/// The name of each field type; a new type does not compile until it is named here.
fn type_name(ty: &FieldType) -> &'static str {
    match ty {
        FieldType::Text { .. } => "text",
        FieldType::Integer => "integer",
        FieldType::Decimal { .. } => "decimal",
        FieldType::Money => "money",
        FieldType::Bool => "bool",
        FieldType::Date => "date",
        FieldType::DateTime => "datetime",
        FieldType::Duration => "duration",
        FieldType::Image => "image",
        FieldType::File => "file",
        FieldType::Email => "email",
        FieldType::Url => "url",
        FieldType::Color => "color",
        FieldType::Secret => "secret",
        FieldType::Folder => "folder",
        FieldType::Language => "language",
        FieldType::Theme => "theme",
        FieldType::Id => "id",
        FieldType::Enum { .. } => "enum",
        FieldType::Ref { .. } => "ref",
        FieldType::Record { .. } => "record",
        FieldType::List { .. } => "list",
        FieldType::Values { .. } => "values",
    }
}

/// Every field type, by name. `record` is only a web API's (DESIGN 5.25),
/// which the project meets when it has one (D2).
const TYPES: [&str; 22] = [
    "text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "image", "file", "email", "url", "color", "secret", "folder", "language", "theme", "id", "enum", "ref", "list", "values",
];

fn page_kind(kind: &PageKind) -> &'static str {
    match kind {
        PageKind::Window => "window",
        PageKind::Dialog => "dialog",
        PageKind::View => "view",
    }
}

#[test]
fn the_every_part_project_uses_every_part() {
    let project = project();
    let mut used = BTreeSet::new();
    for page in project.pages() {
        kinds(&page.widgets, &mut used);
    }
    for app in project.apps() {
        for sidebar in &app.frame.sidebars {
            kinds(&sidebar.widgets, &mut used);
        }
    }
    let missing: Vec<String> = widgets::catalog().into_iter().map(|w| w.keyword).filter(|k| !used.contains(k)).collect();
    assert!(missing.is_empty(), "parts the every-part project does not use yet: {}", missing.join(", "));
}

#[test]
fn the_every_part_project_uses_every_field_type_and_kind_of_page() {
    let project = project();
    let used: BTreeSet<&str> = project.entities().flat_map(|e| &e.fields).map(|f| type_name(&f.ty)).collect();
    let missing: Vec<&str> = TYPES.iter().copied().filter(|t| !used.contains(t)).collect();
    assert!(missing.is_empty(), "field types the every-part project does not use yet: {}", missing.join(", "));
    let pages: BTreeSet<&str> = project.pages().map(|p| page_kind(&p.kind)).collect();
    for kind in ["window", "dialog", "view"] {
        assert!(pages.contains(kind), "the every-part project has no {kind}");
    }
    // And the frame every window shares: menu, toolbar, ribbon, sidebar and footer.
    let frames: Vec<_> = project.apps().map(|a| &a.frame).collect();
    assert!(frames.iter().any(|f| f.menu.is_some()), "no menu");
    assert!(frames.iter().any(|f| f.toolbar.is_some()), "no toolbar");
    assert!(frames.iter().any(|f| f.ribbon.is_some()), "no ribbon");
    assert!(frames.iter().any(|f| !f.sidebars.is_empty()), "no sidebar");
    assert!(frames.iter().any(|f| f.footer.is_some()), "no footer");
}

mod everything_project;

/// Every stack and target, as a project file writes them.
const STACKS: [&str; 10] = [
    "csharp avalonia desktop cli",
    "csharp avalonia web cli",
    "rust web desktop cli",
    "rust web web cli",
    "go web desktop cli",
    "go web web cli",
    "c web desktop cli",
    "c web web cli",
    "cpp web desktop cli",
    "cpp web web cli",
];

#[test]
fn the_every_part_project_checks_and_generates_on_every_stack() {
    for stack in STACKS {
        let (file, project, settings) = everything_project::prepare(&format!("everything-{}", stack.replace(' ', "-")), stack);
        let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| format!("{}: {}", d.span.line, d.message)).collect();
        assert!(errors.is_empty(), "{stack}:\n{}", errors.join("\n"));
        clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }
}

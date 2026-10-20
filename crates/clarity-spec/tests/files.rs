//! File access in code (X2): the files of a file store, read and written
//! as text, and a file as a record's picture. Only the folders the project
//! settings name as file stores are reached.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, load_sources};

const CATALOG: &str = "module catalog

entity Photo
    title: text(200)
    picture: image, store photos
    path: text(400)
    taken: datetime
    size: integer

action index_photos()
    effects db.write, files.read
    let found = files f in photos where f.extension == \"jpg\" or f.extension == \"png\" order by f.path
    for f in found
        let photo = new Photo
        photo.title = f.name
        photo.path = f.path
        photo.size = f.size
        photo.taken = f.modified
        photo.picture = f
        save photo

action export_titles(photos_found: list<file>)
    effects files.read, files.write
    let lines = \"\"
    for f in photos_found
        lines = lines + f.name + \"\\n\"
    let notes = files n in photos where n.name == \"notes.txt\"
    for n in notes
        lines = lines + read text of n
    write lines to photos \"exports/titles.txt\"

page Catalog: window
    data photos: list<Photo>
    button \"Index\" at 1,1 span 2 on click: index_clicked

code Catalog
    handler index_clicked
        index_photos()
        let found = files f in photos
        let n = count(found)
        message \"{n} files\"
        reload photos
";

fn load(source: &str, stores: &[&str]) -> Project {
    let mut project = load_sources([("catalog.clarity", source)]);
    clarity_spec::attach_stores(&mut project, stores.iter().map(|s| s.to_string()).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(CATALOG.contains(old), "{old:?}");
    CATALOG.replace(old, new)
}

#[test]
fn a_catalog_indexes_its_store() {
    assert_eq!(errors(&load(CATALOG, &["photos"])), Vec::<String>::new());
}

#[test]
fn parses_files_reading_writing_and_new_records() {
    let (file, diagnostics) = parse(CATALOG, "catalog.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let body = &file.functions[0].body;
    let StmtKind::Let { value, .. } = &body[0].kind else { panic!("{body:#?}") };
    let ExprKind::Files { var, store, filter, order } = &value.kind else { panic!("{value:#?}") };
    assert_eq!((var.name.as_str(), store.name.as_str()), ("f", "photos"));
    assert!(filter.is_some());
    assert_eq!(order[0].field.name, "path");
    let StmtKind::For { body: inner, .. } = &body[1].kind else { panic!() };
    let StmtKind::Let { value, .. } = &inner[0].kind else { panic!() };
    assert!(matches!(&value.kind, ExprKind::New { entity } if entity.name == "Photo"), "{value:#?}");
    let export = &file.functions[1].body;
    let StmtKind::Write { store, .. } = &export.last().unwrap().kind else { panic!("{export:#?}") };
    assert_eq!(store.name, "photos");
}

#[test]
fn code_reaches_only_the_file_stores_of_the_settings() {
    assert_eq!(
        errors(&load(CATALOG, &["scans"])).len(),
        5,
        "the store of the picture field, the three listings and the write"
    );
    let found = errors(&load(CATALOG, &["scans"]));
    assert!(found.contains(&"there is no file store 'photos'; the project settings have scans".to_string()), "{found:#?}");
}

#[test]
fn reading_and_writing_files_are_effects() {
    let source = with("    effects db.write, files.read\n", "    effects db.write\n");
    assert_eq!(errors(&load(&source, &["photos"])), ["action 'index_photos' lists files, which needs 'effects files.read'"]);
    let source = with("    effects files.read, files.write\n", "    effects files.read\n");
    assert_eq!(errors(&load(&source, &["photos"])), ["action 'export_titles' writes a file, which needs 'effects files.write'"]);
    let function = "\nfunction names() -> text\n    let found = files f in photos\n    return \"\"\n";
    let source = format!("{}{function}", CATALOG);
    assert_eq!(errors(&load(&source, &["photos"])), ["function 'names' cannot reach files; an action can, with 'effects files.read'"]);
}

#[test]
fn a_file_has_its_name_path_extension_size_and_time() {
    let bad = with("f.extension == \"jpg\"", "f.owner == \"me\"");
    assert_eq!(errors(&load(&bad, &["photos"])), ["a file has name, path, extension, size and modified, not 'owner'"]);
    let wrong = with("photo.title = f.name", "photo.title = f.size");
    assert_eq!(errors(&load(&wrong, &["photos"])), ["cannot set text to integer"]);
}

#[test]
fn a_file_becomes_a_picture_and_text_comes_from_text_files() {
    let not_picture = with("photo.picture = f", "photo.title = f");
    assert_eq!(errors(&load(&not_picture, &["photos"])), ["cannot set text to file"]);
    let read_number = with("lines = lines + read text of n", "lines = lines + read text of 5");
    assert_eq!(errors(&load(&read_number, &["photos"])), ["'read text of' needs a file, not integer"]);
}

/// `bytes`: the contents of a file as they are, read with `read bytes of f`,
/// written with `write`, measured with `.size`, and handed to engines and
/// functions; not window data, which the screen shows.
const DOCUMENTS: &str = "module catalog

function reversed(content: bytes) -> bytes
    return content

action copy_pictures()
    effects files.read, files.write
    for f in files f in photos where f.extension == \"png\"
        let content: bytes = read bytes of f
        if content.size > 0
            write reversed(content) to photos \"copies/\" + f.name
";

#[test]
fn bytes_are_read_written_and_measured() {
    let project = load(DOCUMENTS, &["photos"]);
    assert!(errors(&project).is_empty(), "{:#?}", errors(&project));
    let (file, _) = parse(DOCUMENTS, "catalog.clarity");
    let copy = &file.functions[1];
    let StmtKind::For { body, .. } = &copy.body[0].kind else { panic!("{:?}", copy.body[0]) };
    let StmtKind::Let { value, .. } = &body[0].kind else { panic!("{:?}", body[0]) };
    assert!(matches!(value.kind, ExprKind::ReadBytes { .. }), "{value:?}");

    // Bytes are no text, and a function reaches no files.
    let wrong = |source: &str| errors(&load(source, &["photos"]));
    assert!(wrong("module catalog\n\nfunction joined(b: bytes) -> text\n    return \"x\" + b\n").iter().any(|e| e.contains("bytes")));
    assert!(wrong("module catalog\n\npage Doc: window\n    data content: bytes\n").iter().any(|e| e.contains("bytes are no window data")));
}

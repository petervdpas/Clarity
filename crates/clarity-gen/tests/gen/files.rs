//! File access in code (X2) on every stack: the files of a file store,
//! filtered and sorted, read and written as text, a file as a record's
//! picture, and new records.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, FileStore, Ui, starter};
use clarity_spec::{Project, load_converted};

const CATALOG: &str = "module catalog

entity Photo
    title: text(200)
    picture: image, store photos
    cover: image
    size: integer

action indexPhotos()
    effects db.write, files.read
    let found = files f in photos where f.extension == \"jpg\" and f.size > 0 order by f.path desc, f.name
    for f in found
        let photo = new Photo
        photo.title = f.name
        photo.size = f.size
        photo.picture = f
        photo.cover = f
        save photo

action exportTitles()
    effects files.read, files.write
    let lines = \"\"
    for n in files n in photos where n.name == \"notes.txt\"
        lines = lines + read text of n
    write lines to photos \"exports/titles.txt\"

page Catalog: window
    data photos: list<Photo>
";

fn project() -> Project {
    let mut project = load_converted([("catalog.clarity", CATALOG)]);
    clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    project
}

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings = starter("Catalog", "catalog");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.filestores.push(FileStore { name: "photos".into(), location: "pictures".into() });
    generate(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn rust_reaches_the_files_of_a_store() {
    let files = files(Backend::Rust, Ui::Clarity);
    let code = text(&files, "src/modules/catalog.rs");
    has(code, "db.files(\"photos\")?.into_iter().filter(|f| ");
    has(code, ".sort_by(|a, b| b.path.cmp(&a.path).then_with(|| a.name.cmp(&b.name)));");
    has(code, "let mut photo = Photo::default();");
    has(code, "photo.picture = Some(db.picture_from(&f, Some(\"photos\"))?);");
    has(code, "photo.cover = Some(db.picture_from(&f, None)?);");
    has(code, "db.read_text(&n)?");
    has(code, "db.write_text(\"photos\", &String::from(\"exports/titles.txt\"), &lines)?;");
}

#[test]
fn csharp_reaches_the_files_of_a_store() {
    let files = files(Backend::Csharp, Ui::Clarity);
    let code = text(&files, "Modules/Catalog.cs");
    has(code, "db.Files(\"photos\").Where(f => ");
    has(code, ".OrderByDescending(f => f.Path, StringComparer.Ordinal).ThenBy(f => f.Name, StringComparer.Ordinal).ToList()");
    has(code, "var photo = new Photo();");
    has(code, "photo.Picture = db.PictureFrom(f, \"photos\");");
    has(code, "photo.Cover = db.PictureFrom(f, null);");
    has(code, "db.ReadText(n)");
    has(code, "db.WriteText(\"photos\", \"exports/titles.txt\", lines);");
}

#[test]
fn go_reaches_the_files_of_a_store() {
    let files = files(Backend::Go, Ui::Clarity);
    let code = text(&files, "catalog_module.go");
    has(code, "db.Files(\"photos\")");
    has(code, "clarity.SortFiles(");
    has(code, "photo := clarity.New(NewPhoto())");
    has(code, "db.PictureFrom(f, \"photos\")");
    has(code, "db.PictureFrom(f, \"\")");
    has(code, "db.ReadText(n)");
    has(code, "db.WriteText(\"photos\", \"exports/titles.txt\", lines)");
}

/// A folder that is no store, held by a folder field: every stack lists and
/// writes it through the runtime's `folder:` store, with the field's
/// extensions and its hidden option (conformance/files.json, folders).
const NOTES: &str = "module notes

entity Category
    name: text(80)
    folder: folder, extensions \"*.md *.txt\", hidden

action importNotes(category: Category) -> integer
    effects files.read
    let found = files f in folder category.folder where f.size > 0 order by f.path
    return count(found)

action exportNote(category: Category, name: text, body: text)
    effects files.write
    write body to folder category.folder name + \".md\"

page Notes: window
    data categories: list<Category>
";

fn folder_code(backend: Backend) -> String {
    let project = load_converted([("notes.clarity", NOTES)]);
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    let mut settings = starter("Notes", "notes");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let out = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    out.iter().filter(|f| f.text.contains("importNotes") || f.text.contains("import_notes") || f.text.contains("ImportNotes")).map(|f| f.text.clone()).collect::<Vec<_>>().join("\n")
}

#[test]
fn every_stack_lists_and_writes_a_folder_by_its_field() {
    for (backend, listed, written) in [
        (Backend::Rust, "db.files_in(&format!(\"folder:{}\", ", "db.write_text_in(&format!(\"folder:{}\", "),
        (Backend::Go, "db.FilesIn(clarity.FolderStore+", "db.WriteTextIn("),
        (Backend::Csharp, "db.FilesIn(Db.FolderStore + ", "db.WriteTextIn(Db.FolderStore + "),
        (Backend::C, "clarity_files_in(db, clarity_format(\"%s%s\", CLARITY_FOLDER_STORE, ", "clarity_write_text_in(db, clarity_format(\"%s%s\", CLARITY_FOLDER_STORE, "),
        (Backend::Cpp, "db.files_in(clarity::folder_store + ", "db.write_text_in(clarity::folder_store + "),
    ] {
        let code = folder_code(backend);
        has(&code, listed);
        has(&code, written);
        // The field's extensions and its hidden option go along.
        assert!(code.contains("\"*.md\"") && code.contains("\"*.txt\"") || code.contains("\"*.md *.txt\""), "{backend:?}: {code}");
        assert!(code.contains("true"), "{backend:?} hidden");
    }
}

/// An empty list of files, on every stack.
#[test]
fn every_stack_writes_an_empty_list_of_files() {
    let source = "module notes

action none() -> integer
    effects files.read
    let found: list<file> = []
    return count(found)

page Notes: window
    title \"Notes\"
";
    for (backend, empty) in [
        (Backend::Rust, "Vec::new()"),
        (Backend::Go, "[]clarity.FileEntry{}"),
        (Backend::Csharp, "new List<FileEntry>()"),
        (Backend::C, "(ClarityFileList){0}"),
        (Backend::Cpp, "std::vector<clarity::FileEntry>{}"),
    ] {
        let project = load_converted([("notes.clarity", source)]);
        assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
        let mut settings = starter("Notes", "notes");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let out = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        assert!(out.iter().any(|f| f.text.contains(empty)), "{backend:?}: no {empty}");
    }
}

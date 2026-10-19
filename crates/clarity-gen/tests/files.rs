//! File access in code (X2) on every stack: the files of a file store,
//! filtered and sorted, read and written as text, a file as a record's
//! picture, and new records.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, FileStore, Ui, starter};
use clarity_spec::{Project, load_sources};

const CATALOG: &str = "module catalog

entity Photo
    title: text(200)
    picture: image, store photos
    cover: image
    size: integer

action index_photos()
    effects db.write, files.read
    let found = files f in photos where f.extension == \"jpg\" and f.size > 0 order by f.path desc, f.name
    for f in found
        let photo = new Photo
        photo.title = f.name
        photo.size = f.size
        photo.picture = f
        photo.cover = f
        save photo

action export_titles()
    effects files.read, files.write
    let lines = \"\"
    for n in files n in photos where n.name == \"notes.txt\"
        lines = lines + read text of n
    write lines to photos \"exports/titles.txt\"

page Catalog: window
    data photos: list<Photo>
";

fn project() -> Project {
    let mut project = load_sources([("catalog.clarity", CATALOG)]);
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
    let files = files(Backend::Rust, Ui::Web);
    let code = text(&files, "src/modules/catalog.rs");
    has(code, "db.files(\"photos\")?.into_iter().filter(|f| ");
    has(code, ".sort_by(|a, b| b.path.cmp(&a.path).then_with(|| a.name.cmp(&b.name)));");
    has(code, "let mut photo = Photo::default();");
    has(code, "photo.picture = Some(db.picture_from(&f, Some(\"photos\"))?);");
    has(code, "photo.cover = Some(db.picture_from(&f, None)?);");
    has(code, "db.read_text(&n)?");
    has(code, "db.write_text(\"photos\", &String::from(\"exports/titles.txt\"), &lines)?;");
    assert!(files.iter().any(|f| f.path == "clarity-runtime/src/files.rs"));
}

#[test]
fn csharp_reaches_the_files_of_a_store() {
    let files = files(Backend::Csharp, Ui::Avalonia);
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
    let files = files(Backend::Go, Ui::Web);
    let code = text(&files, "catalog_module.go");
    has(code, "db.Files(\"photos\")");
    has(code, "clarity.SortFiles(");
    has(code, "photo := clarity.New(NewPhoto())");
    has(code, "db.PictureFrom(f, \"photos\")");
    has(code, "db.PictureFrom(f, \"\")");
    has(code, "db.ReadText(n)");
    has(code, "db.WriteText(\"photos\", \"exports/titles.txt\", lines)");
    assert!(files.iter().any(|f| f.path == "clarity/files.go"));
}

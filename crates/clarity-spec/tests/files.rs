//! File access in code (X2): the files of a file store, read and written
//! as text, and a file as a record's picture. Only the folders the project
//! settings name as file stores are reached.

use clarity_spec::code::{ExprKind, FilePlace, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, load_converted};

const CATALOG: &str = "module catalog

entity Photo
    title: text(200)
    picture: image, store photos
    path: text(400)
    taken: datetime
    size: integer

action indexPhotos()
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

action exportTitles(photosFound: list<file>)
    effects files.read, files.write
    let lines = \"\"
    for f in photosFound
        lines = lines + f.name + \"\\n\"
    let notes = files n in photos where n.name == \"notes.txt\"
    for n in notes
        lines = lines + read text of n
    write lines to photos \"exports/titles.txt\"

page Catalog: window
    data photos: list<Photo>
    button \"Index\" name button1 at 1,1 span 2 on click: indexClicked

code Catalog
    handler indexClicked
        indexPhotos()
        let found = files f in photos
        let n = count(found)
        message \"{n} files\"
        reload photos
";

fn load(source: &str, stores: &[&str]) -> Project {
    let mut project = load_converted([("catalog.clarity", source)]);
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
    let ExprKind::Files { var, place: FilePlace::Store(store), filter, order } = &value.kind else { panic!("{value:#?}") };
    assert_eq!((var.name.as_str(), store.name.as_str()), ("f", "photos"));
    assert!(filter.is_some());
    assert_eq!(order[0].field.name, "path");
    let StmtKind::For { body: inner, .. } = &body[1].kind else { panic!() };
    let StmtKind::Let { value, .. } = &inner[0].kind else { panic!() };
    assert!(matches!(&value.kind, ExprKind::New { entity } if entity.name == "Photo"), "{value:#?}");
    let export = &file.functions[1].body;
    let StmtKind::Write { place: FilePlace::Store(store), .. } = &export.last().unwrap().kind else { panic!("{export:#?}") };
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
    assert_eq!(errors(&load(&source, &["photos"])), ["action 'indexPhotos' lists files, which needs 'effects files.read'"]);
    let source = with("    effects files.read, files.write\n", "    effects files.read\n");
    assert_eq!(errors(&load(&source, &["photos"])), ["action 'exportTitles' writes a file, which needs 'effects files.write'"]);
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

action copyPictures()
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

/// A folder that is not a store is just a folder: code lists, reads and
/// writes its files the same way, with `folder` before the text that holds
/// it (a `folder` field, a folder setting, any text), and the effects of files.
const FOLDERS: &str = "module notes

entity Category
    name: text(80)
    folder: folder, extensions \"*.md *.txt\"

action importNotes(category: Category) -> integer
    effects files.read
    let found = files f in folder category.folder where f.extension == \"md\" order by f.path
    let n = 0
    for f in found
        let text = read text of f
        n = n + 1
    return n

action exportNote(category: Category, name: text, text: text)
    effects files.write
    write text to folder category.folder name + \".md\"
";

#[test]
fn a_folder_is_listed_read_and_written_like_a_store() {
    let (file, diagnostics) = parse(FOLDERS, "notes.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let StmtKind::Let { value, .. } = &file.functions[0].body[0].kind else { panic!() };
    assert!(matches!(&value.kind, ExprKind::Files { place: FilePlace::Folder(_), .. }), "{value:#?}");
    let StmtKind::Write { place, .. } = &file.functions[1].body[0].kind else { panic!() };
    assert!(matches!(place, FilePlace::Folder(_)), "{place:#?}");
    // No store is named, so none needs to be in the settings.
    assert_eq!(errors(&load(FOLDERS, &[])), Vec::<String>::new());
    // Written back as it was read.
    let printed = clarity_spec::code::print::expr(value);
    assert_eq!(printed, "files f in folder category.folder where f.extension == \"md\" order by f.path");
}

#[test]
fn a_folder_is_text_and_needs_the_effects_of_files() {
    let not_text = FOLDERS.replace("files f in folder category.folder", "files f in folder 3");
    assert!(errors(&load(&not_text, &[])).iter().any(|m| m.contains("the folder to list must be text, not integer")), "{:#?}", errors(&load(&not_text, &[])));
    let no_effect = FOLDERS.replace("    effects files.write\n", "");
    assert!(errors(&load(&no_effect, &[])).iter().any(|m| m.contains("writes a file, which needs 'effects files.write'")), "{:#?}", errors(&load(&no_effect, &[])));
}

/// A folder says which files it holds by their extensions; code reaches a
/// folder through such a field, so it lists only those and writes no other.
#[test]
fn a_folder_field_names_its_extensions_and_code_keeps_to_them() {
    let (file, _) = parse(FOLDERS, "notes.clarity");
    let field = file.entities[0].fields.iter().find(|f| f.name.name == "folder").unwrap();
    assert_eq!(field.modifiers.extensions, ["*.md", "*.txt"]);
    let spec = clarity_spec::dictionary::FieldSpec { name: "folder".into(), type_text: "folder".into(), modifiers: field.modifiers.clone() };
    let line = clarity_spec::dictionary::field_line(&spec);
    assert!(line.ends_with("folder: folder, extensions \"*.md *.txt\""), "{line}");
    let refused = |source: &str, says: &str| {
        let found = errors(&load(source, &[]));
        assert!(found.iter().any(|m| m.contains(says)), "{says}: {found:#?}");
    };
    refused(&FOLDERS.replace("name + \".md\"", "\"notes.csv\""), "category.folder holds *.md *.txt files, not 'notes.csv'");
    refused(&FOLDERS.replace("folder: folder, extensions \"*.md *.txt\"", "folder: text(200), extensions \"*.md\""), "only a folder field has extensions ('folder')");
    let (_, diagnostics) = parse(&FOLDERS.replace("extensions \"*.md *.txt\"", "extensions \"md\""), "notes.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("'md' is no extension of files")), "{diagnostics:#?}");
    // Hidden files only with the folder's hidden option.
    refused(&FOLDERS.replace("name + \".md\"", "\".notes.md\""), "category.folder holds no hidden files, not '.notes.md'");
    let hidden = FOLDERS.replace("extensions \"*.md *.txt\"", "extensions \"*.md *.txt\", hidden").replace("name + \".md\"", "\".notes.md\"");
    assert!(errors(&load(&hidden, &[])).is_empty(), "{:#?}", errors(&load(&hidden, &[])));
    refused(&FOLDERS.replace("files f in folder category.folder", "files f in folder category.name"), "the folder to list is a folder field, as in 'folder category.folder'");
}

/// A folder setting is a folder too: `folder settings.exportFolder`.
#[test]
fn a_folder_setting_is_listed_like_a_folder_field() {
    let source = "module notes

settings
    exportFolder: folder, extensions \"*.csv\"

page Exports: window
    title \"Exports\"

code Exports
    handler listExports
        let found = files f in folder settings.exportFolder
        let n = count(found)
";
    assert_eq!(errors(&load(source, &[])), Vec::<String>::new());
}

/// An empty list of files is written as any empty list is, with its type.
#[test]
fn an_empty_list_of_files_takes_its_type() {
    let source = "module notes

action none() -> list<file>
    effects files.read
    let found: list<file> = []
    return found
";
    assert_eq!(errors(&load(source, &[])), Vec::<String>::new());
}

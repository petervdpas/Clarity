//! Pictures in records (X2): an `image` field kept in the database, or as a
//! file in a file store, on every stack.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, FileStore, SourceKind, Ui, starter};
use clarity_spec::Project;

const MUSIC: &str = "module music

entity Album
    title: text(120)
    cover: image
    photo: image, store photos

page AlbumForm: window
    data album: Album
    image name cover at 1,1 span 3 rows 3 bind album.cover
    image name photo at 4,1 span 3 rows 3 bind album.photo
";

fn project() -> Project {
    let mut project = clarity_spec::keyed::load_keyed([("music.clarity", MUSIC)], &[], "en");
    clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    project
}

fn settings(backend: Backend, ui: Ui, kind: SourceKind) -> clarity_project::Settings {
    let mut settings = starter("Records", "records");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.datasources[0].kind = kind;
    if kind != SourceKind::Sqlite {
        settings.datasources[0].url = "env:RECORDS_DATABASE".into();
    }
    settings.filestores.push(FileStore { name: "photos".into(), location: "env:PHOTO_FOLDER".into() });
    settings
}

fn files(backend: Backend, ui: Ui, kind: SourceKind) -> Vec<OutFile> {
    generate(&project(), &settings(backend, ui, kind)).unwrap_or_else(|e| panic!("{e:#?}"))
}

/// The SQL of every migration step, as clarity-migrate applies them (docs/ledger/migrations.md).
fn steps(kind: SourceKind) -> String {
    clarity_gen::migrated_databases(&project(), &settings(Backend::Go, Ui::Clarity, kind)).unwrap_or_else(|e| panic!("{e:#?}")).iter().flat_map(|d| d.steps.iter().flatten()).map(|s| format!("{s}\n")).collect()
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

/// The picture itself in a binary column; a file store's path in a text one.
#[test]
fn a_picture_column_holds_the_picture_or_the_path_of_its_file() {
    for (kind, picture, path) in [
        (SourceKind::Sqlite, "\"cover\" BLOB", "\"photo\" TEXT"),
        (SourceKind::Postgres, "\"cover\" BYTEA", "\"photo\" VARCHAR(400)"),
        (SourceKind::Mysql, "`cover` LONGBLOB", "`photo` VARCHAR(400)"),
        (SourceKind::Mssql, "[cover] VARBINARY(MAX)", "[photo] NVARCHAR(400)"),
    ] {
        let schema = steps(kind);
        has(&schema, picture);
        has(&schema, path);
    }
}

/// A store the project settings do not have cannot be generated.
#[test]
fn a_store_must_be_in_the_project_settings() {
    let mut settings = starter("Records", "records");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Clarity;
    let errors = generate(&project(), &settings).unwrap_err();
    assert_eq!(errors, ["Album.photo is kept in file store 'photos', which the project settings do not have"]);
}

#[test]
fn rust_records_hold_their_pictures() {
    let files = files(Backend::Rust, Ui::Clarity, SourceKind::Sqlite);
    let music = text(&files, "src/modules/music.rs");
    has(music, "    pub cover: Option<Picture>,
");
    has(music, "            sql::picture(&self.cover),\n");
    has(music, "            cover: sql::get_picture(\"cover\", &values[1])?,\n");
    has(music, "    const PICTURES: &'static [PictureColumn] = &[PictureColumn { column: \"cover\", store: None, name: None }, PictureColumn { column: \"photo\", store: Some(\"photos\"), name: None }];\n");
    let main = text(&files, "src/main.rs");
    has(main, "const STORES: &[(&str, &str)] = &[(\"photos\", \"env:PHOTO_FOLDER\")];");
    has(main, "db.add_store(name, at)?;");
    has(main, "        (\"Album\", \"cover\") => db.picture::<modules::music::Album>(id, \"cover\"),\n");
    // The program gives a picture's bytes to Clarity.UI.
    has(main, "    fn field_bytes(&self, entity: &str, id: i64, field: &str) -> Answer<Option<Vec<u8>>> {");
}

#[test]
fn csharp_records_hold_their_pictures() {
    let files = files(Backend::Csharp, Ui::Clarity, SourceKind::Sqlite);
    let music = text(&files, "Modules/Music.cs");
    has(music, "    public Picture? Cover { get => _cover; set => SetProperty(ref _cover, value); }\n");
    has(music, "    public static IReadOnlyList<PictureColumn> Pictures { get; } = [new(\"cover\", null), new(\"photo\", \"photos\")];\n");
    has(music, "Sql.Picture(Cover), Sql.Picture(Photo)]");
    has(music, "        Cover = Sql.GetPicture(\"cover\", values[1]),\n");
    let program = text(&files, "Program.cs");
    has(program, "static readonly (string, string)[] Stores = [(\"photos\", \"env:PHOTO_FOLDER\")];");
    has(program, "foreach (var (name, at) in Stores)\n            db.AddStore(name, at);");
}

#[test]
fn go_records_hold_their_pictures() {
    let files = files(Backend::Go, Ui::Clarity, SourceKind::Sqlite);
    let music = text(&files, "music_module.go");
    // Aligned with the record's version field, which its migration steps give it.
    has(music, "\tCover      *clarity.Picture `json:\"cover\"`\n");
    has(music, "clarity.PictureValue(r.Cover), clarity.PictureValue(r.Photo)}");
    has(music, "\tr.Cover = read.Picture(1, \"cover\")\n");
    has(music, "func (*Album) Pictures() []clarity.PictureColumn {\n\treturn []clarity.PictureColumn{{Column: \"cover\"}, {Column: \"photo\", Store: \"photos\"}}\n}\n");
    let main = text(&files, "main.go");
    has(main, "var stores = map[string]string{\"photos\": \"env:PHOTO_FOLDER\"}");
    has(main, "\tcase \"Album.cover\":\n\t\treturn clarity.PictureOf[Album](db, id, \"cover\")\n");
}

/// C# fetches a stored picture through the window's view model.
#[test]
fn csharp_fetches_a_picture_through_the_view_model() {
    let files = files(Backend::Csharp, Ui::Clarity, SourceKind::Sqlite);
    let model = text(&files, "Windows/AlbumFormViewModel.cs");
    has(model, "    public Func<Task<byte[]?>> AlbumCoverPicture => () => Fetched(nameof(AlbumCoverPicture), () => Db.PictureOf<Album>(Album.Id, \"cover\"));\n");
}

/// A picture linked from a file before its record is saved shows from its store.
#[test]
fn a_linked_picture_shows_from_its_store_before_it_is_saved() {
    let rust = files(Backend::Rust, Ui::Clarity, SourceKind::Sqlite);
    // The program reads a file of the store for Clarity.UI.
    has(text(&rust, "src/main.rs"), "    fn store_file(&self, store: &str, path: &str) -> Answer<Vec<u8>> {");
    let csharp = files(Backend::Csharp, Ui::Clarity, SourceKind::Sqlite);
    let model = text(&csharp, "Windows/AlbumFormViewModel.cs");
    has(model, "    public Func<string, Task<byte[]?>> AlbumPhotoFile => path => Fetched(nameof(AlbumPhotoFile), () => Db.StoreFile(\"photos\", path), path);\n");
    assert!(!model.contains("AlbumCoverFile"), "a picture kept in the database has no store file");
}

const COPIES: &str = "module music

entity Album
    title: text(120)
    cover: image
    photo: image, store photos
    booklet: blob

page AlbumForm: window
    data album: Album
    data other: Album
    image name cover at 1,1 span 3 rows 3 bind album.cover
    button \"Copy\" name copyButton at 1,4 span 2 on click: copyClicked

code AlbumForm
    handler copyClicked
        album.photo = other.cover
        album.booklet = other.booklet
";

fn copies(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut project = clarity_spec::keyed::load_keyed([("music.clarity", COPIES)], &[], "en");
    clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = starter("Records", "records");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.filestores.push(FileStore { name: "photos".into(), location: "env:PHOTO_FOLDER".into() });
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn all(files: &[OutFile]) -> String {
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

/// Another record's picture or file, copied in code: the runtime reads a
/// stored one's bytes, so it needs the record, the column and both stores.
#[test]
fn code_copies_a_picture_and_a_file_from_another_record() {
    let rust = all(&copies(Backend::Rust, Ui::Clarity));
    has(&rust, "db.copied_picture::<Album>(&other.cover, other.id, \"cover\", None, Some(\"photos\"))?");
    has(&rust, "db.copied_file::<Album>(&other.booklet, other.id, \"booklet\", None, None)?");
    let go = all(&copies(Backend::Go, Ui::Clarity));
    has(&go, "clarity.CopiedPicture[Album](db, w.Other.Cover, w.Other.ID, \"cover\", \"\", \"photos\")");
    has(&go, "clarity.CopiedFile[Album](db, w.Other.Booklet, w.Other.ID, \"booklet\", \"\", \"\")");
    let csharp = all(&copies(Backend::Csharp, Ui::Clarity));
    has(&csharp, "Album.Photo = Db.CopiedPicture<Album>(Other.Cover, Other.Id, \"cover\", null, \"photos\");");
    has(&csharp, "Album.Booklet = Db.CopiedFile<Album>(Other.Booklet, Other.Id, \"booklet\", null, null);");
    let c = all(&copies(Backend::C, Ui::Clarity));
    has(&c, "clarity_copied_picture(db, &album_entity, w->other.cover, w->other.id, \"cover\", NULL, \"photos\", &");
    has(&c, "clarity_copied_file(db, &album_entity, w->other.booklet, w->other.id, \"booklet\", NULL, NULL, &");
    let cpp = all(&copies(Backend::Cpp, Ui::Clarity));
    has(&cpp, "album.photo = db.copied_picture<Album>(other.cover, other.id, \"cover\", std::nullopt, \"photos\");");
    has(&cpp, "album.booklet = db.copied_file<Album>(other.booklet, other.id, \"booklet\", std::nullopt, std::nullopt);");
}

/// A relative file store is written as the program's data folder holds it
/// (data:<program>/<folder>, conformance/data-folder.json), on every stack:
/// no path of the machine it was generated on is in the program. "hard coded
/// folders or files are a problem when the examples run on a other
/// environment" (the user, 2026-11-23).
#[test]
fn a_relative_file_store_is_written_without_the_build_machine_s_folder() {
    let dir = std::path::absolute(std::env::temp_dir().join("clarity-stores-built").join("records")).unwrap();
    for backend in [Backend::Rust, Backend::Go, Backend::Csharp, Backend::C, Backend::Cpp] {
        let mut unbuilt = settings(backend, Ui::Clarity, SourceKind::Sqlite);
        unbuilt.filestores[0].location = "data/photos".into();
        let built = clarity_project::build_settings(&dir, &unbuilt);
        let files = generate(&project(), &built).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        assert!(files.iter().any(|f| f.text.contains("\"data:records/photos\"")), "{backend:?}: no data:records/photos");
        let machine = clarity_project::slashed(&dir);
        let found: Vec<&str> = files.iter().filter(|f| f.text.contains(&machine)).map(|f| f.path.as_str()).collect();
        assert!(found.is_empty(), "{backend:?}: {machine} in {found:?}");
    }
}

/// A database or file store whose environment variable is not set never
/// crashes a Rust program at its start: it says so in Clarity's own words
/// (`variableNotSet`, translated) and stops, on the desktop and the command line.
#[test]
fn a_rust_program_says_why_its_databases_did_not_open() {
    let files = files(Backend::Rust, Ui::Clarity, SourceKind::Sqlite);
    let main = text(&files, "src/main.rs");
    assert!(!main.contains("open_databases().expect("), "{main}");
    has(main, "        Err(err) => {\n            eprintln!(\"{}\", clarity_runtime::program::said(&err).show());\n            std::process::exit(1);\n        }\n");
}

/// A Go program that cannot open its databases (an environment variable not
/// set) says why in Clarity's own words, translated, and stops.
#[test]
fn a_go_program_says_why_its_databases_did_not_open() {
    let files = files(Backend::Go, Ui::Clarity, SourceKind::Sqlite);
    has(text(&files, "main.go"), "\tdb, err := openDatabases()\n\tif err != nil {\n\t\tfmt.Fprintln(os.Stderr, clarity.Shown(err))\n\t\tos.Exit(1)\n\t}\n");
}

/// A C, C++ or C# program that cannot open its databases (an environment
/// variable not set) says why in Clarity's own words, translated, and stops.
#[test]
fn every_other_stack_says_why_its_databases_did_not_open() {
    let c = files(Backend::C, Ui::Clarity, SourceKind::Sqlite);
    has(text(&c, "src/main.c"), "    const ClarityText* said = clarity_said(clarity_error());\n    fprintf(stderr, \"%s\\n\", said ? clarity_text_show(said) : clarity_error()->message);\n");
    let cpp = files(Backend::Cpp, Ui::Clarity, SourceKind::Sqlite);
    has(text(&cpp, "src/main.cpp"), "        auto said = clarity::said(e);\n        std::cerr << (said ? said->show() : std::string(e.what())) << \"\\n\";\n");
    let csharp = files(Backend::Csharp, Ui::Clarity, SourceKind::Sqlite);
    has(text(&csharp, "Program.cs"), "Console.Error.WriteLine(Locale.Show(Unconnecting.Why(e)));");
}

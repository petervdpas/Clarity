//! The `file` field type (docs/ledger/files-pictures.md, step 3): a file on
//! the disk, kept as its path, as a `folder` field keeps a folder's. A file
//! part bound to it shows the path and chooses a file with the system's
//! dialog; a blob (`blob`) is the file kept in the record instead.

use clarity_spec::model::FieldType;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Severity, load_converted};

const MOVIES: &str = "module movies

entity Movie
    title: text
    video: file, label \"Video\"
    poster: blob

page Main: window
    data movie: Movie
    file name videoBox at 1,1 span 6 bind movie.video extensions \"*.mp4 *.webm\" new file
    file name posterBox at 1,2 span 6 bind movie.poster
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("movies.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_file_field_is_a_path_a_file_part_shows() {
    let (file, diagnostics) = parse(MOVIES, "movies.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let movie = &file.entities[0];
    assert_eq!(movie.field("video").unwrap().ty, FieldType::File);
    assert_eq!(movie.field("video").unwrap().type_text, "file");
    assert_eq!(movie.field("poster").unwrap().ty, FieldType::Blob);
    assert_eq!(widgets::widget_for(&FieldType::File), WidgetKind::File, "a form gets a file part for a file");
    assert_eq!(errors(MOVIES), Vec::<String>::new());
}

#[test]
fn the_tools_and_the_ide_offer_it() {
    assert!(clarity_spec::dictionary::SIMPLE_TYPES.contains(&"file"));
    assert!(clarity_spec::dictionary::SIMPLE_TYPES.contains(&"blob"));
}

/// Code reads and sets a file field as the text of its path, as a folder's.
#[test]
fn code_reads_and_sets_its_path() {
    let code = format!("{MOVIES}
code Main
    handler cleared
        if movie.video != \"\"
            movie.video = \"\"
");
    assert_eq!(errors(&code), Vec::<String>::new());
}

/// `new file` names a file that is not there yet: a path can, a blob cannot (it reads the file's bytes).
#[test]
fn new_file_is_for_a_file_field_only() {
    let on_blob = MOVIES.replace("bind movie.poster", "bind movie.poster new file");
    assert!(errors(&on_blob).iter().any(|m| m.contains("new file") && m.contains("blob")), "{:#?}", errors(&on_blob));
}

/// A text part shows no file: only a file part does.
#[test]
fn only_a_file_part_shows_a_file() {
    let typed = MOVIES.replace("file name videoBox at 1,1 span 6 bind movie.video extensions \"*.mp4 *.webm\" new file", "textbox name videoBox at 1,1 span 6 bind movie.video");
    assert!(errors(&typed).iter().any(|m| m.contains("file")), "{:#?}", errors(&typed));
}

// -- `extensions` on a blob or a file field: the files it takes (step 4) ----------

const TYPED: &str = "module movies

entity Movie
    video: file, extensions \"*.mp4 *.webm\"
    poster: blob, extensions \"*.png\"

page Main: window
    data movie: Movie
    file name videoBox at 1,1 span 6 bind movie.video
";

#[test]
fn a_blob_or_file_field_says_the_files_it_takes() {
    assert_eq!(errors(TYPED), Vec::<String>::new());
    let (file, _) = parse(TYPED, "movies.clarity");
    assert_eq!(file.entities[0].field("video").unwrap().modifiers.extensions, ["*.mp4", "*.webm"]);
    let hidden = TYPED.replace("extensions \"*.png\"", "extensions \"*.png\", hidden");
    assert_eq!(errors(&hidden), Vec::<String>::new(), "a blob or a file takes hidden files with hidden");
    let (file, _) = parse(&hidden, "movies.clarity");
    assert!(file.entities[0].field("poster").unwrap().modifiers.hidden);
    let on_text = TYPED.replace("    video: file, extensions", "    title: text, hidden\n    video: file, extensions");
    assert!(errors(&on_text).iter().any(|m| m.contains("only a folder, file or blob field has hidden files ('title')")), "{:#?}", errors(&on_text));
    let on_text = TYPED.replace("    video: file, extensions", "    title: text, extensions \"*.md\"\n    video: file, extensions");
    assert!(errors(&on_text).iter().any(|m| m.contains("only a folder, file or blob field has extensions ('title')")), "{:#?}", errors(&on_text));
}

/// A path written in the code is checked when the project is: one of
/// another type is refused there, before the program runs.
#[test]
fn a_written_path_of_another_type_is_refused() {
    let code = |path: &str| format!("{TYPED}
code Main
    handler set
        movie.video = \"{path}\"
");
    assert_eq!(errors(&code("clips/a.MP4")), Vec::<String>::new());
    assert_eq!(errors(&code("")), Vec::<String>::new(), "an empty path is no file");
    assert!(errors(&code("notes.txt")).iter().any(|m| m == "'video' takes *.mp4 *.webm files, not 'notes.txt'"), "{:#?}", errors(&code("notes.txt")));
    let any = TYPED.replace("video: file, extensions \"*.mp4 *.webm\"", "video: file, extensions \"*.*\"");
    let at = |path: &str| errors(&format!("{any}\ncode Main\n    handler set\n        movie.video = \"{path}\"\n"));
    assert_eq!(at("clips/README"), Vec::<String>::new(), "*.* takes any file");
    assert!(!at("keys/.env").is_empty(), "a hidden one only with hidden");
}

/// What a refusal says is one of Clarity's own words, in a project whose
/// blob or file field names its extensions.
#[test]
fn the_refusal_is_one_of_clarity_s_own_words() {
    let project = clarity_spec::keyed::load_keyed([("movies.clarity", TYPED)], &[], "en");
    assert_eq!(project.translations["en"].get("fileTypeRefused").map(String::as_str), Some("{field} takes {extensions} files, not {name}"));
    let without = clarity_spec::keyed::load_keyed([("movies.clarity", MOVIES)], &[], "en");
    assert_eq!(without.translations["en"].get("fileTypeRefused"), None, "only a project with such a field has it");
}

// -- `store` on a file part: where a web app keeps a file a person picks (step 5) ----

const UPLOADS: &str = "module movies

entity Movie
    video: file, extensions \".mp4\"

page Main: window
    data movie: Movie
    file name videoBox at 1,1 span 6 bind movie.video store uploads
";

#[test]
fn a_file_part_names_the_store_a_web_app_uploads_into() {
    let (file, diagnostics) = parse(UPLOADS, "movies.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let part = &file.pages[0].widgets[0].props;
    assert_eq!(part.store.as_deref(), Some("uploads"));
    assert_eq!(widgets::widget_line(part), "file name videoBox at 1,1 span 6 bind movie.video store uploads");
    assert!(widgets::has_property(WidgetKind::File, clarity_spec::widgets::WidgetProperty::Store));
    let mut project = load_converted([("movies.clarity", UPLOADS)]);
    clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
    let found: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(found.iter().any(|m| m == "videoBox uploads into file store 'uploads', which the project settings do not have; they have photos"), "{found:#?}");
}

/// A web app keeps what a person picks for a file field in a file store on
/// the server, so such a file part names one; a desktop app picks on its disk.
#[test]
fn in_a_web_app_a_file_part_of_a_file_field_names_its_store() {
    let without = UPLOADS.replace(" store uploads", "");
    let project = load_converted([("movies.clarity", without.as_str())]);
    let found: Vec<String> = clarity_spec::file_parts_without_store(&project).into_iter().map(|d| d.message).collect();
    assert_eq!(found, ["videoBox is a file part in a web app, which keeps a picked file in a file store on the server: name one, as in store uploads"]);
    let project = load_converted([("movies.clarity", UPLOADS)]);
    assert!(clarity_spec::file_parts_without_store(&project).is_empty());
}

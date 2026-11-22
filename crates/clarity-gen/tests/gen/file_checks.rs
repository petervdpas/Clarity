//! A blob or file field that names its extensions takes only those files:
//! every stack runs its runtime's check before code sets it
//! (docs/ledger/files-pictures.md, step 4; conformance/files.json, fieldFiles):
//! a path's text, a store file's name, another record's file's name. A field
//! with no extensions, and `none`, are not checked.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

const MOVIES: &str = "module movies

entity Movie
    title: text
    video: file, extensions \".mp4, .webm\"
    poster: blob, extensions \".png\", hidden
    notes: blob

page Main: window
    data movie: Movie
    data other: Movie
    button name setIt at 1,1 span 2 on click: setIt

code Main
    handler setIt
        movie.video = \"clips/a.mp4\"
        movie.video = other.title
        movie.poster = other.poster
        movie.notes = other.notes
        movie.poster = none
        let found = files f in scans
        for f in found
            movie.poster = f
";

fn generated(backend: Backend) -> String {
    let mut project = load_keyed([("movies.clarity", MOVIES)], &[], "en");
    clarity_spec::attach_stores(&mut project, vec!["scans".into()]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Movies", "movies");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn count(code: &str, call: &str) -> usize {
    code.matches(call).count()
}

#[test]
fn every_stack_checks_a_file_before_code_sets_a_field_that_names_extensions() {
    // Two paths, another record's poster and a store's file: four checks; notes and none: none.
    for (backend, call) in [
        (Backend::Rust, "clarity_runtime::files::check_file_type(\"video\""),
        (Backend::Go, "clarity.CheckFileType(\"video\""),
        (Backend::C, "clarity_check_file_type(\"video\""),
        (Backend::Cpp, "clarity::check_file_type(\"video\""),
        (Backend::Csharp, "Db.CheckFileType(\"video\""),
    ] {
        let code = generated(backend);
        assert_eq!(count(&code, call), 2, "{backend:?}: the two paths");
        let poster = call.replace("\"video\"", "\"poster\"");
        assert_eq!(count(&code, &poster), 2, "{backend:?}: another record's poster and a store's file");
        assert_eq!(count(&code, &call.replace("\"video\"", "\"notes\"")), 0, "{backend:?}: notes names no extensions");
    }
}

#[test]
fn the_check_carries_the_field_s_patterns_and_hidden() {
    let rust = generated(Backend::Rust);
    assert!(rust.contains("check_file_type(\"video\", &[\"*.mp4\".to_string(), \"*.webm\".to_string()], false,"), "{rust}");
    assert!(rust.contains("check_file_type(\"poster\", &[\"*.png\".to_string()], true,"));
}

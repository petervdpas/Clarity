//! The options of the folder and file parts for the system's file dialogs
//! (docs/ledger/files-pictures.md, "Folder and file parts with options"),
//! the part's settings in the designer: `extensions` (a list like
//! `.png, .jpg, .gif`, or `*.*` for all), `hidden` (a checkbox: hidden files
//! and folders, whose name starts with a dot, count too), `start folder`
//! (where the dialog opens while nothing is chosen) and, on a file part,
//! `new file` (a file that is not there yet may be named). Each is off by
//! default: its line, its window file's `extensions`, `hidden`,
//! `startFolder` and `newFile`, and the designer's properties.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

entity Movie
    poster: file
    folder: folder

page Main: window
    data movie: Movie
    folder name movies at 1,1 span 6 bind movie.folder extensions \"*.*\" hidden start folder \"videos\"
    file name posterBox at 1,2 span 6 bind movie.poster extensions \".png, .jpg, .gif\" start folder \"pictures\" new file
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_options_read_and_write_back() {
    let (file, diagnostics) = parse(WINDOW, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    let folder = &parts[0].props;
    assert_eq!(folder.extensions.as_deref(), Some("*.*"));
    assert!(folder.hidden && !folder.new_file);
    assert_eq!(folder.start_folder.as_deref(), Some("videos"));
    let poster = &parts[1].props;
    assert_eq!(poster.extensions.as_deref(), Some(".png, .jpg, .gif"));
    assert!(!poster.hidden && poster.new_file);
    assert_eq!(poster.start_folder.as_deref(), Some("pictures"));
    assert_eq!(widgets::widget_line(folder), "folder name movies at 1,1 span 6 bind movie.folder extensions \"*.*\" hidden start folder \"videos\"");
    assert_eq!(widgets::widget_line(poster), "file name posterBox at 1,2 span 6 bind movie.poster extensions \".png, .jpg, .gif\" start folder \"pictures\" new file");
    // Through the window file and back, as a project holds it.
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

/// The pattern form reads too, and is written back as the list.
#[test]
fn the_pattern_form_reads_as_the_list() {
    let source = WINDOW.replace("extensions \".png, .jpg, .gif\"", "extensions \"*.png *.jpg\"");
    let (file, diagnostics) = parse(&source, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(file.pages[0].widgets[1].props.extensions.as_deref(), Some(".png, .jpg"));
}

#[test]
fn the_window_file_names_them_as_the_properties_do() {
    let (file, _) = parse(WINDOW, "m.clarity");
    let mut out = String::new();
    for part in &file.pages[0].widgets {
        clarity_spec::cxaml::part(part, 0, &mut out, &clarity_spec::cxaml::Marks::PLAIN);
    }
    for attribute in [r#"extensions="*.*""#, r#"hidden="true""#, r#"startFolder="videos""#, r#"extensions=".png, .jpg, .gif""#, r#"startFolder="pictures""#, r#"newFile="true""#] {
        assert!(out.contains(attribute), "{attribute}: {out}");
    }
}

#[test]
fn each_is_off_by_default() {
    let line = "file name posterBox at 1,2 span 6 bind movie.poster";
    let source = WINDOW.replace("file name posterBox at 1,2 span 6 bind movie.poster extensions \".png, .jpg, .gif\" start folder \"pictures\" new file", line);
    let (file, _) = parse(&source, "m.clarity");
    let props = &file.pages[0].widgets[1].props;
    assert!(props.extensions.is_none() && !props.hidden && props.start_folder.is_none() && !props.new_file);
    assert_eq!(widgets::widget_line(props), line);
}

#[test]
fn which_parts_take_them() {
    for kind in [WidgetKind::File, WidgetKind::Folder] {
        for property in [WidgetProperty::Extensions, WidgetProperty::Hidden, WidgetProperty::StartFolder] {
            assert!(widgets::has_property(kind, property), "{kind:?} {property:?}");
        }
    }
    assert!(widgets::has_property(WidgetKind::File, WidgetProperty::NewFile));
    assert!(!widgets::has_property(WidgetKind::Folder, WidgetProperty::NewFile), "a folder part chooses a folder that is there");
    let on_a_textbox = WINDOW.replace("file name posterBox at 1,2 span 6 bind movie.poster extensions \".png, .jpg, .gif\" start folder \"pictures\" new file", "textbox name posterBox at 1,2 span 6 hidden");
    assert!(errors(&on_a_textbox).iter().any(|m| m.contains("'hidden' does not apply to a textbox")), "{:#?}", errors(&on_a_textbox));
}

#[test]
fn extensions_are_a_list_of_dotted_ones_or_all() {
    for (written, problem) in [("pdf", "'pdf' is no extension of files"), ("*.*, .pdf", "*.*")] {
        let source = WINDOW.replace("extensions \".png, .jpg, .gif\"", &format!("extensions \"{written}\""));
        assert!(errors(&source).iter().any(|m| m.contains(problem)), "{written}: {:#?}", errors(&source));
    }
}

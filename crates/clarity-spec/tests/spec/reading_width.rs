//! `reading width`: a markdown part, or a memo's Markdown preview, keeps its
//! text in a column of a reading width from the left instead of the whole
//! width, as a book page reads. An option of the part, off by default: its
//! line, its window file's `readingWidth="true"` and the designer's property.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

entity Note
    text: text

page Main: window
    data note: Note
    markdown name page at 1,1 span 6 rows 6 bind note.text reading width
    memo name editor at 7,1 span 6 rows 6 bind note.text markdown reading width
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn reading_width_reads_and_writes_back_on_markdown_parts_and_memos() {
    let (file, diagnostics) = parse(WINDOW, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert!(parts[0].props.reading_width && parts[1].props.reading_width);
    for part in parts {
        let line = widgets::widget_line(&part.props);
        assert!(line.ends_with("reading width"), "{line}");
    }
    for kind in [WidgetKind::Markdown, WidgetKind::Memo] {
        assert!(widgets::has_property(kind, WidgetProperty::ReadingWidth), "{kind:?}");
    }
    assert!(!widgets::has_property(WidgetKind::Textbox, WidgetProperty::ReadingWidth), "a text box shows no Markdown");
    // Through the window file and back, as a project holds it.
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn without_reading_width_the_text_takes_the_whole_width() {
    let line = "markdown name page at 1,1 span 6 rows 6 bind note.text";
    let source = WINDOW.replace("markdown name page at 1,1 span 6 rows 6 bind note.text reading width", line);
    let (file, _) = parse(&source, "m.clarity");
    assert!(!file.pages[0].widgets[0].props.reading_width);
    assert_eq!(widgets::widget_line(&file.pages[0].widgets[0].props), line);
}

#[test]
fn a_part_without_markdown_refuses_reading_width() {
    let on_a_textbox = WINDOW.replace("memo name editor at 7,1 span 6 rows 6 bind note.text markdown reading width", "textbox name editor at 7,1 span 6 bind note.text reading width");
    assert!(errors(&on_a_textbox).iter().any(|m| m.contains("'reading' does not apply to a textbox")), "{:#?}", errors(&on_a_textbox));
}

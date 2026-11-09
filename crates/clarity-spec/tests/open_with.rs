//! A route opens its window with data of the window it is in:
//! `on click -> ChapterForm with book`, from a button, a menu or a toolbar.

use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module writer

entity Book
    title: text

entity Note
    title: text

app Writer
    main Desk
    toolbar name mainToolbar
        button @chaptersCaption name chaptersButton on click -> Chapters with book

page Desk: window
    data book: Book
    button @chaptersCaption name openChapters at 1,1 span 2 on click -> Chapters with book in new window

page Chapters: window
    data book: Book
";

fn errors(text: &str) -> Vec<String> {
    load_sources([("writer.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_route_opens_its_window_with_data_of_the_same_name() {
    assert!(errors(SHOP).is_empty(), "{:#?}", errors(SHOP));
    let project = load_sources([("writer.clarity", SHOP)]);
    let desk = project.files[0].pages.iter().find(|p| p.name.name == "Desk").unwrap();
    assert_eq!(desk.widgets[0].props.open_with, vec!["book".to_string()]);
    assert!(clarity_spec::widgets::widget_line(&desk.widgets[0].props).contains("on click -> Chapters with book in new window"));
    let app = &project.files[0].apps[0];
    assert_eq!(app.frame.toolbar.as_ref().unwrap().items[0].props.open_with, vec!["book".to_string()]);
}

#[test]
fn the_opened_window_and_the_button_s_window_hold_the_data() {
    let e = errors(&SHOP.replace("Chapters with book in new window", "Chapters with books"));
    assert!(e.iter().any(|m| m == "Chapters has no data 'books' to open it with"), "{e:?}");
    let e = errors(&SHOP.replace("    data book: Book\n    button", "    data book: Note\n    button"));
    assert!(e.iter().any(|m| m == "'book' here is Note, and Chapters's 'book' is Book"), "{e:?}");
    let e = errors(&SHOP.replace("    data book: Book\n    button", "    data other: Book\n    button"));
    assert!(e.iter().any(|m| m == "this window has no data 'book' to open Chapters with"), "{e:?}");
}

/// `reorder by number from 0`: the first row's number, kept as written.
#[test]
fn reorder_counts_from_the_number_written() {
    let source = "module writer\n\nentity Chapter\n    title: text\n    number: integer\n\npage Desk: window\n    data chapters: list<Chapter>\n    list name chapterList at 1,1 span 4 rows 10 bind chapters show title reorder by number from 0 on reorder: moved\n\ncode Desk\n    handler moved\n        for c in chapters\n            save c\n";
    assert!(errors(source).is_empty(), "{:#?}", errors(source));
    let project = load_sources([("writer.clarity", source)]);
    let list = &project.files[0].pages[0].widgets[0];
    assert_eq!(list.props.reorder_from, Some(0));
    assert!(clarity_spec::widgets::widget_line(&list.props).contains("reorder by number from 0"));
}

const DESK: &str = "module writer

entity Chapter
    title: text

page Desk: window
    accordion name book at 1,1 span 12 rows 10 horizontal
        section @chaptersCaption
            content name chaptersContent at 1,1 span 12 rows 10
                view Chapters

page Chapters: view
    data chapters: list<Chapter>
    list name chapterTable at 1,1 span 4 rows 10 bind chapters show title
    memo name text at 5,1 span 8 rows 10
    slideout @chapterCaption name chapterProps side right beside chapterTable width 360 pinnable
        textbox name titleBox at 1,1 span 6
";

/// `beside chapterTable`: a slide-out in a view, against a part of it.
#[test]
fn a_slide_out_lies_beside_a_part_of_its_container() {
    assert!(errors(DESK).is_empty(), "{:#?}", errors(DESK));
    let project = load_sources([("writer.clarity", DESK)]);
    let view = project.files[0].pages.iter().find(|p| p.name.name == "Chapters").unwrap();
    let slide = view.widgets.iter().find(|w| w.props.name.as_deref() == Some("chapterProps")).unwrap();
    assert!(clarity_spec::widgets::widget_line(&slide.props).contains("beside chapterTable"));
    let e = errors(&DESK.replace("beside chapterTable", "beside nothing"));
    assert!(e.iter().any(|m| m.contains("there is no part 'nothing' beside this slide-out")), "{e:?}");
    let e = errors(&DESK.replace(" beside chapterTable", ""));
    assert!(e.iter().any(|m| m.contains("has a slide-out of its own")), "{e:?}");
}

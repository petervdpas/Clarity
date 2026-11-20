//! `pdf of text stylesheet css theme true` as every stack calls it
//! (docs/ledger/reports-pdf.md, step 4): texts in, a PDF or a problem out.

use clarity_pdf::Problem;

#[test]
fn a_text_alone_gives_a_pdf() {
    let file = clarity_pdf::made("# Invoice\n\nThank you.", "", "").unwrap();
    assert!(file.starts_with(b"%PDF-"), "{:?}", &file[..8]);
}

#[test]
fn the_same_texts_give_the_same_file() {
    let made = || clarity_pdf::made("# Invoice", "h1 { color: #aa0000 }", "").unwrap();
    assert_eq!(made(), made());
}

#[test]
fn a_stylesheet_with_errors_is_refused_with_each_line() {
    let Err(Problem::Stylesheet(errors)) = clarity_pdf::made("text", "h1 { colr: red }\nh9 { color: red }", "") else { panic!("not refused") };
    assert_eq!(errors.len(), 2, "{errors:?}");
    let said = Problem::Stylesheet(errors).to_string();
    assert!(said.contains("line 1") && said.contains("colr"), "{said}");
    assert!(said.contains("line 2") && said.contains("h9"), "{said}");
}

#[test]
fn the_theme_gives_its_colors_to_the_stylesheet() {
    let theme = r##"{ "colors": { "accent": "#123456" } }"##;
    let laid = clarity_pdf::layout("# Title", &clarity_pdf::Stylesheet::read("h1 { color: var(--accent) }", Some(&clarity_pdf::Theme::read(theme).unwrap())).unwrap()).unwrap();
    assert_eq!(laid.runs(0)[0].color, "#123456");
    assert!(clarity_pdf::made("# Title", "h1 { color: var(--accent) }", theme).is_ok());
    // Without the theme, a variable with no fallback is an error.
    assert!(matches!(clarity_pdf::made("# Title", "h1 { color: var(--accent) }", ""), Err(Problem::Stylesheet(_))));
}

#[test]
fn a_theme_that_is_no_json_is_refused() {
    assert!(matches!(clarity_pdf::made("text", "", "{ not json"), Err(Problem::Theme(_))));
}

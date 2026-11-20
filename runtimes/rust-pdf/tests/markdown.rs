//! Markdown laid out into pages and written as a PDF (docs/ledger/reports-pdf.md, step 1).

use clarity_pdf::{Laid, Stylesheet};

fn laid(markdown: &str, sheet: &Stylesheet) -> Laid {
    clarity_pdf::layout(markdown, sheet).unwrap_or_else(|e| panic!("{markdown:?} did not lay out: {e}"))
}

fn text_of(laid: &Laid, page: usize) -> String {
    laid.runs(page).iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
}

#[test]
fn a_heading_and_a_paragraph_are_one_page_with_their_words() {
    let laid = laid("# Invoice\n\nThank you for your order.\n", &Stylesheet::default());
    assert_eq!(laid.pages(), 1);
    let text = text_of(&laid, 0);
    assert!(text.contains("Invoice"), "{text}");
    assert!(text.contains("Thank you for your order."), "{text}");
}

#[test]
fn a_heading_is_larger_by_its_level_and_larger_than_text() {
    let laid = laid("# One\n\n## Two\n\nPlain.\n", &Stylesheet::default());
    let size = |word: &str| laid.runs(0).iter().find(|w| w.text.contains(word)).unwrap_or_else(|| panic!("no {word}")).size;
    assert!(size("One") > size("Two"), "{} {}", size("One"), size("Two"));
    assert!(size("Two") > size("Plain"), "{} {}", size("Two"), size("Plain"));
}

#[test]
fn bold_and_italic_spans_keep_their_look() {
    let laid = laid("Some **strong** and *leaning* words.\n", &Stylesheet::default());
    let words = laid.runs(0);
    let find = |word: &str| words.iter().find(|w| w.text.contains(word)).unwrap_or_else(|| panic!("no {word} in {words:?}"));
    assert!(find("strong").bold && !find("strong").italic);
    assert!(find("leaning").italic && !find("leaning").bold);
    assert!(!find("Some").bold && !find("Some").italic);
}

#[test]
fn the_text_is_written_as_it_is_whatever_typst_would_read_in_it() {
    let tricky = r#"Costs #5 at $3 * 2 = <six> @home \ "quoted" [x] _y_ `z` // not a comment ~ = - +"#;
    let laid = laid(tricky, &Stylesheet::default());
    let text: String = text_of(&laid, 0).split_whitespace().collect::<Vec<_>>().join(" ");
    // The reader takes _y_ as italic and `z` as code; the rest is the text itself.
    for piece in ["#5", "$3", "* 2 =", "<six>", "@home", "\\", "\"quoted\"", "[x]", "y", "z", "// not a comment", "~ = - +"] {
        assert!(text.contains(piece), "{piece:?} missing from {text:?}");
    }
}

#[test]
fn a_long_text_runs_over_pages() {
    let long: String = (1..=120).map(|i| format!("Paragraph {i} of a long letter.

")).collect();
    let laid = laid(&long, &Stylesheet::default());
    assert!(laid.pages() > 1, "{} pages", laid.pages());
    assert!(text_of(&laid, 1).contains("Paragraph"), "the second page holds text");
}

#[test]
fn the_pdf_is_a_pdf_and_the_same_every_time() {
    let setup = Stylesheet::read("@page { @bottom-center { content: counter(page) } }", None).unwrap();
    let first = clarity_pdf::pdf("# Same\n\nEvery **time**.", &setup).unwrap();
    assert!(first.starts_with(b"%PDF-"), "{:?}", &first[..8]);
    let second = clarity_pdf::pdf("# Same\n\nEvery **time**.", &setup).unwrap();
    assert!(first == second, "two PDFs of the same text differ");
}

#[test]
fn an_empty_text_is_one_empty_page() {
    let laid = laid("", &Stylesheet::default());
    assert_eq!(laid.pages(), 1);
    assert!(laid.runs(0).is_empty());
}

//! The stylesheet: a subset of CSS over Clarity's own sheet, optionally from
//! the app's theme (docs/ledger/reports-pdf.md, step 2).

use clarity_pdf::{CssErrorKind, Laid, Run, Stylesheet, Theme};

fn sheet(css: &str) -> Stylesheet {
    Stylesheet::read(css, None).unwrap_or_else(|e| panic!("{css:?}: {e:?}"))
}

fn laid(markdown: &str, sheet: &Stylesheet) -> Laid {
    clarity_pdf::layout(markdown, sheet).unwrap_or_else(|e| panic!("{markdown:?} did not lay out: {e}"))
}

fn run<'a>(runs: &'a [Run], word: &str) -> &'a Run {
    runs.iter().find(|r| r.text.contains(word)).unwrap_or_else(|| panic!("no {word:?} in {runs:#?}"))
}

fn errors(css: &str) -> Vec<(usize, CssErrorKind, String)> {
    match Stylesheet::read(css, None) {
        Ok(_) => panic!("{css:?} was read without errors"),
        Err(errors) => errors.into_iter().map(|e| (e.line, e.kind, e.subject)).collect(),
    }
}

const THEME: &str = r##"{
  "dark": false,
  "colors": { "accent": "#aa0000", "text": "#112233", "strong_border": "#00aa00", "code_background": "#eeeeee", "muted": "#777777" },
  "fonts": { "text": "Georgia, serif", "heading": "system-ui, sans-serif", "mono": "ui-monospace, monospace" },
  "sizes": { "text": 14, "heading": 18, "small": 12 }
}"##;

#[test]
fn clarity_s_own_sheet_sets_inter_in_the_light_theme_s_text_color() {
    let laid = laid("Plain words.", &Stylesheet::default());
    let words = run(&laid.runs(0), "Plain").clone();
    assert_eq!(words.font, "Inter");
    assert_eq!(words.color, "#1d2330");
    assert!((words.size - 10.5).abs() < 0.01, "{}", words.size);
}

#[test]
fn the_theme_gives_its_fonts_and_colors_to_clarity_s_sheet() {
    let theme = Theme::read(THEME).unwrap();
    let sheet = Stylesheet::read("", Some(&theme)).unwrap();
    let laid = laid("# Title\n\nSome [link](https://example.org) and `code`.", &sheet);
    let runs = laid.runs(0);
    assert_eq!(run(&runs, "Some").font, "Libertinus Serif", "the theme's text font, Georgia, serif");
    assert_eq!(run(&runs, "Some").color, "#112233");
    assert_eq!(run(&runs, "Title").font, "Inter");
    assert_eq!(run(&runs, "link").color, "#aa0000");
    assert_eq!(run(&runs, "code").font, "DejaVu Sans Mono");
}

#[test]
fn a_stylesheet_goes_over_clarity_s_and_text_inherits_from_body() {
    let sheet = sheet("body { font-family: Georgia, 'Libertinus Serif', serif; color: #336699; }\nh1 { font-size: 20pt; color: rgb(10, 20, 30); }");
    let laid = laid("# Head\n\nText.", &sheet);
    let runs = laid.runs(0);
    assert_eq!(run(&runs, "Text").font, "Libertinus Serif");
    assert_eq!(run(&runs, "Text").color, "#336699");
    assert_eq!(run(&runs, "Head").color, "#0a141e");
    assert!((run(&runs, "Head").size - 20.0).abs() < 0.01);
    assert_eq!(run(&runs, "Head").font, "Inter", "Clarity's sheet gives headings the heading font");
    assert!(run(&runs, "Head").bold);
}

#[test]
fn a_later_rule_wins_and_a_group_selector_sets_each() {
    let sheet = sheet("/* two rules */ p { color: #111111 } h2, p { color: #222222; }");
    let laid = laid("## Two\n\nPara.", &sheet);
    assert_eq!(run(&laid.runs(0), "Para").color, "#222222");
    assert_eq!(run(&laid.runs(0), "Two").color, "#222222");
}

#[test]
fn sizes_take_pt_px_mm_and_em() {
    for (css, pt) in [("12pt", 12.0), ("16px", 12.0), ("5mm", 14.173), ("2em", 21.0)] {
        let laid = laid("Sized.", &sheet(&format!("p {{ font-size: {css} }}")));
        let size = run(&laid.runs(0), "Sized").size;
        assert!((size - pt).abs() < 0.01, "{css} gave {size}");
    }
}

#[test]
fn bold_and_italic_are_inter_s_own_faces() {
    let laid = laid("A **bold** and *leaning* and ***both*** word.", &Stylesheet::default());
    let runs = laid.runs(0);
    for (word, bold, italic) in [("bold", true, false), ("leaning", false, true), ("both", true, true), ("word", false, false)] {
        let r = run(&runs, word);
        assert_eq!((r.font.as_str(), r.bold, r.italic), ("Inter", bold, italic), "{word}");
    }
}

#[test]
fn the_page_takes_its_size_and_margin_from_at_page() {
    let cases = [
        ("@page { size: Letter; margin: 10mm }", (215.9, 279.4), 10.0),
        ("@page { size: A4 landscape; margin: 20mm 30mm }", (297.0, 210.0), 30.0),
        ("@page { size: 100mm 150mm; margin: 5mm 6mm 7mm 8mm }", (100.0, 150.0), 8.0),
        ("@page { size: A5 }", (148.0, 210.0), 25.0),
    ];
    for (css, (w, h), left) in cases {
        let laid = laid("Start.", &sheet(css));
        let (pw, ph) = laid.page_size(0);
        assert!((pw - w).abs() < 0.1 && (ph - h).abs() < 0.1, "{css}: {pw} x {ph}");
        let x = laid.runs(0)[0].x;
        assert!((x - left).abs() < 0.5, "{css}: the text starts at {x}");
    }
}

#[test]
fn page_numbers_come_from_a_margin_box() {
    let long: String = (1..=120).map(|i| format!("Paragraph {i} of a long letter.\n\n")).collect();
    let plain = laid(&long, &Stylesheet::default());
    assert!(plain.pages() > 2);
    assert!(plain.runs(0).last().unwrap().text.contains("letter"), "no page number unless the sheet asks");

    let sheet = sheet("@page { @bottom-center { content: \"Page \" counter(page) \" of \" counter(pages); color: #888888 } }");
    let numbered = laid(&long, &sheet);
    let pages = numbered.pages();
    for page in 0..pages {
        let runs = numbered.runs(page);
        let text: String = runs.iter().rev().take(4).rev().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(text.ends_with(&format!("Page {} of {pages}", page + 1)) || text.replace(' ', "").ends_with(&format!("Page{}of{pages}", page + 1)), "page {page}: {text:?}");
        assert_eq!(runs.last().unwrap().color, "#888888");
    }
}

#[test]
fn a_header_box_sits_above_the_text() {
    let laid = laid("Body text.", &sheet("@page { @top-right { content: \"Invoice\" } }"));
    let runs = laid.runs(0);
    assert!(run(&runs, "Invoice").y < run(&runs, "Body").y);
    assert!(run(&runs, "Invoice").x > run(&runs, "Body").x + 50.0, "on the right");
}

#[test]
fn a_quote_has_its_bar_at_its_left() {
    let laid = laid("> Wise words.", &Stylesheet::default());
    let words = run(&laid.runs(0), "Wise").clone();
    let bar = laid.areas(0).into_iter().find(|a| a.color == "#b7bec9").expect("a bar in the strong border color");
    assert!(bar.x < words.x && bar.width < 3.0, "{bar:?} {words:?}");
    assert!(bar.y <= words.y && bar.y + bar.height >= words.y, "the bar runs along the text: {bar:?} {words:?}");
}

#[test]
fn a_code_block_lies_on_its_background_in_the_mono_font() {
    let laid = laid("```\nlet total = 120\n  indented\n```", &Stylesheet::default());
    let runs = laid.runs(0);
    let code = run(&runs, "total");
    assert_eq!(code.font, "DejaVu Sans Mono");
    let back = laid.areas(0).into_iter().find(|a| a.color == "#f4f6f9").expect("the code background");
    assert!(back.x < code.x && back.y < code.y && back.y + back.height > code.y, "{back:?} {code:?}");
    assert!(back.width > 150.0, "as wide as the column: {back:?}");
    let indented = run(&runs, "indented");
    assert_eq!(indented.text.chars().take_while(|c| c.is_whitespace()).count(), 2, "its spaces are kept: {indented:?}");
}

#[test]
fn a_rule_is_a_centred_line_a_quarter_of_the_column() {
    let laid = laid("Above.\n\n---\n\nBelow.", &Stylesheet::default());
    let line = laid.areas(0).into_iter().find(|a| a.color == "#5d6677").expect("the rule");
    let column = 210.0 - 2.0 * 25.0;
    assert!((line.width - column / 4.0).abs() < 0.5, "{line:?}");
    assert!((line.x + line.width / 2.0 - 105.0).abs() < 0.5, "centred: {line:?}");
}

#[test]
fn a_link_is_underlined_when_the_sheet_says() {
    let plain = laid("A [link](https://example.org).", &Stylesheet::default());
    assert!(plain.areas(0).is_empty(), "{:?}", plain.areas(0));
    let laid = laid("A [link](https://example.org).", &sheet("a { text-decoration: underline; color: #0000ff }"));
    let link = run(&laid.runs(0), "link").clone();
    assert_eq!(link.color, "#0000ff");
    let under = laid.areas(0).into_iter().find(|a| a.color == "#0000ff").expect("an underline");
    assert!(under.y > link.y - 0.2 && under.x <= link.x + 0.1, "{under:?} {link:?}");
}

#[test]
fn list_markers_follow_list_style_type() {
    let md = "- one\n- two\n\n1. first\n2. second";
    let laid1 = laid(md, &Stylesheet::default());
    let texts: Vec<String> = laid1.runs(0).iter().map(|r| r.text.clone()).collect();
    assert!(texts.contains(&"\u{2022}".to_string()) && texts.contains(&"1.".to_string()) && texts.contains(&"2.".to_string()), "{texts:?}");
    let laid2 = laid(md, &sheet("ul { list-style-type: square } ol { list-style-type: lower-alpha }"));
    let texts: Vec<String> = laid2.runs(0).iter().map(|r| r.text.clone()).collect();
    assert!(texts.contains(&"\u{25aa}".to_string()) && texts.contains(&"a.".to_string()) && texts.contains(&"b.".to_string()), "{texts:?}");
    let item = run(&laid1.runs(0), "one").x;
    let para = laid("para", &Stylesheet::default()).runs(0)[0].x;
    assert!(item > para + 3.0, "items are set in: {item} {para}");
}

#[test]
fn text_align_centres_a_paragraph() {
    let laid = laid("Centre me.", &sheet("p { text-align: center }"));
    let r = laid.runs(0)[0].clone();
    assert!(r.x > 80.0 && r.x < 105.0, "{r:?}");
}

#[test]
fn what_the_sheet_does_not_know_is_an_error_with_its_line() {
    assert_eq!(errors("h7 { color: red }"), [(1, CssErrorKind::UnknownSelector, "h7".to_string())]);
    assert_eq!(errors("p {\n  colour: red;\n}"), [(2, CssErrorKind::UnknownProperty, "colour".to_string())]);
    assert_eq!(errors("p { color: reddish }"), [(1, CssErrorKind::BadValue, "color: reddish".to_string())]);
    assert_eq!(errors("p { color: red"), [(1, CssErrorKind::NotClosed, "p".to_string())]);
    assert_eq!(errors("body { font-family: Comic Sans }"), [(1, CssErrorKind::NoCarriedFont, "Comic Sans".to_string())]);
    assert_eq!(errors("p { color: var(--nope) }"), [(1, CssErrorKind::UnknownVariable, "--nope".to_string())]);
    assert_eq!(errors("@page { color: red }"), [(1, CssErrorKind::UnknownProperty, "color".to_string())]);
    let both = errors("h7 { }\np { size: 3pt }");
    assert_eq!(both.len(), 2, "every error is told: {both:?}");
}

#[test]
fn a_variable_takes_its_fallback_without_a_theme() {
    let laid = laid("Text.", &sheet("p { color: var(--accent, #123456) }"));
    assert_eq!(run(&laid.runs(0), "Text").color, "#123456");
}

#[test]
fn a_theme_that_is_not_a_theme_is_told() {
    assert!(Theme::read("not json").is_err());
}

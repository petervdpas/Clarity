//! A memo of Markdown: `markdown` on its line makes it an editor with a
//! preview, Write and Preview on its top, with Clarity's words for them.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::Severity;

const PAGE: &str = "module m

entity Chapter
    text: text, label @chapterText

page Desk: window
    data chapter: Chapter
    memo name manuscriptBox at 1,1 span 12 rows 10 bind chapter.text markdown
";

fn errors(text: &str) -> Vec<String> {
    load_keyed([("m.clarity", text)], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error && !d.message.contains("has no value")).map(|d| d.message).collect()
}

#[test]
fn a_memo_of_markdown_is_written_and_read_back() {
    assert!(errors(PAGE).is_empty(), "{:#?}", errors(PAGE));
    let (file, _) = parse(PAGE, "m.clarity");
    let p = &file.pages[0].widgets[0].props;
    assert!(p.markdown);
    assert!(clarity_spec::widgets::widget_line(p).ends_with(" markdown"), "{}", clarity_spec::widgets::widget_line(p));
}

#[test]
fn it_colors_its_markdown_itself_and_only_a_memo_is_one() {
    assert!(errors(&PAGE.replace(" markdown", " markdown code rust")).iter().any(|m| m.contains("leave out 'code'")));
    assert!(errors(&PAGE.replace(" markdown", " code rust markdown")).iter().any(|m| m.contains("leave out 'code'")));
    assert!(!errors(&PAGE.replace("memo name", "textbox name").replace(" rows 10", "")).is_empty(), "a text box is no markdown memo");
}

#[test]
fn clarity_has_its_words_for_write_and_preview() {
    assert_eq!(clarity_spec::translations::MARKDOWN_MEMO_KEYS.map(|k| k.0), ["markdownWrite", "markdownPreview"]);
    let project = load_keyed([("m.clarity", PAGE)], &[], "en");
    let keys: Vec<String> = clarity_spec::translations::keys(&project).into_iter().map(|k| k.key).collect();
    assert!(keys.iter().any(|k| k == "markdownWrite") && keys.iter().any(|k| k == "markdownPreview"), "{keys:?}");
    let plain_text = PAGE.replace(" markdown", "");
    let plain = load_keyed([("m.clarity", plain_text.as_str())], &[], "en");
    assert!(!clarity_spec::translations::keys(&plain).iter().any(|k| k.key == "markdownWrite"));
}

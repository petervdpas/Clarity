//! The markdown part: text written in Markdown, shown with its headings,
//! lists, emphasis and links, wrapped and scrolled like a text over several
//! rows. Its text comes from a key (`markdown @gameRules`), a Markdown file
//! of the resources folder (`source "rules.md"`) or a text it binds.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{BindTarget, Category, WidgetKind, info as widget_info, widget_line};
use clarity_spec::{Severity, load_sources};

const GAME: &str = "module game

page Main: window
    data rules: text
    data turns: integer
    markdown name rulesText at 1,1 span 6 rows 8 bind rules
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("game.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_markdown_part_shows_a_text() {
    let (file, diags) = parse(GAME, "game.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let rules = &file.pages[0].widgets[0];
    assert_eq!(rules.props.kind, WidgetKind::Markdown);
    assert_eq!(widget_line(&rules.props), "markdown name rulesText at 1,1 span 6 rows 8 bind rules");
    assert!(errors(GAME).is_empty(), "{:#?}", errors(GAME));

    let info = widget_info(WidgetKind::Markdown);
    assert_eq!((info.keyword.as_str(), info.name.as_str(), info.category), ("markdown", "Markdown", Category::Text));
    assert_eq!((info.width, info.height), (6, 4));
    assert_eq!(info.bind, BindTarget::Field);
    assert_eq!(info.field_types, ["text"]);
    assert!(info.grows && !info.input && info.events.is_empty());
}

#[test]
fn its_text_is_a_key_a_file_of_the_resources_or_a_text_it_binds() {
    // A Markdown file of the resources folder, as an image shows a picture (tests/resources.rs).
    let (file, diags) = parse(&GAME.replace("bind rules", "source \"rules.md\""), "game.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "markdown name rulesText at 1,1 span 6 rows 8 source \"rules.md\"");
    // Or a key, whose value in each language is a Markdown file or Markdown as text (tests/resources.rs).
    let (file, diags) = parse(&GAME.replace("markdown name rulesText at 1,1 span 6 rows 8 bind rules", "markdown @gameRules name rulesText at 1,1 span 6 rows 8"), "game.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "markdown @gameRules name rulesText at 1,1 span 6 rows 8");
    let both = errors(&GAME.replace("bind rules", "source \"rules.md\" bind rules"));
    assert!(both.iter().any(|e| e == "a markdown shows a key, a file of the resources (source) or what it binds: one of them"), "{both:#?}");
    // Only a text is Markdown.
    let bad = errors(&GAME.replace("bind rules", "bind turns"));
    assert!(bad.iter().any(|e| e.contains("'turns' is integer; a markdown shows text")), "{bad:#?}");
}

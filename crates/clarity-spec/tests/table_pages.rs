//! A table shows its rows a page at a time (U1c): `page 50` after its
//! columns, a bar below it to go through the pages, with Clarity's own
//! words on its buttons.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets;
use clarity_spec::{Severity, load_sources};

const MUSIC: &str = "module music

entity Album
    title: text(120), required
    artist: text(80)

page Main: window
    data albums: list<Album>
    data album: Album
";

fn with(part: &str) -> String {
    format!("{MUSIC}    {part}\n")
}

fn errors(part: &str) -> Vec<String> {
    let text = with(part);
    load_sources([("music.clarity", text.as_str())]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

const TABLE: &str = "table name grid at 1,1 span 12 rows 6 bind albums columns title, artist sortable order by title page 50";

#[test]
fn a_table_reads_its_page_size_and_writes_it_back() {
    let text = with(TABLE);
    let (file, diagnostics) = parse(&text, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.page, Some(50));
    assert_eq!(widgets::widget_line(props), TABLE);
    assert!(errors(TABLE).is_empty());
}

#[test]
fn a_page_holds_1_to_1000_rows_and_only_a_table_has_pages() {
    assert!(errors(&TABLE.replace("page 50", "page 0")).iter().any(|m| m.contains("a page holds 1 to 1000 rows")));
    assert!(errors(&TABLE.replace("page 50", "page 1001")).iter().any(|m| m.contains("a page holds 1 to 1000 rows")));
    assert!(errors("textbox name titleBox at 1,1 span 4 bind album.title page 5").iter().any(|m| m.contains("'page' does not apply to a textbox")));
}

#[test]
fn a_paged_table_has_clarity_s_own_words() {
    let text = with(TABLE);
    let p = load_keyed([("music.clarity", text.as_str())], &[], "en");
    let en = &p.translations["en"];
    for (key, word) in clarity_spec::translations::PAGE_KEYS {
        assert_eq!(en.get(key).map(String::as_str), Some(word), "{key}");
    }
}

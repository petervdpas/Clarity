//! A table's columns (U1e, DESIGN.md): a width in pixels after a field
//! (`columns title 240, artist`), `resizable` and `sortable` as options,
//! and `order by field [desc]` for the order its rows show in first.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Severity, load_sources};

const MUSIC: &str = "module music

entity Album
    title: text(120), required
    artist: text(80)
    release_year: integer
    cover: image

page Main: window
    data albums: list<Album>
";

fn with(part: &str) -> String {
    format!("{MUSIC}    {part}\n")
}

fn errors(part: &str) -> Vec<String> {
    let text = with(part);
    load_sources([("music.clarity", text.as_str())])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_table_reads_widths_options_and_its_first_order_and_writes_them_back() {
    let line = "table name grid at 1,1 span 12 rows 6 bind albums columns title 240, artist, release_year 60 resizable sortable order by release_year desc";
    let text = with(line);
    let (file, diagnostics) = parse(&text, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.columns, ["title", "artist", "release_year"]);
    assert_eq!(props.widths.get("title"), Some(&240));
    assert_eq!(props.widths.get("artist"), None, "a column without a width shares what is left");
    assert_eq!(props.widths.get("release_year"), Some(&60));
    assert!(props.resizable && props.sortable);
    let order = props.order.as_ref().unwrap();
    assert_eq!((order.field.as_str(), order.desc), ("release_year", true));
    assert_eq!(widgets::widget_line(props), line);
    assert!(errors(line).is_empty(), "{:?}", errors(line));
}

#[test]
fn without_the_options_a_table_is_written_as_before() {
    let line = "table name grid at 1,1 span 12 rows 6 bind albums columns title, artist order by title";
    let text = with(line);
    let (file, _) = parse(&text, "music.clarity");
    let props = &file.pages[0].widgets[0].props;
    assert!(!props.resizable && !props.sortable && props.widths.is_empty());
    assert_eq!(props.order.as_ref().map(|o| o.desc), Some(false));
    assert_eq!(widgets::widget_line(props), line);
}

#[test]
fn the_first_order_is_by_a_column_and_widths_stay_in_range() {
    let unknown = errors("table name grid at 1,1 span 12 rows 6 bind albums columns title order by artist");
    assert!(unknown.iter().any(|m| m.contains("'artist' is not a column of the table")), "{unknown:?}");
    let narrow = errors("table name grid at 1,1 span 12 rows 6 bind albums columns title 5");
    assert!(narrow.iter().any(|m| m.contains("20 to 2000")), "{narrow:?}");
    let list = errors("list name names at 1,1 span 6 rows 6 bind albums show title sortable");
    assert!(list.iter().any(|m| m.contains("'sortable' does not apply to a list")), "{list:?}");
}

#[test]
fn the_options_belong_to_the_table() {
    for property in [widgets::WidgetProperty::Resizable, widgets::WidgetProperty::Sortable, widgets::WidgetProperty::OrderBy] {
        assert!(widgets::has_property(WidgetKind::Table, property), "{property:?}");
        assert!(!widgets::has_property(WidgetKind::List, property), "{property:?}");
    }
}

#[test]
fn naming_the_parts_leaves_the_fields_of_a_table_alone() {
    // A part named as a field: renaming it must not touch the field after a width, nor the order.
    let text = format!(
        "{MUSIC}    table name grid at 1,1 span 12 rows 6 bind albums columns title 240, release_year order by release_year\n    text name release_year at 1,7 span 6\n"
    );
    let (named, renames) = clarity_spec::names::camel_names_with(&text);
    assert_eq!(renames, vec![("release_year".to_string(), "releaseYear".to_string())]);
    assert!(named.contains("columns title 240, release_year order by release_year\n"), "{named}");
    assert!(named.contains("text name releaseYear at 1,7"), "{named}");
}

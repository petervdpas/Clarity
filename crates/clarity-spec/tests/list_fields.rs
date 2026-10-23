//! Lists of values as fields (U1c): `list<text>`, `list<integer>`,
//! `list<decimal>`, `list<date>` and `list<Enum>`, kept in one column as
//! JSON. A `tags` part edits one as chips; a table shows it as its items.
//! In code a list field is a list of values like any other: `for`, `count`,
//! `xs[i]`, a new list, and `contains` for one of its items.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::{FieldType, ListItem};
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const MUSIC: &str = "module music

enum Genre
    Jazz
    Rock

entity Album
    title: text(80), required
    tags: list<text>
    genres: list<Genre>
    tracks: list<integer>
    prices: list<decimal>
    shows: list<date>

page AlbumCard: window
    data album: Album
    tags name tagsBox at 1,1 span 6 bind album.tags on change: tagsChanged
    tags name genresBox at 1,2 span 6 bind album.genres
    textbox name countText at 7,1 span 4

page Albums: window
    data albums: list<Album>
    table name grid at 1,1 span 12 rows 4 bind albums columns title, tags, genres, shows
";

const CODE: &str = "
code AlbumCard
    handler tagsChanged
        countText.text = \"\" + count(album.tags)
        if album.tags contains \"jazz\"
            album.genres = [Jazz]
        for show in album.shows
            countText.text = \"\" + show
        album.tracks = [1, 2, 3]
        album.tracks[2] = 7
        album.prices = [1.50]
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("music.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn list_fields_read_and_write_back() {
    let (file, diagnostics) = parse(MUSIC, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let album = &file.entities[0];
    assert_eq!(album.field("tags").unwrap().ty, FieldType::Values { item: ListItem::Text });
    assert_eq!(album.field("tracks").unwrap().ty, FieldType::Values { item: ListItem::Integer });
    assert_eq!(album.field("prices").unwrap().ty, FieldType::Values { item: ListItem::Decimal });
    assert_eq!(album.field("shows").unwrap().ty, FieldType::Values { item: ListItem::Date });
    assert_eq!(album.field("shows").unwrap().type_text, "list<date>");
    let part = &file.pages[0].widgets[0];
    assert_eq!(part.props.kind, WidgetKind::Tags);
    assert_eq!(widgets::widget_line(&part.props), "tags name tagsBox at 1,1 span 6 bind album.tags on change: tagsChanged");
    assert_eq!(widgets::widget_for(&FieldType::Values { item: ListItem::Text }), WidgetKind::Tags, "a dropped list field gets a tags part");
}

#[test]
fn a_list_of_an_enum_is_a_list_of_values_once_loaded() {
    let p = project(MUSIC, CODE);
    let album = p.entities().find(|e| e.name.name == "Album").unwrap();
    match &album.field("genres").unwrap().ty {
        FieldType::Values { item: ListItem::Enum { name } } => assert_eq!(name.name, "Genre"),
        other => panic!("{other:?}"),
    }
    assert_eq!(album.field("genres").unwrap().type_text, "list<Genre>");
    assert!(p.stored_entities().any(|e| e.name.name == "Album"));
}

#[test]
fn code_uses_a_list_field_as_a_list_of_values() {
    let found = errors(MUSIC, CODE);
    assert!(found.is_empty(), "{found:#?}");
    let wrong = errors(MUSIC, &CODE.replace("album.tags contains \"jazz\"", "album.tracks contains \"jazz\""));
    assert!(wrong.iter().any(|m| m.contains("cannot look for text in list<integer>")), "{wrong:#?}");
    let query = errors(MUSIC, &CODE.replace("        album.prices = [1.50]\n", "        let found = find a in Album where a.tags contains \"jazz\"\n"));
    assert!(query.iter().any(|m| m.contains("a query cannot look inside list<text>")), "{query:#?}");
    let enum_wrong = errors(MUSIC, &CODE.replace("album.genres = [Jazz]", "album.genres = [\"Jazz\"]"));
    assert!(!enum_wrong.is_empty(), "text is not a Genre");
}

#[test]
fn the_rules_of_a_list_field() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page, CODE);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(MUSIC.replace("    tags: list<text>\n", "    tags: list<text>, unique\n"), "a list cannot be unique ('tags')");
    says(MUSIC.replace("    tags: list<text>\n", "    tags: list<text>, default \"a\"\n"), "a list has no default; it starts empty ('tags')");
    says(MUSIC.replace("    tracks: list<integer>\n", "    tracks: list<integer>, min 1\n"), "'min' needs a number");
    says(MUSIC.replace("    tags: list<text>\n", "    tags: list<bool>\n"), "a list field holds text, integer, decimal, date or an enum");
    says(MUSIC.replace("tags name genresBox at 1,2 span 6 bind album.genres", "tags name genresBox at 1,2 span 6 bind album.title"), "'title' is text; a tags shows values");
}

#[test]
fn a_tags_part_has_clarity_s_own_words() {
    let p = project(MUSIC, CODE);
    let en = &p.translations["en"];
    for (key, word) in clarity_spec::translations::TAGS_KEYS {
        assert_eq!(en.get(key).map(String::as_str), Some(word), "{key}");
    }
}

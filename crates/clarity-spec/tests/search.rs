//! Full-text search (U1h): `search` on the text fields and lists of text a
//! record is found by, and `where a matches x` in a find, which finds the
//! records holding every word of `x`, each as the start of a word. Each
//! database keeps its own full-text index of those fields, which the
//! migration steps `add search` and `remove search` build.

use clarity_spec::keyed::load_keyed;
use clarity_spec::migrate;
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, load_with_migrations};

const MUSIC: &str = "module music

entity Artist
    name: text(80), required, search

entity Album
    title: text(80), required, search
    notes: text, search
    tags: list<text>, search
    artist: ref Artist
    year: integer

page Albums: window
    data albums: list<Album>
    textbox name searchBox at 1,1 span 6 on change: searchChanged
    table name grid at 1,2 span 12 rows 6 bind albums columns title, year
";

const CODE: &str = "
code Albums
    handler searchChanged
        albums = find a in Album where a matches searchBox.text and a.year > 1950 order by a.title
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("music.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn says(page: &str, code: &str, wanted: &str) {
    let found = errors(page, code);
    assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
}

#[test]
fn search_is_a_field_modifier_that_writes_back() {
    let (file, diagnostics) = parse(MUSIC, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let album = &file.entities[1];
    let searched: Vec<&str> = album.fields.iter().filter(|f| f.modifiers.search).map(|f| f.name.name.as_str()).collect();
    assert_eq!(searched, ["title", "notes", "tags"]);
    let title = &album.fields[0];
    let spec = clarity_spec::dictionary::FieldSpec { name: title.name.name.clone(), type_text: title.type_text.clone(), modifiers: title.modifiers.clone() };
    let written = clarity_spec::dictionary::field_line(&spec);
    assert_eq!(written, "title: text(80), required, search");
    let found = errors(MUSIC, CODE);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn only_text_and_lists_of_text_are_searched() {
    says(&MUSIC.replace("    year: integer\n", "    year: integer, search\n"), CODE, "only text and lists of text can be searched ('year')");
    says(&MUSIC.replace("    artist: ref Artist\n", "    artist: ref Artist, search\n"), CODE, "only text and lists of text can be searched ('artist')");
    says(&MUSIC.replace("    year: integer\n", "    year: integer\n    plays: list<integer>, search\n"), CODE, "only text and lists of text can be searched ('plays')");
    says(&MUSIC.replace("required, search\n", "required, search, search\n"), CODE, "search");
    let email = MUSIC.replace("    year: integer\n", "    year: integer\n    mail: email, search\n    site: url, search\n");
    let found = errors(&email, CODE);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_rules_of_matches() {
    says(MUSIC, &CODE.replace("a matches searchBox.text", "a.title matches searchBox.text"), "'matches' looks for words in the records found, as in 'a matches x'");
    says(MUSIC, &CODE.replace("a matches searchBox.text", "a matches a.year"), "'matches' looks for text, not integer");
    says(
        &MUSIC.replace("    name: text(80), required, search\n", "    name: text(80), required\n"),
        "\ncode Albums\n    handler searchChanged\n        let found = find r in Artist where r matches searchBox.text\n",
        "Artist has no field marked search to look for words in",
    );
    says(
        MUSIC,
        "\ncode Albums\n    handler searchChanged\n        let album = new Album\n        if album matches \"jazz\"\n            searchBox.text = \"\"\n",
        "'matches' looks for words in the records a find looks through, as in 'find a in Album where a matches x'",
    );
}

const FIRST: &str = "migration \"Music\"
    add entity Artist
        name: text(80), search
    add entity Album
        title: text(80), search
        notes: text
        tags: list<text>
        artist: ref Artist
        year: integer
";

#[test]
fn search_comes_and_goes_in_migration_steps() {
    let source = format!("{MUSIC}{CODE}");
    let load = |steps: &[(&str, &str)]| load_with_migrations([("music.clarity", source.as_str())], steps.iter().copied());
    // The dictionary searches notes and tags too: two pending steps.
    let project = load(&[("0001.clarity", FIRST)]);
    let pending: Vec<String> = migrate::pending(&project, "main", &[]).iter().map(migrate::op_text).collect();
    assert_eq!(pending, ["add search Album.notes", "add search Album.tags"]);
    let warnings: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message.clone()).collect();
    assert!(warnings.iter().any(|w| w.contains("search notes")), "{warnings:#?}");

    let second = "migration \"Search notes and tags\"\n    add search Album.notes\n    add search Album.tags\n";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", second)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let schema = migrate::schema(&project, "main");
    let album = schema.table("Album").unwrap();
    let searched: Vec<&str> = album.columns.iter().filter(|c| c.search).map(|c| c.name.as_str()).collect();
    assert_eq!(searched, ["title", "notes", "tags"]);

    // A step written from the dictionary keeps search on the field.
    let written = migrate::step_text("Music", &migrate::pending(&load(&[]), "main", &[]));
    assert!(written.contains("        title: text(80), search\n"), "{written}");

    let third = "migration \"Less\"\n    remove search Album.title\n    remove search Album.year\n    add search Album.notes\n";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", second), ("0003.clarity", third)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.iter().any(|e| e.contains("Album.year is not searched")), "{errors:#?}");
    assert!(errors.iter().any(|e| e.contains("Album.notes is searched already")), "{errors:#?}");
}

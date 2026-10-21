//! Pictures in records (X2): the `image` field type, kept in the database
//! or, with `store`, as files in a file store of the project settings.

use clarity_spec::model::FieldType;
use clarity_spec::{Project, Severity, load_sources};

const MUSIC: &str = "module music

entity Album
    title: text(120)
    cover: image, help \"The scan of the front\"
    photo: image, store photos

page AlbumForm: window
    data album: Album
    data other: Album
    image name cover at 1,1 span 3 rows 3 bind album.cover
    button \"Clear\" name button1 at 1,4 span 2 on click: clearClicked

code AlbumForm
    handler clearClicked
        if album.cover != none
            album.cover = none
";

fn load(source: &str, stores: Option<&[&str]>) -> Project {
    let mut project = load_sources([("music.clarity", source)]);
    if let Some(stores) = stores {
        clarity_spec::attach_stores(&mut project, stores.iter().map(|s| s.to_string()).collect());
    }
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(MUSIC.contains(old), "{old:?}");
    MUSIC.replace(old, new)
}

#[test]
fn an_image_field_is_kept_in_the_database_or_in_a_file_store() {
    let project = load(MUSIC, Some(&["photos"]));
    assert_eq!(errors(&project), Vec::<String>::new());
    let album = project.entity("Album").unwrap();
    let cover = album.field("cover").unwrap();
    assert_eq!(cover.ty, FieldType::Image);
    assert_eq!(cover.modifiers.store, None);
    assert_eq!(album.field("photo").unwrap().modifiers.store.as_deref(), Some("photos"));
}

#[test]
fn a_store_is_one_of_the_project_settings() {
    assert_eq!(
        errors(&load(MUSIC, Some(&["scans"]))),
        ["'photo' is kept in file store 'photos', which the project settings do not have; they have scans"]
    );
    assert_eq!(
        errors(&load(MUSIC, Some(&[]))),
        ["'photo' is kept in file store 'photos', which the project settings do not have; add it there"]
    );
    // Loaded without its settings, nothing is checked.
    assert_eq!(errors(&load(MUSIC, None)), Vec::<String>::new());
}

#[test]
fn only_an_image_field_has_a_store() {
    let source = with("title: text(120)", "title: text(120), store photos");
    assert_eq!(errors(&load(&source, Some(&["photos"]))), ["only an image field has a store ('title')"]);
}

#[test]
fn a_picture_is_not_unique_and_has_no_default() {
    let unique = with("cover: image,", "cover: image, unique,");
    assert_eq!(errors(&load(&unique, Some(&["photos"]))), ["a picture cannot be unique ('cover')"]);
    let default = with("cover: image,", "cover: image, default 1,");
    assert_eq!(errors(&load(&default, Some(&["photos"]))), ["a picture has no default; it starts empty ('cover')"]);
}

#[test]
fn a_picture_shows_in_an_image_part_only() {
    let textbox = with("image name cover at 1,1 span 3 rows 3 bind album.cover", "textbox name cover at 1,1 span 3 bind album.cover");
    let found = errors(&load(&textbox, Some(&["photos"])));
    assert!(found.iter().any(|e| e.contains("image")), "{found:#?}");

    let table = with("    data other: Album\n", "    data other: Album\n    data albums: list<Album>\n    table name table1 at 1,6 span 6 rows 4 bind albums columns title, cover\n");
    assert_eq!(errors(&load(&table, Some(&["photos"]))), ["table at 1,6: 'cover' is a picture; show it in an image part"]);
}

#[test]
fn code_empties_a_picture_and_tests_for_one() {
    let copy = with("            album.cover = none", "            album.cover = other.cover");
    assert_eq!(errors(&load(&copy, Some(&["photos"]))), ["a picture cannot be copied from one record to another yet"]);
    let slot = with("            album.cover = none", "            message \"{album.cover}\"");
    let found = errors(&load(&slot, Some(&["photos"])));
    assert!(found.iter().any(|e| e.contains("album.cover")), "{found:#?}");
}

fn steps(project: &Project) -> Vec<String> {
    clarity_spec::migrate::pending(project, "main", &[]).iter().map(clarity_spec::migrate::op_text).collect()
}

/// A migration step says where a picture is kept, and reads back the same.
#[test]
fn a_migration_step_keeps_the_store() {
    let project = load(MUSIC, Some(&["photos"]));
    let ops = clarity_spec::migrate::pending(&project, "main", &[]);
    let text = clarity_spec::migrate::step_text("Albums", &ops);
    assert!(text.contains("        cover: image\n"), "{text}");
    assert!(text.contains("        photo: image, store photos\n"), "{text}");
    let after = clarity_spec::load_with_migrations([("music.clarity", MUSIC)], [("migrations/main/0001.clarity", text.as_str())]);
    assert!(steps(&after).is_empty(), "{:?}", steps(&after));

    // Moving the pictures between the database and a file store changes the field.
    let moved = clarity_spec::load_with_migrations(
        [("music.clarity", MUSIC.replace("cover: image,", "cover: image, store photos,").as_str())],
        [("migrations/main/0001.clarity", text.as_str())],
    );
    assert_eq!(steps(&moved), ["change field Album.cover: image, store photos"]);
    let ops = clarity_spec::migrate::pending(&moved, "main", &[]);
    let second = clarity_spec::migrate::step_text("Covers as files", &ops);
    let done = clarity_spec::load_with_migrations(
        [("music.clarity", MUSIC.replace("cover: image,", "cover: image, store photos,").as_str())],
        [("migrations/main/0001.clarity", text.as_str()), ("migrations/main/0002.clarity", second.as_str())],
    );
    assert!(steps(&done).is_empty(), "{:?}", steps(&done));
}

/// The words an image part shows of its own are keys the language files
/// give, in a project that has one bound to a picture.
#[test]
fn an_image_part_brings_its_own_keys() {
    let keys: Vec<String> = clarity_spec::translations::keys(&load(MUSIC, Some(&["photos"]))).into_iter().map(|u| u.key).collect();
    for (key, _) in clarity_spec::translations::PICTURE_KEYS {
        assert!(keys.iter().any(|k| k == key), "{key} in {keys:?}");
    }
    let without = MUSIC.replace("    image name cover at 1,1 span 3 rows 3 bind album.cover\n", "");
    let keys: Vec<String> = clarity_spec::translations::keys(&load(&without, Some(&["photos"]))).into_iter().map(|u| u.key).collect();
    assert!(!keys.iter().any(|k| k == "noPicture"), "{keys:?}");
}

/// The data dictionary writes a field back with its store.
#[test]
fn the_dictionary_writes_the_store() {
    let project = load(MUSIC, Some(&["photos"]));
    let photo = project.entity("Album").unwrap().field("photo").unwrap();
    let spec = clarity_spec::dictionary::FieldSpec { name: "photo".into(), type_text: "image".into(), modifiers: photo.modifiers.clone() };
    assert_eq!(clarity_spec::dictionary::field_line(&spec), "photo: image, store photos");
    assert_eq!(clarity_spec::dictionary::validate_field(&spec).unwrap(), "photo: image, store photos");
}

/// An image part bound to a picture is an input: a label or help text can
/// be for it, and an image field is placed as one.
#[test]
fn a_picture_part_takes_a_label_and_help() {
    let source = with("    button \"Clear\"", "    help name coverHelp at 4,1 span 3 for cover\n    button \"Clear\"");
    assert_eq!(errors(&load(&source, Some(&["photos"]))), Vec::<String>::new());
    let cover = load(MUSIC, Some(&["photos"])).entity("Album").unwrap().field("cover").unwrap().ty.clone();
    assert_eq!(clarity_spec::widgets::widget_for(&cover), clarity_spec::widgets::WidgetKind::Image);
}

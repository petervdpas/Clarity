//! Folder settings (X2): the folder a file store keeps its files in, chosen
//! by each person while the application runs and kept with their settings.
//! The project file points the store at it, `filestore photos
//! "setting:photosFolder"`, and a folder part shows it with a button that
//! chooses another.

use clarity_spec::model::FieldType;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity, load_sources};

const SHOP: &str = "module shop

settings
    language
    photosFolder: folder, default \"photos\", label \"Photos\"

entity Album
    title: text
    photo: image, store photos

page Preferences: dialog
    folder name photosBox at 1,1 span 6 bind settings.photosFolder

page Main: window
    button \"Reset\" name reset at 1,1 span 2 on click: resetClicked

code Main
    handler resetClicked
        settings.photosFolder = \"\"
";

fn errors_of(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn errors(source: &str) -> Vec<String> {
    errors_of(&load_sources([("shop.clarity", source)]))
}

fn with(old: &str, new: &str) -> String {
    assert!(SHOP.contains(old), "{old:?}");
    SHOP.replacen(old, new, 1)
}

#[test]
fn a_folder_is_a_setting_a_folder_part_shows() {
    assert_eq!(errors(SHOP), Vec::<String>::new());
    let project = load_sources([("shop.clarity", SHOP)]);
    let settings = project.files[0].entities.iter().find(|e| e.settings).unwrap();
    assert_eq!(settings.fields[1].ty, FieldType::Folder);
    assert_eq!(project.folder_settings(), ["photosFolder"]);
    let part = &project.files[0].pages[0].widgets[0];
    assert_eq!(part.props.kind, WidgetKind::Folder);
    assert_eq!(widgets::widget_line(&part.props), "folder name photosBox at 1,1 span 6 bind settings.photosFolder");
    assert_eq!(widgets::widget_for(&FieldType::Folder), WidgetKind::Folder, "a settings window gets a folder part");
}

#[test]
fn only_the_settings_keep_a_folder() {
    let entity = with("    photo: image, store photos\n", "    photo: image, store photos\n    place: folder\n");
    assert_eq!(errors(&entity), ["a folder is a setting of the person, where a file store keeps its files; entity Album cannot keep 'place' as one"]);
}

#[test]
fn only_a_folder_part_shows_a_folder_setting() {
    // It has its own text field, so a path can be typed where no folder picker opens.
    let typed = with("folder name photosBox at 1,1 span 6", "textbox name photosBox at 1,1 span 6");
    let found = errors(&typed);
    assert!(found.iter().any(|e| e.contains("folder")), "{found:#?}");
    let title = with("    folder name photosBox at 1,1 span 6 bind settings.photosFolder\n", "    data album: Album\n    folder name photosBox at 1,1 span 6 bind album.title\n");
    let found = errors(&title);
    assert!(found.iter().any(|e| e.contains("folder")), "{found:#?}");
}

#[test]
fn a_file_store_keeps_its_files_in_a_folder_setting() {
    let checked = |location: &str| {
        let mut project = load_sources([("shop.clarity", SHOP)]);
        clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
        clarity_spec::check_store_folders(&mut project, &[("photos".into(), location.into())]);
        errors_of(&project)
    };
    assert_eq!(checked("setting:photosFolder"), Vec::<String>::new());
    assert_eq!(checked("data/photos"), Vec::<String>::new());
    assert_eq!(checked("env:PHOTOS"), Vec::<String>::new());
    assert_eq!(checked("setting:language"), ["file store photos keeps its files in setting 'language', which is no folder setting; add it to settings as language: folder"]);
    assert_eq!(checked("setting:scans"), ["file store photos keeps its files in setting 'scans', which is no folder setting; add it to settings as scans: folder"]);
}

#[test]
fn a_folder_part_has_clarity_s_own_word() {
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
    assert_eq!(project.translations["en"].get("chooseFolder").map(String::as_str), Some("Choose a folder"));
    let without = with("    folder name photosBox at 1,1 span 6 bind settings.photosFolder\n", "");
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", without.as_str())], &[], "en");
    assert_eq!(project.translations["en"].get("chooseFolder"), None, "only a project with a folder part has it");
}

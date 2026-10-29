//! A file store whose folder is a folder setting (`filestore photos
//! "setting:photosFolder"`): every stack's program adds it so the runtime
//! reads the person's settings at every use, and its default while none is kept.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, FileStore, Ui, starter};

const SHOP: &str = "module shop

settings
    photosFolder: folder, default \"photos\", label \"Photos\"

entity Album
    title: text
    photo: image, store photos

page Preferences: dialog
    folder name photosBox at 1,1 span 6 bind settings.photosFolder

page Albums: window
    data album: Album
    image name photo at 1,1 span 3 rows 3 bind album.photo
";

fn files(backend: Backend, ui: Ui) -> String {
    let mut project = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
    clarity_spec::attach_stores(&mut project, vec!["photos".into()]);
    clarity_spec::check_store_folders(&mut project, &[("photos".into(), "setting:photosFolder".into())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = starter("Records", "records");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.filestores.push(FileStore { name: "photos".into(), location: "setting:photosFolder".into() });
    let out: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    out.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program");
}

#[test]
fn every_stack_adds_a_store_that_follows_its_folder_setting() {
    has(&files(Backend::Rust, Ui::Web), "db.add_store_setting(\"photos\", \"records\", \"photosFolder\", \"photos\")?;");
    has(&files(Backend::Go, Ui::Web), "db.AddStoreSetting(\"photos\", \"records\", \"photosFolder\", \"photos\")");
    has(&files(Backend::Csharp, Ui::Avalonia), "db.AddStoreSetting(\"photos\", \"records\", \"photosFolder\", \"photos\");");
    has(&files(Backend::C, Ui::Web), "clarity_add_store_setting(db, \"photos\", \"records\", \"photosFolder\", \"photos\")");
    has(&files(Backend::Cpp, Ui::Web), "db.add_store_setting(\"photos\", \"records\", \"photosFolder\", \"photos\");");
}

/// The folder part's button asks the shell for the system's folder picker:
/// Tauri's comes from its dialog plugin, only in a program with a folder part.
#[test]
fn the_tauri_program_opens_the_systems_folder_picker() {
    let rust = files(Backend::Rust, Ui::Web);
    has(&rust, "tauri-plugin-dialog = \"2\"");
    has(&rust, ".plugin(tauri_plugin_dialog::init())");
    has(&rust, "async fn choose_folder(app: tauri::AppHandle, title: String) -> Answer<Option<String>> {");
    has(&rust, "            choose_folder,\n");
}

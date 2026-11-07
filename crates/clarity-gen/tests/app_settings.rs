//! A handler in the app's code (a menu item's) that sets a setting, such as
//! `settings.language = "nl"`, starts from the window's settings and hands
//! them back in its reply, on every stack that dispatches the app's handlers.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};
use clarity_spec::load_sources;

const SOURCE: &str = "module m\n\napp Shop\n    main Desk\n    menu name mainMenu\n        item \"Dutch\" name dutchItem on click: toDutch\n\nsettings\n    language\n\npage Desk: window\n    data note: text\n\ncode Shop\n    handler toDutch\n        settings.language = \"nl\"\n";

#[test]
fn the_app_s_handlers_carry_the_window_s_settings_on_every_stack() {
    let project = load_sources([("m.clarity", SOURCE)]);
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    // What each stack writes to read the window's settings and to give them back.
    let stacks = [
        (Backend::Rust, "main.rs", "app.settings = serde_json::from_value(kept.clone())", "data.insert(\"settings\".to_string()"),
        (Backend::Go, "main.go", "json.Unmarshal(raw, &app.Settings)", "data[\"settings\"] = app.Settings"),
        (Backend::C, "main.c", "settings_from_json(kept, &app.settings)", "cJSON_AddItemToObject(data, \"settings\", settings_json(&app.settings))"),
        (Backend::Cpp, "main.cpp", "app.settings = state.data[\"settings\"].get<Settings>()", "data[\"settings\"] = app.settings"),
    ];
    for (backend, main, read, give) in stacks {
        let mut settings = starter("Shop", "shop");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        let text = files.iter().find(|f| f.path.ends_with(main) && f.path.contains("desktop")).or_else(|| files.iter().find(|f| f.path.ends_with(main))).map(|f| f.text.clone()).unwrap_or_else(|| panic!("{backend:?} has no {main}"));
        assert!(text.contains(read), "{backend:?} reads the window's settings into the app");
        assert!(text.contains(give), "{backend:?} gives the app's settings back");
        // The names it uses are the ones the app's struct and the settings type declare.
        let all: String = files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n");
        let declared: &[&str] = match backend {
            Backend::Rust => &["pub settings: Settings"],
            Backend::Go => &["Settings *Settings `json:\"settings\"`"],
            Backend::C => &["Settings settings;", "bool settings_from_json(const cJSON* in, Settings* out);", "cJSON* settings_json(const Settings* "],
            _ => &["Settings settings;"],
        };
        for d in declared {
            assert!(all.contains(d), "{backend:?} declares {d}");
        }
    }
}

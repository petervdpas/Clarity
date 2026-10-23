//! Settings (DESIGN 5.17): one `settings` block of typed values each person
//! keeps for themselves, with Clarity's own `language` and `theme`. Every
//! window has `settings` as data of its own; parts bind it, code reads and
//! sets it. It is a record type beside the entities, never in the database.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::{DataKind, FieldType};
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

enum Density
    compact
    roomy

entity Customer
    name: text(80), required

settings
    language
    theme
    showToolbar: bool, default true, label \"Show the toolbar\"
    pageSize: integer, min 10, max 200, default 50
    density: Density, default roomy

page Preferences: dialog
    form name settingsForm at 1,1 span 8 rows 5 bind settings labels 3
        dropdown name languageBox at 4,1 span 4 bind settings.language
        dropdown name themeBox at 4,2 span 4 bind settings.theme
        switch name toolbarSwitch at 4,3 span 4 bind settings.showToolbar
        spinbox name pageSizeBox at 4,4 span 2 bind settings.pageSize
        radio name densityRadio at 4,5 span 4 bind settings.density

page Card: window
    data customer: Customer
    textbox name nameBox at 1,1 span 4 bind customer.name
    button \"Fewer\" name fewerButton at 5,1 span 2 on click: fewerClicked
";

const CODE: &str = "
code Card
    handler fewerClicked
        if settings.showToolbar
            settings.pageSize = settings.pageSize - 10
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_settings_block_is_a_record_type_beside_the_entities() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let settings = file.entities.iter().find(|e| e.settings).expect("the settings, read as a record type");
    assert_eq!(settings.name.name, "Settings");
    let types: Vec<(&str, &FieldType)> = settings.fields.iter().map(|f| (f.name.name.as_str(), &f.ty)).collect();
    assert_eq!(types[0], ("language", &FieldType::Language));
    assert_eq!(types[1], ("theme", &FieldType::Theme));
    assert_eq!(types[3], ("pageSize", &FieldType::Integer));
    assert!(file.entities.iter().filter(|e| !e.settings).all(|e| e.name.name == "Customer"));
    assert!(file.pages.iter().all(|p| p.data.iter().all(|d| d.name.name != "settings")), "the parser reads only what is written");
}

#[test]
fn every_window_has_its_settings_and_code_reads_and_sets_them() {
    let p = project(SHOP, CODE);
    let found: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(found.is_empty(), "{found:#?}");
    for page in &p.files[0].pages {
        let settings = page.data.iter().find(|d| d.name.name == "settings").unwrap_or_else(|| panic!("{} has its settings", page.name.name));
        assert!(settings.made, "made by Clarity, not written");
        assert!(matches!(&settings.of, DataKind::Record { entity } if entity.name == "Settings"));
    }
    assert!(widgets::accepts(WidgetKind::Dropdown, &FieldType::Language) && widgets::accepts(WidgetKind::Dropdown, &FieldType::Theme));
    assert!(!widgets::accepts(WidgetKind::Textbox, &FieldType::Language));
    let en = &p.translations["en"];
    assert!(en.values().any(|v| v == "Show the toolbar"), "a setting's label is a key: {en:?}");
    let wrong = errors(SHOP, &CODE.replace("settings.pageSize - 10", "\"ten\""));
    assert!(wrong.iter().any(|m| m.contains("cannot set integer to text")), "{wrong:#?}");
}

#[test]
fn the_settings_are_never_in_the_database() {
    let p = project(SHOP, CODE);
    let pending = clarity_spec::migrate::pending(&p, "main", &[]);
    let text = format!("{pending:?}");
    assert!(text.contains("Customer") && !text.contains("Settings") && !text.contains("pageSize"), "{text}");
}

#[test]
fn the_rules_of_settings() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page, CODE);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(format!("{SHOP}\nsettings\n    other: text\n"), "a project has one 'settings' block");
    says(SHOP.replace("    density: Density, default roomy", "    density: Density, default roomy\n    favourite: ref Customer"), "a setting is a plain value; 'favourite' cannot be ref Customer");
    says(SHOP.replace("entity Customer\n", "entity Settings\n    note: text\n\nentity Customer\n"), "'Settings' is the name of the project's settings");
    says(SHOP.replace("    data customer: Customer\n", "    data customer: Customer\n    data settings: Customer\n"), "every window has 'settings' of its own");
    says(SHOP.replace("    data customer: Customer\n", "    data customer: Customer\n    data mine: Settings\n"), "every window has the settings as 'settings'; bind that");
    says(SHOP.replace("    pageSize: integer, min 10, max 200, default 50\n", "    pageSize: integer, min 10, max 200, default 50\n    language\n"), "duplicate field 'language'");
    let without = SHOP.replace("settings\n    language\n    theme\n    showToolbar: bool, default true, label \"Show the toolbar\"\n    pageSize: integer, min 10, max 200, default 50\n    density: Density, default roomy\n\n", "");
    says(without.clone(), "unknown data 'settings'");
}

#[test]
fn the_settings_window_template_writes_a_form_of_every_setting() {
    use clarity_spec::templates::{PageTemplate, page_from_template};
    let p = project(SHOP, CODE);
    let page = page_from_template(&p, "Options", &PageTemplate::Settings).unwrap();
    assert!(page.starts_with("page Options: dialog\n"), "{page}");
    for line in [
        "    form name settingsForm at 1,1 span 12 rows 5 bind settings labels 3",
        "        dropdown name languageBox at 4,1 span 6 bind settings.language",
        "        dropdown name themeBox at 4,2 span 6 bind settings.theme",
        "        checkbox @settingsShowToolbarLabel name showToolbarBox at 4,3 span 6 bind settings.showToolbar",
        "        spinbox name pageSizeBox at 4,4 span 3 bind settings.pageSize",
        "        dropdown name densityBox at 4,5 span 4 bind settings.density",
        "    button \"Close\" name closeButton at 11,7 span 2 cancel on click -> close",
    ] {
        assert!(page.lines().any(|l| l == line), "missing {line:?} in\n{page}");
    }
    // The window it writes checks clean in the project.
    let text = format!("{SHOP}\n{page}\n{CODE}");
    let found: Vec<String> = load_keyed([("shop.clarity", text.as_str())], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect();
    assert!(found.is_empty(), "{found:#?}");
    let none = project(&SHOP.replace("settings\n    language\n    theme\n    showToolbar: bool, default true, label \"Show the toolbar\"\n    pageSize: integer, min 10, max 200, default 50\n    density: Density, default roomy\n\n", ""), "");
    assert!(page_from_template(&none, "Options", &PageTemplate::Settings).is_err(), "without settings there is nothing to show");
}

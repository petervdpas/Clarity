//! Translations (I1c): every text a person sees is collected from the
//! project, and a file per language gives its translation.

use clarity_spec::translations::{self, Used};
use clarity_spec::{Project, Severity, load_sources};

const SHOP: &str = "module shop

enum Kind
    Private label \"Private person\"
    Business

entity Customer
    name: text(80), help \"As on the invoice\"
    kind: Kind

app Shop
    title \"The shop\"
    main Card
    menu
        item \"File\"
            item \"Quit\" on click -> quit
    footer
        status \"Ready to sell\"

page Card: window
    title \"Customer card\"
    data customer: Customer
    tabs name tabs1 at 1,1 span 12 rows 3
        tab \"General\"
            textbox name nameInput at 1,1 span 4 bind customer.name placeholder \"Full name\" tooltip \"Who buys\"
    button \"Save\" name save at 1,4 span 2 on click: saved

code Card
    handler saved
        message \"Saved {customer.name}\"
        save.caption = \"Saved\"
        nameInput.error = \"Taken\"

action check(customer: Customer)
    refuse \"Not allowed\"
";

const NL: &str = "// Dutch.
translation nl
    \"Customer card\" \"Klantkaart\"
    \"Saved {customer.name}\" \"{customer.name} opgeslagen\"
";

fn project_with(files: &[(&str, &str)]) -> Project {
    let mut project = load_sources([("shop.clarity", SHOP)]);
    translations::attach(&mut project, files.iter().copied());
    project
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

#[test]
fn every_text_a_person_sees_is_collected() {
    let project = load_sources([("shop.clarity", SHOP)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let used = translations::texts(&project);
    let texts: Vec<&str> = used.iter().map(|u| u.text.as_str()).collect();
    for text in [
        // The dictionary: labels, derived or written, and help texts.
        "Name", "Kind", "As on the invoice", "Private person", "Business",
        // Windows and the app.
        "The shop", "Customer card", "General", "Full name", "Who buys", "Save", "File", "Quit", "Ready to sell",
        // Code: messages with their slots, captions, errors and refusals.
        "Saved {customer.name}", "Saved", "Taken", "Not allowed",
        // The renderer's own words.
        "Yes", "No",
    ] {
        assert!(texts.contains(&text), "{text} is not collected: {texts:?}");
    }
    // Each text once, with where it is used.
    let save: &Used = used.iter().find(|u| u.text == "Save").unwrap();
    assert_eq!(save.places, ["window Card"]);
    assert_eq!(texts.iter().filter(|t| **t == "Name").count(), 1);
}

#[test]
fn a_translation_file_gives_texts_in_its_language() {
    let project = project_with(&[("translations/nl.clarity", NL)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.translations["nl"]["Customer card"], "Klantkaart");
    assert_eq!(project.translations["nl"]["Saved {customer.name}"], "{customer.name} opgeslagen");
}

#[test]
fn a_translation_keeps_the_slots_of_its_text() {
    let lost = NL.replace("\"{customer.name} opgeslagen\"", "\"Opgeslagen\"");
    let extra = NL.replace("\"{customer.name} opgeslagen\"", "\"{customer.name} {klant} opgeslagen\"");
    assert_eq!(messages(&project_with(&[("translations/nl.clarity", &lost)]), Severity::Error), [
        "the translation of \"Saved {customer.name}\" must keep the slot {customer.name}"
    ]);
    assert_eq!(messages(&project_with(&[("translations/nl.clarity", &extra)]), Severity::Error), [
        "the translation of \"Saved {customer.name}\" has a slot {klant} that the text does not have"
    ]);
}

#[test]
fn translation_files_are_checked() {
    let unused = NL.to_string() + "    \"Gone\" \"Weg\"\n";
    assert_eq!(messages(&project_with(&[("translations/nl.clarity", &unused)]), Severity::Warning), [
        "\"Gone\" is no longer shown anywhere; its translation can go"
    ]);
    let twice = NL.to_string() + "    \"Customer card\" \"Kaart\"\n";
    assert_eq!(messages(&project_with(&[("translations/nl.clarity", &twice)]), Severity::Error), [
        "\"Customer card\" is translated twice"
    ]);
    assert_eq!(messages(&project_with(&[("translations/de.clarity", NL)]), Severity::Error), [
        "this file is for language 'de' (its name), but it says 'nl'"
    ]);
    let wrong = NL.replace("    \"Customer card\" \"Klantkaart\"", "    \"Customer card\" Klantkaart");
    assert_eq!(messages(&project_with(&[("translations/nl.clarity", &wrong)]), Severity::Error), [
        "write a text and its translation, each in quotes: \"Save\" \"Opslaan\""
    ]);
    assert_eq!(messages(&project_with(&[("translations/xx-y.clarity", "translation xx-y\n")]), Severity::Error), [
        "'xx-y' is not a language tag; write one like en, nl or nl-NL"
    ]);
}

#[test]
fn a_translation_is_set_changed_and_removed_in_the_file() {
    let added = translations::set(Some(NL), "nl", "Save", "Opslaan");
    assert_eq!(added, NL.to_string() + "    \"Save\" \"Opslaan\"\n");
    let changed = translations::set(Some(&added), "nl", "Customer card", "Kaart van de klant");
    assert!(changed.starts_with("// Dutch.\ntranslation nl\n    \"Customer card\" \"Kaart van de klant\"\n"), "{changed}");
    let removed = translations::set(Some(&changed), "nl", "Save", "");
    assert!(!removed.contains("Opslaan"), "{removed}");
    assert_eq!(translations::set(None, "de", "Save", "Speichern"), "translation de\n    \"Save\" \"Speichern\"\n");
}

#[test]
fn a_project_folder_brings_its_translations() {
    let dir = std::env::temp_dir().join(format!("clarity-translations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("shop.clarity"), SHOP).unwrap();
    std::fs::write(dir.join("translations/nl.clarity"), NL).unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.translations["nl"]["Customer card"], "Klantkaart");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_translations_table_has_a_column_per_language() {
    let de = "translation de\n    \"Save\" \"Speichern\"\n";
    let project = project_with(&[("translations/de.clarity", de), ("translations/nl.clarity", NL)]);
    let table = translations::table(&project);
    assert_eq!(table.languages, ["de", "nl"]);
    let card = table.rows.iter().find(|r| r.text == "Customer card").unwrap();
    assert_eq!(card.places, ["window Card"]);
    assert_eq!(card.translations, [None, Some("Klantkaart".to_string())]);
    // What each language still misses.
    assert_eq!(table.missing, [table.rows.len() - 1, table.rows.len() - 2]);
}

#[test]
fn the_example_translation_fits_the_example() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop"));
    let project = clarity_spec::load_dir(dir).unwrap();
    let in_translations: Vec<String> = project
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("translations"))
        .map(|d| d.message.clone())
        .collect();
    assert!(in_translations.is_empty(), "{in_translations:#?}");
    assert_eq!(project.translations["nl"]["Customers"], "Klanten");
    // The example shows the whole application in Dutch.
    let table = clarity_spec::translations::table(&project);
    let nl = table.languages.iter().position(|l| l == "nl").unwrap();
    let untranslated: Vec<&str> = table.rows.iter().filter(|r| r.translations[nl].is_none()).map(|r| r.text.as_str()).collect();
    assert!(untranslated.is_empty(), "not in nl.clarity: {untranslated:?}");
}

#[test]
fn a_menu_item_can_switch_the_language() {
    let text = SHOP.replace("            item \"Quit\" on click -> quit", "            item \"Language\" on click -> language\n            item \"Quit\" on click -> quit");
    let project = load_sources([("shop.clarity", text.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(clarity_spec::Preview::new(project).commands.contains(&"language".to_string()));
}

#[test]
fn the_records_example_is_translated_to_german() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/records"));
    let project = clarity_spec::load_dir(dir).unwrap();
    let problems: Vec<String> = project.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(problems.is_empty(), "{problems:#?}");
    let table = clarity_spec::translations::table(&project);
    let de = table.languages.iter().position(|l| l == "de").expect("a German translation");
    let untranslated: Vec<&str> = table.rows.iter().filter(|r| r.translations[de].is_none()).map(|r| r.text.as_str()).collect();
    assert!(untranslated.is_empty(), "not in de.clarity: {untranslated:#?}");
}

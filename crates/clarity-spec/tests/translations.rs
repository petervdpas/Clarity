//! Translation files (I1c): a file per language gives every key its value,
//! read, checked and written by key. What design and code name by key is in
//! tests/keys.rs.

use clarity_spec::model::Locale;
use clarity_spec::translations;
use clarity_spec::{Project, Severity, load_sources};

const SHOP: &str = "module shop

app Shop
    title @shopTitle
    main Card
    menu name mainMenu
        item @fileItem name fileItem
            item @quitItem name quitItem on click -> quit

page Card: window
    title @cardTitle
    button @saveCaption name save at 1,1 span 2 on click: saveClick

code Card
    handler saveClick
        message @savedOrder(number = 7)
";

const EN: &str = "// English, the default.
translation en
    shopTitle \"The shop\"
    fileItem \"File\"
    quitItem \"Quit\"
    cardTitle \"Customer card\"
    saveCaption \"Save\"
    savedOrder \"Saved order {number}\"
    yes \"Yes\"
    no \"No\"
";

const NL: &str = "// Dutch.
translation nl
    shopTitle \"De winkel\"
    fileItem \"Bestand\"
    quitItem \"Afsluiten\"
    cardTitle \"Klantkaart\"
    saveCaption \"\"
    savedOrder \"Bestelling {number} opgeslagen\"
    yes \"Ja\"
    no \"Nee\"
";

fn project_with(files: &[(&str, &str)]) -> Project {
    let mut project = load_sources([("shop.clarity", SHOP)]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    translations::attach(&mut project, files.iter().copied());
    project
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

#[test]
fn a_language_file_gives_every_key_a_value() {
    let project = project_with(&[("translations/en.clarity", EN), ("translations/nl.clarity", NL)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.translations["nl"]["cardTitle"], "Klantkaart");
    assert_eq!(project.translations["nl"]["savedOrder"], "Bestelling {number} opgeslagen");
    // Left empty: shows the default's until it is translated.
    assert_eq!(project.translations["nl"]["saveCaption"], "");
}

#[test]
fn language_files_are_checked() {
    let errors = |nl: &str| messages(&project_with(&[("translations/en.clarity", EN), ("translations/nl.clarity", nl)]), Severity::Error);
    let twice = NL.to_string() + "    cardTitle \"Kaart\"\n";
    assert_eq!(errors(&twice), ["key 'cardTitle' is in this file twice"]);
    let wrong = NL.replace("    cardTitle \"Klantkaart\"", "    \"Customer card\" \"Klantkaart\"");
    assert!(errors(&wrong).contains(&"write a key and its text in quotes, saveCaption \"Save\", or the resource it points at, gameRules resource \"rules.md\"".to_string()), "{:#?}", errors(&wrong));
    assert_eq!(messages(&project_with(&[("translations/en.clarity", EN), ("translations/de.clarity", NL)]), Severity::Error), [
        "this file is for language 'de' (its name), but it says 'nl'"
    ]);
    let lost = NL.replace("Bestelling {number} opgeslagen", "Opgeslagen");
    assert_eq!(errors(&lost), ["savedOrder in nl must keep the slot {number}"]);
    let extra = NL.replace("Bestelling {number} opgeslagen", "Bestelling {number} {klant}");
    assert_eq!(errors(&extra), ["savedOrder in nl has a slot {klant} that en does not have"]);
    assert!(
        messages(&project_with(&[("translations/en.clarity", EN), ("translations/xx-y.clarity", "translation xx-y\n")]), Severity::Error)
            .contains(&"'xx-y' is not a language tag; write one like en, nl or nl-NL".to_string())
    );
}

#[test]
fn a_value_is_set_changed_and_emptied_in_the_file() {
    let added = translations::set(Some(NL), "nl", "newKey", "Nieuw");
    assert_eq!(added, NL.to_string() + "    newKey \"Nieuw\"\n");
    let changed = translations::set(Some(&added), "nl", "shopTitle", "Onze winkel");
    assert!(changed.starts_with("// Dutch.\ntranslation nl\n    shopTitle \"Onze winkel\"\n"), "{changed}");
    // Emptied, the key stays: every language file has the same keys.
    let emptied = translations::set(Some(&changed), "nl", "newKey", "");
    assert!(emptied.contains("    newKey \"\"\n"), "{emptied}");
    assert_eq!(translations::set(None, "de", "saveCaption", "Speichern"), "translation de\n    saveCaption \"Speichern\"\n");
}

/// A key's value is text, or a reference to a file of the resources
/// folder: `gameRules resource "rules-nl.md"`. Which file each language
/// shows is up to its language file.
#[test]
fn a_value_is_text_or_a_resource() {
    let (parsed, problems) = translations::parse("translation nl\n    gameTitle \"Boter, kaas en eieren\"\n    gameRules resource \"rules-nl.md\"\n");
    assert!(problems.is_empty(), "{problems:#?}");
    assert_eq!(parsed.entries[0].value, translations::Value::Text("Boter, kaas en eieren".into()));
    assert_eq!(parsed.entries[1].value, translations::Value::Resource("rules-nl.md".into()));
    let (_, problems) = translations::parse("translation nl\n    gameRules resource\n");
    assert!(!problems.is_empty());

    // Written back as it was given: a resource replaces a text, and the other way round.
    let file = "translation nl\n    gameRules \"x\"\n";
    let pointed = translations::set_value(Some(file), "nl", "gameRules", &translations::Value::Resource("rules-nl.md".into()));
    assert_eq!(pointed, "translation nl\n    gameRules resource \"rules-nl.md\"\n");
    assert_eq!(translations::set(Some(&pointed), "nl", "gameRules", "Spelregels"), "translation nl\n    gameRules \"Spelregels\"\n");

    // The project keeps the resource each language points at, apart from its texts.
    let mut project = load_sources([("shop.clarity", SHOP)]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    let en = format!("{EN}    gameRules resource \"rules-en.md\"\n");
    let nl = format!("{NL}    gameRules resource \"rules-nl.md\"\n");
    translations::attach(&mut project, [("translations/en.clarity", en.as_str()), ("translations/nl.clarity", nl.as_str())]);
    assert_eq!(project.resource_keys["nl"]["gameRules"], "rules-nl.md");
    assert!(!project.translations["nl"].contains_key("gameRules"));
}

#[test]
fn a_project_folder_brings_its_languages() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/clarity-translations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("shop.clarity"), SHOP).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), EN).unwrap();
    std::fs::write(dir.join("translations/nl.clarity"), NL).unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.translations["nl"]["cardTitle"], "Klantkaart");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_languages_table_has_a_column_per_language_the_default_first() {
    let de = EN.replace("translation en", "translation de").replace("\"Customer card\"", "\"Kundenkarte\"").replace("// English, the default.\n", "");
    let project = project_with(&[("translations/de.clarity", &de), ("translations/en.clarity", EN), ("translations/nl.clarity", NL)]);
    let table = translations::table(&project);
    assert_eq!(table.default, "en");
    assert_eq!(table.languages, ["en", "de", "nl"]);
    let card = table.rows.iter().find(|r| r.key == "cardTitle").unwrap();
    assert_eq!(card.places, ["window Card"]);
    assert_eq!(card.values, [Some("Customer card".to_string()), Some("Kundenkarte".to_string()), Some("Klantkaart".to_string())]);
    // What each language leaves empty.
    assert_eq!(table.missing, [0, 0, 1]);
}

/// A key a markdown or an image shows points each language at a file of
/// the resources: the table says which kind of value a key takes, and the
/// file each language points it at.
#[test]
fn the_languages_table_knows_the_keys_that_point_at_resources() {
    let source = format!("{SHOP}\npage Rules: window\n    title @cardTitle\n    markdown @gameRules name rulesText at 1,1 span 6 rows 6\n    image @bannerPicture name banner at 1,7 span 6 rows 2\n");
    let mut project = load_sources([("shop.clarity", source.as_str())]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    let en = format!("{EN}    gameRules resource \"rules-en.md\"\n    bannerPicture resource \"banner.png\"\n");
    let nl = format!("{NL}    gameRules resource \"rules-nl.md\"\n    bannerPicture \"\"\n");
    translations::attach(&mut project, [("translations/en.clarity", en.as_str()), ("translations/nl.clarity", nl.as_str())]);
    let table = translations::table(&project);
    let row = |key: &str| table.rows.iter().find(|r| r.key == key).unwrap().clone();
    assert_eq!(row("cardTitle").takes, translations::Takes::Text);
    assert_eq!(row("gameRules").takes, translations::Takes::Markdown);
    assert_eq!(row("bannerPicture").takes, translations::Takes::Picture);
    assert_eq!(row("gameRules").resources, [Some("rules-en.md".to_string()), Some("rules-nl.md".to_string())]);
    assert_eq!(row("gameRules").values, [None, None]);
    assert_eq!(row("cardTitle").resources, [None, None]);
    // A key that points at a file has its value; one left empty is missing.
    assert_eq!(table.missing, [0, 2]);
}

/// A text without letters, such as a calculator's `7` or `+`, reads the same
/// in every language: it may be written as it is.
#[test]
fn a_text_without_letters_needs_no_key() {
    assert!(!translations::has_letters("7"));
    assert!(!translations::has_letters("+/-"));
    assert!(!translations::has_letters("÷"));
    assert!(translations::has_letters("C"));
    assert!(!translations::has_letters("{name}"));
    let calc = "module calc\n\npage Calc: window\n    button \"7\" name seven at 1,1\n";
    let mut project = load_sources([("calc.clarity", calc)]);
    translations::attach(&mut project, [("translations/en.clarity", "translation en\n    yes \"Yes\"\n    no \"No\"\n")]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn a_menu_item_can_switch_the_language() {
    let text = SHOP.replace("            item @quitItem name quitItem on click -> quit", "            item @languageItem name languageItem on click -> language");
    let project = load_sources([("shop.clarity", text.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(clarity_spec::Preview::new(project).commands.contains(&"language".to_string()));
}

/// Every example's languages are complete: each key has a value in each.
#[test]
fn every_example_is_complete_in_every_language() {
    let examples = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples"));
    for name in ["calculator", "dial", "noughts", "records", "shop", "todo"] {
        let project = clarity_spec::load_dir(&examples.join(name)).unwrap();
        let problems: Vec<String> = project.diagnostics.iter().map(|d| format!("{}: {}", d.path, d.message)).collect();
        assert!(problems.is_empty(), "{name}: {problems:#?}");
        let table = translations::table(&project);
        assert!(table.languages.len() >= 2, "{name} has a language besides its default");
        for (language, missing) in table.languages.iter().zip(&table.missing) {
            assert_eq!(*missing, 0, "{name}: {language} leaves keys empty");
        }
    }
}

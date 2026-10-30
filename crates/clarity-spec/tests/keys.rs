//! Texts by key (I1c): design and code name a text by its key, `@saveCaption`,
//! and a file per language gives each key its value. Every language file has
//! the same keys. The default language, the project's own, is such a file too
//! and gives every key a value; another language may leave one empty, which
//! shows the default's until it is translated.

use clarity_spec::model::Locale;
use clarity_spec::parser::parse;
use clarity_spec::translations;
use clarity_spec::{Project, Severity, load_sources};

const SHOP: &str = "module shop

entity Customer
    name: text(80), label @customerName, help @customerNameHelp

page Card: window
    title @cardTitle
    data customer: Customer
    data note: text, default @noteStart
    textbox name nameInput at 1,1 span 4 bind customer.name placeholder @fullName
    button @saveCaption name save at 1,2 span 2 on click: saveClick
    button \"7\" name seven at 3,2 span 1

code Card
    handler saveClick
        message @savedCustomer(name = customer.name)
        save.caption = @savedCaption
        note = @checkedBy(name = customer.name)
";

const EN: &str = "translation en
    customerName \"Name\"
    customerNameHelp \"As on the invoice\"
    cardTitle \"Customer card\"
    noteStart \"Ask for the invoice address\"
    fullName \"Full name\"
    saveCaption \"Save\"
    savedCustomer \"Saved {name}\"
    savedCaption \"Saved\"
    checkedBy \"Checked by {name}\"
    yes \"Yes\"
    no \"No\"
    ok \"OK\"
    cancel \"Cancel\"
    dismiss \"Dismiss\"
    undoChangedSince \"This was changed since, so it cannot be taken back\"
";

const NL: &str = "translation nl
    customerName \"Naam\"
    customerNameHelp \"\"
    cardTitle \"Klantkaart\"
    noteStart \"\"
    fullName \"\"
    saveCaption \"Opslaan\"
    savedCustomer \"{name} opgeslagen\"
    savedCaption \"\"
    checkedBy \"\"
    yes \"Ja\"
    no \"Nee\"
    ok \"OK\"
    cancel \"Annuleren\"
    dismiss \"Sluiten\"
    undoChangedSince \"Dit is intussen gewijzigd en kan daarom niet ongedaan worden gemaakt\"
";

fn project(source: &str, files: &[(&str, &str)]) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    translations::attach(&mut project, files.iter().copied());
    project
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn design_and_code_name_texts_by_key() {
    let (file, diags) = parse(SHOP, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let page = &file.pages[0];
    assert_eq!(page.title.as_deref(), Some("@cardTitle"));
    assert_eq!(page.widgets[1].props.caption.as_deref(), Some("@saveCaption"));
    assert_eq!(page.widgets[0].props.placeholder.as_deref(), Some("@fullName"));
    // A text without letters needs no key.
    assert_eq!(page.widgets[2].props.caption.as_deref(), Some("7"));
    let p = project(SHOP, &[("translations/en.clarity", EN), ("translations/nl.clarity", NL)]);
    assert!(messages(&p, Severity::Error).is_empty(), "{:#?}", p.diagnostics);
    assert!(messages(&p, Severity::Warning).is_empty(), "{:#?}", p.diagnostics);
    assert_eq!(p.translations["nl"]["saveCaption"], "Opslaan");
    assert_eq!(p.translations["en"]["saveCaption"], "Save");
}

/// Every key the project uses, with where.
#[test]
fn the_keys_of_a_project_are_collected() {
    let p = project(SHOP, &[("translations/en.clarity", EN)]);
    let keys = translations::keys(&p);
    let names: Vec<&str> = keys.iter().map(|k| k.key.as_str()).collect();
    for key in ["customerName", "customerNameHelp", "cardTitle", "noteStart", "fullName", "saveCaption", "savedCustomer", "savedCaption", "checkedBy", "yes", "no"] {
        assert!(names.contains(&key), "{key} in {names:?}");
    }
    assert_eq!(keys.iter().find(|k| k.key == "saveCaption").unwrap().places, ["window Card"]);
}

#[test]
fn the_default_language_gives_every_key_a_value() {
    let errors = |en: &str| messages(&project(SHOP, &[("translations/en.clarity", en)]), Severity::Error);
    assert_has(&errors(&EN.replace("    saveCaption \"Save\"\n", "")), "key 'saveCaption' has no value in en, the default language");
    assert_has(&errors(&EN.replace("saveCaption \"Save\"", "saveCaption \"\"")), "key 'saveCaption' has no value in en, the default language");
    assert_has(&messages(&project(SHOP, &[]), Severity::Error), "the default language en has no file; add translations/en.clarity");
    assert!(errors(EN).is_empty());
}

/// Every language file has the same keys as the default's; another language
/// may leave a value empty until it is translated.
#[test]
fn every_language_file_has_the_same_keys() {
    let errors = |nl: &str| messages(&project(SHOP, &[("translations/en.clarity", EN), ("translations/nl.clarity", nl)]), Severity::Error);
    assert!(errors(NL).is_empty(), "{:#?}", errors(NL));
    assert_has(&errors(&NL.replace("    fullName \"\"\n", "")), "key 'fullName' is missing in nl; every language file has the same keys");
    assert_has(&errors(&format!("{NL}    extraText \"Meer\"\n")), "key 'extraText' is in nl but not in en, the default language");
}

#[test]
fn a_key_takes_the_slots_its_default_value_has() {
    let errors = |source: &str| messages(&project(source, &[("translations/en.clarity", EN)]), Severity::Error);
    assert_has(&errors(&SHOP.replace("@savedCustomer(name = customer.name)", "@savedCustomer")), "@savedCustomer needs a value for {name}");
    assert_has(&errors(&SHOP.replace("@savedCustomer(name = customer.name)", "@savedCustomer(name = customer.name, city = customer.name)")), "@savedCustomer has no slot {city}");
    let nl = messages(&project(SHOP, &[("translations/en.clarity", EN), ("translations/nl.clarity", &NL.replace("{name} opgeslagen", "opgeslagen"))]), Severity::Error);
    assert_has(&nl, "savedCustomer in nl must keep the slot {name}");
}

#[test]
fn a_text_people_read_is_a_key() {
    let errors = |source: &str| messages(&project(source, &[("translations/en.clarity", EN)]), Severity::Error);
    assert_has(&errors(&SHOP.replace("button @saveCaption", "button \"Save\"")), "\"Save\" is read by people, so it is a key: write @key");
    assert_has(&errors(&SHOP.replace("message @savedCustomer(name = customer.name)", "message \"Saved\"")), "\"Saved\" is read by people, so it is a key: write @key");
    assert_has(&errors(&SHOP.replace("@unknownKey", "@saveCaption").replace("title @cardTitle", "title @nowhere")), "key 'nowhere' has no value in en, the default language");
    // A key is camelCase, as names are.
    let bad = EN.replace("fullName \"Full name\"", "full_name \"Full name\"");
    assert_has(&messages(&project(&SHOP.replace("@fullName", "@full_name"), &[("translations/en.clarity", &bad)]), Severity::Error), "key 'full_name' is not camelCase");
}

#[test]
fn a_key_nobody_uses_is_a_warning() {
    let en = format!("{EN}    oldText \"Gone\"\n");
    let warnings = messages(&project(SHOP, &[("translations/en.clarity", &en)]), Severity::Warning);
    assert_has(&warnings, "key 'oldText' is not used anywhere; it can go");
}

#[test]
fn clarity_s_own_word_missing_from_a_language_file_shows_in_english_with_a_warning() {
    let en = EN.replace("    ok \"OK\"\n", "");
    let nl = NL.replace("    ok \"OK\"\n", "");
    let p = project(SHOP, &[("translations/en.clarity", en.as_str()), ("translations/nl.clarity", nl.as_str())]);
    assert!(p.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", p.diagnostics);
    assert!(p.diagnostics.iter().any(|d| d.message == "Clarity's word 'ok' is not in en yet; it shows \"OK\" until it is"), "{:#?}", p.diagnostics);
}

//! Make texts keys: a project written before texts had keys gets them. Each
//! text people read becomes a key with its text as the default language's
//! value, and translation files that translated texts by what they said
//! give the same keys their values.

use clarity_spec::keyed;

const SHOP: &str = "module shop

enum Kind
    Private label \"Private person\"
    Business

entity Customer
    name: text(80), label \"Full name\", help \"As on the invoice\"
    city: text(40)
    kind: Kind

app Shop
    title \"The shop\"
    main Card
    menu name mainMenu
        item \"Quit\" name quitItem on click -> quit

page Card: window
    title \"Customer card\"
    data customer: Customer
    data note: text, default \"Ask for the address\"
    label for nameInput name nameLabel at 1,1
    textbox name nameInput at 4,1 span 4 bind customer.name placeholder \"Full name\"
    button \"Save\" name save at 1,2 span 2 on click: saveClick
    button \"7\" name seven at 3,2

code Card
    handler saveClick
        message \"Saved {customer.name}\"
        save.caption = \"Saved\"

action check(customer: Customer)
    refuse \"Not allowed\"
";

const NL: &str = "translation nl
    \"Customer card\" \"Klantkaart\"
    \"Saved {customer.name}\" \"{customer.name} opgeslagen\"
    \"Private person\" \"Particulier\"
    \"Yes\" \"Ja\"
";

#[test]
fn texts_become_keys_with_their_values() {
    let made = keyed::make_keys(&[("shop.clarity", SHOP)], &[("translations/nl.clarity", NL)], "en");
    let shop = &made.modules[0].1;
    for line in [
        "    Private label @kindPrivate\n",
        "    Business label @kindBusiness\n",
        "    name: text(80), label @customerNameLabel, help @customerNameHelp\n",
        "    city: text(40), label @customerCityLabel\n",
        "    title @shopTitle\n",
        "        item @quitItem name quitItem on click -> quit\n",
        "    title @cardTitle\n",
        "    data note: text, default @cardNote\n",
        "    textbox name nameInput at 4,1 span 4 bind customer.name placeholder @nameInputPlaceholder\n",
        "    button @saveCaption name save at 1,2 span 2 on click: saveClick\n",
        // A text without letters stays as it is.
        "    button \"7\" name seven at 3,2\n",
        // A text with slots fills them by name.
        "        message @savedName(name = customer.name)\n",
        "        save.caption = @saved\n",
        "    refuse @notAllowed\n",
    ] {
        assert!(shop.contains(line), "{line:?} in\n{shop}");
    }
    let en = &made.languages["en"];
    assert_eq!(en["cardTitle"], "Customer card");
    assert_eq!(en["savedName"], "Saved {name}");
    assert_eq!(en["customerCityLabel"], "City");
    assert_eq!(en["kindBusiness"], "Business");
    assert_eq!(en["yes"], "Yes");
    // The translation of a text goes to its key, its slots renamed alike.
    let nl = &made.languages["nl"];
    assert_eq!(nl["cardTitle"], "Klantkaart");
    assert_eq!(nl["savedName"], "{name} opgeslagen");
    assert_eq!(nl["kindPrivate"], "Particulier");
    assert_eq!(nl["yes"], "Ja");
    // Every language has the same keys; one not translated yet is empty.
    assert_eq!(nl["saveCaption"], "");
    assert_eq!(en.keys().collect::<Vec<_>>(), nl.keys().collect::<Vec<_>>());
}

/// The project with its keys checks clean against the files made for it.
#[test]
fn a_project_made_keyed_checks_clean() {
    let made = keyed::make_keys(&[("shop.clarity", SHOP)], &[("translations/nl.clarity", NL)], "en");
    let mut project = clarity_spec::load_sources(made.modules.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    project.locale.language = Some("en".into());
    let files: Vec<(String, String)> = made.languages.keys().map(|l| (format!("translations/{l}.clarity"), made.language_file(l))).collect();
    clarity_spec::translations::attach(&mut project, files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    assert!(project.diagnostics.is_empty(), "{:#?}\n{}", project.diagnostics, made.modules[0].1);
}

/// Renaming a key renames it where design and code name it, and in every
/// language file; a text that only says the key's name stays as it is.
#[test]
fn a_key_is_renamed_everywhere() {
    let module = "page Card: window\n    title @cardTitle\n    button @saveCaption name save at 1,1 on click: saveClick\n\ncode Card\n    handler saveClick\n        message @saveCaption\n        save.caption = \"saveCaption\"\n";
    let renamed = keyed::rename_key_in_module(module, "saveCaption", "storeCaption");
    assert!(renamed.contains("    button @storeCaption name save"), "{renamed}");
    assert!(renamed.contains("        message @storeCaption\n"), "{renamed}");
    assert!(renamed.contains("save.caption = \"saveCaption\""), "{renamed}");
    let en = "// English.\ntranslation en\n    cardTitle \"Card\"\n    saveCaption \"Save\"\n";
    assert_eq!(keyed::rename_key_in_language(en, "saveCaption", "storeCaption"), "// English.\ntranslation en\n    cardTitle \"Card\"\n    storeCaption \"Save\"\n");
}

/// Only the texts people read become keys, as the checker tells them: text
/// a function returns to build a file, such as Mermaid's arrows, and text
/// a local holds stay as they are, since translating them would break what
/// they build.
#[test]
fn only_the_texts_people_read_become_keys() {
    let source = "module diagram

page Main: window
    data shown: text
    button \"Draw\" name draw at 1,1 span 2 on click: drawClick

code Main
    handler drawClick
        shown = \"Drawn\"
        message \"Drew {shown}\"

function arrow(dashed: bool) -> text
    if dashed
        return \"..>\"
    return \"-->\"

action file() -> text
    let line = \"classDiagram\"
    line = \"has\"
    return line + arrow(true)
";
    let made = keyed::make_keys(&[("diagram.clarity", source)], &[], "en");
    let module = &made.modules[0].1;
    for kept in ["\"..>\"", "\"-->\"", "\"classDiagram\"", "line = \"has\""] {
        assert!(module.contains(kept), "{kept} stays as it is:\n{module}");
    }
    for keyed in ["button @drawCaption", "message @drewShown(shown = shown)"] {
        assert!(module.contains(keyed), "{keyed}:\n{module}");
    }
}

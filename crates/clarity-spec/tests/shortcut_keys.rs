//! The commands a person can give keys in Clarity's keys window (DESIGN
//! 5.28): the app's frame once, under Everywhere, then each window's own
//! frame commands and named buttons, with the keys the design gives them.

use clarity_spec::shortcuts::{KeyCommand, KeyLayer, catalog};
use clarity_spec::load_sources;

const SHOP: &str = "module shop

entity Customer
    name: text

app Shop
    main CustomerCard
    menu name mainMenu
        item \"File\" name fileItem
            item \"New\" name newItem key \"Ctrl+N\" on click -> CustomerCard
            separator
            item \"Keys\" name keysItem on click -> keys
            item \"Quit\" name quitItem key \"Ctrl+Q\" on click -> quit
    ribbon name mainRibbon
        tab \"Home\" name homeTab
            group \"Edit\" name editGroup
                button \"Copy\" name copyButton key \"Ctrl+Shift+C\" on click: copyClicked

page CustomerCard: window
    title \"Customer\"
    toolbar name cardToolbar
        button \"Print\" name printButton on click: printClicked
    button \"Save\" name saveButton at 1,1 span 2 key \"Ctrl+S\" on click: saveClicked
    label \"Name\" name nameLabel at 3,1 span 2
    panel name more at 1,2 span 6 rows 2
        button \"Close\" name closeButton at 1,1 span 2 on click -> close

page Ask: dialog
    title \"Sure?\"
    button \"Yes\" name yesButton at 1,1 span 2 default on click -> close

code Shop
    handler copyClicked
        message \"copied\"

code CustomerCard
    handler printClicked
        message \"printed\"
    handler saveClicked
        message \"saved\"
";

fn command(id: &str, name: &str, caption: &str, keys: &[&str]) -> KeyCommand {
    KeyCommand { id: id.into(), name: name.into(), caption: Some(caption.into()), keys: keys.iter().map(|k| k.to_string()).collect() }
}

#[test]
fn every_named_command_that_does_something_is_listed_by_where_it_is_active() {
    let project = load_sources([("shop.clarity", SHOP)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(
        catalog(&project),
        vec![
            // The app's frame: the submenu item that only opens its submenu is no command.
            KeyLayer {
                id: String::new(),
                title: None,
                commands: vec![
                    command("newItem", "newItem", "New", &["Ctrl+N"]),
                    command("keysItem", "keysItem", "Keys", &[]),
                    command("quitItem", "quitItem", "Quit", &["Ctrl+Q"]),
                    command("copyButton", "copyButton", "Copy", &["Ctrl+Shift+C"]),
                ],
            },
            // A window's own: its frame's commands, then its buttons, also those inside a panel.
            KeyLayer {
                id: "CustomerCard".into(),
                title: Some("Customer".into()),
                commands: vec![
                    command("CustomerCard.printButton", "printButton", "Print", &[]),
                    command("CustomerCard.saveButton", "saveButton", "Save", &["Ctrl+S"]),
                    command("CustomerCard.closeButton", "closeButton", "Close", &[]),
                ],
            },
            KeyLayer { id: "Ask".into(), title: Some("Sure?".into()), commands: vec![command("Ask.yesButton", "yesButton", "Yes", &[])] },
        ]
    );
}

/// The keys window speaks in Clarity's own words, which a project that opens
/// it translates; one that does not, does not need them.
#[test]
fn the_keys_window_has_clarity_s_own_words() {
    let words: Vec<String> = clarity_spec::translations::KEYS_WINDOW_KEYS.iter().map(|(k, _)| k.to_string()).collect();
    let keys_of = |source: &str| -> Vec<String> { clarity_spec::translations::keys(&load_sources([("shop.clarity", source)])).into_iter().map(|k| k.key).collect() };
    let with = keys_of(SHOP);
    assert!(words.iter().all(|w| with.contains(w)), "{with:?}");
    let without = keys_of(&SHOP.replace("            item \"Keys\" name keysItem on click -> keys\n", ""));
    assert!(!words.iter().any(|w| without.contains(w)), "{without:?}");
    let made = clarity_spec::keyed::make_keys(&[("shop.clarity", SHOP)], &[], "en");
    assert!(words.iter().all(|w| made.order.contains(w)), "{:?}", made.order);
}

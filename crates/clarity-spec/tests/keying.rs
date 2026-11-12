//! Make texts keys on the model of a `.cxaml` page or app:
//! every example's pages and app, with their
//! keys put back as the texts they say, and a few pages written by hand,
//! are made keys alike on the model and through the page's block of
//! Clarity: the same markup and the same keys. The comparison stays until
//! the block path goes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clarity_spec::cxaml::make_keys_with_pages;
use clarity_spec::translations::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Each example with design files: its modules, its pages and app, and its
/// default language's texts by key.
fn examples() -> Vec<(PathBuf, Vec<(String, String)>, Vec<(String, String)>, BTreeMap<String, String>)> {
    let mut out = Vec::new();
    let mut folders: Vec<PathBuf> = std::fs::read_dir(repo().join("examples")).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.join("design").is_dir()).collect();
    folders.sort();
    for folder in folders {
        let modules = clarity_spec::spec_files(&folder).unwrap().iter().map(|p| (p.display().to_string(), std::fs::read_to_string(p).unwrap())).collect();
        let pages = clarity_spec::cxaml::page_files(&folder, &[], &[]).unwrap().iter().map(|p| (p.display().to_string(), std::fs::read_to_string(p).unwrap())).collect();
        let english = std::fs::read_to_string(folder.join("translations/en.clarity")).unwrap_or_default();
        let texts = clarity_spec::translations::parse(&english)
            .0
            .entries
            .into_iter()
            .filter_map(|e| match e.value {
                Value::Text(text) => Some((e.key, text)),
                Value::Resource(_) => None,
            })
            .collect();
        out.push((folder, modules, pages, texts));
    }
    out
}

/// `markup` with each attribute that is a key of `texts` written as the
/// text it says, as a page was before its texts were keys.
fn unkeyed(markup: &str, texts: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(at) = rest.find("=\"@") {
        let (before, after) = rest.split_at(at + 2);
        out.push_str(before);
        let end = after.find('"').unwrap();
        let word = &after[1..end];
        match texts.get(word).filter(|_| word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
            // A text default is written as Clarity writes a text, in quotes.
            Some(text) if before.ends_with("default=\"") => out.push_str(&format!("&quot;{}&quot;", text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "\\&quot;"))),
            Some(text) => out.push_str(&text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")),
            None => out.push_str(&after[..end]),
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Making `pages` keys beside `modules`: each file written reads back
/// clean, and the keys made each have a value. (Until windows moved
/// into `.cxaml` files this was compared with keys made through each
/// page's block of Clarity, on every example and these pages, and agreed.)
fn keyed_clean(what: &str, modules: &[(&str, &str)], pages: &[(&str, &str)]) -> clarity_spec::keyed::Keyed {
    let keyed = make_keys_with_pages(modules, pages, &[], "en");
    for (path, text) in keyed.modules.iter().filter(|(p, _)| clarity_spec::cxaml::is_page_file(p)) {
        let designed = clarity_spec::cxaml::read_designed(text, path);
        assert!(designed.is_ok(), "{what}: {path}: {designed:?}\n{text}");
    }
    assert!(keyed.order.iter().all(|k| keyed.languages["en"].get(k).is_some_and(|v| !v.is_empty())), "{what}: a key without a value");
    keyed
}

/// Every example's texts, written as texts again, become keys again: no text
/// of theirs is left in a file.
#[test]
fn every_example_s_texts_become_keys_again() {
    let examples = examples();
    let mut pages = 0;
    for (folder, modules, designed, texts) in &examples {
        let written: Vec<(String, String)> = designed.iter().map(|(p, m)| (p.clone(), unkeyed(m, texts))).collect();
        let modules: Vec<(&str, &str)> = modules.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
        let written: Vec<(&str, &str)> = written.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
        let keyed = keyed_clean(&folder.display().to_string(), &modules, &written);
        for (path, before) in &written {
            let after = &keyed.modules.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("{path} is written")).1;
            let unchanged = |attribute: &str| before.matches(attribute).count() == after.matches(attribute).count();
            assert!(after.matches("caption=\"@").count() >= before.matches("caption=\"@").count() && unchanged("<"), "{path}\n{before}\n----\n{after}");
        }
        pages += written.len();
    }
    assert!(examples.len() > 15 && pages > 90, "{} examples, {pages} files", examples.len());
}

/// The markup the examples give back is their own: each text becomes the key it was.
#[test]
fn an_example_s_texts_become_the_keys_they_were() {
    let (_, modules, designed, texts) = examples().into_iter().find(|(f, ..)| f.ends_with("calculator")).unwrap();
    let modules: Vec<(&str, &str)> = modules.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
    let (path, markup) = designed.iter().find(|(p, _)| p.ends_with("Preferences.cxaml")).unwrap();
    let written = unkeyed(markup, &texts);
    assert!(written.contains("caption=\"Close\""), "the key is a text again: {written}");
    let keyed = make_keys_with_pages(&modules, &[(path.as_str(), written.as_str())], &[], "en");
    let (_, again) = keyed.modules.iter().find(|(p, _)| p == path).unwrap();
    assert!(again.contains("caption=\"@closeButtonCaption\"") && again.contains("title=\"@preferencesTitle\""), "{again}");
}

const MODULE: &str = "module shop\n\nentity Order\n    placed: date, label @placedLabel\n";

/// Pages written by hand with a text in every place one is named from.
const PAGES: [(&str, &str); 3] = [
    (
        "shop/design/Desk.cxaml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- The desk. -->\n<Window class=\"shop.Desk\" title=\"Front desk\">\n  <Data name=\"greeting\" type=\"text\" default=\"&quot;Good morning&quot;\" />\n  <Data name=\"orders\" type=\"list&lt;Order&gt;\" />\n  <Menu name=\"deskMenu\">\n    <Item caption=\"File\">\n      <Item caption=\"Pick a language\" name=\"languageItem\" opens=\"language\" />\n      <Item kind=\"separator\" />\n    </Item>\n  </Menu>\n  <Toolbar name=\"deskToolbar\">\n    <Item kind=\"button\" caption=\"Refresh\" name=\"refreshButton\" onclick=\"refresh\" />\n    <Item kind=\"button\" caption=\"Unnamed\" />\n  </Toolbar>\n  <Ribbon name=\"deskRibbon\" collapsible=\"true\">\n    <Tab title=\"Home\" name=\"homeTab\">\n      <Group title=\"Clipboard stuff\">\n        <Item kind=\"button\" caption=\"Paste\" name=\"pasteButton\" large=\"true\" onclick=\"paste\" />\n      </Group>\n    </Tab>\n  </Ribbon>\n  <Sidebar side=\"left\" name=\"nav\" collapsible=\"true\">\n    <Label name=\"navLabel\" at=\"1,1\" span=\"3\" caption=\"Places\" />\n  </Sidebar>\n  <Footer status=\"Ready when you are\" columns=\"12\" />\n  <Label name=\"nameLabel\" at=\"1,1\" span=\"3\" caption=\"Name\" for=\"nameBox\" />\n  <Textbox name=\"nameBox\" at=\"4,1\" span=\"6\" placeholder=\"Who is it\" tooltip=\"Their full name\" />\n  <Help name=\"nameHelp\" at=\"10,1\" span=\"3\" caption=\"As on the card\" for=\"nameBox\" />\n  <Button at=\"1,2\" span=\"2\" caption=\"No name\" />\n  <Icon name=\"warning\" at=\"3,2\" span=\"1\" icon=\"triangle-alert\" tooltip=\"Look out\" />\n  <Chart name=\"byDay\" at=\"1,3\" span=\"6\" rows=\"4\" bind=\"orders\" chart=\"line\" by=\"placed\" />\n  <Table name=\"grid\" at=\"7,3\" span=\"6\" rows=\"4\" bind=\"orders\" columns=\"placed\" add=\"true\" remove=\"true\" page=\"20\" onopen=\"opened\" />\n  <Date name=\"from\" at=\"1,7\" span=\"3\" />\n  <Button name=\"same\" at=\"4,7\" span=\"2\" caption=\"Front desk\" />\n</Window>\n",
    ),
    (
        "shop/design/Wizard.cxaml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Dialog class=\"shop.Wizard\" title=\"New order\">\n  <Steps name=\"wizard\" at=\"1,1\" span=\"12\" rows=\"6\">\n    <Step title=\"Who orders\">\n      <Label name=\"whoLabel\" at=\"1,1\" span=\"3\" caption=\"Customer\" />\n    </Step>\n    <Step title=\"What\" />\n  </Steps>\n  <Tabs name=\"more\" at=\"1,7\" span=\"12\" rows=\"4\">\n    <Tab title=\"Notes and such\" />\n  </Tabs>\n  <Accordion name=\"parts\" at=\"1,11\" span=\"12\" rows=\"4\">\n    <Section title=\"The extras\" />\n  </Accordion>\n  <Slideout name=\"panel\" at=\"1,1\" span=\"6\" rows=\"4\" caption=\"Details\" />\n  <Canvas name=\"sketch\" at=\"1,15\" span=\"6\" rows=\"4\" mode=\"paint\" zoom=\"true\" toolbox=\"popup\" none=\"surface\" />\n</Dialog>\n",
    ),
    (
        "shop/design/Shop.cxaml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- The frame. -->\n<App class=\"shop.Shop\" title=\"The shop\" main=\"Desk\" onunconnected=\"notConnected\">\n  <Data name=\"motto\" type=\"text\" default=\"&quot;Always open&quot;\" />\n  <Menu name=\"mainMenu\">\n    <Item caption=\"Settings\" name=\"settingsItem\">\n      <Item caption=\"Theme\" name=\"themeItem\" opens=\"theme\" />\n      <Item caption=\"Keys\" name=\"keysItem\" opens=\"keys\" />\n    </Item>\n  </Menu>\n  <Footer status=\"Welcome\" columns=\"12\" />\n</App>\n",
    ),
];

#[test]
fn pages_written_by_hand_are_made_keys() {
    for (path, markup) in PAGES {
        keyed_clean(path, &[("shop/shop.clarity", MODULE)], &[(path, markup)]);
    }
    keyed_clean("all three", &[("shop/shop.clarity", MODULE)], &PAGES);
    // Without the module, a page alone.
    keyed_clean("alone", &[], &PAGES[..1]);
}

/// A page's texts are named on its model as a module's page is.
#[test]
fn a_page_s_texts_are_named_after_where_they_are() {
    let keyed = make_keys_with_pages(&[("shop/shop.clarity", MODULE)], &PAGES, &[], "en");
    let (_, desk) = keyed.modules.iter().find(|(p, _)| p.ends_with("Desk.cxaml")).unwrap();
    assert!(desk.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- The desk. -->\n"), "the comment stays: {desk}");
    for written in [
        "title=\"@deskTitle\"",
        "deskGreeting",
        "caption=\"@fileItem\"",
        "caption=\"@languageItem\"",
        "caption=\"@refreshButtonCaption\"",
        "caption=\"@buttonCaption\"",
        "title=\"@homeTab\"",
        "title=\"@deskClipboardStuffTab\"",
        "status=\"@deskStatus\"",
        "caption=\"@nameBoxLabel\"",
        "placeholder=\"@nameBoxPlaceholder\"",
        "tooltip=\"@nameBoxTooltip\"",
        "caption=\"@nameBoxHelp\"",
        "tooltip=\"@warningTooltip\"",
        // The same text named after another place is a key of its own.
        "caption=\"@sameCaption\"",
    ] {
        assert!(desk.contains(written), "{written} in {desk}");
    }
    let (_, wizard) = keyed.modules.iter().find(|(p, _)| p.ends_with("Wizard.cxaml")).unwrap();
    for written in ["title=\"@wizardWhoOrdersStep\"", "title=\"@wizardNotesAndSuchTab\"", "title=\"@wizardTheExtrasSection\"", "caption=\"@panelCaption\""] {
        assert!(wizard.contains(written), "{written} in {wizard}");
    }
    let en = &keyed.languages["en"];
    assert_eq!(en.get("deskTitle").map(String::as_str), Some("Front desk"));
    // Clarity's own words the pages need: a wizard's, a table's rows and pages, a calendar's, a paint toolbox's.
    for own in ["next", "addRow", "firstPage", "zoomIn"] {
        assert!(en.contains_key(own) || keyed.order.iter().any(|k| k == own), "{own}: {:?}", keyed.order);
    }
}

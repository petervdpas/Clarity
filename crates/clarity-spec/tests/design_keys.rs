//! Each thing the designer shows is known by its file and name
//! (`design/EnumEditor.cxaml#EnumEditor/saveButton`), not by where it is
//! written: the key stays when the text around it changes, and names the
//! file it belongs to.

use std::collections::BTreeMap;

use clarity_spec::cxaml::read_page;
use clarity_spec::design_keys::{page_keys, place_of, split};

const PAGE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<View class="modelling.EnumEditor" columns="6">
  <Data name="values" type="list&lt;EnumValue&gt;" />
  <Tells message="saved" />
  <Tabs name="enumTabs" at="1,1" span="6" rows="12">
    <Tab title="@enumTab">
      <Button name="saveButton" at="5,11" span="2" caption="@saveCaption" onclick="saveClicked" />
    </Tab>
    <Tab title="@codeTab">
      <Memo name="codeBox" at="1,1" span="6" rows="11" bind="values" readonly="true" />
    </Tab>
  </Tabs>
</View>
"#;

const FILE: &str = "design/EnumEditor.cxaml";

fn keys(text: &str) -> BTreeMap<String, u32> {
    let (page, diagnostics) = read_page(text, FILE);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    page_keys(FILE, &page.unwrap()).into_iter().map(|(at, key)| (key, at)).collect()
}

#[test]
fn a_named_part_is_known_by_its_file_and_name() {
    let keys = keys(PAGE);
    let at = keys["design/EnumEditor.cxaml#EnumEditor/saveButton"];
    assert!(PAGE[at as usize..].starts_with("<Button"), "{}", &PAGE[at as usize..]);
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor"));
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/enumTabs"));
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/codeBox"));
}

#[test]
fn what_has_no_name_is_known_by_its_place_under_what_holds_it() {
    let keys = keys(PAGE);
    let second = keys["design/EnumEditor.cxaml#EnumEditor/enumTabs/Tab[2]"];
    assert!(PAGE[second as usize..].starts_with("<Tab title=\"@codeTab\""), "{}", &PAGE[second as usize..]);
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/enumTabs/Tab[1]"));
    // Data and tells by their names, which may be a part's name too.
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/Data[values]"));
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/Tells[saved]"));
}

#[test]
fn a_key_stays_when_the_text_around_it_changes() {
    let before = keys(PAGE);
    let edited = PAGE.replace("<Tells message=\"saved\" />", "<Tells message=\"saved\" />\n  <Tells message=\"closed\" />");
    let after = keys(&edited);
    let key = "design/EnumEditor.cxaml#EnumEditor/saveButton";
    assert_ne!(before[key], after[key]);
    // A part moved to another tab keeps its key.
    let moved = PAGE.replace("      <Button name=\"saveButton\" at=\"5,11\" span=\"2\" caption=\"@saveCaption\" onclick=\"saveClicked\" />\n", "").replace("<Memo ", "<Button name=\"saveButton\" at=\"5,11\" span=\"2\" caption=\"@saveCaption\" onclick=\"saveClicked\" />\n      <Memo ");
    assert!(keys(&moved).contains_key(key));
}

#[test]
fn every_key_is_one_thing() {
    let (page, _) = read_page(PAGE, FILE);
    let all = page_keys(FILE, &page.unwrap());
    let unique: std::collections::BTreeSet<&String> = all.iter().map(|(_, k)| k).collect();
    assert_eq!(unique.len(), all.len(), "{all:?}");
    // A second part of the same name (a mistake the check reports) is known by its place.
    let twice = PAGE.replace("name=\"codeBox\"", "name=\"saveButton\"");
    let keys = keys_unchecked(&twice);
    assert!(keys.contains_key("design/EnumEditor.cxaml#EnumEditor/enumTabs/Tab[2]/Memo[1]"), "{keys:?}");
}

fn keys_unchecked(text: &str) -> BTreeMap<String, u32> {
    let (page, _) = read_page(text, FILE);
    page_keys(FILE, &page.unwrap()).into_iter().map(|(at, key)| (key, at)).collect()
}

#[test]
fn a_key_splits_into_its_file_and_path_and_finds_its_place() {
    assert_eq!(split("design/Main.cxaml#Main/saveButton"), Some(("design/Main.cxaml", "Main/saveButton")));
    assert_eq!(split("saveButton"), None);
    let (page, _) = read_page(PAGE, FILE);
    let page = page.unwrap();
    let at = place_of(FILE, &page, "design/EnumEditor.cxaml#EnumEditor/codeBox").unwrap();
    assert!(PAGE[at as usize..].starts_with("<Memo"));
    assert_eq!(place_of(FILE, &page, "design/Other.cxaml#Other/codeBox"), None);
}

//! What a window's host offers a part to pick from: a list of an enum's
//! values (`formats: list<Format>`, a tags part) offers the enum's values,
//! as a field of the enum does (C# OfferedFor on the item type).

use super::core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::window::ProgramHost;
use clarity_ui::{AppFiles, PartHost};
use core_windows_fake::*;
use serde_json::{Map, json};

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "files": [{
  "pages": [{"name": {"name": "Main"}, "kind": "window", "data": [{"name": {"name": "album"}, "of": {"kind": "record", "entity": {"name": "Album"}}}]}],
  "enums": [{"name": {"name": "Format"}, "variants": [{"name": {"name": "vinyl"}, "label": "Vinyl"}, {"name": {"name": "cd"}, "label": "CD"}]}],
  "entities": [{"name": {"name": "Album"}, "fields": [
    {"name": {"name": "format"}, "type": {"kind": "enum", "name": {"name": "Format"}}, "modifiers": {}},
    {"name": {"name": "formats"}, "type": {"kind": "values", "item": {"kind": "enum", "name": {"name": "Format"}}}, "modifiers": {}}
  ]}]
}]}}"#;

#[test]
fn a_list_of_an_enum_offers_the_enums_values() {
    let files = AppFiles { model: MODEL, ..FILES };
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let host = ProgramHost::new(program, files, WindowState::new("Main", Map::new()), &[]);
    let labels = |path: &str| host.choices(path).into_iter().map(|c| (c.value, c.label)).collect::<Vec<_>>();
    let expected = vec![(json!("vinyl"), "Vinyl".to_string()), (json!("cd"), "CD".to_string())];
    assert_eq!(labels("album.format"), expected);
    assert_eq!(labels("album.formats"), expected);
}

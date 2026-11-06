//! The resources a program carries (pictures, Markdown, sounds), by their
//! names, which its window's host gives its parts: `image source "logo.png"`
//! shows the picture (C# reads them as embedded resources).

use super::core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::window::ProgramHost;
use clarity_ui::{AppFiles, PartHost};
use core_windows_fake::*;
use serde_json::Map;

const LOGO: &[u8] = &[0x89, b'P', b'N', b'G'];

#[test]
fn the_host_answers_a_resource_by_its_name() {
    let files = AppFiles { resources: &[("logo.png", LOGO), ("rules.md", b"# Rules")], ..FILES };
    assert_eq!(files.resource("rules.md"), Some(&b"# Rules"[..]));
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let host = ProgramHost::new(program, files, WindowState::new("Main", Map::new()), &[]);
    assert_eq!(host.resource("logo.png"), Some(LOGO.to_vec()));
    assert_eq!(host.resource("missing.png"), None);
}

/// The application's icon is the picture the project names (`icon "app.png"`,
/// in the model), never a resource found by its file's name.
#[test]
fn the_icon_is_the_picture_the_project_names() {
    let named = AppFiles { model: r#"{"project":{"icon":"app.png"}}"#, resources: &[("app.png", LOGO), ("icon.png", LOGO)], ..FILES };
    assert_eq!(named.icon(), Some(LOGO));
    let unnamed = AppFiles { model: r#"{"project":{}}"#, resources: &[("icon.png", LOGO)], ..FILES };
    assert_eq!(unnamed.icon(), None, "a file named icon.png is no icon until the project names it");
    let missing = AppFiles { model: r#"{"project":{"icon":"gone.png"}}"#, resources: &[], ..FILES };
    assert_eq!(missing.icon(), None);
}

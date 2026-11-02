//! A click whose target names a popover of the window (`opens filters`)
//! opens it, as C#'s generated `Show_filters` does; no handler runs.

use super::core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::markup::{self, HandlerCall};
use clarity_ui::window::ProgramHost;
use clarity_ui::PartHost;
use core_windows_fake::*;
use serde_json::Map;

const PARTS: &str = r#"<Window name="Main">
  <Button name="filterButton" at="1,1" span="3" caption="Filter" opens="filters" />
  <Popover name="filters" at="1,2" span="6" for="filterButton" />
</Window>"#;

#[test]
fn a_click_that_names_a_popover_opens_it() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let parts = markup::read(PARTS).unwrap().parts;
    let host = ProgramHost::new(program, FILES, WindowState::new("Main", Map::new()), &parts);
    fake.calls();
    assert!(host.run(Some("filterButton"), &HandlerCall::new("click", "filters", Vec::new())));
    assert_eq!(host.state(Some("filters")).and_then(|s| s.visible), Some(true));
    assert!(fake.calls().is_empty());
}

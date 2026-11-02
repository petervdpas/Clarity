//! A desktop window of a few parts from inline markup, over a test host:
//! the desktop host shown on its own, without a program.
//!
//! Run: `cargo run -p clarity-ui --example hello`

use std::process::ExitCode;

use clarity_ui::desktop::{self, ShowOptions};
use clarity_ui::element::GridPanel;
use clarity_ui::locale::translate;
use clarity_ui::testing::TestHost;
use clarity_ui::{markup, parts, Screen, ThemeLook};

const HELLO: &str = r#"<Window name="Hello">
  <Heading name="title" at="1,1" span="12" caption="Hello from Clarity.UI" />
  <Label name="nameLabel" at="1,2" span="3" caption="Your name" />
  <Textbox name="name" at="4,2" span="6" />
  <Label name="newsLabel" at="1,3" span="3" caption="News by mail" />
  <Checkbox name="news" at="4,3" span="6" caption="Yes, send it" />
  <Button name="save" at="1,4" span="3" caption="Save" />
  <Button name="cancel" at="4,4" span="3" caption="Cancel" />
</Window>"#;

fn main() -> ExitCode {
    let spec = markup::read(HELLO).expect("the example's markup reads");
    let host = TestHost::new();
    let mut grid = GridPanel::new(spec.columns);
    for part in spec.parts {
        let made = parts::make(part, host.clone());
        let place = parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    let options = ShowOptions::new(translate("helloTitle", "Clarity hello")).size(720.0, 420.0).following(ThemeLook::clarity);
    desktop::show(screen, options)
}

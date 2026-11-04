//! A window with every kind of part in Clarity's own look, light and dark,
//! drawn as the IDE's own pages look (DESIGN 1.5): the window, its fields,
//! buttons, header and chosen row take their colors from the theme's data,
//! not from drawing code. With CLARITY_SHOTS set the two pictures are kept,
//! to lay beside the IDE's.

use super::data_rows_host::{shot_path, DataHost};

use std::rc::Rc;

use clarity_ui::element;
use clarity_ui::geometry::Rect;
use clarity_ui::markup;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_name, Shot};
use clarity_ui::window::WindowView;
use clarity_ui::{Screen, ThemeLook};
use serde_json::json;

const GALLERY: &str = r#"
<Heading name="title" at="1,1" span="6" caption="Every part" />
<Label name="label" at="1,2" span="2" caption="Name" />
<Textbox name="name" at="3,2" span="4" bind="task.name" />
<Label name="passLabel" at="7,2" span="2" caption="Password" />
<Password name="pass" at="9,2" span="4" bind="task.pass" />
<Label name="ownerLabel" at="1,3" span="2" caption="Owner" />
<Dropdown name="owner" at="3,3" span="4" bind="task.owner" />
<Label name="countLabel" at="7,3" span="2" caption="Count" />
<Number name="count" at="9,3" span="4" bind="task.count" />
<Checkbox name="done" at="1,4" span="3" bind="task.done" caption="Done" />
<Switch name="urgent" at="4,4" span="3" bind="task.urgent" caption="Urgent" />
<Radio name="kind" at="7,4" span="6" bind="task.kind" />
<Slider name="level" at="1,5" span="6" bind="task.level" />
<Progress name="part" at="7,5" span="6" bind="task.level" />
<Button name="save" at="1,6" span="2" caption="Save" />
<Button name="cancel" at="3,6" span="2" caption="Cancel" />
<Date name="due" at="7,6" span="6" bind="task.due" />
<Table name="tasks" at="1,7" span="6" rows="5" bind="tasks" columns="name, owner" />
<List name="folders" at="7,7" span="6" rows="5" bind="tasks" show="name" />
<Tabs name="pages" at="1,12" span="12" rows="3">
  <Tab title="One"><Label name="first" at="1,1" span="3" caption="First page" /></Tab>
  <Tab title="Two"><Label name="second" at="1,1" span="3" caption="Second page" /></Tab>
</Tabs>
"#;

fn gallery(dark: bool) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("task", json!({ "name": "Write the report", "pass": "secret", "owner": "ann", "count": 3, "done": true, "urgent": false, "kind": "home", "level": 40, "due": "2026-11-04" }));
    host.set("tasks", json!([{ "name": "Report", "owner": "Ann" }, { "name": "Invoice", "owner": "Bob" }, { "name": "Plan", "owner": "Cy" }]));
    host.offer("task.owner", &[(json!("ann"), "Ann"), (json!("bob"), "Bob")]);
    host.offer("task.kind", &[(json!("home"), "Home"), (json!("work"), "Work")]);
    host.choose("folders", Some(1));
    let spec = markup::read(&format!("<Window name=\"Gallery\">{GALLERY}</Window>")).expect("the markup reads");
    let as_host: Rc<dyn PartHost> = host.clone();
    let view = WindowView::new(&spec, as_host);
    let screen = Screen::new(Box::new(view), ThemeLook::clarity(dark));
    (screen, host)
}

fn place(screen: &Screen, name: &str) -> Rect {
    element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some(name)).unwrap_or_else(|| panic!("no part {name}")).node().bounds
}

#[test]
fn every_part_takes_clarity_s_own_look_from_the_theme_data() {
    for dark in [false, true] {
        let (mut screen, _host) = gallery(dark);
        let mut shot = Shot::of(&mut screen, 900, 640, 1.0);
        if let Some(path) = shot_path(if dark { "gallery-dark.png" } else { "gallery-light.png" }) {
            shot.save(path).expect("the picture is written");
        }
        let look = screen.look().clone();
        assert!(near(look.color("background"), shot.pixel(2, 2), 2), "the window is the theme's background (dark {dark})");
        let save = place(&screen, "save");
        let button = look.part("button", None);
        let fill = shot.pixel((save.x + save.width / 2.0) as i32, (save.y + 4.0) as i32);
        assert!(near(button.background.unwrap(), fill, 6), "a button is filled as the theme's button (dark {dark}): {fill:?}");
        let name = place(&screen, "name");
        let field = look.part("input", None);
        let inside = shot.pixel((name.x + name.width - 6.0) as i32, (name.y + name.height / 2.0) as i32);
        assert!(near(field.background.unwrap(), inside, 6), "a text box is filled as the theme's input (dark {dark}): {inside:?}");
        // The open tab in the tab's selected look, as the IDE's tabs: the soft accent.
        let pages = place(&screen, "pages");
        let open = shot.pixel((pages.x + 4.0) as i32, (pages.y + 16.0) as i32);
        assert!(near(look.part("tab", Some("selected")).background.unwrap(), open, 6), "the open tab (dark {dark}): {open:?}");
        assert!(near(look.color("accent_soft"), open, 6), "Clarity's own open tab is the soft accent (dark {dark})");
    }
}

//! A container frames the data parts in it (DESIGN 7.8), as the C#
//! FramesTests check it: a list, table or tree inside a panel, a form, a tabs
//! page, a step, a section, a pane, a scroll box or a popover shows no border
//! of its own unless it has `frame`; on the window it keeps one.

use clarity_ui::element::{in_order, Element};
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::testing::{window_of, Shot, TestHost};
use clarity_ui::Screen;
use serde_json::json;

const LIST: &str = r#"<List name="folders" at="1,1" span="3" rows="3" bind="folders" show="name" EXTRA />"#;
const TABLE: &str = r#"<Table name="folders" at="1,1" span="3" rows="3" bind="folders" columns="name" EXTRA />"#;
const TREE: &str = r#"<Tree name="folders" at="1,1" span="3" rows="3" bind="folders" show="name" EXTRA />"#;

const CONTAINERS: [&str; 8] = [
    r#"<Panel name="box" at="1,1" span="6" rows="5" caption="Box">PART</Panel>"#,
    r#"<Form at="1,1" span="6" rows="5">PART</Form>"#,
    r#"<Tabs name="pages" at="1,1" span="6" rows="5"><Tab title="One">PART</Tab></Tabs>"#,
    r#"<Steps name="wizard" at="1,1" span="6" rows="6"><Step title="One">PART</Step></Steps>"#,
    r#"<Accordion name="faq" at="1,1" span="6" rows="6"><Section title="One">PART</Section></Accordion>"#,
    r#"<Splitter name="split" at="1,1" span="12" rows="5"><Pane span="6">PART</Pane><Pane /></Splitter>"#,
    r#"<Scroll at="1,1" span="6" rows="5">PART</Scroll>"#,
    r#"<Trend name="save" at="1,1" span="3" /><Popover name="tip" for="save" at="1,2" span="4" rows="4">PART</Popover>"#,
];

fn window(markup: &str) -> Screen {
    let host = TestHost::new();
    host.set("folders", json!([{ "id": 1, "name": "One", "position": 1 }]));
    host.set_state("tip", |s| s.visible = Some(true));
    let mut screen = window_of(markup, host);
    screen.resize(808.0, 400.0, 1.0);
    screen.resize(808.0, 400.0, 1.0);
    screen
}

/// Whether the one list, table or tree on the screen or over it draws its own border.
fn framed(markup: &str) -> bool {
    let screen = window(markup);
    let mut found: Vec<bool> = Vec::new();
    let mut look = |root: &dyn Element| {
        for e in in_order(root) {
            if let Some(list) = e.as_any().downcast_ref::<ListPart>() {
                found.push(list.framed());
            }
            if let Some(table) = e.as_any().downcast_ref::<TablePart>() {
                found.push(table.framed());
            }
        }
    };
    look(screen.root());
    for overlay in screen.overlays() {
        look(overlay.content.as_ref());
    }
    assert_eq!(1, found.len(), "one data part in {markup}");
    found[0]
}

#[test]
fn a_list_table_or_tree_in_a_container_has_no_border_unless_it_has_frame() {
    for container in CONTAINERS {
        for part in [LIST, TABLE, TREE] {
            assert!(!framed(&container.replace("PART", &part.replace("EXTRA", ""))), "{container} {part}");
            assert!(framed(&container.replace("PART", &part.replace("EXTRA", r#"frame="true""#))), "{container} {part} with frame");
        }
    }
}

#[test]
fn a_list_table_or_tree_on_the_window_has_its_border() {
    for part in [LIST, TABLE, TREE] {
        assert!(framed(&part.replace("EXTRA", "")), "{part}");
    }
}

#[test]
fn a_list_in_a_panel_draws_no_border_line() {
    let mut screen = window(&format!(r#"<Panel name="box" at="1,1" span="6" rows="5" caption="Box">{}</Panel>"#, LIST.replace("EXTRA", "")));
    let list = clarity_ui::element::find_type::<ListPart>(screen.root(), |_| true).unwrap().node().bounds;
    let border = screen.look().part("list", None).border;
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    assert_ne!(border, Some(shot.pixel(list.x as i32 + 30, list.y as i32)));
}

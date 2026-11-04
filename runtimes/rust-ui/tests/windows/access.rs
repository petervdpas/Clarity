//! Screen readers (DESIGN 7.9, A1): the accessibility tree made from the
//! screen's elements, per kind of part (role, name from its caption or its
//! label, translated, value and states), the focus following the screen's,
//! actions from the tree done as the person's input, AccessKit's updates
//! sending only what changed, and the tree as the driven API answers it.

use std::rc::Rc;

use clarity_ui::access::{self, act::act, kit, mirror, AccessAction, AccessTree, Role};
use clarity_ui::headless::Driven;
use clarity_ui::parts::FieldFacts;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::Screen;
use serde_json::json;

use crate::core_frame_shared::{draw, window};

fn shown(markup: &str, host: &Rc<TestHost>) -> Screen {
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 600.0, 1.0);
    Shot::of(&mut screen, 800, 600, 1.0);
    screen
}

fn read(screen: &mut Screen) -> AccessTree {
    Shot::of(screen, 800, 600, 1.0);
    access::tree(screen)
}

#[test]
fn a_button_is_named_by_its_caption_translated_as_drawn() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("saveCaption".into(), "Opslaan".into());
    let mut screen = shown(r#"<Button name="save" at="1,1" span="3" caption="@saveCaption" onclick="save" />"#, &host);
    let tree = read(&mut screen);
    let save = tree.part("save").expect("the button is in the tree");
    assert_eq!(save.role, Role::Button);
    assert_eq!(save.name, "Opslaan");
    assert!(save.actions.contains(&AccessAction::Click));
    assert_eq!(tree.root().role, Role::Window);
    assert!(tree.root().children.contains(&save.id));
}

#[test]
fn an_input_is_named_by_its_label_with_its_value_required_and_invalid() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("emailLabel".into(), "E-mail".into());
    host.set("customer.email", json!("ada@example.org"));
    host.fields.borrow_mut().insert("customer.email".into(), FieldFacts { label: "Mail".into(), required: true, ..FieldFacts::default() });
    let mut screen = shown(
        r#"<Label at="1,1" span="3" for="email" caption="@emailLabel" />
           <Textbox name="email" at="4,1" span="6" bind="customer.email" />
           <Textbox name="other" at="4,2" span="6" bind="customer.other" />"#,
        &host,
    );
    let tree = read(&mut screen);
    let email = tree.part("email").unwrap();
    assert_eq!(email.role, Role::TextBox);
    assert_eq!(email.name, "E-mail");
    assert_eq!(email.value.as_deref(), Some("ada@example.org"));
    assert!(email.states.required);
    assert_eq!(email.states.invalid, None);
    assert!(email.actions.contains(&AccessAction::SetValue));
    host.set_state("email", |s| s.error = Some("Not an address".into()));
    let tree = read(&mut screen);
    assert_eq!(tree.part("email").unwrap().states.invalid.as_deref(), Some("Not an address"));
    // Without a label, the field's own label names it; without that, nothing does.
    host.fields.borrow_mut().insert("customer.other".into(), FieldFacts { label: "Other".into(), ..FieldFacts::default() });
    assert_eq!(read(&mut screen).part("other").unwrap().name, "Other");
}

#[test]
fn a_password_is_never_read_out() {
    let host = TestHost::new();
    host.set("login.secret", json!("hunter2"));
    let mut screen = shown(r#"<Password name="secret" at="1,1" span="4" bind="login.secret" caption="Secret" />"#, &host);
    let tree = read(&mut screen);
    let secret = tree.part("secret").unwrap();
    assert_eq!(secret.role, Role::Password);
    assert_eq!(secret.value, None);
    assert!(!tree.json().to_string().contains("hunter2"));
}

#[test]
fn check_boxes_and_switches_say_whether_they_are_ticked() {
    let host = TestHost::new();
    host.set("task.done", json!(true));
    let mut screen = shown(
        r#"<Checkbox name="done" at="1,1" span="4" bind="task.done" caption="Done" />
           <Switch name="late" at="1,2" span="4" bind="task.late" caption="Late" readonly="true" />"#,
        &host,
    );
    let tree = read(&mut screen);
    let done = tree.part("done").unwrap();
    assert_eq!((done.role, done.name.as_str(), done.states.checked), (Role::CheckBox, "Done", Some(true)));
    let late = tree.part("late").unwrap();
    assert_eq!((late.role, late.states.checked, late.states.readonly), (Role::Switch, Some(false), true));
}

#[test]
fn sliders_and_numbers_give_their_number_and_range() {
    let host = TestHost::new();
    host.set("task.rating", json!(4));
    host.set("task.count", json!(7));
    let mut screen = shown(
        r#"<Slider name="rating" at="1,1" span="6" bind="task.rating" min="2" max="10" step="2" caption="Rating" />
           <Number name="count" at="1,2" span="4" bind="task.count" caption="Count" />"#,
        &host,
    );
    let tree = read(&mut screen);
    let rating = tree.part("rating").unwrap();
    assert_eq!(rating.role, Role::Slider);
    let n = rating.numeric.clone().unwrap();
    assert_eq!((n.value, n.min, n.max, n.step), (Some(4.0), Some(2.0), Some(10.0), Some(2.0)));
    assert!(rating.actions.contains(&AccessAction::Increment));
    let count = tree.part("count").unwrap();
    assert_eq!((count.role, count.value.as_deref()), (Role::SpinButton, Some("7")));
    // Increment from the tree is Up on the slider, as the person's key.
    assert!(act(&mut screen, rating.id, AccessAction::Increment, None));
    assert_eq!(host.values.borrow().get("task.rating"), Some(&json!(6)));
}

#[test]
fn tabs_say_which_is_selected_and_an_accordion_which_section_is_open() {
    let host = TestHost::new();
    let mut screen = shown(
        r#"<Tabs name="pages" at="1,1" span="12" rows="4">
             <Tab title="One"><Label name="first" at="1,1" span="3" caption="First" /></Tab>
             <Tab title="Two"><Label name="second" at="1,1" span="3" caption="Second" /></Tab>
           </Tabs>
           <Accordion name="faq" at="1,6" span="12" rows="6">
             <Section title="Why"><Label at="1,1" span="3" caption="Because" /></Section>
             <Section title="How"><Label at="1,1" span="3" caption="So" /></Section>
           </Accordion>"#,
        &host,
    );
    let tree = read(&mut screen);
    assert_eq!(tree.part("pages").unwrap().role, Role::TabList);
    let one = tree.named(Role::Tab, "One").expect("the first tab");
    let two = tree.named(Role::Tab, "Two").expect("the second tab");
    assert_eq!((one.states.selected, two.states.selected), (Some(true), Some(false)));
    assert!(tree.named(Role::Label, "First").is_some());
    assert!(tree.named(Role::Label, "Second").is_none());
    // A click from the tree opens the second tab, as the person's click.
    assert!(act(&mut screen, two.id, AccessAction::Click, None));
    let tree = read(&mut screen);
    assert_eq!(tree.named(Role::Tab, "Two").unwrap().states.selected, Some(true));
    assert!(tree.named(Role::Label, "Second").is_some());
    let headers: Vec<_> = tree.nodes.iter().filter(|n| n.role == Role::Button && (n.name == "Why" || n.name == "How")).collect();
    assert_eq!(headers.len(), 2);
    assert!(headers.iter().all(|h| h.states.expanded.is_some()));
}

#[test]
fn a_table_has_its_header_and_rows_with_cells_and_the_chosen_row_selected() {
    let host = TestHost::new();
    host.set("folders", json!([{ "id": 1, "name": "Inbox" }, { "id": 2, "name": "Sent" }]));
    let mut screen = shown(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" onselect="picked" />"#, &host);
    let tree = read(&mut screen);
    let table = tree.part("folders").unwrap();
    assert_eq!(table.role, Role::Table);
    let rows: Vec<_> = table.children.iter().filter_map(|c| tree.node(*c)).filter(|n| n.role == Role::Row).collect();
    assert_eq!(rows.len(), 3, "the header row and two rows");
    assert!(tree.node(rows[0].children[0]).is_some_and(|h| h.role == Role::ColumnHeader));
    assert_eq!(rows[1].name, "Inbox, 1");
    assert_eq!(tree.node(rows[2].children[0]).unwrap().name, "Sent");
    assert_eq!(rows[2].states.selected, Some(false));
    // Choosing a row from the tree is the person's click on it.
    assert!(act(&mut screen, rows[2].id, AccessAction::Click, None));
    assert!(host.logged().iter().any(|l| l.contains("picked")), "{:?}", host.logged());
    let tree = read(&mut screen);
    let row = tree.nodes.iter().find(|n| n.role == Role::Row && n.place == Some(1)).unwrap();
    assert_eq!(row.states.selected, Some(true));
    // The table with the focus gives it to its chosen row.
    screen.focus(Some(part_named(&screen, "folders")));
    let tree = read(&mut screen);
    assert_eq!(tree.node(tree.focus).unwrap().place, Some(1));
}

#[test]
fn a_tree_row_says_its_depth_and_whether_it_is_open() {
    let host = TestHost::new();
    host.set("folders", json!([{ "id": 1, "name": "Root", "parent": null }, { "id": 2, "name": "Child", "parent": 1 }]));
    let mut screen = shown(r#"<Tree at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name" />"#, &host);
    let tree = read(&mut screen);
    assert_eq!(tree.part("folders").unwrap().role, Role::TreeGrid);
    let root = tree.nodes.iter().find(|n| n.role == Role::Row && n.name == "Root").unwrap().clone();
    assert_eq!((root.level, root.states.expanded), (Some(1), Some(false)));
    assert!(tree.nodes.iter().all(|n| n.name != "Child"));
    assert!(act(&mut screen, root.id, AccessAction::Expand, None));
    let tree = read(&mut screen);
    let child = tree.nodes.iter().find(|n| n.role == Role::Row && n.name == "Child").expect("the open row shows its child");
    assert_eq!(child.level, Some(2));
    assert_eq!(tree.node(root.id).unwrap().states.expanded, Some(true));
}

#[test]
fn the_focus_follows_the_screens() {
    let host = TestHost::new();
    let mut screen = shown(
        r#"<Textbox name="first" at="1,1" span="4" caption="First" />
           <Textbox name="second" at="1,2" span="4" caption="Second" />"#,
        &host,
    );
    assert_eq!(read(&mut screen).focus, access::ROOT);
    let second = part_named(&screen, "second");
    screen.focus(Some(second));
    let tree = read(&mut screen);
    assert_eq!(tree.node(tree.focus).unwrap().part.as_deref(), Some("second"));
    assert!(tree.part("second").unwrap().states.focused);
    assert!(!tree.part("first").unwrap().states.focused);
    // Focus from the tree moves the screen's.
    let first = tree.part("first").unwrap().id;
    assert!(act(&mut screen, first, AccessAction::Focus, None));
    assert_eq!(read(&mut screen).node(screen.focused().unwrap()).unwrap().part.as_deref(), Some("first"));
}

#[test]
fn a_click_from_the_tree_runs_the_buttons_handler() {
    let host = TestHost::new();
    let mut screen = shown(r#"<Button name="save" at="1,1" span="3" caption="Save" onclick="save" />"#, &host);
    let save = read(&mut screen).part("save").unwrap().id;
    assert!(act(&mut screen, save, AccessAction::Click, None));
    assert_eq!(host.logged().iter().filter(|l| l.starts_with("run save click")).count(), 1, "{:?}", host.logged());
    // A disabled button takes no click from the tree either.
    host.set_state("save", |s| s.enabled = Some(false));
    let tree = read(&mut screen);
    assert!(tree.part("save").unwrap().states.disabled);
    assert!(!act(&mut screen, save, AccessAction::Click, None));
    assert_eq!(host.logged().iter().filter(|l| l.starts_with("run save click")).count(), 1);
}

#[test]
fn set_value_from_the_tree_types_over_the_value_and_writes_it_as_one_edit() {
    let host = TestHost::new();
    host.set("customer.email", json!("old@example.org"));
    let mut screen = shown(r#"<Textbox name="email" at="1,1" span="6" bind="customer.email" caption="E-mail" onchange="changed" />"#, &host);
    let email = read(&mut screen).part("email").unwrap().id;
    assert!(act(&mut screen, email, AccessAction::SetValue, Some("new@example.org")));
    assert_eq!(host.values.borrow().get("customer.email"), Some(&json!("new@example.org")));
    assert!(host.logged().iter().any(|l| l == "write email customer.email \"new@example.org\""), "{:?}", host.logged());
    assert_eq!(read(&mut screen).part("email").unwrap().value.as_deref(), Some("new@example.org"));
}

#[test]
fn updates_send_the_whole_tree_first_and_then_only_what_changed() {
    let host = TestHost::new();
    let mut screen = shown(
        r#"<Checkbox name="done" at="1,1" span="4" bind="task.done" caption="Done" />
           <Label at="1,2" span="4" caption="Still" />"#,
        &host,
    );
    let first = read(&mut screen);
    let whole = kit::update(None, &first, 1.0);
    assert!(whole.tree.is_some());
    assert_eq!(whole.nodes.len(), first.nodes.len());
    assert_eq!(kit::update(Some(&first), &first, 1.0).nodes.len(), 0);
    let done = first.part("done").unwrap().id;
    assert!(act(&mut screen, done, AccessAction::Click, None));
    let second = read(&mut screen);
    assert_eq!(second.part("done").unwrap().states.checked, Some(true));
    let changed = kit::update(Some(&first), &second, 1.0);
    assert!(changed.tree.is_none());
    assert!(changed.nodes.iter().any(|(id, node)| id.0 == done && node.toggled() == Some(kit::accesskit::Toggled::True)));
    assert!(changed.nodes.iter().all(|(id, _)| second.node(id.0).unwrap().name != "Still"));
}

#[test]
fn a_custom_part_is_named_by_its_declaration_with_its_description() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("gaugeAbout".into(), "Shows the speed".into());
    let mut screen = shown(r#"<Custom name="speedo" at="1,1" span="4" rows="3" part="Gauge" description="@gaugeAbout" />"#, &host);
    let tree = read(&mut screen);
    let gauge = tree.part("speedo").unwrap();
    assert_eq!(gauge.name, "Gauge");
    // No part Gauge is registered here: it says so instead of its description.
    assert!(gauge.description.as_deref().is_some_and(|d| d.contains("Gauge")));
    clarity_ui::parts::custom::register("AccessGauge", || Box::new(Quiet));
    let mut screen = shown(r#"<Custom name="speedo" at="1,1" span="4" rows="3" part="AccessGauge" caption="Speed" description="@gaugeAbout" />"#, &host);
    let tree = read(&mut screen);
    let gauge = tree.part("speedo").unwrap();
    assert_eq!((gauge.name.as_str(), gauge.description.as_deref()), ("Speed", Some("Shows the speed")));
}

struct Quiet;

impl clarity_ui::parts::custom::CustomPart for Quiet {
    fn set(&mut self, _name: &str, _value: &serde_json::Value, _cx: &mut clarity_ui::parts::custom::CustomCx) {}
    fn hear(&mut self, _hear: &str, _values: &[serde_json::Value], _cx: &mut clarity_ui::parts::custom::CustomCx) {}
    fn draw(&self, _painter: &clarity_ui::Painter, _area: clarity_ui::Rect) {}
}

#[test]
fn the_frame_shows_its_menu_bar_toolbar_and_footer() {
    let (mut screen, _host) = window(
        r#"<Menu name="m"><Item caption="File"><Item caption="Open" onclick="open" /></Item><Item caption="View" /></Menu>
           <Toolbar><Item caption="Save" name="saveCommand" onclick="save" /><Item kind="separator" /><Item caption="Print" onclick="print" /></Toolbar>
           <Footer status="Ready" columns="6" />
           <Label at="1,1" span="3" caption="Body" />"#,
        None,
    );
    draw(&mut screen);
    let tree = access::tree(&screen);
    let bar = tree.nodes.iter().find(|n| n.role == Role::MenuBar).expect("the menu bar");
    let items: Vec<&str> = bar.children.iter().filter_map(|c| tree.node(*c)).map(|n| n.name.as_str()).collect();
    assert_eq!(items, ["File", "View"]);
    let toolbar = tree.nodes.iter().find(|n| n.role == Role::Toolbar).expect("the toolbar");
    let buttons: Vec<&str> = toolbar.children.iter().filter_map(|c| tree.node(*c)).map(|n| n.name.as_str()).collect();
    assert_eq!(buttons, ["Save", "Print"]);
    assert!(tree.nodes.iter().any(|n| n.role == Role::Status && n.name == "Ready"));
    // A click on the menu bar's item from the tree opens its menu, with its items.
    let file = tree.named(Role::MenuItem, "File").unwrap().id;
    assert!(act(&mut screen, file, AccessAction::Click, None));
    draw(&mut screen);
    let tree = access::tree(&screen);
    let menu = tree.nodes.iter().find(|n| n.role == Role::Menu).expect("the open menu");
    assert!(menu.children.iter().filter_map(|c| tree.node(*c)).any(|n| n.name == "Open"));
}

#[test]
fn the_driven_api_answers_the_tree_as_json_and_acts_on_it() {
    let host = TestHost::new();
    let screen = shown(
        r#"<Label at="1,1" span="3" for="done" caption="Finished" />
           <Checkbox name="done" at="4,1" span="4" bind="task.done" />"#,
        &host,
    );
    let mut driven = Driven::over(screen);
    let answer = driven.step(&json!({ "do": "access" }));
    assert_eq!(answer["ok"], json!(true));
    let tree = &answer["access"];
    // Every id as text, a row's too (above what a JavaScript number holds).
    assert_eq!(tree["root"], json!(access::ROOT.to_string()));
    assert!(tree["focus"].is_string());
    assert!(tree["nodes"].as_array().unwrap().iter().all(|n| n["id"].is_string() && n["children"].as_array().unwrap().iter().all(|c| c.is_string())));
    assert_eq!(tree["nodes"][0]["role"], json!("window"));
    let done = tree["nodes"].as_array().unwrap().iter().find(|n| n["part"] == json!("done")).unwrap().clone();
    assert_eq!(done["role"], json!("checkbox"));
    assert_eq!(done["name"], json!("Finished"));
    assert_eq!(done["states"], json!({ "checked": false }));
    assert!(done["actions"].as_array().unwrap().contains(&json!("click")));
    let acted = driven.step(&json!({ "do": "act", "node": done["id"], "action": "click" }));
    assert_eq!(acted["ok"], json!(true), "{acted}");
    let answer = driven.step(&json!({ "do": "access" }));
    let done = answer["access"]["nodes"].as_array().unwrap().iter().find(|n| n["part"] == json!("done")).unwrap().clone();
    assert_eq!(done["states"]["checked"], json!(true));
    assert_eq!(driven.step(&json!({ "do": "act", "node": 12345, "action": "click" }))["ok"], json!(false));
    assert_eq!(driven.step(&json!({ "do": "act", "node": "12345", "action": "click" }))["ok"], json!(false));
}

#[test]
fn a_text_input_gives_the_system_its_text_as_a_run_with_the_caret_at_its_end() {
    let host = TestHost::new();
    host.set("customer.email", json!("ada@example.org"));
    let mut screen = shown(r#"<Textbox name="email" at="1,1" span="6" bind="customer.email" caption="E-mail" />"#, &host);
    let tree = read(&mut screen);
    let email = tree.part("email").unwrap().id;
    let update = kit::update(None, &tree, 1.0);
    let (_, input) = update.nodes.iter().find(|(id, _)| id.0 == email).unwrap();
    let run_id = *input.children().last().expect("the input holds its text run");
    let (_, run) = update.nodes.iter().find(|(id, _)| *id == run_id).unwrap();
    assert_eq!(run.role(), kit::accesskit::Role::TextRun);
    assert_eq!(run.value(), Some("ada@example.org"));
    assert_eq!(input.text_selection().map(|s| s.focus.character_index), Some(15));
}

/// The browser's mirror (phase 2): the whole tree first with ids as text
/// (a row's id is above what a JavaScript number holds), then only what
/// changed and what went, nothing when nothing did, and the focus moving
/// alone is an update too.
#[test]
fn the_browsers_mirror_gets_the_whole_tree_then_only_changes_with_ids_as_text() {
    let host = TestHost::new();
    host.set("tasks", json!([{ "name": "Report" }, { "name": "Plan" }]));
    let mut screen = shown(
        r#"<Checkbox name="done" at="1,1" span="4" bind="task.done" caption="Done" />
           <Label at="1,2" span="4" caption="Still" />
           <Table name="tasks" at="1,3" span="6" rows="4" bind="tasks" columns="name" />"#,
        &host,
    );
    let first = read(&mut screen);
    let whole = mirror::update(None, &first).expect("the whole tree");
    assert_eq!(whole["whole"], json!(true));
    assert_eq!(whole["root"], json!(access::ROOT.to_string()));
    assert_eq!(whole["nodes"].as_array().unwrap().len(), first.nodes.len());
    assert_eq!(whole["say"], json!([]), "nothing is said for the whole tree");
    let row = first.nodes.iter().find(|n| n.role == Role::Row && n.place.is_some()).expect("a row");
    let row_json = whole["nodes"].as_array().unwrap().iter().find(|n| n["id"] == json!(row.id.to_string())).expect("the row by its id as text");
    assert!(row.id > 1 << 53, "a row's id needs text");
    assert!(row_json["children"].as_array().unwrap().iter().all(|c| c.is_string()));
    assert_eq!(mirror::update(Some(&first), &first), None);

    let done = first.part("done").unwrap().id;
    assert!(act(&mut screen, done, AccessAction::Click, None));
    let second = read(&mut screen);
    let changed = mirror::update(Some(&first), &second).expect("a change");
    assert_eq!(changed["whole"], json!(false));
    let ids: Vec<&str> = changed["nodes"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&done.to_string().as_str()), "{ids:?}");
    assert!(changed["nodes"].as_array().unwrap().iter().all(|n| n["name"] != json!("Still")));
    assert_eq!(changed["focus"], json!(second.focus.to_string()));

    // A row gone is said gone.
    host.set("tasks", json!([{ "name": "Report" }]));
    screen.invalidate();
    let third = read(&mut screen);
    let gone = mirror::update(Some(&second), &third).expect("a row went");
    assert!(!gone["gone"].as_array().unwrap().is_empty(), "{gone}");
    for id in gone["gone"].as_array().unwrap() {
        assert!(third.node(id.as_str().unwrap().parse().unwrap()).is_none());
    }

    // Only the focus moved: still an update, with nothing else in it.
    let mut moved = third.clone();
    moved.focus = if third.focus == done { access::ROOT } else { done };
    let focus = mirror::update(Some(&third), &moved).expect("the focus moved");
    assert_eq!(focus["focus"], json!(moved.focus.to_string()));
    assert_eq!(focus["nodes"], json!([]));
}

/// What a person is to hear without going there: a status that came or
/// changed (politely), an alert dialog that opened (as an alert).
#[test]
fn the_mirror_says_a_new_status_and_an_alert() {
    let host = TestHost::new();
    let mut screen = shown(r#"<Label at="1,1" span="4" caption="Plain" />"#, &host);
    let before = read(&mut screen);
    let mut after = before.clone();
    // Ids no element has: element ids count up across the process (tests in parallel), so a fixed number may be a live element's.
    let label = before.nodes.iter().find(|n| n.role == Role::Label).and_then(|n| access::owner_of(n.id)).unwrap();
    let (status_id, alert_id) = (access::virtual_id(label, 1), access::virtual_id(label, 2));
    assert!(before.node(status_id).is_none() && before.node(alert_id).is_none());
    let mut status = access::AccessNode::new(status_id, Role::Status, "Saved", clarity_ui::Rect::new(0.0, 0.0, 10.0, 10.0));
    after.nodes.push(status.clone());
    let mut alert = access::AccessNode::new(alert_id, Role::AlertDialog, "Careful", clarity_ui::Rect::new(0.0, 0.0, 10.0, 10.0));
    alert.description = Some("The disk is full".into());
    after.nodes.push(alert);
    let said = mirror::update(Some(&before), &after).unwrap();
    assert_eq!(said["say"], json!([{ "text": "Saved", "alert": false }, { "text": "Careful. The disk is full", "alert": true }]));
    // Said once: the same again says nothing; a status that changed is said again.
    assert!(mirror::update(Some(&after), &after).is_none());
    let mut changed = after.clone();
    status.name = "Saved twice".into();
    changed.nodes[before.nodes.len()] = status;
    assert_eq!(mirror::update(Some(&after), &changed).unwrap()["say"], json!([{ "text": "Saved twice", "alert": false }]));
}

/// Text typed and not yet taken (the input still has the focus) is the
/// input's value in the tree: a screen reader hears what was typed.
#[test]
fn text_typed_and_not_yet_taken_is_the_inputs_value() {
    let host = TestHost::new();
    host.set("customer.name", json!("Ada"));
    let mut screen = shown(r#"<Textbox name="name" at="1,1" span="6" bind="customer.name" caption="Name" />"#, &host);
    let name = read(&mut screen).part("name").unwrap().id;
    assert!(act(&mut screen, name, AccessAction::Focus, None));
    screen.key_pressed(clarity_ui::KeyInput::plain(clarity_ui::Key::End));
    screen.text_typed("!");
    assert_eq!(read(&mut screen).part("name").unwrap().value.as_deref(), Some("Ada!"));
    assert_eq!(host.values.borrow().get("customer.name"), Some(&json!("Ada")), "not taken yet");
}

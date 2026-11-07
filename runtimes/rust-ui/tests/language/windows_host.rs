//! The host of one open window's parts over a program's `Program` (C#
//! ViewModelHostTests and UndoScopeTests): parts read and change the
//! window's values by their paths, run its handlers, read its part states,
//! texts and choices, and the host counts when it changed. Only a part the
//! project gave an undo scope makes edits.
//!
//! Left out, as C# alone has them: the property names in Pascal case, the
//! value converted to the type of a view model's property, the command names
//! the C# generator writes (`Digit7`, `RedInk_`) and the generated fetchers;
//! here the program takes the written call and the JSON value as they are.

use super::core_windows_fake;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Level, RefOption, Text, WindowState};
use clarity_ui::markup::{self, HandlerCall};
use clarity_ui::window::{address, Effect, ProgramHost, Running};
use clarity_ui::{AppFiles, PartHost};
use core_windows_fake::{Fake, MAIN};
use serde_json::{json, Map, Value};

const MODEL: &str = r##"{
  "project": {
    "locale": { "language": "en" },
    "translations": { "en": { "orderTotalLabel": "Total" } },
    "files": [{
      "pages": [
        { "name": { "name": "Orders" }, "kind": "window", "title": "Orders",
          "data": [ { "name": { "name": "order" }, "of": { "kind": "record", "entity": { "name": "Order" } } },
                    { "name": { "name": "orders" }, "of": { "kind": "list", "entity": { "name": "Order" } } } ] }
      ],
      "apps": [],
      "enums": [ { "name": { "name": "Status" }, "variants": [ { "name": { "name": "open" }, "label": "Open" }, { "name": { "name": "done" }, "label": "Done" } ] } ],
      "records": [],
      "entities": [
        { "name": { "name": "Order" }, "fields": [
          { "name": { "name": "note" }, "type": { "kind": "text" }, "label": "Note", "modifiers": {} },
          { "name": { "name": "status" }, "type": { "kind": "enum", "name": { "name": "Status" } }, "label": "Status", "modifiers": {} },
          { "name": { "name": "total" }, "type": { "kind": "decimal", "precision": 10, "scale": 2 }, "label": "@orderTotalLabel", "modifiers": { "required": true, "min": { "kind": "int", "value": 0 }, "max": { "kind": "int", "value": 500 } } },
          { "name": { "name": "finished_by" }, "type": { "kind": "ref", "entity": { "name": "Helper" } }, "label": "Finished by", "modifiers": {} },
          { "name": { "name": "count" }, "type": { "kind": "integer" }, "label": "Count", "modifiers": {} }
        ] },
        { "name": { "name": "Helper" }, "fields": [ { "name": { "name": "name" }, "type": { "kind": "text" }, "label": "Name", "modifiers": {} } ] }
      ]
    }]
  }
}"##;

const FILES: AppFiles = AppFiles { windows: &[("Orders", "<Window name=\"Orders\" />"), ("Users", MAIN)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 500.0 };

fn data() -> Map<String, Value> {
    let value = json!({
        "order": { "id": 4, "note": "hello", "status": "open", "finished_by": 2, "total": 0, "count": 1 },
        "orders": [ { "id": 1, "note": "first", "status": "open" }, { "id": 2, "note": "second", "status": "open" } ],
        "shown": 0
    });
    value.as_object().unwrap().clone()
}

/// The host of window Orders with the parts of `markup`, over `fake`.
fn made_with(markup: &str, fake: Fake) -> (Rc<ProgramHost>, Arc<Fake>) {
    let fake = Arc::new(fake);
    let program: Arc<dyn Program> = fake.clone();
    let parts = markup::read(&format!("<Window name=\"Orders\">{markup}</Window>")).unwrap().parts;
    let host = Rc::new(ProgramHost::new(program.clone(), FILES, WindowState::new("Orders", data()), &parts));
    host.set_running(Running::new(program, FILES));
    (host, fake)
}

fn made(markup: &str) -> (Rc<ProgramHost>, Arc<Fake>) {
    made_with(markup, Fake::default())
}

fn edits(fake: &Fake) -> Vec<String> {
    std::mem::take(&mut *fake.edits.lock().unwrap())
}

fn click(handler: &str) -> HandlerCall {
    markup::handler_of("click", handler)
}

#[test]
fn only_a_part_with_an_undo_scope_makes_edits() {
    let _alone = super::one_at_a_time();
    let (host, _) = made(
        r#"<List name="people" at="1,1" span="6" bind="people" />
           <Textbox name="nameBox" at="1,2" span="6" bind="name" undoScope="nameBox" />
           <Accordion name="fold" at="1,3" span="6" rows="3"><Section title="One" /></Accordion>"#,
    );
    assert_eq!(None, host.scope("people"));
    assert_eq!(None, host.scope("fold"));
    assert_eq!(Some("nameBox".to_string()), host.scope("nameBox"));
    // A part the markup does not know (code's own) is its own scope.
    assert_eq!(Some("madeByCode".to_string()), host.scope("madeByCode"));
}

#[test]
fn a_path_reads_the_windows_values() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    assert_eq!(Some(json!("hello")), host.read("order.note"));
    assert_eq!(Some(json!(2)), host.read("order.finished_by"));
    assert_eq!(Some(json!(0)), host.read("shown"));
    assert_eq!(Some(json!("second")), host.read("orders[1].note"));
    assert_eq!(None, host.read("orders[5].note"));
    assert_eq!(None, host.read("order.nothing"));
    assert_eq!(None, host.read("nothing.at.all"));
    assert_eq!(Some("Status".to_string()), host.type_of("order.status"));
}

#[test]
fn a_table_cell_edit_writes_the_field_of_the_record_at_its_place() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Table name="grid" bind="orders" columns="note, status" edit="note, status" undoScope="grid" />"#);
    host.write("grid", "orders[1].note", json!("changed"));
    host.write("grid", "orders[0].status", json!("done"));
    assert_eq!(Some(json!("changed")), host.read("orders[1].note"));
    assert_eq!(Some(json!("done")), host.read("orders[0].status"));
    // It is one edit of the table, which Ctrl+Z takes back.
    host.flush();
    assert_eq!(vec!["grid grid None"], edits(&fake));
    fake.calls();
    host.undo("grid", true);
    assert_eq!(vec!["undo grid true"], fake.calls());
}

#[test]
fn a_list_gives_its_rows() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    assert_eq!(2, host.rows("orders").len());
    assert!(host.rows("order.note").is_empty());
    assert!(host.rows("nothing").is_empty());
}

#[test]
fn a_write_is_the_persons_edit_which_undo_takes_back() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Textbox name="note" bind="order.note" undoScope="form" />"#);
    host.write("note", "order.note", json!("typed"));
    assert_eq!(Some(json!("typed")), host.read("order.note"));
    host.flush();
    assert_eq!(vec!["note form None"], edits(&fake));
    // Writing the same value again is no edit.
    host.write("note", "order.note", json!("typed"));
    host.flush();
    assert!(edits(&fake).is_empty());
}

#[test]
fn a_part_the_markup_gives_no_undo_scope_makes_no_edit() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Number name="shownBox" bind="shown" />"#);
    let before = host.changes();
    host.write("shownBox", "shown", json!(12));
    assert_eq!(Some(json!(12)), host.read("shown"));
    assert!(host.changes() > before);
    host.flush();
    assert!(edits(&fake).is_empty());
    let (scoped, fake) = made(r#"<Number name="shownBox" bind="shown" undoScope="shownBox" />"#);
    scoped.write("shownBox", "shown", json!(12));
    scoped.flush();
    assert_eq!(vec!["shownBox shownBox None"], edits(&fake));
}

#[test]
fn a_part_with_no_undo_changes_the_value_without_an_edit() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Textbox name="note" bind="order.note" noUndo="true" undoScope="note" />"#);
    host.write("note", "order.note", json!("typed"));
    host.flush();
    assert_eq!(Some(json!("typed")), host.read("order.note"));
    assert!(edits(&fake).is_empty());
}

#[test]
fn a_handler_runs_through_the_program_and_its_reply_is_the_windows() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made("");
    let before = host.changes();
    assert!(host.run(Some("approveButton"), &click("approveClicked")));
    assert_eq!(vec!["dispatch approveClicked"], fake.calls());
    assert!(host.changes() > before);
    // A call with values goes as Clarity writes it.
    assert!(host.run(Some("seven"), &click("digit(7)")));
    assert!(host.run(Some("half"), &click(r#"scale(0.5, "half")"#)));
    assert_eq!(vec!["dispatch digit(7)", r#"dispatch scale(0.5, "half")"#], fake.calls());
}

#[test]
fn a_change_handler_runs_inside_the_edit() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Textbox name="note" bind="order.note" onchange="noteChanged" undoScope="note" />"#);
    host.write("note", "order.note", json!("typed"));
    host.run(Some("note"), &markup::handler_of("change", "noteChanged"));
    // One edit, with its handler: what the handler changes is taken back with it.
    assert_eq!(vec![r#"note note Some("noteChanged")"#], edits(&fake));
    host.flush();
    assert!(edits(&fake).is_empty());
}

#[test]
fn a_click_that_names_a_window_opens_it_and_a_built_in_target_runs() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made("");
    assert!(host.run(Some("usersButton"), &click("Users")));
    assert_eq!(vec![Effect::Open("Users".into(), Map::new())], host.take_effects());
    for (target, effect) in [("close", Effect::Close), ("quit", Effect::Quit), ("keys", Effect::Keys)] {
        assert!(host.run(Some("b"), &click(target)));
        assert_eq!(vec![effect], host.take_effects());
    }
    // None of them is the program's handler.
    assert!(fake.calls().is_empty());
}

#[test]
fn a_handler_says_whether_it_ran_or_refused_and_the_window_goes_on() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    assert!(!host.run(Some("next"), &click("refuse")));
    match host.take_effects().as_slice() {
        [Effect::Say(said)] => assert_eq!((Level::Error, "A title first".to_string()), (said[0].level, said[0].text.show())),
        other => panic!("refusing says why: {other:?}"),
    }
    assert!(host.run(Some("approveButton"), &click("approveClicked")));
    assert!(host.take_effects().is_empty());
}

#[test]
fn a_part_state_is_the_windows_by_its_name_and_the_host_counts_changes() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    assert_eq!(None, host.state(None));
    let before = host.changes();
    clarity_ui::parts::change_state(&*host, "approveButton", |s| s.enabled = Some(false));
    assert_eq!(Some(false), host.state(Some("approveButton")).unwrap().enabled);
    assert_eq!(before + 1, host.changes());
    host.write("x", "shown", json!(3));
    assert_eq!(before + 2, host.changes());
}

#[test]
fn a_text_is_translated_by_its_key_and_plain_text_stays_with_its_braces_single_when_translated() {
    let _alone = super::one_at_a_time();
    clarity_runtime::i18n::load(MODEL);
    let (host, _) = made("");
    assert_eq!("", host.text(None));
    assert_eq!("plain words", host.text(Some("plain words")));
    assert_eq!("Total", host.text(Some("@orderTotalLabel")));
    assert_eq!("someMissingKey", host.text(Some("@someMissingKey")));
    assert_eq!("Type {name}", host.text(Some("@Type {{name}}")));
    assert_eq!("Type {{name}}", host.text(Some("Type {{name}}")));
}

#[test]
fn an_enum_offers_its_values_and_a_reference_its_records_but_a_plain_integer_none() {
    let _alone = super::one_at_a_time();
    let refs = vec![RefOption { id: 2, text: "Cy".into(), fields: Map::new() }];
    let (host, fake) = made_with("", Fake { refs, ..Fake::default() });
    let labels = |path: &str| host.choices(path).into_iter().map(|c| c.label).collect::<Vec<_>>();
    assert_eq!(vec!["Open", "Done"], labels("order.status"));
    assert_eq!(json!("done"), host.choices("order.status")[1].value);
    assert_eq!(vec!["Cy"], labels("order.finished_by"));
    assert_eq!(vec!["Cy"], labels("orders.finished_by"));
    assert_eq!(vec!["refs Helper", "refs Helper"], fake.calls());
    assert!(labels("order.note").is_empty());
    assert!(labels("order.count").is_empty());
    assert!(labels("orders.count").is_empty());
    assert!(fake.calls().is_empty());
}

#[test]
fn a_typed_dropdown_offers_its_fields_values_and_a_search_one_its_records_with_their_cells() {
    let _alone = super::one_at_a_time();
    let found = json!([{ "id": 2, "text": "Cy", "fields": { "name": "Cy", "role": "lead" } }]);
    let refs = vec![RefOption { id: 2, text: "Cy".into(), fields: Map::new() }];
    let (host, fake) = made_with("", Fake { values: vec!["first".into(), "second".into()], found, refs, ..Fake::default() });
    let part = |line: &str| markup::read(&format!("<Window name=\"Orders\">{line}</Window>")).unwrap().parts.remove(0);
    let typed = part(r#"<Dropdown at="1,1" bind="order.note" typed="true" />"#);
    let offered: Vec<String> = host.offered(&typed, "order.note").into_iter().map(|c| c.label).collect();
    assert_eq!(vec!["first", "second"], offered);
    let search = part(r#"<Dropdown at="1,1" bind="order.finished_by" search="true" show="name, role" />"#);
    let records = host.offered(&search, "order.finished_by");
    assert_eq!((json!(2), Some(vec!["Cy".to_string(), "lead".to_string()])), (records[0].value.clone(), records[0].cells.clone()));
    // A search without fields to show picks from the reference's own choices.
    let plain = part(r#"<Dropdown at="1,1" bind="order.finished_by" search="true" />"#);
    assert_eq!(vec!["Cy"], host.offered(&plain, "order.finished_by").into_iter().map(|c| c.label).collect::<Vec<_>>());
    assert_eq!(vec!["values Order.note", "find search Helper name,role", "refs Helper"], fake.calls());
}

#[test]
fn a_stored_field_is_fetched_through_the_program() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made_with("", Fake { bytes: Some(vec![1]), ..Fake::default() });
    assert_eq!(Some(vec![1]), host.fetch("order.note", None));
    assert_eq!(vec!["bytes Order 4 note"], fake.calls());
    // A path that is no field of a record fetches nothing.
    assert_eq!(None, host.fetch("order", None));
    assert_eq!(None, host.fetch("nowhere.note", None));
}

#[test]
fn what_a_part_says_is_among_the_windows_messages() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    host.say(Level::Info, Text::plain("saved"));
    match host.take_effects().as_slice() {
        [Effect::Say(said)] => assert_eq!("saved", said[0].text.show()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_fields_facts_are_its_records_types() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    let total = host.field("order.total").unwrap();
    assert_eq!(("@orderTotalLabel", true), (total.label.as_str(), total.required));
    assert_eq!(Some("0".to_string()), host.field("orders[1].total").unwrap().min);
    assert_eq!(Some("500".to_string()), host.field("orders.total").unwrap().max);
    assert_eq!(Some("Helper".to_string()), host.field("orders.finished_by").unwrap().refers);
    assert_eq!(None, host.field("shown"));
    assert_eq!(None, host.field("nowhere.total"));
}

#[test]
fn an_unbound_parts_own_text_is_an_edit_undo_takes_back() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<Textbox name="note" undoScope="note" /><Textbox name="free" />"#);
    clarity_ui::parts::change_state(&*host, "note", |s| s.text = Some("typed".into()));
    host.edited("note");
    host.flush();
    assert_eq!(vec!["note note None"], edits(&fake));
    // One without an undo scope changes without an edit.
    host.edited("free");
    host.flush();
    assert!(edits(&fake).is_empty());
}

#[test]
fn an_address_goes_to_the_hosts_opener() {
    let _alone = super::one_at_a_time();
    let opened = Rc::new(RefCell::new(Vec::new()));
    let kept = opened.clone();
    address::set_opener(Some(Rc::new(move |a: &str| kept.borrow_mut().push(a.to_string()))));
    let (host, _) = made("");
    host.open_address("https://example.org");
    address::set_opener(None);
    assert_eq!(vec!["https://example.org"], opened.borrow().clone());
    assert_eq!(None, host.resource("not-there.png"));
    assert_eq!(None, host.icon_path("not-there"));
}

#[test]
fn the_system_opens_only_web_pages_mail_and_phone_numbers_in_its_own_program() {
    let _alone = super::one_at_a_time();
    use address::{command_for, Os};
    assert!(address::opens("https://example.org"));
    assert!(address::opens("MAILTO:ada@example.org"));
    assert!(address::opens("tel:+31201234567"));
    assert!(!address::opens("file:///etc/passwd"));
    assert!(!address::opens("https://example.org/\nrm"));
    assert_eq!(vec!["https://x.org".to_string()], command_for(Os::Linux, "https://x.org").unwrap().arguments);
    assert_eq!("xdg-open", command_for(Os::Linux, "https://x.org").unwrap().program);
    assert_eq!("open", command_for(Os::MacOs, "https://x.org").unwrap().program);
    // On Windows an address with `&` reaches the shell's handler whole.
    let windows = command_for(Os::Windows, "https://x.org/?a=1&b=2").unwrap();
    assert_eq!(Some(&"https://x.org/?a=1&b=2".to_string()), windows.arguments.last());
    assert_eq!(None, command_for(Os::Linux, "javascript:alert(1)"));
}

/// C# SaidTextTests: a text value a handler set from a key shows again in
/// the new language, as long as nothing else changed it since.
#[test]
fn a_said_key_shows_again_in_the_new_language_unless_the_value_changed_since() {
    let _alone = super::one_at_a_time();
    const SAID: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {
        "en": {"orderTotalLabel": "Total", "yourMove": "Your move", "youWin": "You win"},
        "nl": {"yourMove": "Jouw zet", "youWin": "Je wint"}}}}"#;
    clarity_runtime::i18n::load(SAID);
    clarity_runtime::i18n::set_language(None);
    let (host, _) = made("");
    let said = |data: &str, key: &str| {
        let mut ui = clarity_runtime::ui::Ui::default();
        ui.set_shown(data, clarity_runtime::ui::Text::key(key));
        host.apply(ui.reply("Orders", data_now(&host)));
    };
    said("status", "yourMove");
    said("result", "youWin");
    assert_eq!(Some(json!("Your move")), host.read("status"));
    host.write("x", "result", json!("typed by the person"));
    clarity_runtime::i18n::set_language(Some("nl"));
    host.reshow();
    clarity_runtime::i18n::set_language(None);
    assert_eq!(Some(json!("Jouw zet")), host.read("status"));
    assert_eq!(Some(json!("typed by the person")), host.read("result"));
    clarity_runtime::i18n::load(MODEL);
}

fn data_now(host: &ProgramHost) -> Map<String, Value> {
    let mut data = data();
    for name in ["status", "result"] {
        if let Some(value) = host.read(name) {
            data.insert(name.to_string(), value);
        }
    }
    data
}

/// `on click -> Users with order`: the opened window gets this window's data of
/// that name; a name this window does not have is left out (Users starts it new).
#[test]
fn a_route_opens_its_window_with_the_data_it_names() {
    let _alone = super::one_at_a_time();
    let (host, _) = made("");
    let call = HandlerCall::new("click", "Users", vec!["order".to_string(), "missing".to_string()]);
    assert!(host.run(Some("usersButton"), &call));
    let mut with = Map::new();
    with.insert("order".into(), data()["order"].clone());
    assert_eq!(vec![Effect::Open("Users".into(), with)], host.take_effects());
}

/// Back goes to the row the person chose before, as a choice of theirs: its `select` handler runs; Forward comes back.
#[test]
fn back_and_forward_choose_the_rows_chosen_before_and_run_their_handlers() {
    let _alone = super::one_at_a_time();
    let (host, fake) = made(r#"<List name="orders" at="1,1" span="6" rows="5" bind="orders" show="note" onselect="orderPicked" />"#);
    clarity_ui::PartHost::select(&*host, "orders", Some(0));
    host.note_place();
    clarity_ui::PartHost::select(&*host, "orders", Some(1));
    host.note_place();
    fake.calls();
    assert!(host.can_go(true));
    assert!(host.run(Some("backButton"), &click("back")));
    assert_eq!(Some(0), clarity_ui::PartHost::selected(&*host, "orders"));
    assert!(fake.calls().iter().any(|c| c.contains("orderPicked")), "the row's select handler runs");
    host.note_place();
    host.go(false);
    assert_eq!(Some(1), clarity_ui::PartHost::selected(&*host, "orders"));
}

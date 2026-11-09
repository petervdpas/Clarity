use clarity_spec::edit::Edit;
use clarity_spec::templates::PageTemplate;
use clarity_spec::{Project, load_dir, load_dir_with};
use std::path::PathBuf;

fn example_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop"))
}

/// What adding a window from a template beside counter.clarity writes, as
/// the IDE writes it: the window's and its view's files (`.cxaml`), the
/// module with their code; and the project loaded with them and their keys.
struct Added {
    window: String,
    view: String,
    module: String,
    after: Project,
}

fn add_window(template: PageTemplate) -> Added {
    let dir = example_dir();
    let path = dir.join("counter.clarity");
    let project = load_dir(&dir).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let edit = Edit::AddWindow { name: "FreshWindow".into(), template };
    let added = clarity_spec::cxaml::added(&project, &path, &source, &edit).unwrap().unwrap_or_else(|e| panic!("{e}"));
    // The texts the template wrote are keys, which the language files get.
    let languages = clarity_spec::keyed::plan_keys(&dir, "en", &added.keys).unwrap();
    let mut replaced: Vec<(&std::path::Path, &str)> = vec![(path.as_path(), added.module.as_str())];
    replaced.extend(added.files.iter().map(|(p, t)| (p.as_path(), t.as_str())));
    replaced.extend(languages.iter().map(|(p, _, t)| (p.as_path(), t.as_str())));
    let after = clarity_spec::load_dir_with_files_in(&dir, &replaced, &[]).unwrap();
    let file = |name: &str| added.files.iter().find(|(p, _)| p.ends_with(format!("design/{name}.cxaml"))).map(|(_, t)| t.clone()).unwrap_or_default();
    Added { window: file("FreshWindow"), view: file("FreshWindowView"), module: added.module.clone(), after }
}

#[test]
fn every_template_gives_a_clean_window() {
    for template in [
        PageTemplate::Blank,
        PageTemplate::Form { entity: "Customer".into() },
        PageTemplate::Browse { entity: "Order".into() },
    ] {
        let added = add_window(template.clone());
        assert!(added.window.contains("<Window class=\"counter.FreshWindow\" title=\"@freshWindowTitle\""), "{}", added.window);
        assert!(added.after.diagnostics.is_empty(), "{template:?}: {:#?}", added.after.diagnostics);
    }
}

#[test]
fn a_form_has_a_label_and_input_per_field() {
    let page = add_window(PageTemplate::Form { entity: "Customer".into() }).view;
    assert!(page.contains("  <Data name=\"customer\" type=\"Customer\" />\n"), "{page}");
    assert!(page.contains("\n  <Form name=\"customerForm\" at=\"1,1\" span=\"12\" rows=\"12\" bind=\"customer\">\n"), "{page}");
    assert!(
        page.contains("    <Label name=\"nameLabel\" at=\"1,1\" span=\"3\" caption=\"@customerNameLabel\" for=\"name\" />\n    <Textbox name=\"name\" at=\"4,1\" span=\"6\" bind=\"customer.name\" />\n"),
        "{page}"
    );
    assert!(page.contains("    <Checkbox name=\"active\" at=\"4,"), "{page}");
    // A field's help becomes a help text next to its input.
    assert!(page.contains("    <Help name=\"vatNumberHelp\" at=\"8,4\" span=\"5\" for=\"vatNumber\" />\n"), "{page}");
    // Text without a maximum length gets a memo.
    assert!(
        page.contains("    <Memo name=\"notes\" at=\"4,8\" span=\"6\" rows=\"3\" bind=\"customer.notes\" />\n    <Help name=\"notesHelp\" at=\"10,8\" span=\"3\" for=\"notes\" />\n"),
        "{page}"
    );
    assert!(page.contains("  <Button name=\"save\" at=\"9,14\" span=\"2\" caption=\"@saveCaption\" default=\"true\" />\n"), "{page}");
    assert!(page.contains("  <Button name=\"close\" at=\"11,14\" span=\"2\" caption=\"@closeCaption\" cancel=\"true\" />"), "{page}");
}

#[test]
fn a_browse_has_a_filter_a_table_and_buttons() {
    let page = add_window(PageTemplate::Browse { entity: "Customer".into() }).view;
    assert!(page.contains("  <Data name=\"customers\" type=\"list&lt;Customer&gt;\" />\n"), "{page}");
    assert!(
        page.contains("  <Label name=\"nameFilterLabel\" at=\"1,1\" span=\"2\" caption=\"@customerNameLabel\" for=\"nameFilter\" />\n  <Textbox name=\"nameFilter\" at=\"3,1\" span=\"4\" onchange=\"filterChanged\" />\n"),
        "{page}"
    );
    assert!(page.contains("  <Table name=\"customersTable\" at=\"1,2\" span=\"12\" rows=\"8\" bind=\"customers\" columns=\"name, email, kind, vatNumber, creditLimit\""), "{page}");
    assert!(page.contains("caption=\"@openCaption\""), "{page}");
}

#[test]
fn templates_need_a_known_entity_and_a_free_name() {
    let dir = example_dir();
    let project = load_dir(&dir).unwrap();
    let path = dir.join("counter.clarity");
    let add = |name: &str, entity: &str| clarity_spec::cxaml::added(&project, &path, "module counter\n", &Edit::AddWindow { name: name.into(), template: PageTemplate::Form { entity: entity.into() } }).unwrap();
    assert!(add("X", "Nope").unwrap_err().contains("'Nope' is not an entity"));
    let taken = project.pages().next().unwrap().name.name.clone();
    assert!(add(&taken, "Customer").unwrap_err().contains("already declared"));
    // On a module's text, a new window is no edit: it is files of its own.
    assert!(clarity_spec::edit::apply("module m\n", &Edit::AddWindow { name: "X".into(), template: PageTemplate::Blank }).is_err());
}

#[test]
fn a_browse_window_filters_and_refreshes_when_a_form_closes() {
    let added = add_window(PageTemplate::Browse { entity: "Order".into() });
    // Window, content, view: the window keeps its events, the view its data, parts and their handlers.
    assert!(added.window.contains("<Window class=\"counter.FreshWindow\" title=\"@freshWindowTitle\" onreturn=\"windowReturned\">\n  <Content name=\"main\" at=\"1,1\" span=\"12\" rows=\"10\">\n    <View name=\"FreshWindowView\" />\n"), "{}", added.window);
    assert!(added.view.contains("  <Data name=\"orders\" type=\"list&lt;Order&gt;\" />\n"), "{}", added.view);
    assert!(
        added.module.trim_end().ends_with(
            "\ncode FreshWindow\n    handler windowReturned\n        // A window opened from here may have changed the records.\n        filterChanged()\n\ncode FreshWindowView\n    handler filterChanged\n        orders = find o in Order where o.number contains numberFilter.text order by o.number"
        ),
        "{}",
        added.module
    );
    let problems: Vec<_> = added.after.diagnostics.iter().map(|d| &d.message).collect();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_dropped_field_brings_its_label_and_help() {
    // The customer card's view is a window's view in its own file, as the designer edits it.
    let dir = example_dir();
    let path = dir.join("design/CustomerCardView.cxaml");
    let project = load_dir(&dir).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCardView")
        .unwrap();
    let edit = Edit::PlaceField {
        container: page.span.start,
        col: 1,
        row: 20,
        room: 12,
        bind: "customer.vatNumber".into(),
    };
    let placed = |text: &str| -> Vec<(String, u32, u32, u32)> {
        let (page, d) = clarity_spec::cxaml::read_page(text, "");
        assert!(d.is_empty(), "{d:#?}");
        page.unwrap().widgets.iter().filter(|w| w.props.row >= 20).map(|w| (w.props.name.clone().unwrap_or_default(), w.props.col, w.props.row, w.props.width)).collect()
    };
    let outcome = clarity_spec::cxaml::apply(Some(&project), &source, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(placed(&outcome.text), [("vatNumberLabel".to_string(), 1, 20, 3), ("vatNumber".to_string(), 4, 20, 4), ("vatNumberHelp".to_string(), 8, 20, 5)], "{}", outcome.text);
    assert!(outcome.text.contains("caption=\"@customerVatNumberLabel\""), "{}", outcome.text);
    let after = load_dir_with(&dir, &path, &outcome.text).unwrap();
    assert!(after.diagnostics.is_empty(), "{:#?}", after.diagnostics);

    // Dropped again, the input gets a free name and the label follows it.
    let again = clarity_spec::cxaml::apply(Some(&after), &outcome.text, &edit_at(&after, 21)).unwrap_or_else(|e| panic!("{e}"));
    assert!(placed(&again.text).iter().any(|(name, _, row, _)| name == "vatNumber1" && *row == 21), "{}", again.text);

    // Without room for a label, only the input is placed.
    let narrow = Edit::PlaceField {
        container: page.span.start,
        col: 10,
        row: 20,
        room: 3,
        bind: "customer.since".into(),
    };
    let outcome = clarity_spec::cxaml::apply(Some(&project), &source, &narrow).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(placed(&outcome.text), [("since".to_string(), 10, 20, 3)], "{}", outcome.text);
}

fn edit_at(project: &Project, row: u32) -> Edit {
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCardView")
        .unwrap();
    Edit::PlaceField {
        container: page.span.start,
        col: 1,
        row,
        room: 12,
        bind: "customer.vatNumber".into(),
    }
}

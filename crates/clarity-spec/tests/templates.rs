use clarity_spec::edit::{Edit, apply, apply_in};
use clarity_spec::templates::PageTemplate;
use clarity_spec::{Project, load_dir, load_dir_with};
use std::path::PathBuf;

fn example_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop"))
}

/// Adds a window from a template to desk.clarity and loads the project
/// with the result.
fn add_window(template: PageTemplate) -> (String, Project) {
    let dir = example_dir();
    let path = dir.join("desk.clarity");
    let project = load_dir(&dir).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let edit = Edit::AddWindow {
        name: "FreshWindow".into(),
        template,
    };
    let outcome = apply_in(&project, &source, &edit).unwrap_or_else(|e| panic!("{e}"));
    let after = load_dir_with(&dir, &path, &outcome.text).unwrap();
    (outcome.text, after)
}

#[test]
fn every_template_gives_a_clean_window() {
    for template in [
        PageTemplate::Blank,
        PageTemplate::Form {
            entity: "Customer".into(),
        },
        PageTemplate::Browse {
            entity: "Order".into(),
        },
    ] {
        let (text, after) = add_window(template.clone());
        assert!(
            text.contains("\n\npage FreshWindow: window\n    title \"Fresh window\""),
            "{text}"
        );
        assert!(
            after.diagnostics.is_empty(),
            "{template:?}: {:#?}",
            after.diagnostics
        );
    }
}

#[test]
fn a_form_has_a_label_and_input_per_field() {
    let (text, _) = add_window(PageTemplate::Form {
        entity: "Customer".into(),
    });
    let page = &text[text.find("page FreshWindow").unwrap()..];
    assert!(page.contains("    data customer: Customer\n"), "{page}");
    assert!(
        page.contains("\n    form name customerForm at 1,1 span 12 rows 10 bind customer\n"),
        "{page}"
    );
    assert!(
        page.contains(
            "        label \"Name\" name nameLabel at 1,1 span 3 for name\n        textbox name name at 4,1 span 6 bind customer.name\n"
        ),
        "{page}"
    );
    assert!(page.contains("        checkbox name active at 4,"), "{page}");
    // A field's help becomes a help text next to its input.
    assert!(
        page.contains("        help name vatNumberHelp at 8,4 span 5 for vatNumber\n"),
        "{page}"
    );
    // Text without a maximum length gets a memo.
    assert!(
        page.contains("        memo name notes at 4,8 span 6 rows 3 bind customer.notes\n        help name notesHelp at 10,8 span 3 for notes\n"),
        "{page}"
    );
    assert!(page.contains("    button \"Save\" name save at 9,12 span 2 default\n"), "{page}");
    assert!(page.contains("    button \"Close\" name close at 11,12 span 2 cancel"), "{page}");
}

#[test]
fn a_browse_has_a_filter_a_table_and_buttons() {
    let (text, _) = add_window(PageTemplate::Browse {
        entity: "Customer".into(),
    });
    let page = &text[text.find("page FreshWindow").unwrap()..];
    assert!(page.contains("    data customers: list<Customer>\n"), "{page}");
    assert!(
        page.contains(
            "    label \"Name\" name nameFilterLabel at 1,1 span 2 for nameFilter\n    textbox name nameFilter at 3,1 span 4 on change: filterChanged\n"
        ),
        "{page}"
    );
    assert!(
        page.contains("table name customersTable at 1,2 span 12 rows 8 bind customers columns name, email, kind, vat_number, credit_limit"),
        "{page}"
    );
    assert!(page.contains("button \"Open\""), "{page}");
}

#[test]
fn templates_need_the_project_and_a_known_entity() {
    let edit = Edit::AddWindow {
        name: "X".into(),
        template: PageTemplate::Form {
            entity: "Customer".into(),
        },
    };
    assert!(
        apply("module m\n", &edit)
            .unwrap_err()
            .0
            .contains("needs the whole project")
    );
    let project = load_dir(&example_dir()).unwrap();
    let edit = Edit::AddWindow {
        name: "X".into(),
        template: PageTemplate::Form {
            entity: "Nope".into(),
        },
    };
    assert!(
        apply_in(&project, "module m\n", &edit)
            .unwrap_err()
            .0
            .contains("'Nope' is not an entity")
    );
}

#[test]
fn a_browse_window_filters_and_refreshes_when_a_form_closes() {
    let (text, after) = add_window(PageTemplate::Browse {
        entity: "Order".into(),
    });
    assert!(text.contains("    data orders: list<Order>\n    on return: windowReturned\n"), "{text}");
    assert!(
        text.ends_with(
            "\ncode FreshWindow\n    handler windowReturned\n        // A window opened from here may have changed the records.\n        filterChanged()\n\n    handler filterChanged\n        orders = find o in Order where o.number contains numberFilter.text order by o.number\n"
        ),
        "{text}"
    );
    let problems: Vec<_> = after.diagnostics.iter().map(|d| &d.message).collect();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_dropped_field_brings_its_label_and_help() {
    let dir = example_dir();
    let path = dir.join("desk.clarity");
    let project = load_dir(&dir).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCard")
        .unwrap();
    let edit = Edit::PlaceField {
        container: page.span.start,
        col: 1,
        row: 20,
        room: 12,
        bind: "customer.vat_number".into(),
    };
    let outcome = apply_in(&project, &source, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        outcome.text.contains(
            "    label \"Vat number\" name vatNumberLabel at 1,20 span 3 for vatNumber\n    textbox name vatNumber at 4,20 span 4 bind customer.vat_number\n    help name vatNumberHelp at 8,20 span 5 for vatNumber\n"
        ),
        "{}",
        outcome.text
    );
    let after = load_dir_with(&dir, &path, &outcome.text).unwrap();
    assert!(after.diagnostics.is_empty(), "{:#?}", after.diagnostics);

    // Dropped again, the input gets a free name and the label follows it.
    let again = apply_in(&after, &outcome.text, &edit_at(&after, 21)).unwrap_or_else(|e| panic!("{e}"));
    assert!(again.text.contains("for vatNumber1\n    textbox name vatNumber1 at 4,21"), "{}", again.text);

    // Without room for a label, only the input is placed.
    let narrow = Edit::PlaceField {
        container: page.span.start,
        col: 10,
        row: 20,
        room: 3,
        bind: "customer.since".into(),
    };
    let outcome = apply_in(&project, &source, &narrow).unwrap_or_else(|e| panic!("{e}"));
    assert!(outcome.text.contains("\n    date name since at 10,20 span 3 bind customer.since\n"), "{}", outcome.text);
}

fn edit_at(project: &Project, row: u32) -> Edit {
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCard")
        .unwrap();
    Edit::PlaceField {
        container: page.span.start,
        col: 1,
        row,
        room: 12,
        bind: "customer.vat_number".into(),
    }
}

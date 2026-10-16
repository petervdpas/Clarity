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
        page.contains("    label \"Name\" at 1,1 span 3\n    textbox name name at 4,1 span 6 bind customer.name\n"),
        "{page}"
    );
    assert!(page.contains("checkbox name active at 4,"), "{page}");
    assert!(page.contains("button \"Save\" name save"), "{page}");
}

#[test]
fn a_browse_has_a_filter_a_table_and_buttons() {
    let (text, _) = add_window(PageTemplate::Browse {
        entity: "Customer".into(),
    });
    let page = &text[text.find("page FreshWindow").unwrap()..];
    assert!(page.contains("    data customers: list<Customer>\n"), "{page}");
    assert!(
        page.contains("    label \"Name\" at 1,1 span 2\n    textbox name name_filter at 3,1 span 4\n"),
        "{page}"
    );
    assert!(
        page.contains("table name customers_table at 1,2 span 12 rows 8 bind customers columns name, email, kind, vat_number, credit_limit"),
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
fn a_browse_window_reloads_its_list_when_a_form_closes() {
    let (text, after) = add_window(PageTemplate::Browse {
        entity: "Order".into(),
    });
    assert!(text.contains("    data orders: list<Order>\n    on return: window_returned\n"), "{text}");
    assert!(
        text.ends_with("\ncode FreshWindow\n    handler window_returned\n        reload orders\n"),
        "{text}"
    );
    let problems: Vec<_> = after.diagnostics.iter().map(|d| &d.message).collect();
    assert!(problems.is_empty(), "{problems:?}");
}

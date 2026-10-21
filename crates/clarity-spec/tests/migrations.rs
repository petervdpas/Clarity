//! Migrations (D1b): steps written in Clarity, the schema they build when
//! replayed, and the changes in the data dictionary that no step covers
//! yet.

use clarity_spec::migrate::{self, Rename};
use clarity_spec::model::FieldType;
use clarity_spec::{Project, Severity, load_with_migrations};

const SHOP: &str = "module shop

enum Kind
    Private
    Business

entity Customer
    name: text(80), required
    email: email, unique
    kind: Kind, default Private
    credit_limit: money, default 0

entity Order
    number: text(12), unique
    customer: ref Customer
    total: money
";

/// The schema SHOP had before: no credit limit, a vat number, a Note.
const FIRST: &str = "// The first step.
migration \"Customers and orders\"
    add entity Customer
        name: text(80)
        email: email, unique
        kind: Kind, default Private
        vat_number: text(14)
    add entity Order
        number: text(12), unique
        customer: ref Customer
        total: money
    add entity Note
        text: text
";

fn load(migrations: &[(&str, &str)]) -> Project {
    load_with_migrations([("shop.clarity", SHOP)], migrations.iter().copied())
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.clone())
        .collect()
}

/// The pending changes as the lines of a step.
fn pending(project: &Project, renames: &[Rename]) -> Vec<String> {
    migrate::pending(project, "main", renames).iter().map(migrate::op_text).collect()
}

#[test]
fn replaying_the_steps_gives_the_schema() {
    let project = load(&[("0001.clarity", FIRST)]);
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    let schema = migrate::schema(&project, "main");
    let names: Vec<&str> = schema.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Customer", "Order", "Note"]);
    let customer = schema.table("Customer").unwrap();
    let email = customer.column("email").unwrap();
    assert!(email.unique);
    assert_eq!(email.ty, FieldType::Email);
    assert!(customer.column("kind").unwrap().default.is_some());
}

#[test]
fn every_step_changes_the_schema_it_finds() {
    let second = "migration \"Tidy up\"
    rename field Customer.vat_number to vat
    add field Customer.credit_limit: money, default 0
    change field Order.number: text(20)
    remove unique Order.number
    remove field Customer.vat
    rename entity Note to Remark
    remove entity Remark
";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", second)]);
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    let schema = migrate::schema(&project, "main");
    let customer = schema.table("Customer").unwrap();
    let fields: Vec<&str> = customer.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(fields, ["name", "email", "kind", "credit_limit"]);
    let number = schema.table("Order").unwrap().column("number").unwrap();
    assert_eq!(number.ty, FieldType::Text { max_length: Some(20) });
    assert!(!number.unique);
    assert!(schema.table("Note").is_none() && schema.table("Remark").is_none());
}

#[test]
fn a_step_that_does_not_fit_is_an_error() {
    let bad = "migration \"Broken\"
    rename field Customer.nickname to nick
    add field Order.total: money
    remove entity Invoice
    add entity Customer
        name: text
";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", bad)]);
    let errors = messages(&project, Severity::Error);
    for expected in [
        "Customer has no field 'nickname'",
        "Order already has a field 'total'",
        "there is no entity 'Invoice'",
        "there is already an entity 'Customer'",
    ] {
        assert!(errors.iter().any(|e| e.contains(expected)), "missing {expected:?} in {errors:#?}");
    }
    let unknown = "migration \"Odd\"\n    paint field Customer.name\n";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", unknown)]);
    assert!(messages(&project, Severity::Error).iter().any(|e| e.contains("unknown migration step 'paint'")));
}

#[test]
fn dictionary_changes_not_in_a_step_are_pending() {
    let project = load(&[("0001.clarity", FIRST)]);
    // Without decisions, a field that went and a field that came are a removal and an addition.
    assert_eq!(
        pending(&project, &[]),
        [
            "add field Customer.credit_limit: money, default 0",
            "remove field Customer.vat_number",
            "remove entity Note",
        ]
    );
    let warnings = messages(&project, Severity::Warning);
    assert!(
        warnings.iter().any(|w| w == "Customer has changes that are not in a migration yet: add field credit_limit, remove field vat_number"),
        "{warnings:#?}"
    );
    assert!(warnings.iter().any(|w| w == "entity 'Note' is removed, but not in a migration yet"), "{warnings:#?}");
}

#[test]
fn a_project_in_step_with_its_migrations_has_nothing_pending() {
    let first = migrate::step_text("Customers and orders", &migrate::pending(&load(&[]), "main", &[]));
    let project = load(&[("0001.clarity", first.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(migrate::pending(&project, "main", &[]).is_empty());
    assert!(first.starts_with("// Written by Clarity"), "{first}");
    assert!(first.contains("\nmigration \"Customers and orders\"\n    add entity Customer\n        name: text(80)\n"), "{first}");
}

#[test]
fn renames_are_proposed_and_decided() {
    let renamed = SHOP.replace("    credit_limit: money, default 0\n", "    vat: text(14)\n");
    let project = load_with_migrations([("shop.clarity", renamed.as_str())], [("0001.clarity", FIRST)]);
    let proposed = migrate::proposed_renames(&project, "main");
    assert_eq!(
        proposed,
        [Rename {
            entity: Some("Customer".into()),
            from: "vat_number".into(),
            to: "vat".into(),
        }]
    );
    let taken = pending(&project, &proposed);
    assert_eq!(taken[0], "rename field Customer.vat_number to vat");
    // Declined, the old field goes and a new one comes.
    let declined = pending(&project, &[]);
    assert!(declined.contains(&"remove field Customer.vat_number".to_string()), "{declined:#?}");
    assert!(declined.contains(&"add field Customer.vat: text(14)".to_string()), "{declined:#?}");
}

#[test]
fn an_entity_rename_is_proposed_when_its_fields_stay() {
    let renamed = SHOP.replace("entity Order\n", "entity SalesOrder\n").replace("    credit_limit: money, default 0\n", "");
    let project = load_with_migrations([("shop.clarity", renamed.as_str())], [("0001.clarity", FIRST)]);
    let proposed = migrate::proposed_renames(&project, "main");
    assert!(
        proposed.contains(&Rename {
            entity: None,
            from: "Order".into(),
            to: "SalesOrder".into(),
        }),
        "{proposed:#?}"
    );
    assert_eq!(pending(&project, &proposed)[0], "rename entity Order to SalesOrder");
}

#[test]
fn changes_that_can_lose_data_are_marked() {
    let changed = SHOP.replace("number: text(12), unique", "number: integer, unique");
    let project = load_with_migrations([("shop.clarity", changed.as_str())], [("0001.clarity", FIRST)]);
    let ops = migrate::pending(&project, "main", &[]);
    let risky: Vec<String> = ops.iter().filter(|op| op.loses_data()).map(migrate::op_text).collect();
    assert_eq!(
        risky,
        ["remove field Customer.vat_number", "change field Order.number: integer", "remove entity Note"]
    );
}

#[test]
fn the_next_step_gets_the_next_number() {
    assert_eq!(migrate::next_file_name(&[]), "0001.clarity");
    assert_eq!(migrate::next_file_name(&["0001.clarity".into(), "0002.clarity".into()]), "0003.clarity");
}

#[test]
fn a_written_step_is_the_next_file_in_the_folder() {
    let dir = std::env::temp_dir().join(format!("clarity-migrations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    // A project with its texts by key, as the checker asks.
    let keyed = clarity_spec::keyed::make_keys(&[("shop.clarity", SHOP)], &[], "en");
    std::fs::write(dir.join("shop.clarity"), &keyed.modules[0].1).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), keyed.language_file("en")).unwrap();

    let before = clarity_spec::load_dir(&dir).unwrap();
    assert!(!messages(&before, Severity::Warning).is_empty());
    let ops = migrate::pending(&before, "main", &[]);
    let path = clarity_spec::write_step(&dir, "main", "First", &ops).unwrap();
    assert_eq!(path, dir.join("migrations/main/0001.clarity"));

    let after = clarity_spec::load_dir(&dir).unwrap();
    assert!(after.diagnostics.is_empty(), "{:#?}", after.diagnostics);
    // Nothing is pending, so nothing is written.
    assert!(clarity_spec::write_step(&dir, "main", "Second", &[]).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A duration is stored as whole seconds, like an integer: an integer field
/// that becomes a duration under another name is proposed as a rename.
#[test]
fn an_integer_that_becomes_a_duration_is_a_rename() {
    let first = "migration \"Tracks\"\n    add entity Track\n        title: text(80)\n        seconds: integer\n";
    let now = "module music\n\nentity Track\n    title: text(80)\n    length: duration\n";
    let project = load_with_migrations([("music.clarity", now)], [("0001.clarity", first)]);
    let proposed = migrate::proposed_renames(&project, "main");
    assert_eq!(proposed, [Rename { entity: Some("Track".into()), from: "seconds".into(), to: "length".into() }]);
    assert_eq!(pending(&project, &proposed), ["rename field Track.seconds to length", "change field Track.length: duration"]);
}

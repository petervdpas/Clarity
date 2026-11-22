//! A value of an enum renamed (`rename value Kind.Private to Personal`):
//! the step's SQL gives every stored record the new name, in a column of the
//! enum and inside a list of its values, on every database the generator
//! writes for, as the runtime runs it.

use clarity_project::SourceKind;
use clarity_runtime::{Db, SqlValue};

const SHOP: &str = "module shop

enum Kind
    Personal
    Business

entity Customer
    name: text(80)
    kind: Kind
    kinds: list<Kind>
";

const FIRST: &str = "migration \"First\"
    add entity Customer
        name: text(80)
        kind: Kind
        kinds: list<Kind>
";

const SECOND: &str = "migration \"Kinds\"
    rename value Kind.Private to Personal
";

fn steps(kind: SourceKind) -> Vec<Vec<String>> {
    let project = clarity_spec::load_with_migrations([("shop.clarity", SHOP)], [("0001.clarity", FIRST), ("0002.clarity", SECOND)]);
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    clarity_gen::migration_sql(&project, "main", kind).unwrap().into_iter().map(|(_, s)| s).collect()
}

#[test]
fn stored_values_take_the_new_name() {
    let steps = steps(SourceKind::Sqlite);
    let db = Db::memory().unwrap();
    assert_eq!(migrated(clarity_migrate::migrate_open(&db, "main", &steps[..1])), 1);
    db.batch("main", r#"INSERT INTO customer (name, kind, kinds) VALUES ('Ada', 'Private', '["Private","Business"]'), ('Grace', 'Business', '["Business"]')"#).unwrap();
    assert_eq!(migrated(clarity_migrate::migrate_open(&db, "main", &steps)), 1);
    let rows = db.rows("main", "SELECT kind, kinds FROM customer ORDER BY name", &[]).unwrap();
    assert_eq!(rows[0], vec![SqlValue::Text("Personal".into()), SqlValue::Text(r#"["Personal","Business"]"#.into())]);
    assert_eq!(rows[1], vec![SqlValue::Text("Business".into()), SqlValue::Text(r#"["Business"]"#.into())]);
}

#[test]
fn every_database_updates_the_column_and_the_lists() {
    for kind in [SourceKind::Sqlite, SourceKind::Postgres, SourceKind::Mysql, SourceKind::Mssql] {
        let second = steps(kind).pop().unwrap().join("\n");
        assert!(second.contains("UPDATE") && second.contains("'Personal'") && second.contains("'Private'"), "{kind:?}: {second}");
        assert!(second.contains("REPLACE("), "{kind:?}: the lists of values too: {second}");
    }
}

/// How many steps a migration applied (clarity-migrate, as Clarity runs it).
fn migrated(outcome: Result<clarity_migrate::Outcome, String>) -> usize {
    match outcome.unwrap() {
        clarity_migrate::Outcome::UpToDate => 0,
        clarity_migrate::Outcome::Migrated(n) => n,
    }
}

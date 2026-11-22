//! An app checks its database's migration steps at start and does not migrate
//! (docs/ledger/migrations.md, Design): behind or ahead, it stops with one
//! English line (conformance/migrate-check.json).

use clarity_runtime::{Db, Dialect};

#[path = "support/apply_steps.rs"]
mod apply_steps;
use apply_steps::apply_steps;

#[test]
fn a_database_is_checked_as_the_cases_say() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/migrate-check.json")).unwrap();
    for c in cases["cases"].as_array().unwrap() {
        let source = c["source"].as_str().unwrap();
        let db = Db::connect(&[(source, Dialect::Sqlite, ":memory:")]).unwrap();
        let had = c["had"].as_u64().unwrap() as usize;
        let tables: Vec<String> = (0..had).map(|n| format!("CREATE TABLE t{n} (id INTEGER PRIMARY KEY)")).collect();
        let statements: Vec<Vec<&str>> = tables.iter().map(|t| vec![t.as_str()]).collect();
        let steps: Vec<&[&str]> = statements.iter().map(Vec::as_slice).collect();
        apply_steps(&db, source, &steps).unwrap();
        let checked = db.check_steps(source, c["steps"].as_u64().unwrap() as usize);
        match c["line"].as_str() {
            Some(line) => assert_eq!(checked.unwrap_err().to_string(), line, "{c}"),
            None => assert!(checked.is_ok(), "{c}: {checked:?}"),
        }
    }
}

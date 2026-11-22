//! What the slow build tests share: a scratch server database emptied
//! before and after a generated program migrates it.

/// Empties a scratch database: drops every table of `tables` and the
/// migration record, as often as references between them ask.
pub fn empty(at: &str, dialect: clarity_runtime::Dialect, tables: &[String]) {
    let db = clarity_runtime::Db::connect(&[("main", dialect, at)]).unwrap();
    // The tables of the server scenario (tests/servers.rs) may be there too.
    let mut left: Vec<String> = tables.iter().cloned().chain(["purchase", "order", "clarity_schema"].map(String::from)).collect();
    left.sort_unstable();
    left.dedup();
    for _ in 0..=left.len() {
        left.retain(|t| db.batch("main", &format!("DROP TABLE IF EXISTS {}", dialect.quote(t))).is_err());
    }
    assert!(left.is_empty(), "cannot drop {left:?}");
}

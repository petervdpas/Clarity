//! A test's database set up as clarity-migrate leaves it (docs/ledger/migrations.md):
//! the program does not migrate, it checks (`Db::check_steps`).

use clarity_runtime::{Db, Dialect, Result, SqlValue};

/// Runs the steps of `source` its database has not had yet and counts each
/// in `clarity_schema`, with references unchecked on SQLite while they run
/// (a step may build a table again that others refer to); gives how many ran.
pub fn apply_steps(db: &Db, source: &str, steps: &[&[&str]]) -> Result<usize> {
    let dialect = db.dialect(source)?;
    assert_ne!(dialect, Dialect::Mssql, "apply_steps sets up the databases the runtime's tests use");
    db.batch(source, "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)")?;
    let done = match db.rows(source, "SELECT MAX(version) FROM clarity_schema", &[])?.first().and_then(|r| r.first()) {
        Some(SqlValue::Integer(n)) => *n as usize,
        _ => 0,
    };
    let sqlite = dialect == Dialect::Sqlite;
    if sqlite {
        db.batch(source, "PRAGMA foreign_keys = OFF")?;
    }
    let ran = (|| {
        for (i, step) in steps.iter().enumerate().skip(done) {
            for statement in *step {
                db.batch(source, statement)?;
            }
            db.batch(source, &format!("INSERT INTO clarity_schema (version) VALUES ({})", i + 1))?;
        }
        Ok(steps.len().saturating_sub(done))
    })();
    if sqlite {
        db.batch(source, "PRAGMA foreign_keys = ON")?;
    }
    ran
}

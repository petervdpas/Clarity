//! Another program holding the database a moment (Clarity's data view,
//! clarity-migrate, a second copy of the app) makes a write wait for it, up to
//! five seconds, instead of failing with SQLITE_BUSY at once, as on every stack.

use clarity_runtime::{Db, SqlValue};

#[test]
fn a_write_waits_while_another_program_holds_the_database() {
    let dir = std::env::temp_dir().join(format!("clarity-runtime-busy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let db = Db::open(&dir.join("shop.db")).unwrap();
    assert_eq!(db.rows("", "PRAGMA busy_timeout", &[]).unwrap(), vec![vec![SqlValue::Integer(5000)]]);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

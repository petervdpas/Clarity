//! The database side of undo (DESIGN 5.29): everything one handler run
//! changed is one step, with an id the screen's edit keeps. Undo puts the
//! rows back as they were, with their own ids, cascades included; redo does
//! it again; a handler that fails leaves nothing behind; an undo whose rows
//! changed since is refused.

use clarity_runtime::query::{Cond, Op, Query};
use clarity_runtime::ui::{Ui, WindowState};
use clarity_runtime::{Db, Error, Id, Record, Result, SqlValue, sql, stepped};

#[derive(Debug, Clone, Default, PartialEq)]
struct Shape {
    id: Id,
    name: String,
    x: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Note {
    id: Id,
    shape: Option<Id>,
    text: String,
}

impl Record for Shape {
    const TABLE: &'static str = "shape";
    const COLUMNS: &'static [&'static str] = &["name", "x"];
    const CASCADES: bool = true;
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name), SqlValue::Integer(self.x)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Shape { id, name: sql::get_text("name", &v[0])?, x: sql::get_integer("x", &v[1])? })
    }
    fn cascade(db: &Db, id: Id) -> Result<()> {
        for note in db.find::<Note>(&Query::new().filter(Cond::compare("shape", Op::Eq, SqlValue::Integer(id))))? {
            db.delete(&note)?;
        }
        Ok(())
    }
}

impl Record for Note {
    const TABLE: &'static str = "note";
    const COLUMNS: &'static [&'static str] = &["shape", "text"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::reference(self.shape), sql::text(&self.text)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Note { id, shape: sql::get_reference("shape", &v[0])?, text: sql::get_text("text", &v[1])? })
    }
}

fn db() -> Db {
    let db = Db::memory().unwrap();
    db.batch("", "CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL); CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER REFERENCES shape(id), text TEXT NOT NULL);").unwrap();
    db
}

fn shape(db: &Db, name: &str, x: i64) -> Shape {
    let mut s = Shape { id: 0, name: name.into(), x };
    db.save(&mut s).unwrap();
    s
}

#[test]
fn a_step_of_saves_is_undone_and_done_again_with_the_same_ids() {
    let db = db();
    let mut a = db.step(|| Ok(shape(&db, "A", 10))).unwrap();
    let made = db.last_step();
    // One handler: moves A and makes B.
    let b = db
        .step(|| {
            a.x = 300;
            db.save(&mut a)?;
            Ok(shape(&db, "B", 20))
        })
        .unwrap();
    let moved = db.last_step();
    assert_ne!(made, moved);
    assert!(db.undo(moved).unwrap());
    assert_eq!(db.all::<Shape>().unwrap(), vec![Shape { x: 10, ..a.clone() }]);
    // Undone already: nothing more to do.
    assert!(!db.undo(moved).unwrap());
    assert!(db.redo(moved).unwrap());
    assert_eq!(db.all::<Shape>().unwrap(), vec![a.clone(), b.clone()]);
    assert!(db.undo(moved).unwrap());
    assert!(db.undo(made).unwrap());
    assert!(db.all::<Shape>().unwrap().is_empty());
}

#[test]
fn a_delete_with_its_cascade_comes_back_whole() {
    let db = db();
    let a = shape(&db, "A", 10);
    let mut note = Note { id: 0, shape: Some(a.id), text: "keep".into() };
    db.save(&mut note).unwrap();
    db.step(|| db.delete(&a)).unwrap();
    assert!(db.all::<Note>().unwrap().is_empty());
    db.undo(db.last_step()).unwrap();
    assert_eq!(db.all::<Shape>().unwrap(), vec![a]);
    assert_eq!(db.all::<Note>().unwrap(), vec![note]);
}

#[test]
fn a_handler_that_fails_leaves_nothing_and_is_no_step() {
    let db = db();
    let failed: Result<()> = db.step(|| {
        shape(&db, "A", 10);
        Err(Error::Invalid("no".into()))
    });
    assert!(failed.is_err());
    assert!(db.all::<Shape>().unwrap().is_empty());
    assert_eq!(db.last_step(), 0);
    // A step that changed nothing is no step either.
    db.step(|| db.all::<Shape>().map(|_| ())).unwrap();
    assert_eq!(db.last_step(), 0);
}

#[test]
fn an_undo_whose_rows_changed_since_is_refused_and_changes_nothing() {
    let db = db();
    let mut a = shape(&db, "A", 10);
    db.step(|| {
        a.x = 50;
        db.save(&mut a)
    })
    .unwrap();
    // Someone else moved it since.
    db.batch("", "UPDATE shape SET x = 99").unwrap();
    assert!(db.undo(db.last_step()).is_err());
    assert_eq!(db.all::<Shape>().unwrap()[0].x, 99);
}

/// The reply of a handler that changed the database carries its step, for the screen's edit to keep.
#[test]
fn a_reply_carries_the_step_its_handler_made() {
    let db = db();
    let state = WindowState::new("Main", serde_json::Map::new());
    let quiet = stepped(&db, || Ok(Ui::from_state(&state).reply("Main", serde_json::Map::new()))).unwrap();
    assert_eq!(quiet.step, 0);
    let saved = stepped(&db, || {
        shape(&db, "A", 10);
        Ok(Ui::from_state(&state).reply("Main", serde_json::Map::new()))
    })
    .unwrap();
    assert_eq!(saved.step, db.last_step());
    assert!(serde_json::to_string(&quiet).unwrap().find("\"step\"").is_none());
}

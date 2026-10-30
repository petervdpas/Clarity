//! Cascading deletes: a record whose records refer to it with `cascade`
//! deletes them first, each with its own cascades and data events, all in
//! the transaction of its delete.

use std::cell::RefCell;

use clarity_runtime::query::{Cond, Op, Query};
use clarity_runtime::{Db, Error, Id, Record, Result, SqlValue, sql};

thread_local! {
    /// What the delete events of the tasks saw, in order.
    static DELETED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

macro_rules! record {
    ($name:ident, $table:literal, $($field:ident: $ty:ty => $column:literal),*) => {
        #[derive(Debug, Clone, Default, PartialEq)]
        struct $name {
            id: Id,
            $($field: $ty,)*
        }
    };
}

record!(Project, "project", name: String => "name");
record!(Task, "task", project: Option<Id> => "project", title: String => "title");
record!(Step, "step", task: Option<Id> => "task", note: String => "note");

impl Record for Project {
    const TABLE: &'static str = "project";
    const COLUMNS: &'static [&'static str] = &["name"];
    const CASCADES: bool = true;
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Project { id, name: sql::get_text("name", &v[0])? })
    }
    // As generated code writes it: the tasks of the project, each deleted as any record is.
    fn cascade(db: &Db, id: Id) -> Result<()> {
        for task in db.find::<Task>(&Query::new().filter(Cond::compare("project", Op::Eq, SqlValue::Integer(id))))? {
            db.delete(&task)?;
        }
        Ok(())
    }
}

impl Record for Task {
    const TABLE: &'static str = "task";
    const COLUMNS: &'static [&'static str] = &["project", "title"];
    const EVENTS: bool = true;
    const CASCADES: bool = true;
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::reference(self.project), sql::text(&self.title)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Task { id, project: sql::get_reference("project", &v[0])?, title: sql::get_text("title", &v[1])? })
    }
    fn cascade(db: &Db, id: Id) -> Result<()> {
        for step in db.find::<Step>(&Query::new().filter(Cond::compare("task", Op::Eq, SqlValue::Integer(id))))? {
            db.delete(&step)?;
        }
        Ok(())
    }
    fn before_delete(&mut self, _db: &Db) -> Result<()> {
        if self.title == "locked" {
            return Err(Error::Refused("This task is locked".into()));
        }
        DELETED.with(|d| d.borrow_mut().push(self.title.clone()));
        Ok(())
    }
}

impl Record for Step {
    const TABLE: &'static str = "step";
    const COLUMNS: &'static [&'static str] = &["task", "note"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::reference(self.task), sql::text(&self.note)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Step { id, task: sql::get_reference("task", &v[0])?, note: sql::get_text("note", &v[1])? })
    }
}

fn plan() -> (Db, Project) {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&[
        "CREATE TABLE project (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE task (id INTEGER PRIMARY KEY, project INTEGER REFERENCES project(id), title TEXT NOT NULL)",
        "CREATE TABLE step (id INTEGER PRIMARY KEY, task INTEGER REFERENCES task(id), note TEXT NOT NULL)",
    ]])
    .unwrap();
    let mut project = Project { name: "Model".into(), ..Project::default() };
    db.save(&mut project).unwrap();
    for title in ["Draw", "Write"] {
        let mut task = Task { project: Some(project.id), title: title.into(), ..Task::default() };
        db.save(&mut task).unwrap();
        db.save(&mut Step { task: Some(task.id), note: format!("{title} first"), ..Step::default() }).unwrap();
    }
    (db, project)
}

#[test]
fn deleting_a_record_deletes_what_cascades_from_it_first() {
    let (db, project) = plan();
    DELETED.with(|d| d.borrow_mut().clear());
    db.delete(&project).unwrap();
    assert!(db.find::<Project>(&Query::new()).unwrap().is_empty());
    assert!(db.find::<Task>(&Query::new()).unwrap().is_empty());
    assert!(db.find::<Step>(&Query::new()).unwrap().is_empty());
    // Each task's own events ran.
    assert_eq!(DELETED.with(|d| d.borrow().clone()), ["Draw", "Write"]);
}

#[test]
fn a_refused_child_keeps_all_of_it() {
    let (db, project) = plan();
    db.save(&mut Task { project: Some(project.id), title: "locked".into(), ..Task::default() }).unwrap();
    assert!(matches!(db.delete(&project), Err(Error::Refused(_))));
    assert_eq!(db.find::<Project>(&Query::new()).unwrap().len(), 1);
    assert_eq!(db.find::<Task>(&Query::new()).unwrap().len(), 3);
    assert_eq!(db.find::<Step>(&Query::new()).unwrap().len(), 2);
}

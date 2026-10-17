//! Data events: a record whose entity has them runs them around its save
//! and delete, in one transaction with them.

use clarity_runtime::{Db, Error, Id, Record, Result, SqlValue, sql};

/// What the events of an account wrote, in order.
#[derive(Debug, Clone, Default, PartialEq)]
struct Log {
    id: Id,
    text: String,
}

impl Record for Log {
    const TABLE: &'static str = "log";
    const COLUMNS: &'static [&'static str] = &["text"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.text)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Log { id, text: sql::get_text("text", &values[0])? })
    }
}

fn log(db: &Db, text: String) -> Result<()> {
    db.save(&mut Log { text, ..Log::default() })
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Account {
    id: Id,
    name: String,
    balance: i64,
}

impl Record for Account {
    const TABLE: &'static str = "account";
    const COLUMNS: &'static [&'static str] = &["name", "balance"];
    const EVENTS: bool = true;
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name), sql::integer(self.balance)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Account {
            id,
            name: sql::get_text("name", &values[0])?,
            balance: sql::get_integer("balance", &values[1])?,
        })
    }

    fn before_save(&mut self, _db: &Db) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Refused("An account needs a name".into()));
        }
        self.name = self.name.trim().to_string();
        Ok(())
    }

    fn after_save(&mut self, db: &Db) -> Result<()> {
        log(db, format!("saved {} with {}", self.name, self.balance))?;
        if self.balance < 0 {
            return Err(Error::Refused("An account cannot be in debt".into()));
        }
        Ok(())
    }

    fn before_delete(&mut self, _db: &Db) -> Result<()> {
        if self.balance != 0 {
            return Err(Error::Refused(format!("{} still holds {}", self.name, self.balance)));
        }
        Ok(())
    }

    fn after_delete(&mut self, db: &Db) -> Result<()> {
        log(db, format!("deleted {}", self.name))
    }
}

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&[
        "CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, balance INTEGER NOT NULL);",
        "CREATE TABLE log (id INTEGER PRIMARY KEY, text TEXT NOT NULL);",
    ]])
    .unwrap();
    db
}

fn logged(db: &Db) -> Vec<String> {
    db.all::<Log>().unwrap().into_iter().map(|l| l.text).collect()
}

#[test]
fn events_run_around_a_save() {
    let db = db();
    let mut ada = Account { name: "  Ada ".into(), balance: 10, ..Account::default() };
    db.save(&mut ada).unwrap();
    // Before save changed the record that was stored.
    assert_eq!(ada.name, "Ada");
    assert_eq!(db.get::<Account>(ada.id).unwrap().name, "Ada");
    assert_eq!(logged(&db), ["saved Ada with 10"]);

    let mut nameless = Account::default();
    assert_eq!(db.save(&mut nameless), Err(Error::Refused("An account needs a name".into())));
    assert_eq!(Error::Refused("An account needs a name".into()).to_string(), "An account needs a name");
    assert_eq!(db.all::<Account>().unwrap().len(), 1);
}

#[test]
fn a_failing_event_keeps_nothing() {
    let db = db();
    let mut debtor = Account { name: "Grace".into(), balance: -5, ..Account::default() };
    assert_eq!(db.save(&mut debtor), Err(Error::Refused("An account cannot be in debt".into())));
    // The record, and what after save wrote, are gone; the record is new again.
    assert_eq!(debtor.id, 0);
    assert!(db.all::<Account>().unwrap().is_empty());
    assert!(logged(&db).is_empty());

    // The database is usable afterwards.
    debtor.balance = 0;
    db.save(&mut debtor).unwrap();
    assert_eq!(logged(&db), ["saved Grace with 0"]);
}

#[test]
fn events_run_around_a_delete() {
    let db = db();
    let mut ada = Account { name: "Ada".into(), balance: 10, ..Account::default() };
    db.save(&mut ada).unwrap();
    assert_eq!(db.delete(&ada), Err(Error::Refused("Ada still holds 10".into())));
    assert_eq!(db.all::<Account>().unwrap().len(), 1);

    ada.balance = 0;
    db.save(&mut ada).unwrap();
    db.delete(&ada).unwrap();
    assert!(db.all::<Account>().unwrap().is_empty());
    assert_eq!(logged(&db), ["saved Ada with 10", "saved Ada with 0", "deleted Ada"]);
}

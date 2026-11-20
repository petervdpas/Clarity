//! A stored record's list (order.lines) loads with the record, one level
//! deep: `find`, `all` and `get` call `load_lists`, which reads the lines
//! with `find_alone` (docs/ledger/runtime-data.md, step 2).

use clarity_runtime::query::{Cond, Op, Query};
use clarity_runtime::{Db, Id, Record, Result, SqlValue, sql};

#[derive(Debug, Clone, Default, PartialEq)]
struct Order {
    id: Id,
    number: String,
    lines: Vec<Line>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Line {
    id: Id,
    order: Option<Id>,
    product: String,
}

impl Record for Order {
    const TABLE: &'static str = "held_order";
    const COLUMNS: &'static [&'static str] = &["number"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.number)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Order { id, number: sql::get_text("number", &v[0])?, lines: Vec::new() })
    }
    // As generated code writes it.
    fn load_lists(&mut self, db: &Db) -> Result<()> {
        self.lines = db.find_alone::<Line>(&Query::new().filter(Cond::compare("held_order", Op::Eq, SqlValue::Integer(self.id))).order("id", false))?;
        Ok(())
    }
}

impl Record for Line {
    const TABLE: &'static str = "held_line";
    const COLUMNS: &'static [&'static str] = &["held_order", "product"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::reference(self.order), sql::text(&self.product)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Line { id, order: sql::get_reference("held_order", &v[0])?, product: sql::get_text("product", &v[1])? })
    }
}

#[test]
fn a_stored_record_s_lists_load_with_it() {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&[
        "CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
        "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)",
    ]])
    .unwrap();
    let mut first = Order { number: "ORD-1".into(), ..Default::default() };
    let mut second = Order { number: "ORD-2".into(), ..Default::default() };
    db.save(&mut first).unwrap();
    db.save(&mut second).unwrap();
    for product in ["pen", "ink"] {
        db.save(&mut Line { order: Some(first.id), product: product.into(), ..Default::default() }).unwrap();
    }
    let found = db.find::<Order>(&Query::new().order("number", false)).unwrap();
    assert_eq!(found[0].lines.iter().map(|l| l.product.as_str()).collect::<Vec<_>>(), ["pen", "ink"]);
    assert!(found[1].lines.is_empty());
    assert_eq!(db.all::<Order>().unwrap()[0].lines.len(), 2);
    assert_eq!(db.get::<Order>(first.id).unwrap().lines.len(), 2);
    assert!(db.find_alone::<Order>(&Query::new()).unwrap()[0].lines.is_empty());
}

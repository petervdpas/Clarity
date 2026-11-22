//! A stored record's list (order.lines) loads with the record, one level
//! deep: `find` and `all` call `load_lists_of` for every record found, which
//! reads each list once for all of them (`find_by_owners`); `get` loads one
//! record's the same way (docs/ledger/runtime-data.md, step 2).

use std::cell::Cell;

use clarity_runtime::query::Query;
use clarity_runtime::{Db, Id, Record, Result, SqlValue, sql};

#[path = "support/apply_steps.rs"]
mod apply_steps;
use apply_steps::apply_steps;

thread_local! {
    /// How many times the lines were read on this test's thread: once per list for a whole find.
    static LINE_READS: Cell<usize> = const { Cell::new(0) };
}

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
        Self::load_lists_of(std::slice::from_mut(self), db)
    }
    fn load_lists_of(records: &mut [Self], db: &Db) -> Result<()> {
        let owners: Vec<Id> = records.iter().map(|r| r.id).collect();
        LINE_READS.set(LINE_READS.get() + owners.len().div_ceil(clarity_runtime::OWNERS_PER_READ));
        let mut lines = clarity_runtime::by_owner(db.find_by_owners::<Line>("held_order", &owners)?, |l| l.order);
        for record in records {
            record.lines = lines.remove(&record.id).unwrap_or_default();
        }
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
    let db = Db::memory().unwrap();
    apply_steps(&db, "", &[&[
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

/// Every case of conformance/held-lists.json: a find reads each list once
/// per `ownersPerRead` orders and gives each order its own lines in order.
#[test]
fn a_find_reads_each_list_once_for_all_the_records_it_finds() {
    let file: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/held-lists.json")).unwrap();
    assert_eq!(file["ownersPerRead"].as_u64(), Some(clarity_runtime::OWNERS_PER_READ as u64));
    for case in file["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let db = Db::memory().unwrap();
        apply_steps(&db, "", &[&[
            "CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
            "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)",
        ]])
        .unwrap();
        for n in 1..=case["orders"].as_i64().unwrap() {
            db.save(&mut Order { number: format!("ORD-{n:05}"), ..Default::default() }).unwrap();
        }
        for line in case["lines"].as_array().unwrap() {
            db.save(&mut Line { order: line[0].as_i64(), product: line[1].as_str().unwrap().into(), ..Default::default() }).unwrap();
        }
        LINE_READS.set(0);
        let found = db.find::<Order>(&Query::new().order("number", false)).unwrap();
        assert_eq!(LINE_READS.get() as u64, case["reads"].as_u64().unwrap(), "{name}: reads");
        for (id, products) in case["expect"].as_object().unwrap() {
            let order = found.iter().find(|o| o.id.to_string() == *id).unwrap_or_else(|| panic!("{name}: no order {id}"));
            let got: Vec<&str> = order.lines.iter().map(|l| l.product.as_str()).collect();
            let want: Vec<&str> = products.as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
            assert_eq!(got, want, "{name}: order {id}");
            assert_eq!(db.get::<Order>(order.id).unwrap().lines, order.lines, "{name}: one order loads its lines the same way");
        }
    }
}

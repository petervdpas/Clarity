//! The SQL of queries per database, against the cases every stack's
//! runtime shares (conformance/queries.json).

use std::str::FromStr;

use clarity_runtime::{Cond, Decimal, Dialect, Hop, NaiveDate, Op, Query, SqlValue};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/queries.json");

const DIALECTS: [(&str, Dialect); 4] = [
    ("sqlite", Dialect::Sqlite),
    ("postgres", Dialect::Postgres),
    ("mysql", Dialect::Mysql),
    ("mssql", Dialect::Mssql),
];

/// Column names live as long as the test; the runtime takes generated names.
fn name(value: &Value) -> &'static str {
    Box::leak(value.as_str().expect("a name").to_string().into_boxed_str())
}

fn op(value: &Value) -> Op {
    match value.as_str().expect("an op") {
        "Eq" => Op::Eq,
        "NotEq" => Op::NotEq,
        "Lt" => Op::Lt,
        "LtEq" => Op::LtEq,
        "Gt" => Op::Gt,
        "GtEq" => Op::GtEq,
        other => panic!("unknown op {other}"),
    }
}

fn sql_value(value: &Value) -> SqlValue {
    let (kind, inner) = value.as_object().and_then(|o| o.iter().next()).expect("a value");
    match kind.as_str() {
        "text" => SqlValue::Text(inner.as_str().expect("text").to_string()),
        "integer" => SqlValue::Integer(inner.as_i64().expect("an integer")),
        "decimal" => SqlValue::Decimal(Decimal::from_str(inner.as_str().expect("a decimal")).expect("a decimal")),
        "bool" => SqlValue::Bool(inner.as_bool().expect("a bool")),
        "date" => SqlValue::Date(NaiveDate::parse_from_str(inner.as_str().expect("a date"), "%Y-%m-%d").expect("a date")),
        "null" => SqlValue::Null,
        other => panic!("unknown value {other}"),
    }
}

fn values(value: &Value) -> Vec<SqlValue> {
    value.as_array().expect("values").iter().map(sql_value).collect()
}

fn cond(value: &Value) -> Cond {
    let (kind, inner) = value.as_object().and_then(|o| o.iter().next()).expect("a condition");
    let pair = |key: &str| {
        let both = inner.as_array().expect(key);
        (cond(&both[0]), cond(&both[1]))
    };
    match kind.as_str() {
        "compare" => Cond::compare(name(&inner["column"]), op(&inner["op"]), sql_value(&inner["value"])),
        "number" => Cond::number(name(&inner["column"]), op(&inner["op"]), sql_value(&inner["value"])),
        "contains" => Cond::contains(name(&inner["column"]), inner["text"].as_str().expect("text")),
        "oneOf" => Cond::one_of(name(&inner["column"]), values(&inner["values"])),
        "oneOfNumbers" => Cond::one_of_numbers(name(&inner["column"]), values(&inner["values"])),
        "isTrue" => Cond::is_true(name(inner)),
        "isNone" => Cond::is_none(name(inner)),
        "and" => {
            let (left, right) = pair("and");
            left.and(right)
        }
        "or" => {
            let (left, right) = pair("or");
            left.or(right)
        }
        "not" => cond(inner).negated(),
        "through" => Cond::through(name(&inner["column"]), name(&inner["table"]), name(&inner["key"]), cond(&inner["filter"])),
        other => panic!("unknown condition {other}"),
    }
}

fn query(value: &Value) -> Query {
    let mut query = Query::new();
    if let Some(filter) = value.get("filter") {
        query = query.filter(cond(filter));
    }
    for key in value.get("order").and_then(Value::as_array).into_iter().flatten() {
        let column = name(&key["column"]);
        let descending = key["descending"].as_bool().expect("descending");
        let hops: Vec<Hop> = key
            .get("through")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|h| Hop::new(name(&h["column"]), name(&h["table"]), name(&h["key"])))
            .collect();
        query = match (hops.is_empty(), key["numeric"].as_bool().expect("numeric")) {
            (true, true) => query.order_number(column, descending),
            (true, false) => query.order(column, descending),
            (false, numeric) => query.order_through(&hops, column, descending, numeric),
        };
    }
    query
}

#[test]
fn every_case_gives_its_sql_and_parameters_on_every_database() {
    let cases: Value = serde_json::from_str(CASES).expect("queries.json is JSON");
    let cases = cases["cases"].as_array().expect("cases");
    assert!(!cases.is_empty());
    for case in cases {
        let case_name = case["name"].as_str().expect("a name");
        let key = case["key"].as_str().expect("a key");
        let table = case["table"].as_str().expect("a table");
        let query = query(&case["query"]);
        for (word, dialect) in DIALECTS {
            let expect = &case["expect"][word];
            let columns: Vec<String> = std::iter::once(key)
                .chain(case["columns"].as_array().expect("columns").iter().map(|c| c.as_str().expect("a column")))
                .map(|c| dialect.quote(c))
                .collect();
            let (sql, params) = query.sql(dialect, table, key, &columns.join(", "));
            assert_eq!(sql, expect["sql"].as_str().expect("sql"), "{case_name} on {word}");
            assert_eq!(params, values(&expect["params"]), "{case_name} on {word}: parameters");
        }
    }
}

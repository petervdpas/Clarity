//! Text functions alike in every runtime (conformance/texts.json).

use clarity_runtime::texts;
use serde_json::{Value, json};

#[test]
fn text_functions_read_as_every_runtime_reads_them() {
    let cases: Value = serde_json::from_str(include_str!("../../../conformance/texts.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let a = case["args"].as_array().unwrap();
        let s = |i: usize| a[i].as_str().unwrap();
        let n = |i: usize| a[i].as_i64().unwrap();
        let got = match case["function"].as_str().unwrap() {
            "words" => json!(texts::words(s(0))),
            "split" => json!(texts::split(s(0), s(1))),
            "trim" => json!(texts::trim(s(0))),
            "lower" => json!(texts::lower(s(0))),
            "upper" => json!(texts::upper(s(0))),
            "replace" => json!(texts::replace(s(0), s(1), s(2))),
            "startsWith" => json!(texts::starts_with(s(0), s(1))),
            "endsWith" => json!(texts::ends_with(s(0), s(1))),
            "slice" => json!(texts::slice(s(0), n(1), n(2))),
            "indexOf" => json!(texts::index_of(s(0), s(1))),
            other => panic!("unknown {other}"),
        };
        assert_eq!(case["result"], got, "{case}");
    }
}

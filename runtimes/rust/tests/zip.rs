//! zip(paths, texts) alike in every runtime (conformance/zip.json).

use serde_json::Value;

#[test]
fn a_zip_is_written_as_every_runtime_writes_it() {
    let cases: Value = serde_json::from_str(include_str!("../../../conformance/zip.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let list = |k: &str| case[k].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect::<Vec<_>>();
        let got: String = clarity_runtime::zip::stored(&list("paths"), &list("texts")).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(got, case["hex"].as_str().unwrap(), "{}", case["name"]);
    }
}

//! The JSON Clarity writes keeps its keys in sorted order: the generated
//! files, the windows' markup and what the runtimes send. A dependency that
//! switches on serde_json's `preserve_order` changes that for the whole
//! workspace when it builds together (as the checks do), and once changed
//! the key order of every file Clarity wrote (docs/ACP-PLAN.md). This test
//! fails in such a build, naming the cause, before the files do.

#[test]
fn no_dependency_switches_on_serde_json_s_preserve_order() {
    let mut map = serde_json::Map::new();
    map.insert("b".into(), 1.into());
    map.insert("a".into(), 2.into());
    let keys: Vec<&str> = map.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["a", "b"],
        "serde_json keeps keys in insertion order: a dependency switches on its `preserve_order` feature (find it with `cargo tree -e features -i serde_json --workspace`)"
    );
}

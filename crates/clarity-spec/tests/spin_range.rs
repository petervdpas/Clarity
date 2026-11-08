//! A spin box's range set from code: `levelBox.min` and `levelBox.max`, read
//! as the line wrote them until code sets them, on every stack.

use clarity_spec::{Severity, load_sources};

const APP: &str = "module m

page Main: window
    data level: integer
    spinbox name levelBox at 1,1 span 2 bind level min 1 max 99 on change: picked
    textbox name nameBox at 3,1 span 2

code Main
    handler picked
        levelBox.max = 3
        levelBox.min = levelBox.max - 2
        if level > levelBox.max
            level = levelBox.max
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn code_reads_and_sets_a_spin_box_s_range() {
    assert_eq!(errors(APP), Vec::<String>::new());
}

#[test]
fn only_a_spin_box_has_a_range_and_it_is_whole() {
    let other = errors(&APP.replace("        levelBox.max = 3\n", "        nameBox.max = 3\n"));
    assert!(other.iter().any(|e| e.contains("nameBox") && e.contains("'max'")), "{other:#?}");
    let text = errors(&APP.replace("levelBox.max = 3", "levelBox.max = \"3\""));
    assert!(!text.is_empty(), "a text set as a spin box's max");
}

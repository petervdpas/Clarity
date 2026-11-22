//! What the checker passes, every generator writes: each project of the
//! examples folder that checks clean generates on its own stack, without a
//! refusal of a generator ("list fields like 'order.lines' are not loaded
//! yet"). A refusal the checker does not know first fails here, for the
//! whole kind; the build tests then build them (tests/examples_build.rs).

use crate::ide_build;

#[test]
fn every_example_that_checks_clean_generates_on_its_stack() {
    let mut names: Vec<String> = std::fs::read_dir(ide_build::repo().join("examples"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".clarity").map(String::from))
        .collect();
    names.sort();
    let mut refused = Vec::new();
    for name in &names {
        let (_, project, settings) = ide_build::example(name);
        if project.has_errors() {
            continue;
        }
        if let Err(errors) = clarity_gen::programs(&project, &settings) {
            refused.push(format!("{name}: {errors:?}"));
        }
    }
    assert!(refused.is_empty(), "checked clean, but a generator refused:\n{}", refused.join("\n"));
}

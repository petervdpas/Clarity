//! Cascading deletes: `cascade` on a reference field deletes the record
//! when the record it refers to is deleted, children first, with their data
//! events, in the same transaction.

use clarity_spec::{Severity, load_sources};

const SOURCE: &str = "module plan

entity Project
    name: text(80), required

entity Task
    project: ref Project, required, cascade
    title: text(80)

entity Step
    task: ref Task, cascade
    note: text
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("plan.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_reference_field_can_cascade() {
    assert_eq!(errors(SOURCE), Vec::<String>::new());
    let project = load_sources([("plan.clarity", SOURCE)]);
    let task = project.entity("Task").unwrap();
    assert!(task.field("project").unwrap().modifiers.cascade);
    let spec = clarity_spec::dictionary::FieldSpec { name: "project".into(), type_text: "ref Project".into(), modifiers: task.field("project").unwrap().modifiers.clone() };
    assert_eq!(clarity_spec::dictionary::field_line(&spec), "project: ref Project, required, cascade");
    // What deleting a project deletes first, and what deleting a task does.
    let children = |entity: &str| -> Vec<String> { project.cascades(entity).iter().map(|(e, f)| format!("{}.{}", e.name.name, f.name.name)).collect() };
    assert_eq!(children("Project"), ["Task.project"]);
    assert_eq!(children("Task"), ["Step.task"]);
    assert_eq!(children("Step"), Vec::<String>::new());
}

#[test]
fn only_a_reference_cascades() {
    let text = SOURCE.replace("    title: text(80)\n", "    title: text(80), cascade\n");
    assert_eq!(errors(&text), ["only a reference cascades: deleting the record it refers to deletes this one ('title')"]);
}

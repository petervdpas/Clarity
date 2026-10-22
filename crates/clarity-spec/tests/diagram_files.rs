//! Written diagrams in the project's `diagrams` folder (DG1, DESIGN section
//! 5.13): read with the project, each by its name and kind, and a starter
//! for each kind Clarity maps.

use clarity_spec::diagrams::{self, DiagramKind};

#[test]
fn the_diagrams_folder_is_read_with_the_project() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("diagram_files_project");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(diagrams::DIAGRAMS_DIR)).unwrap();
    std::fs::write(dir.join("shop.clarity"), "module shop\n\nentity Customer\n    name: text, label @customerNameLabel\n").unwrap();
    std::fs::write(dir.join("diagrams/orders.mermaid"), "erDiagram\n    Order {\n        date ordered_on\n    }\n").unwrap();
    std::fs::write(dir.join("diagrams/plan.mermaid"), "gantt\n    title A plan\n").unwrap();
    std::fs::write(dir.join("diagrams/notes.txt"), "not a diagram").unwrap();

    let project = clarity_spec::load_dir(&dir).unwrap();
    let found: Vec<(&str, Option<DiagramKind>)> = project.diagrams.iter().map(|d| (d.name.as_str(), d.kind)).collect();
    assert_eq!(found, vec![("orders.mermaid", Some(DiagramKind::Erd)), ("plan.mermaid", None)]);
    assert!(project.diagrams[0].text.contains("ordered_on"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_without_the_folder_has_no_diagrams() {
    let project = clarity_spec::load_sources([("shop.clarity", "module shop\n")]);
    assert!(project.diagrams.is_empty());
}

#[test]
fn each_kind_clarity_maps_has_a_starter_that_reads_clean() {
    for kind in [DiagramKind::Erd, DiagramKind::Class] {
        let text = diagrams::starter(kind);
        assert_eq!(diagrams::kind_of(&text), Some(kind));
        let (_, problems) = diagrams::parse(&text);
        assert!(problems.is_empty(), "{kind:?}: {problems:?}");
    }
}

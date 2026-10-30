// A guard for undo (DESIGN 5.29) on Avalonia: every part of the every-part
// project that has undo carries its scope in the generated window
// (`c:Edits.Scope`), so Ctrl+Z reaches it; one without undo does not. A
// part kind that is written without its scope fails here.

mod everything_project;

use clarity_gen::generate;
use clarity_spec::model::Widget;

fn parts<'a>(widgets: &'a [Widget], out: &mut Vec<&'a Widget>) {
    for w in widgets {
        out.push(w);
        parts(&w.children, out);
        for page in &w.pages {
            parts(&page.children, out);
        }
    }
}

#[test]
fn every_part_with_undo_carries_its_scope_on_avalonia() {
    let (_, project, settings) = everything_project::prepare("everything-undo", "csharp avalonia desktop cli");
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let mut checked = 0;
    for page in project.files.iter().flat_map(|f| &f.pages) {
        let Some(view) = files.iter().find(|f| f.path.ends_with(&format!("{}View.axaml", page.name.name))) else { continue };
        let mut all = Vec::new();
        parts(&page.widgets, &mut all);
        for sidebar in &page.frame.sidebars {
            parts(&sidebar.widgets, &mut all);
        }
        for w in all {
            let Some(name) = w.props.name.as_deref() else { continue };
            let Some(line) = view.text.lines().find(|l| l.contains(&format!("x:Name=\"{name}\""))) else { continue };
            match &w.props.undo_scope {
                Some(scope) => {
                    assert!(line.contains(&format!("c:Edits.Scope=\"{scope}\"")), "{:?} {name} has no undo scope in {}:\n{line}", w.props.kind, page.name.name);
                    checked += 1;
                }
                None => assert!(!line.contains("c:Edits.Scope"), "{name} has no undo, yet a scope:\n{line}"),
            }
        }
    }
    assert!(checked > 10, "only {checked} parts with undo checked");
}

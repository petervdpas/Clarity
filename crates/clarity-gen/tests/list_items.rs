//! A field of a record in a list is set in place (`items[2].done = true`)
//! on every stack: the record the list holds changes. Go once wrote
//! `&w.Items[2-1].Done`, which takes the field's address and does not compile.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const FIRST: &str = "migration \"First\"
    add entity Item
        title: text(80)
        done: bool
";

const PLAN: &str = "module plan

entity Item
    title: text(80)
    done: bool

app Plan
    main Main

page Main: window
    data items: list<Item>
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        items[2].done = true
";

#[test]
fn every_stack_sets_a_field_of_a_record_in_a_list() {
    let written = [
        (Backend::Rust, Ui::Clarity, "(*clarity_runtime::at_mut(&mut w.items[..], 2i64)?).done = true;"),
        (Backend::Go, Ui::Clarity, "(&w.Items[2-1]).Done = true"),
        (Backend::C, Ui::Clarity, "->done = true;"),
        (Backend::Cpp, Ui::Clarity, "clarity::at(items, int64_t{2}).done = true;"),
        (Backend::Csharp, Ui::Clarity, "Lists.At(Items, 2L).Done = true;"),
    ];
    for (backend, ui, line) in written {
        let mut settings = starter("Plan", "plan");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in([("plan.clarity", PLAN)], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        // The code, not the screen it carries (a file, or text in page.c and page.cpp).
        let code: String = files.iter().filter(|f| !f.path.contains("runtime") && !f.path.ends_with(".json") && !f.path.ends_with("page.c") && !f.path.ends_with("page.cpp")).map(|f| f.text.as_str()).collect();
        assert!(code.contains(line), "{backend:?} sets the field of the record in the list: {line}");
    }
}

//! A spin box's range read and set from code (`levelBox.min`, `levelBox.max`)
//! on every stack: read as its line writes it until a handler sets it, and
//! sent to the screen in the part's state.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module levels

page Main: window
    data level: integer, default 1
    spinbox name levelBox at 1,1 span 2 bind level min 1 max 99 on change: picked

code Main
    handler picked
        levelBox.max = 3
        if level > levelBox.max
            level = levelBox.min
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("levels.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Levels", "levels");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}", f.path, f.text)).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    let near: Vec<&str> = text.lines().filter(|l| l.len() < 200 && (l.contains("level") || l.contains("Level"))).collect();
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program, whose lines about the spin box are\n{}", near.join("\n"));
}

#[test]
fn every_stack_reads_and_sets_a_spin_box_s_range() {
    let rust = generated(Backend::Rust);
    has(&rust, "ui.set_spin_max(\"levelBox\", 3");
    has(&rust, "ui.spin_max(\"levelBox\", 99)");
    has(&rust, "ui.spin_min(\"levelBox\", 1)");
    let go = generated(Backend::Go);
    has(&go, "ui.SetSpinMax(\"levelBox\", 3");
    has(&go, "ui.SpinMax(\"levelBox\", 99)");
    let c = generated(Backend::C);
    has(&c, "clarity_ui_set_spin_max(ui, \"levelBox\", ");
    has(&c, "clarity_ui_spin_max(ui, \"levelBox\", 99)");
    let cpp = generated(Backend::Cpp);
    has(&cpp, "ui.set_spin_max(\"levelBox\", ");
    has(&cpp, "ui.spin_max(\"levelBox\", 99)");
    let csharp = generated(Backend::Csharp);
    has(&csharp, "{ SpinMin = 1, SpinMax = 99 }");
    has(&csharp, "SpinMax = 3");
}

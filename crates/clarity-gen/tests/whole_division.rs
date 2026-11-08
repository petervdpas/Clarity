//! Whole-number division on every stack: `a div b` and `a mod b` call each
//! runtime's helper, which gives the quotient rounded toward zero (or what is
//! left, with the sign of a) and fails on a division by zero.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module grid

function middle(width: integer, columns: integer) -> integer
    return width div 2 + width mod columns

page Main: window
    data width: integer, default 7
    button \"Go\" name goButton at 1,1 span 2 on click: go

code Main
    handler go
        width = middle(width, 3) div 2
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("grid.clarity", SOURCE)], &[], "en");
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("Grid", "grid");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}", f.path, f.text)).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    let near: Vec<&str> = text.lines().filter(|l| l.contains("whole") || l.contains("Whole")).collect();
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program, whose lines dividing are\n{}", near.join("\n"));
}

#[test]
fn every_stack_divides_whole_numbers_with_its_runtime() {
    let rust = generated(Backend::Rust);
    has(&rust, "clarity_runtime::whole_div(width, 2i64)?");
    has(&rust, "clarity_runtime::whole_mod(width, columns)?");
    let go = generated(Backend::Go);
    has(&go, "clarity.WholeDiv(width, 2)");
    has(&go, "clarity.WholeMod(width, columns)");
    let c = generated(Backend::C);
    has(&c, "clarity_whole_div(width, 2, &");
    has(&c, "clarity_whole_mod(width, columns, &");
    let cpp = generated(Backend::Cpp);
    has(&cpp, "clarity::whole_div(width, int64_t{2})");
    let csharp = generated(Backend::Csharp);
    has(&csharp, "Lists.WholeDiv(width, 2L)");
    has(&csharp, "Lists.WholeMod(width, columns)");
    // The program carries Clarity's word for a division by zero.
    assert!(rust.contains("divisionByZero"), "the word is in the translations");
}

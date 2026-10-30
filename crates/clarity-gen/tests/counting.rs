//! Counting and joining in code on every stack: `a to b`, the whole numbers
//! from a to b, and `join(texts, separator)`.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module counting

page Main: window
    data shown: text
    data numbers: list<integer>
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        let names: list<text> = []
        for i in 1 to 3
            names = names + [\"row{i}\"]
        numbers = 2 to 5
        shown = join(names, \", \")
";

fn generated(backend: Backend, ui: Ui) -> String {
    let project = clarity_spec::keyed::load_keyed([("counting.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Counting", "counting");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program");
}

#[test]
fn every_stack_counts_and_joins() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "(1i64..=3i64).collect::<Vec<i64>>()");
    has(&rust, ".join(");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "clarity.Range(1, 3)");
    has(&go, "clarity.Join(");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "Lists.Range(1L, 3L)");
    has(&csharp, "string.Join(");
    let c = generated(Backend::C, Ui::Web);
    has(&c, "clarity_range(1, 3)");
    has(&c, "clarity_joined(");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "clarity::range(int64_t{1}, int64_t{3})");
    has(&cpp, "clarity::joined(");
}

//! The built-in functions of code call each runtime by the name the table in
//! clarity-gen gives it: `zip(paths, texts)` and `words(text)` on every stack.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};
use clarity_spec::load_converted;

const SOURCE: &str = "module m\n\npage Main: window\n    data note: text\n\naction exportBook()\n    effects files.write\n    let paths = [\"mimetype\", \"a.txt\"]\n    let texts = [\"application/epub+zip\", \"one two\"]\n    let n = count(words(\"one two\"))\n    write zip(paths, texts) to export \"book.epub\"\n";

#[test]
fn built_ins_call_each_runtime_by_its_name() {
    let project = load_converted([("m.clarity", SOURCE)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let stacks = [
        (Backend::Rust, "clarity_runtime::zip::stored(&(", "clarity_runtime::texts::words(&("),
        (Backend::Go, "clarity.Zip(", "clarity.TextWords("),
        (Backend::Csharp, "Clarity.Runtime.Zip.Of(", "Clarity.Runtime.Texts.Words("),
        (Backend::C, "clarity_zip(", "clarity_text_words("),
        (Backend::Cpp, "clarity::zip(", "clarity::words("),
    ];
    for (backend, zip, words) in stacks {
        let mut settings = starter("Shop", "shop");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        let all: String = files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n");
        assert!(all.contains(zip), "{backend:?} calls {zip}");
        assert!(all.contains(words), "{backend:?} calls {words}");
    }
}

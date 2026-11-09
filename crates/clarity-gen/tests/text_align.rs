//! A canvas's text is centered on its x, or starts or ends there
//! (`canvas.text(x, y, text, color, size, "left")`): every backend sends the
//! alignment to its screen, and a text without one is centered.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SHEET: &str = "module sheet

page Main: window
    canvas name sheet at 1,1 span 6 rows 6 units 200 x 100 on draw: drawSheet

code Main
    handler drawSheet
        sheet.text(10, 20, \"Order\", \"text\", 11, \"left\")
        sheet.text(10, 40, \"Total\")
";

#[test]
fn every_backend_sends_where_a_text_is_aligned() {
    for (backend, ui) in [(Backend::Rust, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity), (Backend::Csharp, Ui::Clarity)] {
        let mut settings = starter("Sheet", "sheet");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in(clarity_spec::converted([("sheet.clarity", SHEET)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let code: String = generate(&project, &settings).unwrap().iter().map(|f| f.text.as_str()).collect();
        assert!(code.contains("\"left\""), "{backend:?} sends 'left'");
        // C# leaves it to the record's default, "center" (Shape.Words), as every value left out.
        assert!(code.contains("\"center\"") || backend == Backend::Csharp, "{backend:?} centers a text without an alignment");
    }
}

/// `length(text)`: how many characters a text has, so code can size what it
/// draws to its text (a class box as wide as its longest line).
#[test]
fn every_backend_counts_the_characters_of_a_text() {
    let source = SHEET.replace("        sheet.text(10, 40, \"Total\")\n", "        sheet.text(10, 40, \"Total\")\n        sheet.rect(0, 0, length(\"Größe\") * 7, 10, \"text\")\n");
    let wanted = [
        (Backend::Rust, Ui::Clarity, ".chars().count() as i64"),
        (Backend::Go, Ui::Clarity, "int64(len([]rune("),
        (Backend::C, Ui::Clarity, "clarity_length("),
        (Backend::Cpp, Ui::Clarity, "clarity::length("),
        (Backend::Csharp, Ui::Clarity, "Clarity.Runtime.Texts.Length("),
    ];
    for (backend, ui, code_of) in wanted {
        let mut settings = starter("Sheet", "sheet");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in(clarity_spec::converted([("sheet.clarity", source.as_str())]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let code: String = generate(&project, &settings).unwrap().iter().map(|f| f.text.as_str()).collect();
        assert!(code.contains(code_of), "{backend:?}: {code_of}");
    }
}

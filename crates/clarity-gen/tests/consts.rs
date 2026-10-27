//! Constants (DESIGN 5.12) are written out where they are used before any
//! stack is written: no stack needs constants of its own.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const PLAY: &str = "module play

const lives = 3
const bonus: decimal = 2

app Play
    main Main

page Main: window
    data left: integer
    data score: decimal
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        left = lives + 1
        score = bonus * 2
";

#[test]
fn every_stack_writes_a_constant_out_as_its_value() {
    for (backend, ui) in [(Backend::Rust, Ui::Web), (Backend::Go, Ui::Web), (Backend::C, Ui::Web), (Backend::Cpp, Ui::Web), (Backend::Csharp, Ui::Avalonia)] {
        let mut settings = starter("Play", "play");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in([("play.clarity", PLAY)], [], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        // The code, not the model the renderer reads (a file, or text in page.c and page.cpp).
        let code: String = files.iter().filter(|f| !f.path.contains("runtime") && !f.path.ends_with(".json") && !f.path.ends_with("page.c") && !f.path.ends_with("page.cpp")).map(|f| f.text.as_str()).collect();
        assert!(!code.contains("lives") && !code.contains("bonus"), "{backend:?} still names a constant");
        if backend == Backend::Rust {
            assert!(code.contains("w.score = (Decimal::new(20, 1) * Decimal::from(2i64))"), "a decimal constant stays a decimal");
        }
    }
}

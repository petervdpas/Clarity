//! Slide-outs on every stack: the markup carries the slide-out to
//! Clarity.UI, which draws it on every stack; code opens it with `visible`
//! and pins it with `pinned`, part state on the web stacks and in the C#
//! window's view model.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SHOP: &str = include_str!("fixtures/slideouts.clarity");

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(code: &str, backend: Backend, expected: &str) {
    assert!(code.contains(expected), "{backend:?} misses {expected:?}");
}

#[test]
fn the_markup_carries_the_slideout_on_every_stack() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let code = generated(backend);
        has(&code, backend, "<Slideout name=\"details\"");
        has(&code, backend, "caption=\"@detailsCaption\"");
        has(&code, backend, "side=\"left\"");
        has(&code, backend, "slideWidth=\"480\"");
        has(&code, backend, "pinnable=\"true\"");
        has(&code, backend, "onopen=\"loadDetails\"");
        has(&code, backend, "side=\"bottom\"");
        has(&code, backend, "slideHeight=\"320\"");
    }
}

#[test]
fn the_web_stacks_keep_visible_and_pinned_in_the_part_state() {
    has(&generated(Backend::Rust), Backend::Rust, "ui.set_pinned(\"details\", false)");
    has(&generated(Backend::Rust), Backend::Rust, "ui.pinned(\"details\")");
    has(&generated(Backend::Go), Backend::Go, "ui.SetPinned(\"details\", false)");
    has(&generated(Backend::Go), Backend::Go, "ui.Pinned(\"details\")");
    has(&generated(Backend::C), Backend::C, "clarity_ui_set_pinned(ui, \"details\", false)");
    has(&generated(Backend::C), Backend::C, "clarity_ui_pinned(ui, \"details\")");
    has(&generated(Backend::Cpp), Backend::Cpp, "ui.set_pinned(\"details\", false)");
    has(&generated(Backend::Cpp), Backend::Cpp, "ui.pinned(\"details\")");
    // Closed until it is opened: `visible` reads false until then.
    has(&generated(Backend::Rust), Backend::Rust, "ui.visible_or(\"details\", false)");
    has(&generated(Backend::Go), Backend::Go, "ui.VisibleOr(\"details\", false)");
    has(&generated(Backend::C), Backend::C, "clarity_ui_visible_or(ui, \"details\", false)");
    has(&generated(Backend::Cpp), Backend::Cpp, "ui.visible_or(\"details\", false)");
}

#[test]
fn csharp_keeps_the_slideout_in_its_state() {
    let code = generated(Backend::Csharp);
    has(&code, Backend::Csharp, "public PartState Details { get; } = new(\"details\") { IsVisible = false };");
    has(&code, Backend::Csharp, "void Show_details() => Details.IsVisible = true;");
    has(&code, Backend::Csharp, "Details.IsPinned = false;");
    has(&code, Backend::Csharp, "if (Details.IsPinned)");
}

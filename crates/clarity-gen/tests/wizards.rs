//! Wizards (DESIGN 5.15) on every stack: the web stacks read and set the
//! step in the part's state and the shared renderer draws the steps part;
//! Avalonia writes the runtime's StepsView, its step bound to the part's
//! state and its Next asking the handler whether it refused.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture: a dialog that is a wizard of three steps.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: Backend, ui: Ui) -> String {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn the_web_stacks_read_and_set_the_step_in_the_parts_state() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "ui.step(\"wizard\")");
    has(&rust, "ui.set_step(\"wizard\", 3");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "ui.Step(\"wizard\")");
    has(&go, "ui.SetStep(\"wizard\", 3)");
    let c = generated(Backend::C, Ui::Web);
    has(&c, "clarity_ui_step(ui, \"wizard\")");
    has(&c, "clarity_ui_set_step(ui, \"wizard\", 3");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "ui.step(\"wizard\")");
    has(&cpp, "ui.set_step(\"wizard\", int64_t{3});");
}

#[test]
fn avalonia_writes_a_steps_view_whose_next_asks_whether_the_handler_refused() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "<c:StepsView x:Name=\"wizard\"");
    has(&code, "Step=\"{Binding Wizard.Step}\"");
    has(&code, "<c:StepItem Header=");
    has(&code, "public bool Next_Wizard() => Handle(() => OnCheckStep());");
    // The view model is looked up when Next is pressed: the view has none yet while it is made.
    has(&code, "wizard.Next = () => Vm.Next_Wizard();");
    has(&code, "wizard.Finished += () => Vm.DoneCommand.Execute(null);");
    has(&code, "Wizard.Step = 3");
}

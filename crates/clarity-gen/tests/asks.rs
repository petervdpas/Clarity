//! Asking the person (DESIGN 5.14) on every stack: the handler that asks
//! puts the question in its reply (or, on Avalonia, asks its window when it
//! has finished), and each stack runs the handler the answer goes to, with
//! the values written for it and the answer last: a record by its id or as
//! it was written, any other value as it is.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// A window that asks each kind of question; shared with the build tests.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

/// Every file the stack writes, as one text.
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
fn rust_asks_in_the_reply_and_answers_through_its_own_command() {
    let code = generated(Backend::Rust, Ui::Web);
    has(&code, "ui.ask(");
    has(&code, "clarity_runtime::ui::AskForm::Confirm { yes: Some(");
    has(&code, "clarity_runtime::ui::AskForm::Value { ty: \"record\".to_string(), of: Some(\"Friend\".to_string()), only: Some(");
    has(&code, "\"lend\", vec![to_value(&");
    has(&code, "fn answer_event(");
    has(&code, "            answer_event,");
    has(&code, "clarity_runtime::ui::answer_record(db, value(0)?)?, clarity_runtime::ui::answer_record(db, value(1)?)?");
    has(&code, "clarity_runtime::ui::answer(value(0)?)?");
}

#[test]
fn go_asks_in_the_reply_and_answers_through_its_windows() {
    let code = generated(Backend::Go, Ui::Web);
    has(&code, "ui.Ask(");
    has(&code, "clarity.Confirm(clarity.Button(");
    has(&code, "clarity.ValueForm(\"record\", clarity.Named(\"Friend\"), clarity.IDs(w.Friends), nil)");
    has(&code, "func (windows) Answer(db *clarity.Db, state clarity.WindowState, handler string, values []json.RawMessage)");
    has(&code, "clarity.AnswerRecord[Friend](db, values[1])");
    has(&code, "clarity.Answer[int64](values[0])");
}

#[test]
fn c_asks_in_the_reply_and_answers_through_its_windows() {
    let code = generated(Backend::C, Ui::Web);
    has(&code, "clarity_ui_ask(ui, ");
    has(&code, "clarity_ask_confirm(");
    has(&code, "clarity_ask_value(\"record\", \"Friend\", ");
    has(&code, "static bool answer(ClarityDb* db, const ClarityWindowState* state, const char* handler, const cJSON* values, ClarityReply** out)");
    has(&code, "picture, redraw, answer}");
    has(&code, "friend_get(db, (int64_t)a1->valuedouble, &v1)");
}

#[test]
fn cpp_asks_in_the_reply_and_answers_through_its_windows() {
    let code = generated(Backend::Cpp, Ui::Web);
    has(&code, "ui.ask(");
    has(&code, "clarity::ask_confirm(");
    has(&code, "clarity::ask_value(\"record\", std::string(\"Friend\"), ");
    has(&code, "clarity::Reply answer(");
    has(&code, "db.get<Friend>(values.at(1).get<int64_t>())");
    has(&code, "values.at(0).get<int64_t>()");
}

#[test]
fn avalonia_asks_its_window_and_the_answer_calls_the_handler() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "Ask(");
    has(&code, "new ConfirmForm(");
    has(&code, "_ => OnDeleteConfirmed()");
    has(&code, "new ValueForm(null, ");
    has(&code, ".Select(r => new Choice(r.Id, r.Name)).ToList(), null), answer => OnLend(");
    has(&code, "Db.Get<Friend>((long)answer!))");
    has(&code, "new ValueForm(new FieldFormat(FormatKind.Integer), null, (object?)(");
    has(&code, "answer => OnSetFormat((Format)answer!)");
}

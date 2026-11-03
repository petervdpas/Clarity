//! A new record starts with its date and moment defaults on every stack
//! (`new Entity` gives a record its defaults): each generator writes the
//! default out from its parts (clarity_spec::dates).

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const FIRST: &str = "migration \"First\"
    add entity Visit
        day: date, default \"2026-10-20\"
        due: datetime, default \"2026-10-20T14:05:00\"
        left: date
";

const PLAN: &str = "module plan

entity Visit
    day: date, default \"2026-10-20\"
    due: datetime, default \"2026-10-20T14:05:00\"
    left: date

app Plan
    main Main

page Main: window
    data visit: Visit
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        visit = new Visit
";

fn code(backend: Backend, ui: Ui) -> String {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let project = clarity_spec::load_in([("plan.clarity", PLAN)], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    // The code, not the screen it carries (a file, or text in page.c and page.cpp).
    files.iter().filter(|f| !f.path.contains("runtime") && !f.path.ends_with(".json") && !f.path.ends_with("page.c") && !f.path.ends_with("page.cpp")).map(|f| f.text.as_str()).collect()
}

#[test]
fn every_stack_starts_a_new_record_with_its_date_defaults() {
    let written = [
        (Backend::Rust, Ui::Clarity, ["NaiveDate::from_ymd_opt(2026, 10, 20)", "NaiveDate::from_ymd_opt(2026, 10, 20).and_then(|d| d.and_hms_opt(14, 5, 0))"]),
        (Backend::Go, Ui::Clarity, ["&clarity.Date{Year: 2026, Month: 10, Day: 20}", "&clarity.DateTime{Date: clarity.Date{Year: 2026, Month: 10, Day: 20}, Hour: 14, Minute: 5, Second: 0}"]),
        (Backend::C, Ui::Clarity, ["(ClarityDate){2026, 10, 20}", "(ClarityDateTime){2026, 10, 20, 14, 5, 0}"]),
        (Backend::Cpp, Ui::Clarity, ["clarity::Date{2026, 10, 20}", "clarity::DateTime{clarity::Date{2026, 10, 20}, 14, 5, 0}"]),
        (Backend::Csharp, Ui::Clarity, ["new DateOnly(2026, 10, 20)", "new DateTime(2026, 10, 20, 14, 5, 0)"]),
    ];
    for (backend, ui, [day, due]) in written {
        let code = code(backend, ui);
        assert!(code.contains(day), "{backend:?} starts the day at its default: {day}");
        assert!(code.contains(due), "{backend:?} starts the moment at its default: {due}");
    }
}

/// A record read from the screen's JSON starts as a new one, so a date the
/// screen sends as null must empty it, not leave its default (C: the
/// runtime's clarity_json_get_date, runtimes/c/tests/ui_test.c).
#[test]
fn a_null_date_from_the_screen_is_no_date_in_cpp() {
    let read = "if (in.contains(\"due\")) r.due = in[\"due\"].is_null() ? std::nullopt : std::optional<clarity::DateTime>(in[\"due\"].get<clarity::DateTime>());";
    assert!(code(Backend::Cpp, Ui::Clarity).contains(read));
}

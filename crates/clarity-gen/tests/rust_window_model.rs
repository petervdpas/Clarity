//! The Rust program's side of the window model, as the C# generator does it:
//! an input bound to a required field shows why a save refused it, a window
//! starts with the settings the person kept that still fit, names never clash
//! with what the generated code uses itself, a button can open the keys
//! window and an engine counts `for i in a to b`.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

fn files(source: &str) -> Vec<OutFile> {
    files_with(source, false)
}

/// With `keys`, the project lets the person change their keys.
fn files_with(source: &str, keys: bool) -> Vec<OutFile> {
    let mut project = clarity_spec::load_sources([("shop.clarity", source)]);
    project.keys_editable = keys;
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    settings.keys_editable = keys;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn all(files: &[OutFile]) -> String {
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn inputs_bound_to_a_required_field_show_why_a_save_refused_it() {
    let files = files(
        "module shop

entity Customer
    name: text(80), required, label @customerNameLabel
    city: text(80)

page Card: window
    data customer: Customer
    form name customerForm at 1,1 span 12 rows 2 bind customer
        textbox name nameBox at 1,1 span 6 bind customer.name
        textbox name cityBox at 1,2 span 6 bind customer.city
    button \"Save\" name saveButton at 1,3 span 2 on click: saveClicked

code Card
    handler saveClicked
        save customer
",
    );
    let main = file(&files, "src/main.rs");
    has(main, "        \"Card\" => &[Bound { part: \"nameBox\", data: \"customer\", table: \"customer\", column: \"name\" }],\n");
    assert!(!main.contains("part: \"cityBox\""), "city is not required");
    // Every handler a person starts runs where a refusal shows at its input.
    has(main, "locked(self, |db| shown_at_input(db, state, |state| clarity_runtime::stepped(db, || dispatch(db, state, handler)))))");
    has(main, "locked(self, |db| shown_at_input(db, state, |state| answer_to(db, state, handler, values))))");
    has(main, "locked(self, |db| shown_at_input(db, state, |state| clarity_runtime::edits::guarded(");
    has(main, "locked(self, |db| shown_at_input(db, state, |state| clarity_runtime::steps::event(");
}

const SETTINGS: &str = "module shop

enum Density
    compact
    roomy

settings
    pageSize: integer, default 20, min 10, max 200, label @pageSizeLabel
    density: Density, label @densityLabel
    showDone: bool, default true, label @showDoneLabel

page Prefs: window
    form name settingsForm at 1,1 span 12 rows 2 bind settings
        checkbox name showDoneBox at 1,1 span 6 bind settings.showDone
";

#[test]
fn a_window_starts_with_the_kept_settings_that_still_fit() {
    let files = files(SETTINGS);
    let all = all(&files);
    has(&all, "    pub const DESCRIBED: &'static [(&'static str, clarity_runtime::settings::Setting)] = &[\n");
    has(&all, "        (\"pageSize\", clarity_runtime::settings::Setting { kind: \"integer\", min: Some(\"10\"), max: Some(\"200\"), values: &[] }),\n");
    has(&all, "        (\"density\", clarity_runtime::settings::Setting { kind: \"enum\", min: None, max: None, values: &[\"compact\", \"roomy\"] }),\n");
    has(&all, "        (\"showDone\", clarity_runtime::settings::Setting { kind: \"bool\", min: None, max: None, values: &[] }),\n");
    has(&all, "    pub fn from_kept(kept: &serde_json::Map<String, serde_json::Value>) -> Settings {\n");
    // The window's settings are the kept ones, read when it opens.
    has(&all, "            settings: crate::kept_settings(),\n");
    has(file(&files, "src/main.rs"), "fn kept_settings() -> modules::shop::Settings {\n    modules::shop::Settings::from_kept(&clarity_runtime::serve::page_settings_over(clarity_runtime::settings::read(PROGRAM)))\n}\n");
}

#[test]
fn a_name_the_generated_code_uses_itself_is_stepped_around() {
    let files = files(
        "module shop

page Pad: window
    data ui: integer
    data w: text
    button \"One\" name oneButton at 1,1 span 2 on click: pressed(1)

code Pad
    handler pressed(w: integer)
        ui = w
",
    );
    let all = all(&files);
    // The data keeps its Clarity name for the screen; in Rust it steps around the handler's own.
    has(&all, "    #[serde(rename = \"ui\")]\n    pub ui_: i64,\n");
    has(&all, "    #[serde(rename = \"w\")]\n    pub w_: String,\n");
    has(&all, "(db: &Db, w: &mut PadWindow, ui: &mut Ui, w_: i64) -> Result<()>");
    has(&all, "w.ui_ = w_;");
}

#[test]
fn a_button_opens_the_keys_window() {
    let files = files_with(
        "module shop

page Card: window
    button \"Keys\" name keysButton at 1,1 span 2 on click -> keys
",
        true,
    );
    // The screen opens Clarity's keys window for the click's own target.
    has(file(&files, "windows/Card.cxaml"), "<Button name=\"keysButton\" at=\"1,1\" span=\"2\" caption=\"Keys\" opens=\"keys\" />");
}

#[test]
fn an_engine_counts_from_one_number_to_another() {
    let files = files(
        "module shop

engine Counter
    // The sum of 1 to n, counted without making the list.
    function sum(n: integer) -> integer
        let total = 0
        for i in 1 to n
            total = total + i
        return total

page Card: window
    data total: integer
    button \"Sum\" name sumButton at 1,1 span 2 on click: summed

code Card
    handler summed
        total = Counter.sum(4)
",
    );
    let wasm = files.iter().find(|f| f.path.ends_with("counter.wasm")).and_then(|f| f.bytes.as_deref()).expect("counter.wasm");
    assert_eq!(&wasm[..4], b"\0asm");
}

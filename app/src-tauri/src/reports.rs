//! Reports in the project tree (docs/ledger/reports-pdf.md): the `.report` files of
//! the `reports` folder, made, saved (one undo step) and removed here, and
//! a report's pages for the report tab's preview: laid out by Clarity.Pdf
//! with sample data of its Data's entities, in its stylesheet, as SVG.

use clarity_spec::Preview;
use clarity_spec::reports::REPORTS_DIR;
use clarity_tools::structural::reports;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record};

/// A report's text as the report tab's designer holds it (clarity_spec::report_design),
/// as JSON whose words keep their order; or why it cannot be read.
#[tauri::command]
pub fn report_read(text: String) -> Result<String, String> {
    clarity_spec::report_design::read(&text).map(|design| clarity_spec::report_design::to_json(&design))
}

/// The text of the design the report tab's designer holds (its JSON), bands in the order they print.
#[tauri::command]
pub fn report_write(design: String) -> Result<String, String> {
    clarity_spec::report_design::from_json(&design).map(|design| clarity_spec::report_design::write(&design))
}

/// What a new part of `kind` starts with at `at` in its band, as JSON.
#[tauri::command]
pub fn report_new_part(kind: String, at: (f64, f64)) -> Result<String, String> {
    clarity_spec::report_design::new_part(&kind, at).map(|part| clarity_spec::report_design::to_json(&part)).ok_or_else(|| format!("a band has no part '{kind}'"))
}

/// A report's stylesheet with one declaration of `selector` set, or taken out when `value` is empty.
#[tauri::command]
pub fn report_with_declaration(css: String, selector: String, property: String, value: String) -> String {
    clarity_spec::report_design::with_declaration(&css, &selector, &property, &value)
}

/// A report's stylesheet's rules by selector (`.title`, `body`), each with its declarations.
#[tauri::command]
pub fn report_rules(css: String) -> Vec<(String, Vec<(String, String)>)> {
    clarity_spec::report_design::rules(&css)
}

/// Makes report `name`, with what a new report starts with, the one way
/// (clarity_tools::structural::reports, as the agents' add_report).
#[tauri::command]
pub fn add_report(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let changes = reports::add_report_file(&dir, &name, &[])?;
    record(&history, changes)?;
    load(&dir)
}

/// Writes report `name` as the report tab designed it, refused when the
/// report would gain an error; one undo step.
#[tauri::command]
pub fn save_report(name: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let changes = reports::save_report(&dir, &name, &text)?;
    if !changes.is_empty() {
        record(&history, changes)?;
    }
    load(&dir)
}

/// Removes report `name`; one undo step brings it back.
#[tauri::command]
pub fn delete_report(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = reports::report_path(&dir, &name)?;
    let text = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let change = FileChange { file, before: Some(text), after: None };
    change.make()?;
    record(&history, vec![change])?;
    load(&dir)
}

/// Writes a stylesheet of the reports folder (`invoice.css`), as the report
/// tab's look of a part changes its class, refused when a report would gain
/// an error; one undo step.
#[tauri::command]
pub fn save_report_stylesheet(file: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let changes = reports::save_stylesheet(&dir, &file, &text)?;
    if !changes.is_empty() {
        record(&history, changes)?;
    }
    load(&dir)
}

/// A report's pages for the preview, or why it shows none.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReportPreview {
    pub pages: Vec<String>,
    pub problem: Option<String>,
}

/// Report `name` as `text` writes it, laid out with sample data: its pages as SVG.
#[tauri::command(async)]
pub fn report_preview(name: String, text: String, state: State<WatchState>) -> Result<ReportPreview, String> {
    let (_, dir) = open_project_of(&state)?;
    let mut project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    match project.reports.iter_mut().find(|r| r.name == name) {
        Some(report) => report.text = text.clone(),
        None => project.reports.push(clarity_spec::reports::ReportFile { name: name.clone(), path: String::new(), text: text.clone(), stylesheet: None }),
    }
    let stylesheet = clarity_spec::xml::read(&text).ok().and_then(|root| root.get("stylesheet").map(str::to_string)).and_then(|css| std::fs::read_to_string(dir.join(REPORTS_DIR).join(css)).ok()).unwrap_or_default();
    let language = clarity_spec::translations::default_language(&project);
    let texts = serde_json::to_string(&project.translations.get(&language).cloned().unwrap_or_default()).unwrap_or_default();
    let data = clarity_spec::reports::sample_data(&project, &name).to_string();
    // The project's pictures, which Image parts name by file.
    let images: std::collections::BTreeMap<String, Vec<u8>> = project.resources.iter().flat_map(|r| &r.files).map(|f| (f.name.clone(), f.bytes.clone())).collect();
    let types = clarity_spec::reports::formats_of(&project, &name);
    let extras = clarity_pdf::ReportExtras { images, language: project.locale.language.clone(), currency: project.locale.currency.clone(), types };
    Ok(match clarity_pdf::report_layout_with(&text, &data, &texts, &stylesheet, "", &extras) {
        Ok(laid) => ReportPreview { pages: (0..laid.pages()).map(|p| laid.svg(p)).collect(), problem: None },
        Err(problem) => ReportPreview { pages: Vec::new(), problem: Some(problem.to_string()) },
    })
}

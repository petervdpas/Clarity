//! Reports in the project tree (docs/ledger/reports-pdf.md): the `.report` files of
//! the `reports` folder, made, saved (one undo step) and removed here, and
//! a report's pages for the report tab's preview: laid out by Clarity.Pdf
//! with sample data of its Data's entities, in its stylesheet, as SVG.

use std::path::{Path, PathBuf};

use clarity_spec::Preview;
use clarity_spec::reports::{REPORT_EXTENSION, REPORTS_DIR};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record, write_atomically};

/// A report's file in the module folder `dir`; refused for a name that is no plain name.
fn report_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let plain = name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && name.chars().all(|c| c.is_ascii_alphanumeric());
    if !plain {
        return Err(format!("a report's name is a word starting with a capital, as in Invoice, not '{name}'"));
    }
    Ok(dir.join(REPORTS_DIR).join(format!("{name}.{REPORT_EXTENSION}")))
}

/// Makes report `name`, with what a new report starts with.
#[tauri::command]
pub fn add_report(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = report_file(&dir, &name)?;
    if file.exists() {
        return Err(format!("the reports have {name} already"));
    }
    std::fs::create_dir_all(dir.join(REPORTS_DIR)).map_err(|err| format!("cannot make the reports folder: {err}"))?;
    let text = clarity_spec::reports::starter(&name);
    write_atomically(&file, &text)?;
    record(&history, vec![FileChange { file, before: None, after: Some(text) }])?;
    load(&dir)
}

/// Writes report `name` as the report tab designed it; one undo step.
#[tauri::command]
pub fn save_report(name: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = report_file(&dir, &name)?;
    let before = std::fs::read_to_string(&file).ok();
    if before.as_deref() != Some(text.as_str()) {
        write_atomically(&file, &text)?;
        record(&history, vec![FileChange { file, before, after: Some(text) }])?;
    }
    load(&dir)
}

/// Removes report `name`; one undo step brings it back.
#[tauri::command]
pub fn delete_report(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = report_file(&dir, &name)?;
    let text = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let change = FileChange { file, before: Some(text), after: None };
    change.make()?;
    record(&history, vec![change])?;
    load(&dir)
}

/// Writes a stylesheet of the reports folder (`invoice.css`), as the report
/// tab's look of a part changes its class; one undo step.
#[tauri::command]
pub fn save_report_stylesheet(file: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let plain = file.ends_with(".css") && file.len() > 4 && file.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !plain {
        return Err(format!("a report's stylesheet is a file of the reports folder ending in .css, as in invoice.css, not '{file}'"));
    }
    std::fs::create_dir_all(dir.join(REPORTS_DIR)).map_err(|err| format!("cannot make the reports folder: {err}"))?;
    let path = dir.join(REPORTS_DIR).join(&file);
    let before = std::fs::read_to_string(&path).ok();
    if before.as_deref() != Some(text.as_str()) {
        write_atomically(&path, &text)?;
        record(&history, vec![FileChange { file: path, before, after: Some(text) }])?;
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

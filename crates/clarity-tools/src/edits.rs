//! The designer's way of changing a project, shared by the IDE and every
//! agent's tools (docs/AGENT-TOOLS-PLAN.md): a structural edit
//! (`clarity_spec::edit::Edit`) made on the file it is asked of, split
//! between a window's `.cxaml` and its module where it must be, a new
//! window's files added, the keys it writes given to every language, and
//! refused when the project would check worse than before. The files are
//! written; the changes are given back for Undo.

use std::path::{Path, PathBuf};

use clarity_spec::edit::{Edit, EditOutcome};

use crate::files::{FileChange, write_atomically};

/// What an edit changed.
#[derive(Default)]
pub struct Applied {
    /// Each file it wrote, made or removed, with its text before and after.
    pub changes: Vec<FileChange>,
    /// What the edit focuses, in the file it was asked of.
    pub focus: Option<u32>,
}

/// Makes `edit` on `file` (a module or a `.cxaml` of the project in `dir`)
/// and writes what it changes; refused, writing nothing, when the project
/// would gain an error.
pub fn apply(dir: &Path, file: &Path, edit: &Edit) -> Result<Applied, String> {
    let source = std::fs::read_to_string(file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let before = clarity_project::load_modules(dir).map_err(read_err)?;
    // A window's, view's or app's own file: the edit is made on what it holds and the file written;
    // what belongs in the module (the handlers a wiring writes, code) goes to the module's file.
    // Its whole markup typed in the Design text replaces the file, checked as any edit.
    let mut outcomes: Vec<(PathBuf, String, EditOutcome)> = Vec::new();
    let mut new_pages: Vec<(PathBuf, String)> = Vec::new();
    match edit {
        Edit::ReplaceDecl { text, .. } if is_page_file(file) => {
            outcomes.push((file.to_path_buf(), source.clone(), EditOutcome { text: text.clone(), focus: None, keys: Vec::new() }));
        }
        // Removing a window, view or app itself: its file goes, and its code in the module with it.
        Edit::Remove { item } if is_page_file(file) && clarity_spec::cxaml::read_designed(&source, "").is_ok_and(|d| d.start() == *item) => {
            let module = code_file_of(file)?;
            let text = std::fs::read_to_string(&module).map_err(|err| format!("cannot read {}: {err}", module.display()))?;
            let without = clarity_spec::cxaml::removed(&source, *item, &text).ok_or("the file holds no window, view or app")??;
            outcomes.push((file.to_path_buf(), source.clone(), EditOutcome { text: String::new(), focus: None, keys: Vec::new() }));
            outcomes.push((module, text, EditOutcome { text: without, focus: None, keys: Vec::new() }));
        }
        _ if is_page_file(file) => {
            let (design, code) = clarity_spec::design_edit::split(edit);
            if let Some(design) = design {
                let outcome = clarity_spec::cxaml::apply(Some(&before), &source, &design).map_err(|err| err.to_string())?;
                outcomes.push((file.to_path_buf(), source.clone(), outcome));
            }
            if let Some(code) = code {
                let module = code_file_of(file)?;
                let text = std::fs::read_to_string(&module).map_err(|err| format!("cannot read {}: {err}", module.display()))?;
                let outcome = clarity_spec::edit::apply_in(&before, &text, &code).map_err(|err| err.to_string())?;
                outcomes.push((module, text, outcome));
            }
        }
        _ => match clarity_spec::cxaml::added(&before, file, &source, edit) {
            // A new window, view or app is files of its own in design/; the code it starts with goes in the module.
            Some(added) => {
                let added = added?;
                new_pages = added.files;
                outcomes.push((file.to_path_buf(), source.clone(), EditOutcome { text: added.module, focus: None, keys: added.keys }));
            }
            None => {
                let outcome = clarity_spec::edit::apply_in(&before, &source, edit).map_err(|err| err.to_string())?;
                outcomes.push((file.to_path_buf(), source.clone(), outcome));
            }
        },
    }
    // What the edit focuses: in the file it was asked of.
    let focus = outcomes.first().and_then(|(path, _, o)| (path == file).then_some(o.focus).flatten());
    let changed: Vec<&(PathBuf, String, EditOutcome)> = outcomes.iter().filter(|(_, before_text, o)| o.text != *before_text).collect();
    if changed.is_empty() && new_pages.is_empty() {
        return Ok(Applied { changes: Vec::new(), focus });
    }
    // A text the edit wrote became a key: every language file gets it, the default language with its text.
    let default = clarity_spec::translations::default_language(&before);
    // Every outcome's keys, a changed file's or not: a new view's title is a key while its module stays as it was.
    let keys: Vec<(String, String)> = outcomes.iter().flat_map(|(_, _, o)| o.keys.clone()).collect();
    let languages = clarity_spec::keyed::plan_keys(dir, &default, &keys).map_err(read_err)?;
    let mut replaced: Vec<(&Path, &str)> = changed.iter().map(|(path, _, o)| (path.as_path(), o.text.as_str())).collect();
    replaced.extend(new_pages.iter().map(|(path, text)| (path.as_path(), text.as_str())));
    replaced.extend(languages.iter().map(|(path, _, after)| (path.as_path(), after.as_str())));
    // No edit leaves parts reaching past what holds them: they are cut to fit, in the same write and Undo.
    let fits = fitted(dir, &replaced)?;
    replaced.retain(|(path, _)| !fits.iter().any(|(fit, _, _)| fit.as_path() == *path));
    replaced.extend(fits.iter().map(|(path, _, after)| (path.as_path(), after.as_str())));
    // The checker decides what is valid: an edit that adds an error anywhere in the project (an overlap, a wrong binding) is refused.
    let after = clarity_project::load_modules_with_files(dir, &replaced).map_err(read_err)?;
    if let Some(error) = clarity_spec::new_error(&before, &after) {
        return Err(refusal(dir, &replaced, error));
    }
    for (path, _) in &new_pages {
        if path.exists() {
            return Err(format!("{} is there already", path.display()));
        }
    }
    let mut changes = Vec::new();
    let fit_of = |path: &Path| fits.iter().find(|(fit, _, _)| fit.as_path() == path).map(|(_, _, after)| after.clone());
    for (path, before_text, outcome) in &changed {
        // A design file written as nothing is removed: its window, view or app is gone.
        if outcome.text.is_empty() && is_page_file(path) {
            std::fs::remove_file(path).map_err(|err| format!("cannot remove {}: {err}", path.display()))?;
            changes.push(FileChange { file: path.clone(), before: Some(before_text.clone()), after: None });
            continue;
        }
        let text = fit_of(path).unwrap_or_else(|| outcome.text.clone());
        write_atomically(path, &text)?;
        changes.push(FileChange { file: path.clone(), before: Some(before_text.clone()), after: Some(text) });
    }
    // Files only the fitting changed.
    for (path, before_text, after_text) in fits.iter().filter(|(path, _, _)| !changed.iter().any(|(p, _, _)| p == path) && !new_pages.iter().any(|(p, _)| p == path)) {
        write_atomically(path, after_text)?;
        changes.push(FileChange { file: path.clone(), before: Some(before_text.clone()), after: Some(after_text.clone()) });
    }
    for (path, text) in new_pages {
        let text = fit_of(&path).unwrap_or(text);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        write_atomically(&path, &text)?;
        changes.push(FileChange { file: path, before: None, after: Some(text) });
    }
    for (path, before_text, after_text) in languages {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        write_atomically(&path, &after_text)?;
        changes.push(FileChange { file: path, before: before_text, after: Some(after_text) });
    }
    Ok(Applied { changes, focus })
}

/// The design files a project with `replaced` needs written again so nothing
/// reaches past what holds it (`clarity_spec::fit`): each with its text before
/// and after. A window's parts end by the last row its height holds under its
/// bars, a slide-out is no taller, a view's parts end by the rows the window
/// holds from its content area's top.
fn fitted(dir: &Path, replaced: &[(&Path, &str)]) -> Result<Vec<(PathBuf, String, String)>, String> {
    use clarity_spec::layout;
    use clarity_spec::model::PageKind;
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let project = clarity_project::load_modules_with_files(dir, replaced).map_err(read_err)?;
    let settings = clarity_project::load(&dir.with_extension(clarity_spec::SPEC_EXTENSION)).settings;
    let row = settings.as_ref().map_or(layout::ROW_HEIGHT, clarity_project::row_height);
    let app = project.files.iter().flat_map(|f| &f.apps).next().map(|a| a.frame.clone());
    let text_of = |path: &Path| replaced.iter().find(|(p, _)| *p == path).map(|(_, t)| t.to_string()).or_else(|| std::fs::read_to_string(path).ok());
    let files = clarity_spec::cxaml::page_files(dir, &[], replaced).map_err(read_err)?;
    let mut out = Vec::new();
    let mut views: Vec<(String, u32)> = Vec::new();
    // The windows first: what they hold decides what their views have.
    for path in &files {
        let Some(text) = text_of(path) else { continue };
        let Ok(clarity_spec::design_edit::Designed::Page(mut page)) = clarity_spec::cxaml::read_designed(&text, &path.to_string_lossy()) else { continue };
        if !matches!(page.kind, PageKind::Window | PageKind::Dialog) {
            continue;
        }
        let height = page.size.as_ref().map_or(layout::WINDOW_HEIGHT, |s| s.height);
        // A dialog shows its own bars only, never the app's frame.
        let bars = layout::frame_bars(if page.kind == PageKind::Dialog { None } else { app.as_ref() }, &page.frame);
        let holds = layout::rows_held(height, row, &bars);
        let changed = clarity_spec::fit::fit_window(&mut page, holds);
        views.extend(clarity_spec::fit::shown_views(&page, holds));
        if changed {
            let after = clarity_spec::cxaml::written_again(&text, &clarity_spec::design_edit::Designed::Page(page));
            out.push((path.clone(), text, after));
        }
    }
    for path in &files {
        let Some(text) = text_of(path) else { continue };
        let Ok(clarity_spec::design_edit::Designed::Page(mut page)) = clarity_spec::cxaml::read_designed(&text, &path.to_string_lossy()) else { continue };
        let Some(rows) = views.iter().filter(|(name, _)| *name == page.name.name).map(|(_, rows)| *rows).min() else { continue };
        if page.kind == PageKind::View && clarity_spec::fit::fit_view(&mut page, rows) {
            let after = clarity_spec::cxaml::written_again(&text, &clarity_spec::design_edit::Designed::Page(page));
            out.push((path.clone(), text, after));
        }
    }
    Ok(out)
}

/// The error an edit would add, with the file and line it would be at, so
/// the agent finds what to change first.
fn refusal(dir: &Path, replaced: &[(&Path, &str)], error: &clarity_spec::Diagnostic) -> String {
    if error.path.is_empty() {
        return error.message.clone();
    }
    let path = dir.join(&error.path);
    let text = replaced.iter().find(|(p, _)| *p == path.as_path()).map(|(_, t)| t.to_string()).or_else(|| std::fs::read_to_string(&path).ok());
    match text {
        Some(text) => {
            let line = text.get(..error.span.start as usize).map_or(1, |before| before.matches('\n').count() + 1);
            format!("{} ({}:{line})", error.message, error.path)
        }
        None => format!("{} ({})", error.message, error.path),
    }
}

/// Whether `path` is a window's, view's or app's own `.cxaml`.
pub fn is_page_file(path: &Path) -> bool {
    clarity_spec::cxaml::is_page_file(&path.to_string_lossy())
}

/// The file whose code blocks a file's pages run: a `.cxaml`'s module, else the file itself.
pub fn code_file_of(file: &Path) -> Result<PathBuf, String> {
    if !is_page_file(file) {
        return Ok(file.to_path_buf());
    }
    clarity_spec::cxaml::module_file(file).ok_or_else(|| format!("{} is in no module's folder", file.display()))
}

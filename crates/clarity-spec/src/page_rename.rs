//! Renaming a window or a view (any page) across the project: its `.cxaml`
//! file, which takes the new name, and its class; the code block named after
//! it; and every place that names it, found through the parsed model: a
//! click of a part or a menu item or button that opens it, a view line of
//! content, a slide-out's view, a sign-in part's `shows`, a canvas's toolbox,
//! the app's main window and splash, and `open` and `ask` in code. A rename
//! is asked for by name, never inferred.

use crate::code::{AskForm, ExprKind, Stmt, StmtKind};
use crate::design_edit::Designed;
use crate::model::{Command, Frame, Project, Widget};
use crate::rename::Renamed;
use crate::span::Span;
use crate::widgets::is_identifier;

/// Renames the page `old` to `new` in every file that names it. `source`
/// gives the current text of a file by its path in the project.
pub fn rename_page(project: &Project, source: impl Fn(&str) -> Option<String>, old: &str, new: &str) -> Result<Vec<Renamed>, String> {
    if !is_identifier(new) {
        return Err(format!("'{new}' is not a valid window or view name"));
    }
    if !new.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(format!("a window or view is named in PascalCase, like {}", pascal(new)));
    }
    let owner = project.files.iter().find(|f| f.pages.iter().any(|p| p.name.name == old)).ok_or_else(|| format!("there is no window or view '{old}'"))?;
    if new == old {
        return Ok(Vec::new());
    }
    let taken = project.files.iter().any(|f| {
        f.pages.iter().map(|p| &p.name.name).chain(f.apps.iter().map(|a| &a.name.name)).chain(f.entities.iter().map(|e| &e.name.name)).chain(f.enums.iter().map(|e| &e.name.name)).any(|n| n == new)
    });
    if taken {
        return Err(format!("'{new}' is already the name of a window, view, app, entity or enum"));
    }
    let read = |path: &str| source(path).ok_or_else(|| format!("cannot read {path}"));
    let mut changed = Vec::new();
    for file in &project.files {
        if crate::cxaml::is_page_file(&file.path) {
            let text = read(&file.path)?;
            let mut designed = crate::cxaml::read_designed(&text, &file.path)?;
            let own = std::ptr::eq(file, owner);
            if renamed_in_design(&mut designed, old, new) || own {
                let text = crate::cxaml::written_again(&text, &designed);
                if own {
                    changed.push(Renamed { path: moved_path(&file.path, new), text, moved_from: Some(file.path.clone()) });
                } else {
                    changed.push(Renamed { path: file.path.clone(), text, moved_from: None });
                }
            }
            continue;
        }
        // A module: the code block named after the page, `open` and `ask` in any code.
        let mut spans: Vec<Span> = file.code.iter().filter(|b| b.name.name == old).map(|b| b.name.span).collect();
        for block in &file.code {
            for handler in &block.handlers {
                opened(&handler.body, old, &mut spans);
            }
            for event in &block.events {
                opened(&event.body, old, &mut spans);
            }
        }
        for function in &file.functions {
            opened(&function.body, old, &mut spans);
        }
        for command in &file.commands {
            opened(&command.body, old, &mut spans);
        }
        if spans.is_empty() {
            continue;
        }
        let mut text = read(&file.path)?;
        spans.sort_unstable_by_key(|s| (s.start, s.end));
        spans.dedup();
        for span in spans.into_iter().rev() {
            let (start, end) = (span.start as usize, span.end as usize);
            if text.get(start..end) != Some(old) {
                return Err(format!("{} changed while renaming; try again", file.path));
            }
            text.replace_range(start..end, new);
        }
        changed.push(Renamed { path: file.path.clone(), text, moved_from: None });
    }
    Ok(changed)
}

/// `name` with its first letter made capital.
fn pascal(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

/// A page's file path with the file named after `new`: `design/Card.cxaml`
/// becomes `design/Customer.cxaml`, in the same folder.
fn moved_path(path: &str, new: &str) -> String {
    let file = crate::cxaml::file_name(new);
    match path.rfind(['/', '\\']) {
        Some(at) => format!("{}{file}", &path[..=at]),
        None => file,
    }
}

/// The page or app of a `.cxaml` with every naming of `old` made `new`:
/// the page's own name, the app's main window and splash, its parts and
/// frame. Whether anything changed.
fn renamed_in_design(designed: &mut Designed, old: &str, new: &str) -> bool {
    let mut changed = false;
    match designed {
        Designed::Page(page) => {
            if page.name.name == old {
                page.name.name = new.to_string();
                changed = true;
            }
            changed |= in_frame(&mut page.frame, old, new);
            changed |= in_widgets(&mut page.widgets, old, new);
        }
        Designed::App(app) => {
            for named in app.main.iter_mut().chain(app.splash.iter_mut()) {
                if named.name == old {
                    named.name = new.to_string();
                    changed = true;
                }
            }
            changed |= in_frame(&mut app.frame, old, new);
        }
    }
    changed
}

fn swap(value: &mut Option<String>, old: &str, new: &str) -> bool {
    if value.as_deref() == Some(old) {
        *value = Some(new.to_string());
        return true;
    }
    false
}

fn in_frame(frame: &mut Frame, old: &str, new: &str) -> bool {
    let mut changed = false;
    if let Some(menu) = &mut frame.menu {
        changed |= in_commands(&mut menu.items, old, new);
    }
    if let Some(toolbar) = &mut frame.toolbar {
        changed |= in_commands(&mut toolbar.items, old, new);
    }
    for group in frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()).flat_map(|t| t.groups.iter_mut()) {
        changed |= in_commands(&mut group.items, old, new);
    }
    for sidebar in &mut frame.sidebars {
        changed |= in_widgets(&mut sidebar.widgets, old, new);
    }
    if let Some(footer) = &mut frame.footer {
        changed |= in_widgets(&mut footer.widgets, old, new);
    }
    changed
}

fn in_commands(items: &mut [Command], old: &str, new: &str) -> bool {
    let mut changed = false;
    for command in items {
        changed |= swap(&mut command.props.on_click, old, new);
        changed |= in_commands(&mut command.items, old, new);
    }
    changed
}

fn in_widgets(widgets: &mut [Widget], old: &str, new: &str) -> bool {
    let mut changed = false;
    for w in widgets {
        changed |= swap(&mut w.props.on_click, old, new);
        changed |= swap(&mut w.props.view, old, new);
        changed |= swap(&mut w.props.shows, old, new);
        changed |= swap(&mut w.props.toolbox_page, old, new);
        if let Some(menu) = &mut w.menu {
            changed |= in_commands(&mut menu.items, old, new);
        }
        changed |= in_widgets(&mut w.children, old, new);
        for page in &mut w.pages {
            changed |= swap(&mut page.view, old, new);
            changed |= in_widgets(&mut page.children, old, new);
        }
    }
    changed
}

/// Where the statements `body` open or ask for the page `name`.
fn opened(body: &[Stmt], name: &str, found: &mut Vec<Span>) {
    // A command's `ask` is an expression.
    crate::code::each_expr_in(body, &mut |e| {
        if let ExprKind::Ask { form, .. } = &e.kind
            && let AskForm::Window { window, .. } = form.as_ref()
            && window.name == name
        {
            found.push(window.span);
        }
    });
    for stmt in body {
        match &stmt.kind {
            StmtKind::Open { window, .. } if window.name == name => found.push(window.span),
            StmtKind::Ask { form, .. } => {
                if let AskForm::Window { window, .. } = form.as_ref()
                    && window.name == name
                {
                    found.push(window.span);
                }
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    opened(&branch.body, name, found);
                }
                if let Some(body) = otherwise {
                    opened(body, name, found);
                }
            }
            StmtKind::For { body, .. } => opened(body, name, found),
            _ => {}
        }
    }
}

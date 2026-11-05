//! Views shown in a window's parts (DESIGN 5.33): `slideout "Class" name
//! classPanel view ClassEditor` shows the view `page ClassEditor: view` in
//! the slide-out, and `section "Class" view ClassEditor` in an accordion's
//! section (a tab or a wizard's step alike). A view is designed as a window
//! is (its parts on its grid, its data, its code), without a frame of its
//! own. Where it shows, its parts, data and handlers join the window's: the
//! window's code reaches them by name, so a window sets what its view shows,
//! and the view's own code reaches only its own. The checker keeps the names
//! apart; the generators and the preview take the project with each view
//! placed in its part (`placed`), so every stack writes it as parts of the
//! window.

use crate::code::{CodeBlock, ExprKind, Stmt, StmtKind};
use crate::model::{Page, PageKind, Project, SpecFile, TellHandler, Widget};
use crate::span::Span;
use crate::widgets::WidgetKind;

/// A place in a window that shows a view: a slide-out, or a tab, step or section.
#[derive(Debug, Clone, Copy)]
pub struct Place<'a> {
    /// The name of the view it shows.
    pub view: &'a str,
    /// What the place is, as people say it: "slide-out", "tab", "step" or "section".
    pub what: &'static str,
    /// Whether it holds parts of its own besides.
    pub holds_parts: bool,
    /// What it hears of the view: its tells wired to the window's handlers.
    pub tells: &'a [TellHandler],
    pub span: Span,
}

/// The places among `widgets` (and the containers in them) that show a view, in the order they are written.
pub fn places(widgets: &[Widget]) -> Vec<Place<'_>> {
    let mut out = Vec::new();
    fn walk<'a>(widgets: &'a [Widget], out: &mut Vec<Place<'a>>) {
        for w in widgets {
            if let Some(view) = w.props.view.as_deref() {
                out.push(Place { view, what: "slide-out", holds_parts: !w.children.is_empty(), tells: &w.props.tells, span: w.span });
            }
            for page in &w.pages {
                if let Some(view) = page.view.as_deref() {
                    out.push(Place { view, what: page_word(w.props.kind), holds_parts: !page.children.is_empty(), tells: &page.tells, span: page.span });
                }
            }
            walk(&w.children, out);
            for page in &w.pages {
                walk(&page.children, out);
            }
        }
    }
    walk(widgets, &mut out);
    out
}

/// What a container's page is called: a tab, a wizard's step or an accordion's section.
fn page_word(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Steps => "step",
        WidgetKind::Accordion => "section",
        _ => "tab",
    }
}

/// The view named `name` among `files`; none when there is no page of that name or it is no view.
pub fn find<'a>(files: impl IntoIterator<Item = &'a SpecFile>, name: &str) -> Option<&'a Page> {
    files.into_iter().flat_map(|f| &f.pages).find(|p| p.name.name == name && p.kind == PageKind::View)
}

/// The views a window shows, in the order its parts name them.
pub fn shown_in<'a>(files: &'a [SpecFile], page: &'a Page) -> Vec<&'a Page> {
    places(&page.widgets).into_iter().filter_map(|place| find(files, place.view)).collect()
}

/// The project with every view placed where a window shows it: the view's
/// parts as the slide-out's or section's own, its data among the window's,
/// and its handlers in the window's code, where each `tell` runs the
/// window's handler that hears it (and an unheard one does nothing). The
/// view's page and code block go, being the windows' now; the designer
/// draws a view on its own from the project as written.
pub fn placed(project: &Project) -> Project {
    if !project.files.iter().flat_map(|f| &f.pages).any(|p| p.kind == PageKind::View) {
        return project.clone();
    }
    let mut placed = project.clone();
    let views: Vec<(Page, Option<CodeBlock>)> = project
        .files
        .iter()
        .flat_map(|f| f.pages.iter().filter(|p| p.kind == PageKind::View).map(move |p| (p.clone(), f.code.iter().find(|c| c.name.name == p.name.name).cloned())))
        .collect();
    for file in &mut placed.files {
        let mut added: Vec<(String, CodeBlock)> = Vec::new();
        for page in file.pages.iter_mut().filter(|p| p.kind != PageKind::View) {
            let mut shown: Vec<(&(Page, Option<CodeBlock>), Vec<TellHandler>)> = Vec::new();
            fill(&mut page.widgets, &views, &mut shown);
            for ((view, code), heard) in shown {
                // Data the window has already (the project's settings, which every page has) stays the window's.
                let fresh: Vec<_> = view.data.iter().filter(|d| !page.data.iter().any(|w| w.name.name == d.name.name)).cloned().collect();
                page.data.extend(fresh);
                if let Some(code) = code {
                    let mut code = code.clone();
                    for handler in &mut code.handlers {
                        heard_as_calls(&mut handler.body, &heard);
                    }
                    added.push((page.name.name.clone(), code));
                }
            }
        }
        // A view is the windows' that show it, its parts and its code: never a window or code of its own in a program.
        file.code.retain(|c| !views.iter().any(|(v, _)| v.name.name == c.name.name));
        file.pages.retain(|p| p.kind != PageKind::View);
        for (window, code) in added {
            match file.code.iter_mut().find(|c| c.name.name == window) {
                Some(own) => {
                    own.handlers.extend(code.handlers);
                    own.events.extend(code.events);
                }
                None => {
                    let mut own = code;
                    own.name.name = window;
                    file.code.push(own);
                }
            }
        }
    }
    placed
}

/// Each `tell` in `body` as a call of the window's handler that hears it (`heard`); an unheard one goes.
fn heard_as_calls(body: &mut Vec<Stmt>, heard: &[TellHandler]) {
    body.retain_mut(|stmt| {
        match &mut stmt.kind {
            StmtKind::Tell { tell, args } => {
                let Some(wired) = heard.iter().find(|h| h.tell == tell.name) else { return false };
                let function = crate::model::Ident { name: wired.handler.clone(), span: tell.span };
                let call = crate::code::Expr { kind: ExprKind::Call { function, args: std::mem::take(args), named: Vec::new() }, span: stmt.span };
                stmt.kind = StmtKind::Call { call };
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    heard_as_calls(&mut branch.body, heard);
                }
                if let Some(otherwise) = otherwise {
                    heard_as_calls(otherwise, heard);
                }
            }
            StmtKind::For { body, .. } => heard_as_calls(body, heard),
            _ => {}
        }
        true
    });
}

/// Puts each view's parts in the places among `widgets` that show it, and notes the views shown with what each place hears of it.
fn fill<'v>(widgets: &mut [Widget], views: &'v [(Page, Option<CodeBlock>)], shown: &mut Vec<(&'v (Page, Option<CodeBlock>), Vec<TellHandler>)>) {
    let find = |name: &str| views.iter().find(|(v, _)| v.name.name == name);
    for widget in widgets.iter_mut() {
        if let Some(found) = widget.props.view.as_deref().and_then(find) {
            widget.children = found.0.widgets.clone();
            // What it hears becomes calls in the view's code, so no stack sees it as an event of the part.
            shown.push((found, std::mem::take(&mut widget.props.tells)));
        }
        for page in widget.pages.iter_mut() {
            if let Some(found) = page.view.as_deref().and_then(find) {
                page.children = found.0.widgets.clone();
                shown.push((found, std::mem::take(&mut page.tells)));
            }
        }
        if widget.props.view.is_none() {
            fill(&mut widget.children, views, shown);
        }
        for page in widget.pages.iter_mut().filter(|p| p.view.is_none()) {
            fill(&mut page.children, views, shown);
        }
    }
}

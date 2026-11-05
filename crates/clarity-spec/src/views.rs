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

use crate::code::CodeBlock;
use crate::model::{Page, PageKind, Project, SpecFile, Widget};
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
    pub span: Span,
}

/// The places among `widgets` (and the containers in them) that show a view, in the order they are written.
pub fn places(widgets: &[Widget]) -> Vec<Place<'_>> {
    let mut out = Vec::new();
    fn walk<'a>(widgets: &'a [Widget], out: &mut Vec<Place<'a>>) {
        for w in widgets {
            if let Some(view) = w.props.view.as_deref() {
                out.push(Place { view, what: "slide-out", holds_parts: !w.children.is_empty(), span: w.span });
            }
            for page in &w.pages {
                if let Some(view) = page.view.as_deref() {
                    out.push(Place { view, what: page_word(w.props.kind), holds_parts: !page.children.is_empty(), span: page.span });
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
/// and its handlers in the window's code. The view's page and code stay as they are.
pub fn placed(project: &Project) -> Project {
    let shows_any = project.files.iter().flat_map(|f| &f.pages).any(|p| !places(&p.widgets).is_empty());
    if !shows_any {
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
            let mut shown: Vec<&(Page, Option<CodeBlock>)> = Vec::new();
            fill(&mut page.widgets, &views, &mut shown);
            for (view, code) in shown {
                // Data the window has already (the project's settings, which every page has) stays the window's.
                let fresh: Vec<_> = view.data.iter().filter(|d| !page.data.iter().any(|w| w.name.name == d.name.name)).cloned().collect();
                page.data.extend(fresh);
                if let Some(code) = code {
                    added.push((page.name.name.clone(), code.clone()));
                }
            }
        }
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

/// Puts each view's parts in the places among `widgets` that show it, and notes the views shown.
fn fill<'v>(widgets: &mut [Widget], views: &'v [(Page, Option<CodeBlock>)], shown: &mut Vec<&'v (Page, Option<CodeBlock>)>) {
    let find = |name: &str| views.iter().find(|(v, _)| v.name.name == name);
    for widget in widgets.iter_mut() {
        if let Some(found) = widget.props.view.as_deref().and_then(find) {
            widget.children = found.0.widgets.clone();
            shown.push(found);
        }
        for page in widget.pages.iter_mut() {
            if let Some(found) = page.view.as_deref().and_then(find) {
                page.children = found.0.widgets.clone();
                shown.push(found);
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

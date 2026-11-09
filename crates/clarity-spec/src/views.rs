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
    /// What the place is, as people say it: "content" (the one place a view shows), or "slide-out", "tab", "step" or "section", which the checker refuses.
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
        WidgetKind::Content => "content",
        _ => "tab",
    }
}

/// The view named `name` among `files`; none when there is no page of that name or it is no view.
pub fn find<'a>(files: impl IntoIterator<Item = &'a SpecFile>, name: &str) -> Option<&'a Page> {
    files.into_iter().flat_map(|f| &f.pages).find(|p| p.name.name == name && p.kind == PageKind::View)
}

/// The data of a page and of the views it shows (and theirs): what opening
/// it `with` a value may set, since a window's data lives in its views.
pub fn page_data<'a>(files: impl IntoIterator<Item = &'a SpecFile> + Clone, page: &'a Page) -> Vec<&'a crate::model::DataDecl> {
    let mut out: Vec<&'a crate::model::DataDecl> = page.data.iter().collect();
    let mut queue: Vec<&'a Page> = vec![page];
    let mut seen: Vec<&'a Page> = Vec::new();
    while let Some(next) = queue.pop() {
        if seen.iter().any(|s| std::ptr::eq(*s, next)) || seen.len() > 64 {
            continue;
        }
        seen.push(next);
        for place in places(&next.widgets) {
            if let Some(view) = find(files.clone(), place.view) {
                out.extend(view.data.iter());
                queue.push(view);
            }
        }
    }
    out
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
    // A tree of several kinds of records is a tree of its rows once its window holds what its views show.
    crate::trees::lowered(&placed_views(project))
}

fn placed_views(project: &Project) -> Project {
    // A toolbox (`page X: toolbox`) is placed as a view is: its parts in the canvas that names it, its code the window's.
    let placeable = |kind: PageKind| matches!(kind, PageKind::View | PageKind::Toolbox);
    if !project.files.iter().flat_map(|f| &f.pages).any(|p| placeable(p.kind)) {
        return project.clone();
    }
    let mut placed = project.clone();
    let views: Vec<(Page, Option<CodeBlock>)> = project
        .files
        .iter()
        .flat_map(|f| f.pages.iter().filter(|p| placeable(p.kind)).map(move |p| (p.clone(), f.code.iter().find(|c| c.name.name == p.name.name).cloned())))
        .collect();
    for file in &mut placed.files {
        let mut added: Vec<(String, CodeBlock)> = Vec::new();
        for page in file.pages.iter_mut().filter(|p| !placeable(p.kind)) {
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
        // `main.swap(X)` in a window's content opens the page of X, counted from 1, as a tab is opened.
        let contents: Vec<(String, Vec<(String, Vec<String>)>)> = file.pages.iter().filter(|p| !placeable(p.kind)).map(|p| (p.name.name.clone(), content_views(&p.widgets))).collect();
        // A view is the windows' that show it, its parts and its code: never a window or code of its own in a program.
        file.code.retain(|c| !views.iter().any(|(v, _)| v.name.name == c.name.name));
        file.pages.retain(|p| !placeable(p.kind));
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
        for (window, listed) in &contents {
            if listed.is_empty() {
                continue;
            }
            if let Some(code) = file.code.iter_mut().find(|c| &c.name.name == window) {
                for handler in &mut code.handlers {
                    views_as_pages(&mut handler.body, listed);
                }
            }
        }
    }
    placed
}

/// Each window's content among `widgets` (and the containers in them), by name, with the views it lists in order.
fn content_views(widgets: &[Widget]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for w in widgets {
        if w.props.kind == WidgetKind::Content
            && let Some(name) = &w.props.name
        {
            out.push((name.clone(), w.pages.iter().filter_map(|p| p.view.clone()).collect()));
        }
        out.extend(content_views(&w.children));
        for page in &w.pages {
            out.extend(content_views(&page.children));
        }
    }
    out
}

/// Each `main.swap(X)` in `body` as `main.tab = n`, n the place of X in the views main lists, from 1.
fn views_as_pages(body: &mut [Stmt], listed: &[(String, Vec<String>)]) {
    for stmt in body {
        match &mut stmt.kind {
            StmtKind::Draw { canvas, shape, args } if shape.name == "swap" => {
                if let Some((_, views)) = listed.iter().find(|(n, _)| *n == canvas.name)
                    && let [crate::code::Expr { kind: ExprKind::Name { name }, .. }] = args.as_slice()
                    && let Some(at) = views.iter().position(|v| v == name)
                {
                    let span = stmt.span;
                    let part = crate::code::Expr { kind: ExprKind::Name { name: canvas.name.clone() }, span: canvas.span };
                    let target = crate::code::Expr { kind: ExprKind::Member { object: Box::new(part), member: crate::model::Ident { name: "tab".to_string(), span: shape.span } }, span };
                    let value = crate::code::Expr { kind: ExprKind::Number { value: (at + 1).to_string() }, span };
                    stmt.kind = StmtKind::Assign { target, value };
                }
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    views_as_pages(&mut branch.body, listed);
                }
                if let Some(otherwise) = otherwise {
                    views_as_pages(otherwise, listed);
                }
            }
            StmtKind::For { body, .. } => views_as_pages(body, listed),
            _ => {}
        }
    }
}

/// The toolbox's parts work on the canvas that shows the toolbox: their `for` names it.
fn for_canvas(parts: &mut [Widget], canvas: &str) {
    for part in parts {
        if matches!(part.props.kind, WidgetKind::Tool | WidgetKind::ToolGroup | WidgetKind::ToolSize | WidgetKind::ToolOpacity | WidgetKind::ToolColors | WidgetKind::ToolLayers) {
            part.props.for_part = Some(canvas.to_string());
        }
        for_canvas(&mut part.children, canvas);
    }
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
    fill_at(widgets, views, shown, 0);
}

/// How deep views may be placed inside views: a view that places itself stops here.
const VIEW_DEPTH: usize = 8;

fn fill_at<'v>(widgets: &mut [Widget], views: &'v [(Page, Option<CodeBlock>)], shown: &mut Vec<(&'v (Page, Option<CodeBlock>), Vec<TellHandler>)>, depth: usize) {
    if depth > VIEW_DEPTH {
        return;
    }
    let find = |name: &str| views.iter().find(|(v, _)| v.name.name == name);
    for widget in widgets.iter_mut() {
        // A canvas's toolbox: the toolbox's parts are the canvas's, each tool for that canvas.
        if let Some(found) = widget.props.toolbox_page.as_deref().and_then(find).filter(|f| f.0.kind == PageKind::Toolbox) {
            let mut parts = found.0.widgets.clone();
            if let Some(canvas) = widget.props.name.clone() {
                for_canvas(&mut parts, &canvas);
            }
            widget.children = parts;
            widget.props.toolbox_columns = Some(found.0.columns);
            shown.push((found, Vec::new()));
            continue;
        }
        if let Some(found) = widget.props.view.as_deref().and_then(find) {
            widget.children = found.0.widgets.clone();
            // What it hears becomes calls in the view's code, so no stack sees it as an event of the part.
            shown.push((found, std::mem::take(&mut widget.props.tells)));
        }
        for page in widget.pages.iter_mut() {
            if let Some(found) = page.view.as_deref().and_then(find) {
                page.children = found.0.widgets.clone();
                // The view keeps its own grid: its parts are placed on its columns, not the container's span.
                page.columns.get_or_insert(found.0.columns);
                shown.push((found, std::mem::take(&mut page.tells)));
            }
        }
        // A view's own parts are placed in turn: a toolbox on its canvas, a view inside it.
        let deeper = if widget.props.view.is_some() { depth + 1 } else { depth };
        fill_at(&mut widget.children, views, shown, deeper);
        for page in widget.pages.iter_mut() {
            let deeper = if page.view.is_some() { depth + 1 } else { depth };
            fill_at(&mut page.children, views, shown, deeper);
        }
    }
}

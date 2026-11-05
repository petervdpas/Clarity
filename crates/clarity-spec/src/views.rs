//! Views shown in a window's parts (DESIGN 5.33): `slideout "Class" name
//! classPanel view ClassEditor` shows the view `page ClassEditor: view` in
//! the slide-out. A view is designed as a window is (its parts on its grid,
//! its data, its code), without a frame of its own. Where it shows, its
//! parts, data and handlers join the window's: the window's code reaches
//! them by name, so a window sets what its view shows, and the view's own
//! code reaches only its own. The checker keeps the names apart; the
//! generators and the preview take the project with each view placed in
//! its part (`placed`), so every stack writes it as parts of the window.

use crate::code::CodeBlock;
use crate::model::{Page, PageKind, Project, SpecFile, Widget};

/// The parts of `widgets` that show a view, with the view's name: the
/// window's slide-outs, which stand on the window.
pub fn showing(widgets: &[Widget]) -> impl Iterator<Item = (&Widget, &str)> {
    widgets.iter().filter_map(|w| w.props.view.as_deref().map(|v| (w, v)))
}

/// The view named `name` among `files`; none when there is no page of that name or it is no view.
pub fn find<'a>(files: impl IntoIterator<Item = &'a SpecFile>, name: &str) -> Option<&'a Page> {
    files.into_iter().flat_map(|f| &f.pages).find(|p| p.name.name == name && p.kind == PageKind::View)
}

/// The views a window shows, in the order its parts name them.
pub fn shown_in<'a>(files: &'a [SpecFile], page: &'a Page) -> Vec<&'a Page> {
    showing(&page.widgets).filter_map(|(_, name)| find(files, name)).collect()
}

/// The project with every view placed where a window shows it: the view's
/// parts as the part's own, its data among the window's, and its handlers in
/// the window's code. The view's page and code stay as they are.
pub fn placed(project: &Project) -> Project {
    let shows_any = project.files.iter().flat_map(|f| &f.pages).any(|p| showing(&p.widgets).next().is_some());
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
            for widget in page.widgets.iter_mut() {
                let Some(name) = widget.props.view.clone() else { continue };
                let Some((view, code)) = views.iter().find(|(v, _)| v.name.name == name) else { continue };
                widget.children = view.widgets.clone();
                page.data.extend(view.data.iter().cloned());
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

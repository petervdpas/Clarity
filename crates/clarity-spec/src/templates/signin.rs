//! New window's sign-in window, made from the project's signin block: window,
//! content and a sign-in view per way of signing in, guided as the checker
//! says (`crate::check`'s sign-in rules).

use super::*;

/// The sign-in window to start from (docs/APP-CAPABILITIES-PLAN.md, "The
/// sign-in window is designed, and guided"): window, content, views. Each
/// way of signing in that takes typed inputs (password, recovery code,
/// token) gets a view of its own: the language dropdown, a field per input,
/// keep me signed in, its button, an "other way" to each other view and the
/// refusal. A way that takes nothing typed (the computer's account) is a
/// button on the first view. The views follow the block's `shows`, else its
/// order; the first opens first.
pub(super) fn signin_window(project: &Project, name: &str, title: &str) -> Result<Made, String> {
    use crate::design_edit::Designed;
    use crate::model::{PageKind, WidgetPage};
    let block = project.signin.as_ref().ok_or("the project has no sign-in; turn it on in Sign-in first")?;
    let offered: Vec<&str> = if block.shows.is_empty() { block.strategies.iter().map(|s| s.name.as_str()).collect() } else { block.shows.iter().map(String::as_str).collect() };
    let inputs_of = |strategy: &str| crate::signin::strategy_info(strategy).map(|i| i.inputs.to_vec()).unwrap_or_default();
    let typed: Vec<&str> = offered.iter().copied().filter(|s| !inputs_of(s).is_empty()).collect();
    let one_click: Vec<&str> = offered.iter().copied().filter(|s| inputs_of(s).is_empty()).collect();
    // A view per typed way, named after it; with none typed, one view of the one-click buttons.
    let ways: Vec<&str> = if typed.is_empty() { vec!["signIn"] } else { typed.clone() };
    let mut names = Vec::new();
    for way in &ways {
        let word = match *way {
            "recovery" => "Code".to_string(),
            other => {
                let mut c = other.chars();
                c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
            }
        };
        let plain = format!("{word}View");
        let view = if free_name(project, &plain).is_ok() { plain } else { format!("{name}{word}View") };
        free_name(project, &view)?;
        names.push(view);
    }
    let upper = |word: &str| {
        let mut c = word.chars();
        c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
    };
    let mut views = Vec::new();
    let mut tallest = 1;
    for (i, way) in ways.iter().enumerate() {
        let mut view = page_of(&names[i], PageKind::View, None);
        let mut row = 1;
        let mut parts = Vec::new();
        let mut place = |kind: WidgetKind, part: &str, rows: u32, set: &dyn Fn(&mut WidgetProps)| {
            let mut props = new_props(kind, 1, row);
            props.name = Some(part.to_string());
            props.width = DEFAULT_PAGE_COLUMNS;
            props.height = rows;
            set(&mut props);
            parts.push(widget_of(props));
            row += rows;
        };
        if block.language_picker {
            place(WidgetKind::SigninLanguage, &format!("{way}Language"), 2, &|_| {});
        }
        if typed.contains(way) {
            for input in inputs_of(way) {
                place(WidgetKind::SigninField, &format!("{way}{}Field", upper(input)), 2, &|p| p.input = Some(input.to_string()));
            }
            if block.remember_asks && *way != "recovery" {
                place(WidgetKind::SigninRemember, &format!("{way}Remember"), 1, &|_| {});
            }
            place(WidgetKind::SigninButton, &format!("{way}Button"), 1, &|p| p.strategy = Some(way.to_string()));
        }
        if i == 0 {
            for click in &one_click {
                place(WidgetKind::SigninButton, &format!("{click}Button"), 1, &|p| p.strategy = Some(click.to_string()));
            }
        }
        for (j, other) in names.iter().enumerate().filter(|(j, _)| *j != i) {
            let target = other.clone();
            place(WidgetKind::SigninOther, &format!("{way}To{}", upper(ways[j])), 1, &|p| p.shows = Some(target.clone()));
        }
        place(WidgetKind::SigninRefusal, &format!("{way}Refusal"), 1, &|_| {});
        tallest = tallest.max(row - 1);
        view.widgets = parts;
        views.push(view);
    }
    let mut window = page_of(name, PageKind::Signin, Some(title.to_string()));
    // As high as the tallest view, a row being its height and the gap under it, within the window sizes.
    let height = 80 + tallest * (crate::layout::ROW_HEIGHT + 8);
    window.size = Some(crate::model::WindowSize { width: crate::layout::SIGNIN_SIZE.0, height: height.max(crate::layout::SIGNIN_SIZE.1) });
    let mut content = new_props(WidgetKind::Content, 1, 1);
    content.name = Some("ways".into());
    content.width = DEFAULT_PAGE_COLUMNS;
    content.height = tallest;
    let mut content = widget_of(content);
    for view in &names {
        content.pages.push(WidgetPage { title: None, columns: None, view: Some(view.clone()), tells: Vec::new(), color: None, children: Vec::new(), span: crate::span::Span::default() });
    }
    window.widgets = vec![content];
    let designed = std::iter::once(Designed::Page(window)).chain(views.into_iter().map(Designed::Page)).collect();
    Ok(Made { designed, code: Vec::new() })
}

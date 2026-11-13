//! A design edit never leaves parts reaching past what holds them
//! (docs/ledger/app-mcp.md, 2026-11-13): after every edit of the design,
//! each part of a window is cut to end by the last row the window's height
//! holds under its bars (`layout::rows_held`), a slide-out to that many rows,
//! and each part of a view to the rows the window holds from the top of the
//! content area that shows it (a content area grows to fill the window).
//! The window keeps its size. A part that starts below the last row stays
//! where it is, as moving it could cover another; the checker says so.

use crate::model::{Page, Widget};
use crate::widgets::{off_grid, WidgetKind};

/// Whether a part lies over the window from its edge (a slide-out not beside a part).
fn slides(w: &Widget) -> bool {
    w.props.kind == WidgetKind::Slideout && w.props.beside.is_none()
}

/// `w` cut to end by row `last`; true when it changed.
fn cut(w: &mut Widget, last: u32) -> bool {
    let rows = w.props.height.max(1);
    if w.props.row <= last && w.props.row + rows - 1 > last {
        w.props.height = last - w.props.row + 1;
        return true;
    }
    false
}

/// A window or dialog fitted to the `holds` rows its height holds: its parts
/// end by the last of them, and a slide-out is no taller. True when it changed.
pub fn fit_window(page: &mut Page, holds: u32) -> bool {
    let holds = holds.max(1);
    let mut changed = false;
    for w in page.widgets.iter_mut() {
        if slides(w) {
            if w.props.height.max(1) > holds {
                w.props.height = holds;
                changed = true;
            }
        } else if !off_grid(w.props.kind) {
            changed |= cut(w, holds);
        }
    }
    changed
}

/// A view fitted to the `rows` it has: what the window holds from its content area's top. True when it changed.
pub fn fit_view(view: &mut Page, rows: u32) -> bool {
    let rows = rows.max(1);
    let mut changed = false;
    for w in view.widgets.iter_mut().filter(|w| !off_grid(w.props.kind)) {
        changed |= cut(w, rows);
    }
    changed
}

/// The views the content areas of `page` show, with the rows each has in a
/// window holding `holds` rows: a content area grows to fill the window, so
/// from its top row to the last (in a slide-out, the slide-out's rows).
pub fn shown_views(page: &Page, holds: u32) -> Vec<(String, u32)> {
    fn walk(widgets: &[Widget], holds: u32, out: &mut Vec<(String, u32)>) {
        for w in widgets {
            if w.props.kind == WidgetKind::Content {
                let rows = holds.saturating_sub(w.props.row.max(1) - 1).max(1);
                for view in w.pages.iter().filter_map(|p| p.view.clone()) {
                    out.push((view, rows));
                }
            }
            // Inside a slide-out the rows are its own, which are no more than the window's.
            let inner = if slides(w) { w.props.height.max(1).min(holds) } else { holds };
            walk(&w.children, inner, out);
        }
    }
    let mut out = Vec::new();
    walk(&page.widgets, holds, &mut out);
    out
}

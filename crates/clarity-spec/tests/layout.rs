//! The layout values every renderer shares (design section 2): the web
//! renderer writes them in TypeScript, the generators read them here, and
//! this test keeps the two equal.

use clarity_spec::layout::{BUTTON_SIDES, CONTENT_PADDING, GAP, INPUT_SIDES, MESSAGE_MS, ROW_HEIGHT, WINDOW_HEIGHT, WINDOW_WIDTH};

const GRID_TS: &str = include_str!("../../../runtimes/web/src/screen/grid.ts");
/// The running window of every generated program: Clarity.UI (DESIGN 7.9).
const MESSAGES_RS: &str = include_str!("../../../runtimes/rust-ui/src/window/messages.rs");
const GRID_RS: &str = include_str!("../../../runtimes/rust-ui/src/grid.rs");
const VIEW_RS: &str = include_str!("../../../runtimes/rust-ui/src/window/view.rs");
const SIDEBARS_TS: &str = include_str!("../../../runtimes/web/src/frame/sidebars.ts");
const RENDER_CSS: &str = include_str!("../../../runtimes/web/src/screen/render.css");
const SHOWN_WINDOW: &str = include_str!("../../../app/src/lib/ShownWindow.svelte");
const PAGE_DESIGNER: &str = include_str!("../../../app/src/lib/PageDesigner.svelte");
const APP: &str = include_str!("../../../app/src/App.svelte");

/// Every language opens its windows through the one Clarity.UI library
/// (DESIGN 7.9), whose desktop host this is.
const SHELLS: [(&str, &str); 1] = [("Clarity.UI", include_str!("../../../runtimes/rust-ui/src/desktop/mod.rs"))];

/// The number a `const NAME = 123;` line in `source` gives.
fn constant(source: &str, name: &str) -> u32 {
    let line = source
        .lines()
        .find(|l| l.contains(&format!("const {name} = ")))
        .unwrap_or_else(|| panic!("no constant {name}"));
    line.split('=').nth(1).and_then(|v| v.trim().trim_end_matches(';').parse().ok()).unwrap_or_else(|| panic!("{line}"))
}

#[test]
fn the_web_renderer_uses_the_shared_values() {
    assert_eq!(constant(GRID_TS, "ROW_HEIGHT"), ROW_HEIGHT);
    // Clarity.UI's rows, and the row heights a project may say.
    let (least, most) = clarity_spec::layout::ROW_HEIGHTS;
    assert!(GRID_RS.contains(&format!("pub const ROW_HEIGHT: f64 = {ROW_HEIGHT}.0;")), "Clarity.UI's standard row");
    assert!(GRID_RS.contains(&format!("pub const ROW_HEIGHTS: (f64, f64) = ({least}.0, {most}.0);")), "Clarity.UI's row heights");
    assert_eq!(constant(GRID_TS, "GAP"), GAP);
    assert_eq!(constant(GRID_TS, "WINDOW_WIDTH"), WINDOW_WIDTH);
    assert_eq!(constant(GRID_TS, "WINDOW_HEIGHT"), WINDOW_HEIGHT);
    assert_eq!(constant(GRID_TS, "CONTENT_PADDING"), CONTENT_PADDING);
    assert_eq!((constant(GRID_TS, "WINDOW_MIN_WIDTH"), constant(GRID_TS, "WINDOW_MIN_HEIGHT")), clarity_spec::layout::WINDOW_MIN);
    assert_eq!((constant(GRID_TS, "WINDOW_MAX_WIDTH"), constant(GRID_TS, "WINDOW_MAX_HEIGHT")), clarity_spec::layout::WINDOW_MAX);
    assert!(MESSAGES_RS.contains(&format!("Duration::from_millis({MESSAGE_MS})")), "Clarity.UI keeps an info message MESSAGE_MS");
    // A sidebar's widths.
    let (narrowest, widest) = clarity_spec::model::SIDEBAR_WIDTHS;
    assert_eq!((constant(SIDEBARS_TS, "NARROWEST"), constant(SIDEBARS_TS, "WIDEST")), (narrowest, widest));
    // The space around a running window's grid, in Clarity.UI.
    assert!(VIEW_RS.contains(&format!("pub const CONTENT_PADDING: f64 = {CONTENT_PADDING}.0;")), "Clarity.UI pads a window's grid by CONTENT_PADDING");
}

#[test]
fn the_web_renderer_pads_parts_by_the_shared_multiples() {
    // Beside the text of a button and of an input: a multiple of the theme's padding.
    assert!(RENDER_CSS.contains(&format!("padding: var(--padding) calc(var(--padding) * {BUTTON_SIDES});")), "button");
    assert!(RENDER_CSS.contains(&format!("padding: var(--padding) calc(var(--padding) * {INPUT_SIDES});")), "input");
}

/// The designer shows a window at the size it opens at, so what is placed
/// there is what the running window shows: every shell opens at that size.
#[test]
fn every_window_opens_at_the_size_the_designer_shows() {
    for (stack, shell) in SHELLS {
        assert!(shell.contains(&WINDOW_WIDTH.to_string()) && shell.contains(&WINDOW_HEIGHT.to_string()), "{stack}");
    }
    assert!(SHOWN_WINDOW.contains("WINDOW_WIDTH"));
    // The designer and the preview both show the window so, at its own size
    // (or the standard one), drawn by Clarity.UI as it runs (DESIGN 8.1), its padding Clarity.UI's own.
    assert!(APP.contains("<ShownWindow") && APP.contains("clientWidth={windowSize(page).width}") && APP.contains("windowSize(page).height") && APP.contains("<ClarityWindow"), "preview");
    // The designer shows the size a window's edge is dragged to while it is dragged.
    assert!(PAGE_DESIGNER.contains("const shownSize = $derived(sizing ?? windowSize(page));"), "designer");
    assert!(PAGE_DESIGNER.contains("<ShownWindow {area} clientWidth={shownSize.width}") && PAGE_DESIGNER.contains("style:height=\"{shownSize.height}px\"") && PAGE_DESIGNER.contains("<ClarityWindow"), "designer");
}

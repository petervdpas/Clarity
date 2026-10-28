//! The layout values every renderer shares (design section 2): the web
//! renderer writes them in TypeScript, the generators read them here, and
//! this test keeps the two equal.

use clarity_spec::layout::{BUTTON_SIDES, CONTENT_PADDING, GAP, INPUT_SIDES, MESSAGE_MS, ROW_HEIGHT, WINDOW_HEIGHT, WINDOW_WIDTH};

const GRID_TS: &str = include_str!("../../../app/src/lib/grid.ts");
const RUNTIME_SVELTE: &str = include_str!("../../../app/src/runtime/Runtime.svelte");
const SIDEBARS_TS: &str = include_str!("../../../app/src/lib/sidebars.ts");
const SIDEBAR_VIEW: &str = include_str!("../../../runtimes/csharp/Clarity.Avalonia/SidebarView.cs");
const FRAME_VIEW: &str = include_str!("../../../app/src/lib/FrameView.svelte");
const RENDER_CSS: &str = include_str!("../../../app/src/render.css");
const SHOWN_WINDOW: &str = include_str!("../../../app/src/lib/ShownWindow.svelte");
const PAGE_DESIGNER: &str = include_str!("../../../app/src/lib/PageDesigner.svelte");
const APP: &str = include_str!("../../../app/src/App.svelte");

/// The shells of the stacks whose runtime opens the window itself.
const SHELLS: [(&str, &str); 4] = [
    ("Go", include_str!("../../../runtimes/go/desktop/desktop.go")),
    ("Avalonia", include_str!("../../../runtimes/csharp/Clarity.Avalonia/Shell.cs")),
    ("C", include_str!("../../../runtimes/c/shell/service.c")),
    ("C++", include_str!("../../../runtimes/cpp/shell/service.cpp")),
];

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
    assert_eq!(constant(GRID_TS, "GAP"), GAP);
    assert_eq!(constant(GRID_TS, "WINDOW_WIDTH"), WINDOW_WIDTH);
    assert_eq!(constant(GRID_TS, "WINDOW_HEIGHT"), WINDOW_HEIGHT);
    assert_eq!(constant(GRID_TS, "CONTENT_PADDING"), CONTENT_PADDING);
    assert_eq!((constant(GRID_TS, "WINDOW_MIN_WIDTH"), constant(GRID_TS, "WINDOW_MIN_HEIGHT")), clarity_spec::layout::WINDOW_MIN);
    assert_eq!((constant(GRID_TS, "WINDOW_MAX_WIDTH"), constant(GRID_TS, "WINDOW_MAX_HEIGHT")), clarity_spec::layout::WINDOW_MAX);
    assert_eq!(constant(RUNTIME_SVELTE, "MESSAGE_MS"), MESSAGE_MS);
    // A sidebar's widths, the same on the web and in Avalonia.
    let (narrowest, widest) = clarity_spec::model::SIDEBAR_WIDTHS;
    assert_eq!((constant(SIDEBARS_TS, "NARROWEST"), constant(SIDEBARS_TS, "WIDEST")), (narrowest, widest));
    assert!(SIDEBAR_VIEW.contains(&format!("Narrowest = {narrowest}, Widest = {widest};")), "SidebarView.cs");
    assert!(FRAME_VIEW.contains("sidebarWidth("), "FrameView sizes sidebars by sidebars.ts");
    // The space around a running window's grid, written in its CSS.
    let client = RUNTIME_SVELTE.split("  .client {").nth(1).expect("the .client rule");
    assert!(client.split('}').next().unwrap().contains(&format!("padding: {CONTENT_PADDING}px;")), "{client}");
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
    // (or the standard one) and with the running window's padding around its grid.
    assert!(APP.contains("<ShownWindow") && APP.contains("clientWidth={windowSize(page).width}") && APP.contains("windowSize(page).height") && APP.contains("CONTENT_PADDING"), "preview");
    // The designer shows the size a window's edge is dragged to while it is dragged.
    assert!(PAGE_DESIGNER.contains("const shownSize = $derived(sizing ?? windowSize(page));"), "designer");
    assert!(PAGE_DESIGNER.contains("<ShownWindow {area} clientWidth={shownSize.width}") && PAGE_DESIGNER.contains("style:height=\"{shownSize.height}px\"") && PAGE_DESIGNER.contains("CONTENT_PADDING"), "designer");
}

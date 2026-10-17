//! The layout values every renderer shares (design section 2): the web
//! renderer writes them in TypeScript, the generators read them here, and
//! this test keeps the two equal.

use clarity_spec::layout::{BUTTON_SIDES, CONTENT_PADDING, GAP, INPUT_SIDES, MESSAGE_MS, ROW_HEIGHT, SIDEBAR_COLUMN};

const PAGES_TS: &str = include_str!("../../../app/src/lib/pages.ts");
const RUNTIME_SVELTE: &str = include_str!("../../../app/src/runtime/Runtime.svelte");
const FRAME_VIEW: &str = include_str!("../../../app/src/lib/FrameView.svelte");
const RENDER_CSS: &str = include_str!("../../../app/src/render.css");

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
    assert_eq!(constant(PAGES_TS, "ROW_HEIGHT"), ROW_HEIGHT);
    assert_eq!(constant(PAGES_TS, "GAP"), GAP);
    assert_eq!(constant(RUNTIME_SVELTE, "MESSAGE_MS"), MESSAGE_MS);
    assert_eq!(constant(FRAME_VIEW, "SIDEBAR_COLUMN_PX"), SIDEBAR_COLUMN);
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

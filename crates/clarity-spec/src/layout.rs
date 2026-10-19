//! The layout values every renderer shares (design section 2), so a window
//! has the same rows and spacing on the web and in Avalonia. The web
//! renderer writes them in TypeScript; a test keeps those equal to these.

/// The size a window opens at on every stack, and the designer shows it at,
/// in pixels: the width the columns share and the height growing rows fill.
/// A window that says its own `size` opens at that instead.
pub const WINDOW_WIDTH: u32 = 1100;
pub const WINDOW_HEIGHT: u32 = 760;

/// The smallest and largest size a window may say, in pixels: room for a
/// part, and no larger than a large screen.
pub const WINDOW_MIN: (u32, u32) = (200, 150);
pub const WINDOW_MAX: (u32, u32) = (3840, 2160);

/// Whether `width` x `height` is a size a window may have.
pub fn window_size_fits(width: u32, height: u32) -> bool {
    (WINDOW_MIN.0..=WINDOW_MAX.0).contains(&width) && (WINDOW_MIN.1..=WINDOW_MAX.1).contains(&height)
}

/// What is said when a size does not fit.
pub fn window_size_problem() -> String {
    format!(
        "a window is {} to {} pixels wide and {} to {} high",
        WINDOW_MIN.0, WINDOW_MAX.0, WINDOW_MIN.1, WINDOW_MAX.1
    )
}

/// The height of a grid row that does not grow, in pixels.
pub const ROW_HEIGHT: u32 = 36;

/// The space between grid columns and rows, in pixels.
pub const GAP: u32 = 8;

/// How long an info message stays on screen, in milliseconds; warnings and
/// errors stay until they are dismissed.
pub const MESSAGE_MS: u32 = 5000;

/// The space around the grid of a running window, in pixels.
pub const CONTENT_PADDING: u32 = 12;

/// The width of one column of a sidebar, in pixels.
pub const SIDEBAR_COLUMN: u32 = 64;

/// The space beside a button's text, as a multiple of the theme's
/// `padding` (above and below it is the padding itself).
pub const BUTTON_SIDES: u32 = 3;

/// The space beside an input's text, as a multiple of the theme's `padding`.
pub const INPUT_SIDES: u32 = 2;

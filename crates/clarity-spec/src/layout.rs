//! The layout values every renderer shares (design section 2), so a window
//! has the same rows and spacing on the web and in Avalonia. The web
//! renderer writes them in TypeScript; a test keeps those equal to these.

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

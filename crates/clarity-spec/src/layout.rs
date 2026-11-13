//! The layout values every renderer shares (design section 2), so a window
//! has the same rows and spacing on every stack. The web renderer writes
//! them in TypeScript; a test keeps those equal to these.

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

/// The height of a grid row that does not grow, in pixels, when the
/// project file says no `row height`.
pub const ROW_HEIGHT: u32 = 36;

/// The size of a splash page that names none, in the screen's units.
pub const SPLASH_SIZE: (u32, u32) = (480, 280);

/// The size of a sign-in window that names none, in the screen's units.
pub const SIGNIN_SIZE: (u32, u32) = (420, 380);

/// The least and most a project's `row height` may be, in pixels.
pub const ROW_HEIGHTS: (u32, u32) = (24, 64);

/// The least and most a row of a page's own exact grid may be, in pixels (`grid 56 columns rows 10`).
pub const CELL_HEIGHTS: (u32, u32) = (8, 64);

/// Whether `height` is a row height a project may have.
pub fn row_height_fits(height: u32) -> bool {
    (ROW_HEIGHTS.0..=ROW_HEIGHTS.1).contains(&height)
}

/// What is said when a row height does not fit.
pub fn row_height_problem() -> String {
    format!("write 'row height' and a whole number from {} to {}: how high a row of the windows' grid is, in pixels ({} without the line)", ROW_HEIGHTS.0, ROW_HEIGHTS.1, ROW_HEIGHT)
}

/// The space between grid columns and rows, in pixels.
pub const GAP: u32 = 8;

/// How long an info message stays on screen, in milliseconds; warnings and
/// errors stay until they are dismissed.
pub const MESSAGE_MS: u32 = 5000;

/// The space around the grid of a running window, in pixels.
pub const CONTENT_PADDING: u32 = 12;

/// The width of one column of a sidebar, in pixels.
pub const SIDEBAR_COLUMN: u32 = 64;

/// The width of one column of a popover's grid, in pixels: a popover stands
/// outside the window's grid, so its width does not follow the window's.
pub const POPOVER_COLUMN: u32 = 80;

/// The space beside a button's text, as a multiple of the theme's
/// `padding` (above and below it is the padding itself).
pub const BUTTON_SIDES: u32 = 3;

/// The space beside an input's text, as a multiple of the theme's `padding`.
pub const INPUT_SIDES: u32 = 2;

/// How wide a column of a toolbox page is shown, about a tool button with its room.
pub const TOOLBOX_COLUMN: u32 = 44;
/// What the window around a page takes beyond its grid: its padding, title and menu.
pub const PAGE_CHROME: (u32, u32) = (24, 84);

/// The size a toolbox page is drawn at on its own (the designer, the
/// preview): as wide as its columns of tools and as high as its rows, not a
/// window's size, as a canvas shows it as a small panel. At least room for its title.
pub fn toolbox_size(columns: u32, rows: u32) -> (u32, u32) {
    let columns = columns.max(1);
    let rows = rows.max(1);
    let width = PAGE_CHROME.0 + columns * TOOLBOX_COLUMN + (columns - 1) * GAP;
    let height = PAGE_CHROME.1 + rows * ROW_HEIGHT + (rows - 1) * GAP;
    (width.max(220), height)
}

/// How high the frame's bars are, as Clarity.UI draws them (conformance/window-room.json).
pub const MENU_BAR_HEIGHT: u32 = 30;
pub const TOOLBAR_HEIGHT: u32 = 38;
pub const RIBBON_TABS_HEIGHT: u32 = 30;
pub const RIBBON_BODY_HEIGHT: u32 = 92;
/// A footer with its status line alone.
pub const STATUS_HEIGHT: u32 = 28;
/// A footer with parts is a row high and this much more.
pub const FOOTER_PARTS_BEYOND_ROW: u32 = 8;

/// A bar of the frame above or below a window's grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    Menu,
    Toolbar,
    /// A ribbon with its body open.
    Ribbon,
    /// A ribbon folded to its tabs, or without a tab.
    RibbonFolded,
    /// A footer with its status line alone.
    Status,
    /// A footer with parts.
    FooterParts,
}

impl Bar {
    /// How high it is at the project's row height.
    pub fn height(self, row_height: u32) -> u32 {
        match self {
            Bar::Menu => MENU_BAR_HEIGHT,
            Bar::Toolbar => TOOLBAR_HEIGHT,
            Bar::Ribbon => RIBBON_TABS_HEIGHT + RIBBON_BODY_HEIGHT,
            Bar::RibbonFolded => RIBBON_TABS_HEIGHT,
            Bar::Status => STATUS_HEIGHT,
            Bar::FooterParts => row_height + FOOTER_PARTS_BEYOND_ROW,
        }
    }
}

/// The bars a window has: each of its own, else the app's of the same kind.
pub fn frame_bars(app: Option<&crate::model::Frame>, own: &crate::model::Frame) -> Vec<Bar> {
    let mut bars = Vec::new();
    if own.menu.is_some() || app.is_some_and(|a| a.menu.is_some()) {
        bars.push(Bar::Menu);
    }
    if own.toolbar.is_some() || app.is_some_and(|a| a.toolbar.is_some()) {
        bars.push(Bar::Toolbar);
    }
    if let Some(ribbon) = own.ribbon.as_ref().or(app.and_then(|a| a.ribbon.as_ref())) {
        bars.push(if ribbon.collapsed || ribbon.tabs.is_empty() { Bar::RibbonFolded } else { Bar::Ribbon });
    }
    if let Some(footer) = own.footer.as_ref().or(app.and_then(|a| a.footer.as_ref())) {
        bars.push(if footer.widgets.is_empty() { Bar::Status } else { Bar::FooterParts });
    }
    bars
}

/// How many rows of the grid a window `height` pixels high holds under `bars`:
/// its height less its padding and the bars, in rows of `row_height` with gaps.
pub fn rows_held(height: u32, row_height: u32, bars: &[Bar]) -> u32 {
    let room = height as i64 - 2 * CONTENT_PADDING as i64 - bars.iter().map(|b| b.height(row_height) as i64).sum::<i64>();
    ((room + GAP as i64) / (row_height + GAP) as i64).max(0) as u32
}

/// How high a window must be to hold `rows` rows under `bars`.
pub fn height_for_rows(rows: u32, row_height: u32, bars: &[Bar]) -> u32 {
    2 * CONTENT_PADDING + bars.iter().map(|b| b.height(row_height)).sum::<u32>() + rows * row_height + rows.saturating_sub(1) * GAP
}

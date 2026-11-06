//! How a window's grid places its parts, as every version of Clarity.UI does
//! (conformance/grid-layout.json): the columns share the width; a row is as
//! high as the project says (`row height`, 36 without it), and the rows of a
//! part that grows share the height left over, never less than a row; a part
//! that does not grow keeps its own height at the top of its place. A part that holds a grid of its own (a splitter, tabs, a
//! panel) adds room around it (`around`): its rows are made high enough for both.
//! When no row grows, a part marked `bottom` moves down by the height to
//! spare, with every row from its own down.

use std::cell::Cell;
use std::collections::BTreeSet;

use crate::geometry::Rect;

/// The height of a row when the project says none (`row height`).
pub const ROW_HEIGHT: f64 = 36.0;

/// The least and most a project's row height may be.
pub const ROW_HEIGHTS: (f64, f64) = (24.0, 64.0);

/// The least and most a row of a page's own exact grid may be (`cell="10"`).
pub const CELL_HEIGHTS: (f64, f64) = (8.0, 64.0);

thread_local! {
    static ROW: Cell<f64> = const { Cell::new(ROW_HEIGHT) };
}

/// How high a row is in the windows shown now: the project's `row height`,
/// which every window's markup carries (`rowHeight`), else `ROW_HEIGHT`.
pub fn row_height() -> f64 {
    ROW.with(Cell::get)
}

/// The row height the windows are laid out by, kept within `ROW_HEIGHTS`, or
/// `CELL_HEIGHTS` for a page's own exact grid; a window sets it from its markup as it is made.
pub fn set_row_height(height: f64) {
    let height = if height.is_finite() { height.clamp(CELL_HEIGHTS.0, ROW_HEIGHTS.1) } else { ROW_HEIGHT };
    ROW.with(|r| r.set(height));
}

/// The gap between columns and rows.
pub const GAP: f64 = 8.0;

/// A part's place in a grid: at `col`, `row` (from 1), so many columns wide
/// and rows high; `around` is the room it needs beyond its rows; `bottom`
/// sticks it, its rows and those below, to the bottom when no row grows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridItem {
    pub col: i64,
    pub row: i64,
    pub width: i64,
    pub height: i64,
    pub grows: bool,
    pub around: f64,
    pub bottom: bool,
}

impl GridItem {
    pub fn new(col: i64, row: i64, width: i64, height: i64, grows: bool) -> GridItem {
        GridItem { col, row, width, height, grows, around: 0.0, bottom: false }
    }

    /// The same place, stuck to the bottom or not (`bottom`).
    pub fn at_bottom(self, bottom: bool) -> GridItem {
        GridItem { bottom, ..self }
    }
}

/// Where the parts are, in order, and how high the content is (more than the grid when it scrolls).
#[derive(Debug, Clone, PartialEq)]
pub struct GridPlacement {
    pub boxes: Vec<Rect>,
    pub content: f64,
    /// How high each row is, from the first (the designer finds a row under the pointer by them).
    pub rows: Vec<f64>,
}

/// Places `items` in a grid of `columns` over `width` and `height`, its rows as high as `row_height()`.
pub fn place(items: &[GridItem], columns: i64, width: f64, height: f64, gap: f64) -> GridPlacement {
    place_rows(items, columns, width, height, gap, row_height())
}

/// `place` with rows `row_size` high.
pub fn place_rows(items: &[GridItem], columns: i64, width: f64, height: f64, gap: f64, row_size: f64) -> GridPlacement {
    let columns = columns.max(1);
    let rows = if items.is_empty() { 1 } else { items.iter().map(|i| i.row + i.height.max(1) - 1).max().unwrap_or(1).max(1) } as usize;
    let mut growing = BTreeSet::new();
    for item in items.iter().filter(|i| i.grows) {
        for r in item.row..item.row + item.height.max(1) {
            growing.insert(r);
        }
    }
    let span = |item: &GridItem| {
        let row = (item.row.clamp(1, rows as i64) - 1) as usize;
        let high = item.height.max(1).clamp(1, (rows - row) as i64) as usize;
        (row, high)
    };
    // The least height of each row: a row, more where a part needs room around its rows.
    let mut least = vec![row_size; rows];
    for item in items.iter().filter(|i| i.around > 0.0) {
        let (row, high) = span(item);
        let have: f64 = least[row..row + high].iter().sum::<f64>() + (high as f64 - 1.0) * gap;
        let missing = high as f64 * row_size + (high as f64 - 1.0) * gap + item.around - have;
        if missing > 0.0 {
            least[row + high - 1] += missing;
        }
    }
    let mut heights = least.clone();
    // The growing rows share what the others leave; a row whose least is more
    // than its share keeps it, the rest share what is left.
    let mut spare = height - gap * (rows as f64 - 1.0) - (1..=rows as i64).filter(|r| !growing.contains(r)).map(|r| least[r as usize - 1]).sum::<f64>();
    let mut sharing: BTreeSet<i64> = growing.iter().copied().filter(|r| *r >= 1 && *r <= rows as i64).collect();
    while !sharing.is_empty() {
        let each = spare / sharing.len() as f64;
        let keep: Vec<i64> = sharing.iter().copied().filter(|r| least[*r as usize - 1] > each).collect();
        if keep.is_empty() {
            for r in &sharing {
                heights[*r as usize - 1] = each;
            }
            break;
        }
        for r in keep {
            sharing.remove(&r);
            spare -= least[r as usize - 1];
        }
    }
    let mut tops = vec![0.0; rows];
    for r in 1..rows {
        tops[r] = tops[r - 1] + heights[r - 1] + gap;
    }
    // No row grows: the rows from the first of a part marked `bottom` down
    // move down by the height to spare, keeping their heights. The designer
    // sees the spare as the row above them, when there is one.
    let bottom = items.iter().filter(|i| i.bottom).map(|i| (i.row.clamp(1, rows as i64) - 1) as usize).min();
    if let (Some(first), true) = (bottom, growing.is_empty() && spare > 0.0) {
        for top in &mut tops[first..] {
            *top += spare;
        }
        if first > 0 {
            heights[first - 1] += spare;
        }
    }
    let column = ((width - gap * (columns as f64 - 1.0)) / columns as f64).max(0.0);
    let boxes = items
        .iter()
        .map(|item| {
            let (row, high) = span(item);
            let wide = item.width.max(1) as f64;
            let x = (item.col - 1) as f64 * (column + gap);
            let w = wide * column + (wide - 1.0) * gap;
            let h = if item.grows {
                heights[row..row + high].iter().sum::<f64>() + gap * (high as f64 - 1.0)
            } else {
                high as f64 * row_size + (high as f64 - 1.0) * gap + item.around
            };
            Rect::new(x, tops[row], w, h)
        })
        .collect();
    GridPlacement { boxes, content: tops[rows - 1] + heights[rows - 1], rows: heights }
}

//! How a window's grid places its parts, as every version of Clarity.UI does
//! (conformance/grid-layout.json): the columns share the width; a row is 36
//! high, and the rows of a part that grows share the height left over, never
//! less than 36; a part that does not grow keeps its own height at the top
//! of its place. A part that holds a grid of its own (a splitter, tabs, a
//! panel) adds room around it (`around`): its rows are made high enough for both.

use std::collections::BTreeSet;

use crate::geometry::Rect;

/// The height of a row.
pub const ROW_HEIGHT: f64 = 36.0;

/// The gap between columns and rows.
pub const GAP: f64 = 8.0;

/// A part's place in a grid: at `col`, `row` (from 1), so many columns wide
/// and rows high; `around` is the room it needs beyond its rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridItem {
    pub col: i64,
    pub row: i64,
    pub width: i64,
    pub height: i64,
    pub grows: bool,
    pub around: f64,
}

impl GridItem {
    pub fn new(col: i64, row: i64, width: i64, height: i64, grows: bool) -> GridItem {
        GridItem { col, row, width, height, grows, around: 0.0 }
    }
}

/// Where the parts are, in order, and how high the content is (more than the grid when it scrolls).
#[derive(Debug, Clone, PartialEq)]
pub struct GridPlacement {
    pub boxes: Vec<Rect>,
    pub content: f64,
}

/// Places `items` in a grid of `columns` over `width` and `height`.
pub fn place(items: &[GridItem], columns: i64, width: f64, height: f64, gap: f64) -> GridPlacement {
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
    // The least height of each row: 36, more where a part needs room around its rows.
    let mut least = vec![ROW_HEIGHT; rows];
    for item in items.iter().filter(|i| i.around > 0.0) {
        let (row, high) = span(item);
        let have: f64 = least[row..row + high].iter().sum::<f64>() + (high as f64 - 1.0) * gap;
        let missing = high as f64 * ROW_HEIGHT + (high as f64 - 1.0) * gap + item.around - have;
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
                high as f64 * ROW_HEIGHT + (high as f64 - 1.0) * gap + item.around
            };
            Rect::new(x, tops[row], w, h)
        })
        .collect();
    GridPlacement { boxes, content: tops[rows - 1] + heights[rows - 1] }
}

//! Data parts: list (with cards), table (with pages, order, columns, edited
//! cells and reordering), tree and tree table, chart (of rows or of numbers),
//! trend, and canvas (drawn by code, with its modes, or by an engine).
//! Each kind is a module in the folder of this group; KINDS lists the kinds
//! the group draws with the function that makes each.

pub mod canvas;
pub mod chart;
pub mod groups;
pub mod list;
pub mod rows;
pub mod table;
pub mod trend;

use super::MakePart;

/// The kinds this group draws, each with the module that makes it.
pub const KINDS: &[(&str, MakePart)] = &[
    ("list", list::make),
    ("table", table::make),
    ("tree", table::make),
    ("treeTable", table::make),
    ("chart", chart::make),
    ("trend", trend::make),
    ("canvas", canvas::make),
];

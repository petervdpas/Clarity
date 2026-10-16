use serde::Serialize;
use ts_rs::TS;

/// A location in a spec file. `start` and `end` are byte offsets,
/// `line` and `col` are 1-based and point at `start`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub col: u32,
}

//! The slider and the spin box of U1c, decided here, not in the screen
//! (DESIGN 1.7): how a key, a drag or a typed number moves a value within
//! its range. Numbers come as text and stay exact: the sums are done on
//! whole numbers scaled by ten as often as needed. Every runtime moves alike
//! (conformance/ranges.json).

use crate::ui::{Reply, WindowState};
use crate::{Db, Error, Result};
use serde::Deserialize;
use serde_json::Value;

/// A range: null is no end.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Range {
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: String,
}

/// The keys a slider and a spin box answer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeKey {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

/// What the person did: a key, a drag to a place, or a typed number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeMove {
    Key(RangeKey),
    At(String),
    Typed(String),
}

impl RangeMove {
    /// The move a message gives: one of its key, place and typed text.
    pub fn of(key: Option<RangeKey>, at: Option<String>, typed: Option<String>) -> Result<RangeMove> {
        match (key, at, typed) {
            (Some(key), _, _) => Ok(RangeMove::Key(key)),
            (_, Some(at), _) => Ok(RangeMove::At(at)),
            (_, _, Some(typed)) => Ok(RangeMove::Typed(typed)),
            _ => Err(Error::Invalid("a range event takes a key, a place or typed text".into())),
        }
    }
}

/// A number as whole digits and how many of them are decimals.
#[derive(Clone, Copy)]
struct Exact {
    n: i128,
    scale: u32,
}

fn exact(value: &str) -> Option<Exact> {
    let text = value.trim();
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, part) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() && part.is_empty() || !(whole.bytes().chain(part.bytes())).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let all = format!("{}{part}", if whole.is_empty() { "0" } else { whole });
    let n: i128 = all.parse().ok()?;
    Some(Exact { n: if negative { -n } else { n }, scale: u32::try_from(part.len()).ok()? })
}

/// Whole numbers at a scale of `scale` decimals.
fn at(e: Exact, scale: u32) -> Option<i128> {
    e.n.checked_mul(10i128.checked_pow(scale - e.scale)?)
}

fn text(n: i128, scale: u32) -> String {
    let digits = format!("{:0>width$}", n.unsigned_abs(), width = scale as usize + 1);
    let (whole, part) = digits.split_at(digits.len() - scale as usize);
    let part = part.trim_end_matches('0');
    let sign = if n < 0 { "-" } else { "" };
    if part.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{part}") }
}

fn within(mut n: i128, low: Option<i128>, high: Option<i128>) -> i128 {
    if let Some(high) = high {
        n = n.min(high);
    }
    if let Some(low) = low {
        n = n.max(low);
    }
    n
}

/// The numbers given, the ends of the range among them, at one scale.
struct Scaled {
    scale: u32,
    low: Option<i128>,
    high: Option<i128>,
}

fn scaled(range: &Range, values: &[Exact], with_step: bool) -> Option<(Scaled, Option<i128>)> {
    let min = range.min.as_deref().map(exact).map_or(Some(None), |e| e.map(Some))?;
    let max = range.max.as_deref().map(exact).map_or(Some(None), |e| e.map(Some))?;
    let step = if with_step { Some(exact(&range.step)?) } else { None };
    let scale = values.iter().chain(min.iter()).chain(max.iter()).chain(step.iter()).map(|e| e.scale).max().unwrap_or(0);
    let low = min.map(|e| at(e, scale)).map_or(Some(None), |n| n.map(Some))?;
    let high = max.map(|e| at(e, scale)).map_or(Some(None), |n| n.map(Some))?;
    let step = step.map(|e| at(e, scale)).map_or(Some(None), |n| n.map(Some))?;
    Some((Scaled { scale, low, high }, step))
}

/// The value after `key`: a step up or down, ten, or an end; within the
/// range. No value counts as the lowest, or 0. None when a number is no number.
pub fn keyed(value: Option<&str>, range: &Range, key: RangeKey) -> Option<String> {
    let value = value.filter(|v| !v.trim().is_empty()).map(exact).map_or(Some(None), |e| e.map(Some))?;
    let (s, step) = scaled(range, value.as_slice(), true)?;
    let step = step?;
    let now = match value {
        Some(e) => at(e, s.scale)?,
        None => s.low.unwrap_or(0),
    };
    let next = match key {
        RangeKey::Up => now.checked_add(step)?,
        RangeKey::Down => now.checked_sub(step)?,
        RangeKey::PageUp => now.checked_add(step.checked_mul(10)?)?,
        RangeKey::PageDown => now.checked_sub(step.checked_mul(10)?)?,
        RangeKey::Home => s.low.unwrap_or(now),
        RangeKey::End => s.high.unwrap_or(now),
    };
    Some(text(within(next, s.low, s.high), s.scale))
}

/// The value a slider takes where it is dragged to: the nearest step from
/// the lowest value, at most the highest step in the range. Without a step
/// it is the place, within the range.
pub fn dragged(place: &str, range: &Range) -> Option<String> {
    let place = exact(place)?;
    let (s, step) = scaled(range, &[place], true)?;
    let step = step?;
    if step <= 0 {
        return typed_in(&text(place.n, place.scale), range);
    }
    let low = s.low.unwrap_or(0);
    let from = at(place, s.scale)?.checked_sub(low)?;
    // Rounded half up, as a whole number of steps.
    let steps = (from.checked_mul(2)?.checked_add(step)?).div_euclid(step.checked_mul(2)?);
    let most = s.high.map(|high| (high - low).div_euclid(step));
    let steps = within(steps, Some(0), most);
    Some(text(low.checked_add(steps.checked_mul(step)?)?, s.scale))
}

/// A typed number, kept within the range.
pub fn typed_in(value: &str, range: &Range) -> Option<String> {
    let value = exact(value)?;
    let (s, _) = scaled(range, &[value], false)?;
    Some(text(within(at(value, s.scale)?, s.low, s.high), s.scale))
}

/// `range_event` (DESIGN 1.7): the value at `path` in the window's data
/// after `mv`, written as a number when the field is `whole`, else as text.
/// A changed value runs `run`, the handler, as an edit ends
/// (`edits::ended`); one that stays, or text that is no number, answers
/// the window as it is (`draw`).
#[allow(clippy::too_many_arguments)]
pub fn event(
    db: &Db,
    mut state: WindowState,
    part: &str,
    path: &str,
    range: &Range,
    whole: bool,
    mv: &RangeMove,
    scope: Option<&str>,
    run: impl FnOnce(WindowState) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let now = match crate::ui::data_at(&state.data, path) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    };
    let after = match mv {
        RangeMove::Key(key) => keyed(now.as_deref(), range, *key),
        RangeMove::At(place) => dragged(place, range),
        RangeMove::Typed(typed) => typed_in(typed, range),
    };
    let Some(after) = after else { return draw(state) };
    let value = if whole { after.parse::<i64>().map(Value::from).unwrap_or(Value::String(after)) } else { Value::String(after) };
    if crate::ui::data_at(&state.data, path) == Some(&value) {
        return draw(state);
    }
    crate::ui::set_data_at(&mut state.data, path, value)?;
    crate::edits::ended(db, state, part, scope, run)
}

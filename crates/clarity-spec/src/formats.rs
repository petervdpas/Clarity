//! Named formats: how a field's value is shown, chosen by name with
//! `format` (design section 1.5). A name means the same on every stack and
//! in every locale; each renderer turns it into its own formatting.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use crate::model::FieldType;

/// A format a field can name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FormatInfo {
    pub name: &'static str,
    /// What it shows, for the designer.
    pub shows: &'static str,
}

const NUMBER: [FormatInfo; 1] = [FormatInfo { name: "plain", shows: "without thousands separators, for codes and years" }];
const DECIMAL: [FormatInfo; 2] = [
    FormatInfo { name: "plain", shows: "without thousands separators" },
    FormatInfo { name: "percent", shows: "as a percentage: 0.25 shows 25%" },
];
const MONEY: [FormatInfo; 1] = [FormatInfo { name: "plain", shows: "the amount without the currency" }];
const DATE: [FormatInfo; 3] = [
    FormatInfo { name: "short", shows: "in digits, as the locale writes them: 17-10-2026 (the default)" },
    FormatInfo { name: "long", shows: "with the month in words: 17 October 2026" },
    FormatInfo { name: "iso", shows: "year, month and day: 2026-10-17, the same everywhere" },
];
const DATE_TIME: [FormatInfo; 3] = [
    FormatInfo { name: "short", shows: "in digits with hours and minutes: 17-10-2026 09:30 (the default)" },
    FormatInfo { name: "long", shows: "with the month in words: 17 October 2026 09:30" },
    FormatInfo { name: "iso", shows: "2026-10-17 09:30, the same everywhere" },
];

/// The formats a field of type `ty` can name; empty for types that are
/// shown one way only.
pub fn of(ty: &FieldType) -> &'static [FormatInfo] {
    match ty {
        FieldType::Integer => &NUMBER,
        FieldType::Decimal { .. } => &DECIMAL,
        FieldType::Money => &MONEY,
        FieldType::Date => &DATE,
        FieldType::DateTime => &DATE_TIME,
        _ => &[],
    }
}

/// The formats per type, keyed by the type's kind as the designer sees it
/// (`date`, `dateTime`).
pub fn catalog() -> BTreeMap<&'static str, &'static [FormatInfo]> {
    BTreeMap::from([
        ("integer", of(&FieldType::Integer)),
        ("decimal", of(&FieldType::Decimal { precision: 1, scale: 0 })),
        ("money", of(&FieldType::Money)),
        ("date", of(&FieldType::Date)),
        ("dateTime", of(&FieldType::DateTime)),
    ])
}

/// Why `name` is not a format of `ty`, if it is not.
pub fn problem(ty: &FieldType, field: &str, name: &str) -> Option<String> {
    let formats = of(ty);
    if formats.iter().any(|f| f.name == name) {
        return None;
    }
    let names: Vec<&str> = formats.iter().map(|f| f.name).collect();
    Some(match names.split_last() {
        None => format!("field '{field}' is shown one way only; it takes no format"),
        Some((last, [])) => format!("unknown format '{name}' for field '{field}'; it can be {last}"),
        Some((last, rest)) => format!("unknown format '{name}' for field '{field}'; it can be {} or {last}", rest.join(", ")),
    })
}

/// Why `tag` is not a language tag Clarity takes: a language of two or
/// three lowercase letters, optionally a script (`Latn`) and a region (`NL`
/// or `419`), joined by hyphens: `en`, `nl-NL`, `sr-Latn-RS`.
pub fn language_problem(tag: &str) -> Option<String> {
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or("");
    let lower = |s: &str, n: std::ops::RangeInclusive<usize>| n.contains(&s.len()) && s.chars().all(|c| c.is_ascii_lowercase());
    let mut fits = lower(language, 2..=3);
    let rest: Vec<&str> = parts.collect();
    let (script, region) = match rest.as_slice() {
        [] => (None, None),
        [one] if one.len() == 4 => (Some(*one), None),
        [one] => (None, Some(*one)),
        [script, region] => (Some(*script), Some(*region)),
        _ => (None, None),
    };
    fits &= rest.len() <= 2 && (rest.len() < 2 || script.is_some_and(|s| s.len() == 4));
    if let Some(script) = script {
        let mut chars = script.chars();
        fits &= chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_lowercase());
    }
    if let Some(region) = region {
        fits &= (region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase()))
            || (region.len() == 3 && region.chars().all(|c| c.is_ascii_digit()));
    }
    (!fits).then(|| format!("'{tag}' is not a language tag; write one like en, nl or nl-NL"))
}

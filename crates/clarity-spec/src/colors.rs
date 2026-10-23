//! The color field type (U1c): kept as text `#rrggbb`, read from what is
//! typed the same way on every renderer, and picked from a palette they
//! share (conformance/colors.json).

/// The colors the color input offers, in rows: grays first, then light to
/// dark shades of slate, red, orange, amber, green, teal, blue, indigo,
/// purple and pink.
pub const PALETTE: [[&str; 10]; 6] = [
    ["#ffffff", "#f8fafc", "#e5e7eb", "#d1d5db", "#9ca3af", "#6b7280", "#4b5563", "#374151", "#1f2937", "#000000"],
    ["#e2e8f0", "#fecaca", "#fed7aa", "#fde68a", "#bbf7d0", "#99f6e4", "#bfdbfe", "#c7d2fe", "#e9d5ff", "#fbcfe8"],
    ["#94a3b8", "#f87171", "#fb923c", "#fbbf24", "#4ade80", "#2dd4bf", "#60a5fa", "#818cf8", "#c084fc", "#f472b6"],
    ["#64748b", "#ef4444", "#f97316", "#f59e0b", "#22c55e", "#14b8a6", "#3b82f6", "#6366f1", "#a855f7", "#ec4899"],
    ["#475569", "#dc2626", "#ea580c", "#d97706", "#16a34a", "#0d9488", "#2563eb", "#4f46e5", "#9333ea", "#db2777"],
    ["#1e293b", "#991b1b", "#9a3412", "#92400e", "#166534", "#115e59", "#1e40af", "#3730a3", "#6b21a8", "#9d174d"],
];

/// How a color is written, for messages.
pub const COLOR_PROBLEM: &str = "a color is written as \"#rrggbb\", as in \"#3b82f6\"";

/// The color `typed` is, as `#` and six lowercase digits: with or without
/// `#`, three or six digits, any case, around spaces. Empty text is no
/// color (`""`); anything else is `None`.
pub fn read(typed: &str) -> Option<String> {
    let text = typed.trim();
    if text.is_empty() {
        return Some(String::new());
    }
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let six: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_string(),
        _ => return None,
    };
    Some(format!("#{}", six.to_ascii_lowercase()))
}

/// Whether a color is written as it is kept: `#` and six lowercase digits, or empty.
pub fn is_kept(text: &str) -> bool {
    read(text).as_deref() == Some(text)
}

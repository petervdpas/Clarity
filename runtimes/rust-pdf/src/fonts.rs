//! The fonts a PDF is set in: only those carried in the crate, so a PDF is
//! the same on every machine and needs no internet. Inter (Clarity.UI's own
//! font) in its four faces, and Typst's own: Libertinus Serif, New Computer
//! Modern and DejaVu Sans Mono.

use typst::foundations::Bytes;
use typst::text::Font;

const INTER: [&[u8]; 4] = [
    include_bytes!("../fonts/Inter-Regular.ttf"),
    include_bytes!("../fonts/Inter-Bold.ttf"),
    include_bytes!("../fonts/Inter-Italic.ttf"),
    include_bytes!("../fonts/Inter-BoldItalic.ttf"),
];

/// The families carried, by the name Typst knows them by.
pub(crate) const FAMILIES: [&str; 5] = ["Inter", "Libertinus Serif", "New Computer Modern", "New Computer Modern Math", "DejaVu Sans Mono"];

/// Every carried font.
pub(crate) fn all() -> Vec<Font> {
    INTER.into_iter().chain(typst_assets::fonts()).flat_map(|data| Font::iter(Bytes::new(data))).collect()
}

/// The first family of a `font-family` list that is carried: a family by
/// its name, or a generic one (`sans-serif` and `system-ui` are Inter,
/// `serif` Libertinus Serif, `monospace` DejaVu Sans Mono).
pub(crate) fn family(list: &str) -> Option<String> {
    for written in list.split(',') {
        let name = written.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        let generic = match name.to_ascii_lowercase().as_str() {
            "sans-serif" | "system-ui" | "-apple-system" | "ui-sans-serif" => Some("Inter"),
            "serif" | "ui-serif" => Some("Libertinus Serif"),
            "monospace" | "ui-monospace" => Some("DejaVu Sans Mono"),
            _ => None,
        };
        if let Some(family) = generic.or_else(|| FAMILIES.iter().copied().find(|f| f.eq_ignore_ascii_case(name))) {
            return Some(family.to_string());
        }
    }
    None
}

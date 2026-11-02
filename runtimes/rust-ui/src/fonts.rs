//! The fonts text is drawn in: a theme's font role names its families, most
//! wanted first (`system-ui, Segoe UI, sans-serif`); the first the system
//! has is taken. Where the system has none, Clarity.UI's own Inter is (it
//! carries Regular and Bold, under the SIL Open Font License).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use skia_safe::font::Edging;
use skia_safe::{Data, Font, FontMgr, FontStyle, Typeface};

use crate::theme::ThemeLook;

const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

thread_local! {
    static CARRIED: RefCell<Option<(Typeface, Typeface)>> = const { RefCell::new(None) };
}

/// Clarity.UI's own font, Inter, regular or bold.
pub fn carried(bold: bool) -> Typeface {
    CARRIED.with(|cell| {
        let mut cell = cell.borrow_mut();
        let (regular, heavy) = cell.get_or_insert_with(|| {
            let manager = FontMgr::new();
            let load = |bytes: &'static [u8]| {
                manager.new_from_data(Data::new_copy(bytes), None).expect("Clarity.UI carries Inter, which Skia reads")
            };
            (load(INTER_REGULAR), load(INTER_BOLD))
        });
        if bold { heavy.clone() } else { regular.clone() }
    })
}

/// The fonts of a theme: a typeface per font role and weight, found once.
pub struct Fonts {
    look: Arc<ThemeLook>,
    manager: FontMgr,
    typefaces: RefCell<HashMap<(String, bool), Typeface>>,
}

impl Fonts {
    pub fn new(look: Arc<ThemeLook>) -> Fonts {
        Fonts { look, manager: FontMgr::new(), typefaces: RefCell::new(HashMap::new()) }
    }

    /// The typeface of a font role of the theme.
    pub fn typeface(&self, role: &str, bold: bool) -> Typeface {
        if let Some(found) = self.typefaces.borrow().get(&(role.to_string(), bold)) {
            return found.clone();
        }
        let typeface = self.of_families(&self.look.font(role), bold);
        self.typefaces.borrow_mut().insert((role.to_string(), bold), typeface.clone());
        typeface
    }

    fn of_families(&self, families: &str, bold: bool) -> Typeface {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        for written in families.split(',') {
            let family = match written.trim().trim_matches(|c| c == '"' || c == '\'') {
                "system-ui" | "-apple-system" | "sans-serif" => "sans-serif",
                "ui-monospace" | "monospace" => "monospace",
                "serif" | "ui-serif" => "serif",
                named => named,
            };
            if family == "Inter" {
                return carried(bold);
            }
            if !family.is_empty() {
                if let Some(typeface) = self.manager.match_family_style(family, style) {
                    return typeface;
                }
            }
        }
        carried(bold)
    }

    /// The font of a role at a size.
    pub fn font(&self, role: &str, size: f64, bold: bool) -> Font {
        let mut font = Font::from_typeface(self.typeface(role, bold), size as f32);
        font.set_subpixel(true);
        font.set_edging(Edging::SubpixelAntiAlias);
        font
    }

    /// How wide `text` is in a font role, at `size`.
    pub fn measure(&self, text: &str, role: &str, size: f64, bold: bool) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        self.font(role, size, bold).measure_str(text, None).0 as f64
    }
}

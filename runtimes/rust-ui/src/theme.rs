//! A theme as Clarity.UI draws it (DESIGN 1.5, 7.9): what Clarity resolves
//! for light or dark mode, read as JSON. A project's theme lies over
//! Clarity's own, which Clarity.UI carries; what the project's leaves out is
//! Clarity's. The look is defined in Clarity; this only reads it.

use std::sync::{Arc, OnceLock};

use serde_json::Value;
use skia_safe::Color;

/// Clarity's own light theme, as Clarity resolves it (the same file as the other stacks carry).
pub const CLARITY_LIGHT: &str = include_str!("../themes/clarity.light.json");

/// Clarity's own dark theme.
pub const CLARITY_DARK: &str = include_str!("../themes/clarity.dark.json");

/// How a part looks in a state, or a style: what the theme sets, none for what it leaves.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LookValues {
    pub background: Option<Color>,
    pub text: Option<Color>,
    pub border: Option<Color>,
    pub radius: Option<i64>,
    pub weight: Option<String>,
    pub size: Option<i64>,
    pub font: Option<String>,
    pub align: Option<String>,
}

impl LookValues {
    pub fn bold(&self) -> bool {
        matches!(self.weight.as_deref(), Some("bold" | "700" | "800" | "900"))
    }

    /// What this sets, and what it leaves from `under`.
    pub fn over(&self, under: &LookValues) -> LookValues {
        LookValues {
            background: self.background.or(under.background),
            text: self.text.or(under.text),
            border: self.border.or(under.border),
            radius: self.radius.or(under.radius),
            weight: self.weight.clone().or_else(|| under.weight.clone()),
            size: self.size.or(under.size),
            font: self.font.clone().or_else(|| under.font.clone()),
            align: self.align.clone().or_else(|| under.align.clone()),
        }
    }
}

/// A resolved theme, with the one it lies over.
#[derive(Debug)]
pub struct ThemeLook {
    own: Value,
    under: Option<Arc<ThemeLook>>,
}

/// A color as a theme writes it: `#rrggbb`, `#rgb` or `#rrggbbaa`.
pub fn parse_color(written: &str) -> Option<Color> {
    let hex = written.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let nibble = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok().map(|n| n * 17);
    match hex.len() {
        3 => Some(Color::from_rgb(nibble(0)?, nibble(1)?, nibble(2)?)),
        6 => Some(Color::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_argb(byte(6)?, byte(0)?, byte(2)?, byte(4)?)),
        _ => None,
    }
}

impl ThemeLook {
    /// Clarity's own theme, for light or dark mode.
    pub fn clarity(dark: bool) -> Arc<ThemeLook> {
        static LIGHT: OnceLock<Arc<ThemeLook>> = OnceLock::new();
        static DARK: OnceLock<Arc<ThemeLook>> = OnceLock::new();
        let (cell, text) = if dark { (&DARK, CLARITY_DARK) } else { (&LIGHT, CLARITY_LIGHT) };
        cell.get_or_init(|| Arc::new(ThemeLook { own: serde_json::from_str(text).unwrap_or(Value::Null), under: None })).clone()
    }

    /// A project's theme, resolved by Clarity, over `under`; one that is no JSON leaves everything to `under`.
    pub fn over(resolved: &str, under: Arc<ThemeLook>) -> Arc<ThemeLook> {
        Arc::new(ThemeLook { own: serde_json::from_str(resolved).unwrap_or(Value::Null), under: Some(under) })
    }

    pub fn dark(&self) -> bool {
        self.own.get("dark").and_then(Value::as_bool).unwrap_or(false)
    }

    fn table(&self, name: &str, key: &str) -> Option<&Value> {
        self.own.get(name)?.as_object()?.get(key).filter(|v| !v.is_null())
    }

    fn number(&self, name: &str) -> Option<i64> {
        self.own.get(name)?.as_f64().map(|n| n as i64)
    }

    /// The color of a role; one the themes do not know is the text's, so a part always shows.
    pub fn color(&self, role: &str) -> Color {
        self.try_color(role).unwrap_or_else(|| if role == "text" { Color::BLACK } else { self.color("text") })
    }

    pub fn try_color(&self, role: &str) -> Option<Color> {
        match self.table("colors", role) {
            Some(value) => self.read(value.as_str()),
            None => self.under.as_ref().and_then(|u| u.try_color(role)),
        }
    }

    /// A written color: `#rrggbb`, `@role` for Clarity's own value of a role, `none` for none.
    fn read(&self, written: Option<&str>) -> Option<Color> {
        let written = written?;
        if written == "none" {
            return Some(Color::TRANSPARENT);
        }
        if let Some(role) = written.strip_prefix('@') {
            return match &self.under {
                Some(under) => under.try_color(role),
                None => self.try_color(role),
            };
        }
        parse_color(written)
    }

    pub fn size(&self, role: &str) -> f64 {
        if let Some(size) = self.table("sizes", role).and_then(Value::as_f64) {
            return size;
        }
        if let Some(size) = self.under.as_ref().map(|u| u.size(role)) {
            return size;
        }
        if role == "text" { 14.0 } else { self.size("text") }
    }

    pub fn radius(&self, role: &str) -> f64 {
        self.table("radii", role)
            .or_else(|| self.table("radii", "all"))
            .and_then(Value::as_f64)
            .or_else(|| self.under.as_ref().map(|u| u.radius(role)))
            .unwrap_or(0.0)
    }

    /// A font role's families, most wanted first, as a list: `Inter` or `system-ui, sans-serif`.
    pub fn font(&self, role: &str) -> String {
        if let Some(font) = self.table("fonts", role).and_then(Value::as_str) {
            return font.to_string();
        }
        if let Some(font) = self.under.as_ref().map(|u| u.font(role)) {
            return font;
        }
        if role == "text" { "sans-serif".to_string() } else { self.font("text") }
    }

    pub fn padding(&self) -> f64 {
        self.number("padding").map(|n| n as f64).or_else(|| self.under.as_ref().map(|u| u.padding())).unwrap_or(4.0)
    }

    pub fn border(&self) -> f64 {
        self.number("border").map(|n| n as f64).or_else(|| self.under.as_ref().map(|u| u.border())).unwrap_or(1.0)
    }

    fn own_look(&self, table: &str, key: &str) -> LookValues {
        let Some(look) = self.table(table, key) else {
            return LookValues::default();
        };
        let text = |name: &str| match look.get(name) {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        };
        let int = |name: &str| look.get(name).and_then(Value::as_f64).map(|n| n as i64);
        let color = |name: &str| text(name).and_then(|w| self.read(Some(&w)));
        LookValues {
            background: color("background"),
            text: color("text"),
            border: color("border"),
            radius: int("radius"),
            weight: text("weight"),
            size: int("size"),
            font: text("font"),
            align: text("align"),
        }
    }

    fn look(&self, table: &str, key: &str) -> LookValues {
        let own = self.own_look(table, key);
        match &self.under {
            Some(under) => own.over(&under.look(table, key)),
            None => own,
        }
    }

    /// The look of a kind of part in a state: the theme writes it under `kind:state`.
    fn state_look(&self, kind: &str, state: &str) -> LookValues {
        let own = self
            .own
            .get("parts")
            .and_then(Value::as_object)
            .and_then(|parts| parts.keys().find(|k| k.split_once(':') == Some((kind, state))).cloned())
            .map(|key| self.own_look("parts", &key))
            .unwrap_or_default();
        match &self.under {
            Some(under) => own.over(&under.state_look(kind, state)),
            None => own,
        }
    }

    /// How a kind of part looks in a state (hover, pressed, focus, selected), or without one.
    pub fn part(&self, kind: &str, state: Option<&str>) -> LookValues {
        let plain = self.look("parts", kind);
        match state {
            None => plain,
            Some(state) => self.state_look(kind, state).over(&plain),
        }
    }

    /// Only what a kind of part's state sets itself, without its plain look under it; nothing without a state.
    pub fn part_own(&self, kind: &str, state: Option<&str>) -> LookValues {
        match state {
            None => LookValues::default(),
            Some(state) => self.state_look(kind, state),
        }
    }

    /// A style of the theme: `style primary background accent`.
    pub fn style(&self, name: &str) -> LookValues {
        self.look("styles", name)
    }

    /// How a part that is pressed (a button, a toggle) of `kind` looks with
    /// its style in a state: the part's look, the style over it, and the
    /// state's own look over both. A style that fills the part keeps its
    /// colors in a state, so its text stays readable: the state shades the
    /// style's fill away from its text (a little on hover, more pressed)
    /// and takes only its border from the state's look.
    pub fn pressable(&self, kind: &str, style: Option<&str>, state: Option<&str>) -> LookValues {
        let plain = self.part(kind, None);
        let style = style.filter(|s| !s.is_empty()).map(|s| self.style(s)).unwrap_or_default();
        let styled = style.over(&plain);
        let Some(state) = state else { return styled };
        let own = self.part_own(kind, Some(state));
        let Some(fill) = style.background else { return own.over(&styled) };
        let text = styled.text.unwrap_or_else(|| self.color("text"));
        let shaded = LookValues { background: Some(self.shade(fill, text, state)), text: Some(text), border: own.border, ..LookValues::default() };
        shaded.over(&styled)
    }

    /// `fill` shaded for a state, toward whichever of the theme's text and
    /// background colors is farther from `text`, so the text only gains
    /// contrast: a little on hover, more for any other state.
    fn shade(&self, fill: Color, text: Color, state: &str) -> Color {
        let amount = if state == "hover" { 0.12 } else { 0.24 };
        let (dark, light) = (self.color("text"), self.color("background"));
        let far = |c: Color| (luminance(c) - luminance(text)).abs();
        let toward = if far(dark) >= far(light) { dark } else { light };
        let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * amount).round() as u8;
        Color::from_argb(fill.a(), mix(fill.r(), toward.r()), mix(fill.g(), toward.g()), mix(fill.b(), toward.b()))
    }
}

/// A color's relative luminance, as WCAG measures it.
pub fn luminance(color: Color) -> f64 {
    let channel = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

/// The contrast between two colors, from 1 (none) to 21 (black on white), as WCAG measures it.
pub fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

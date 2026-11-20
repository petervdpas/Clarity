//! The app's theme as a stylesheet sees it: its roles as custom properties.
//! A theme file (`runtimes/rust-ui/themes/*.json`, the theme a program is
//! given) names its colors, fonts and sizes; a stylesheet reads them as
//! `var(--accent)`, `var(--textFont)`, `var(--textSize)`: the role's name in
//! camelCase, the fonts and sizes with `Font` and `Size` after theirs.

use std::collections::BTreeMap;

/// A theme's roles, by their custom property names (without `--`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Theme {
    pub(crate) values: BTreeMap<String, String>,
}

impl Theme {
    /// Reads a theme file's text.
    pub fn read(json: &str) -> Result<Theme, String> {
        let theme: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let Some(object) = theme.as_object() else { return Err("a theme is a JSON object".into()) };
        let mut values = BTreeMap::new();
        let section = |name: &str| object.get(name).and_then(|v| v.as_object());
        for (role, value) in section("colors").into_iter().flatten() {
            if let Some(color) = value.as_str() {
                values.insert(camel(role), color.to_string());
            }
        }
        for (role, value) in section("fonts").into_iter().flatten() {
            if let Some(fonts) = value.as_str() {
                values.insert(format!("{}Font", camel(role)), fonts.to_string());
            }
        }
        for (role, value) in section("sizes").into_iter().flatten() {
            if let Some(size) = value.as_f64() {
                values.insert(format!("{}Size", camel(role)), format!("{size}px"));
            }
        }
        Ok(Theme { values })
    }

    /// A role's value by its custom property name, without `--`.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}

/// `strong_border` as `strongBorder`.
fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' || c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

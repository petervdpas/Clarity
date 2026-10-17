//! Themes (design section 1.5, T1): named values for how an application
//! looks, not a style sheet. The web renderer turns a theme into CSS
//! variables; a native toolkit into its own resources and styles.
//!
//! ```text
//! theme Shop
//!     mode system
//!     color accent "#2f6fdf" dark "#5b8cff"
//!     font "Inter"
//!     size 14
//!     radius 6
//!     style primary background accent, text "#ffffff"
//! ```

use crate::diagnostic::Diagnostic;
use crate::lexer::TokenKind;
use crate::model::{Ident, Project, SpecFile, ThemeColor, ThemeDecl, ThemeMode, ThemeStyle, Widget};
use crate::parser::{Cursor, Node, ParseResult, first_word, no_children};

/// The colors every theme has, which the renderer itself uses. A theme
/// sets them with `color`, and can name colors of its own for its styles.
pub const ROLES: [&str; 8] = ["background", "panel", "text", "muted", "border", "accent", "error", "warning"];

/// What a style sets.
const STYLE_PARTS: [&str; 3] = ["background", "text", "border"];

pub(crate) fn parse_theme(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> ThemeDecl {
    let mut theme = ThemeDecl {
        class: class_name("theme", &name.name),
        name,
        mode: None,
        colors: Vec::new(),
        font: None,
        size: None,
        radius: None,
        styles: Vec::new(),
        span: node.span(),
    };
    for child in &node.children {
        if let Err(diag) = parse_line(child, &mut theme) {
            diagnostics.push(diag);
        }
    }
    theme
}

fn number(cursor: &mut Cursor, what: &str) -> ParseResult<u32> {
    let token = cursor.expect(TokenKind::Number, what)?;
    token
        .text
        .parse()
        .map_err(|_| crate::diagnostic::Diagnostic::error(format!("{what} is a whole number"), token.span))
}

fn parse_line(node: &Node, theme: &mut ThemeDecl) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = first_word(node).unwrap_or("");
    cursor.next();
    match keyword {
        "mode" => {
            let word = cursor.expect_ident("light, dark or system")?;
            theme.mode = Some(match word.name.as_str() {
                "light" => ThemeMode::Light,
                "dark" => ThemeMode::Dark,
                "system" => ThemeMode::System,
                other => {
                    return Err(Diagnostic::error(
                        format!("unknown mode '{other}'; a theme is light, dark or system"),
                        word.span,
                    ));
                }
            });
        }
        "color" => {
            let name = cursor.expect_ident("a color name")?;
            let light = cursor.expect_string("the color, like \"#2f6fdf\"")?;
            let dark = if cursor.peek().is_some_and(|t| t.text == "dark") {
                cursor.next();
                Some(cursor.expect_string("the color in dark mode")?)
            } else {
                None
            };
            theme.colors.push(ThemeColor {
                name,
                light,
                dark,
                span: node.line.span,
            });
        }
        "font" => theme.font = Some(cursor.expect_string("a font name")?),
        "size" => theme.size = Some(number(&mut cursor, "the text size")?),
        "radius" => theme.radius = Some(number(&mut cursor, "the corner radius")?),
        "style" => {
            let name = cursor.expect_ident("a style name")?;
            let mut style = ThemeStyle {
                class: class_name("style", &name.name),
                name,
                background: None,
                text: None,
                border: None,
                span: node.line.span,
            };
            loop {
                let part = cursor.expect_ident("background, text or border")?;
                let value = match cursor.peek() {
                    Some(t) if matches!(t.kind, TokenKind::Ident | TokenKind::Str) => {
                        cursor.next();
                        t.text.clone()
                    }
                    _ => return Err(cursor.error_here("expected a color: a name of the theme, or \"#rrggbb\"")),
                };
                let slot = match part.name.as_str() {
                    "background" => &mut style.background,
                    "text" => &mut style.text,
                    "border" => &mut style.border,
                    other => {
                        return Err(Diagnostic::error(
                            format!("a style sets background, text and border, not '{other}'"),
                            part.span,
                        ));
                    }
                };
                *slot = Some(value);
                if !cursor.eat(TokenKind::Comma) {
                    break;
                }
            }
            theme.styles.push(style);
        }
        other => {
            return Err(Diagnostic::error(
                format!("unknown theme setting '{other}'; a theme has mode, color, font, size, radius and style"),
                node.line.span,
            ));
        }
    }
    cursor.expect_end()
}

/// Whether `text` is a color written out: `#rgb` or `#rrggbb`.
fn is_hex(text: &str) -> bool {
    text.strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

fn listed(words: &[String]) -> String {
    match words.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The colors a style of `theme` can name: the roles, then its own.
fn color_names(theme: &ThemeDecl) -> Vec<String> {
    let mut names: Vec<String> = ROLES.iter().map(|r| r.to_string()).collect();
    for color in &theme.colors {
        if !names.contains(&color.name.name) {
            names.push(color.name.name.clone());
        }
    }
    names
}

/// Checks the themes of a file: colors written out, each named once, and
/// styles that use colors the theme has.
pub(crate) fn check_decls(file: &SpecFile) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for theme in &file.themes {
        let name = &theme.name.name;
        for (i, color) in theme.colors.iter().enumerate() {
            if theme.colors[..i].iter().any(|c| c.name.name == color.name.name) {
                found.push(Diagnostic::error(
                    format!("theme {name} has color '{}' twice", color.name.name),
                    color.span,
                ));
            }
            for value in std::iter::once(&color.light).chain(&color.dark) {
                if !is_hex(value) {
                    found.push(Diagnostic::error(
                        format!("'{value}' is not a color; write it as \"#rrggbb\" or \"#rgb\""),
                        color.span,
                    ));
                }
            }
        }
        let names = color_names(theme);
        for style in &theme.styles {
            let parts = [&style.background, &style.text, &style.border];
            for value in parts.into_iter().flatten() {
                let known = is_hex(value) || names.contains(value) || (value == "none" && style.border.as_ref() == Some(value));
                if !known {
                    found.push(Diagnostic::error(
                        format!(
                            "style '{}' uses color '{value}', which theme {name} does not have; its colors are {}",
                            style.name.name,
                            listed(&names)
                        ),
                        style.span,
                    ));
                }
            }
        }
    }
    let _ = STYLE_PARTS;
    found.into_iter().map(|d| d.in_file(&file.path)).collect()
}

fn styled(widgets: &[Widget], out: &mut Vec<(String, crate::span::Span)>) {
    for widget in widgets {
        if let Some(style) = &widget.props.style {
            out.push((style.clone(), widget.span));
        }
        styled(&widget.children, out);
        for page in &widget.pages {
            styled(&page.children, out);
        }
    }
}

/// Sets the theme the project file names (`theme Shop`), and checks that
/// it is declared and that every part's style is one of its styles.
pub fn use_theme(project: &mut Project, theme: Option<&str>) {
    project.theme = theme.map(str::to_string);
    let mut found = Vec::new();
    let declared = theme.and_then(|t| project.files.iter().flat_map(|f| &f.themes).find(|d| d.name.name == t));
    if let (Some(name), None) = (theme, declared) {
        found.push(Diagnostic::error(
            format!("the project file names theme {name}, which no module declares"),
            crate::span::Span::default(),
        ));
    }
    for file in &project.files {
        let mut uses = Vec::new();
        for page in &file.pages {
            styled(&page.widgets, &mut uses);
        }
        for (style, span) in uses {
            let message = match (theme, declared) {
                (None, _) => format!(
                    "style '{style}' comes from the project's theme; name it in the project file, like: theme {}",
                    project.files.iter().flat_map(|f| &f.themes).next().map_or("Shop", |t| t.name.name.as_str())
                ),
                (Some(_), Some(decl)) if decl.styles.iter().any(|s| s.name.name == style) => continue,
                (Some(_), Some(decl)) => {
                    let names: Vec<String> = decl.styles.iter().map(|s| s.name.name.clone()).collect();
                    format!("theme {} has no style '{style}'; its styles are {}", decl.name.name, listed(&names))
                }
                // The missing theme is reported once, above.
                (Some(_), None) => continue,
            };
            found.push(Diagnostic::error(message, span).in_file(&file.path));
        }
    }
    project.diagnostics.extend(found);
}

/// A style's colors as resolved: `#rrggbb`, `none` for a border, or
/// `@role` for a role the theme leaves to the renderer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResolvedStyle {
    pub background: Option<String>,
    pub text: Option<String>,
    pub border: Option<String>,
}

/// A theme for light or dark mode: what every stack shows, whatever its
/// means (conformance/themes.json).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Resolved {
    pub dark: bool,
    pub colors: std::collections::BTreeMap<String, String>,
    pub font: Option<String>,
    pub size: Option<u32>,
    pub radius: Option<u32>,
    pub styles: std::collections::BTreeMap<String, ResolvedStyle>,
}

/// `#abc` as `#aabbcc`, lowercase.
pub fn full_hex(color: &str) -> String {
    let digits = color.trim_start_matches('#').to_ascii_lowercase();
    if digits.len() == 3 {
        format!("#{}", digits.chars().flat_map(|d| [d, d]).collect::<String>())
    } else {
        format!("#{digits}")
    }
}

/// Whether `theme` shows dark when the system is `system_dark`: a theme
/// with mode light or dark keeps to it.
pub fn is_dark(theme: &ThemeDecl, system_dark: bool) -> bool {
    match theme.mode {
        Some(ThemeMode::Dark) => true,
        Some(ThemeMode::Light) => false,
        Some(ThemeMode::System) | None => system_dark,
    }
}

/// `theme` for light or dark mode, when the system is `system_dark`.
pub fn resolve(theme: &ThemeDecl, system_dark: bool) -> Resolved {
    let dark = is_dark(theme, system_dark);
    let colors: std::collections::BTreeMap<String, String> = theme
        .colors
        .iter()
        .map(|c| (c.name.name.clone(), full_hex(c.dark.as_deref().filter(|_| dark).unwrap_or(&c.light))))
        .collect();
    let value = |written: &Option<String>| -> Option<String> {
        let written = written.as_deref()?;
        Some(match written {
            "none" => "none".to_string(),
            hex if hex.starts_with('#') => full_hex(hex),
            name => colors.get(name).cloned().unwrap_or_else(|| format!("@{name}")),
        })
    };
    let styles = theme
        .styles
        .iter()
        .map(|s| {
            let style = ResolvedStyle {
                background: value(&s.background),
                text: value(&s.text),
                border: value(&s.border),
            };
            (s.name.name.clone(), style)
        })
        .collect();
    Resolved {
        dark,
        colors,
        font: theme.font.clone(),
        size: theme.size,
        radius: theme.radius,
        styles,
    }
}

/// A class name for a theme or style: `theme-night-owl`, `style-primary`.
fn class_name(kind: &str, name: &str) -> String {
    format!("{kind}-{}", crate::model::snake_case(name).replace('_', "-"))
}

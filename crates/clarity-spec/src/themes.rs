//! Themes (design section 1.5, T1): named values for how an application
//! looks, not a style sheet. Every theme starts from Clarity's own (the
//! built-in `theme Clarity`, which sets every value) and changes what it
//! wants. The web renderer turns a theme into a CSS file of variables; a
//! native toolkit into its own resources and styles.
//!
//! ```text
//! theme Shop
//!     mode system
//!     color accent "#2f6fdf" dark "#5b8cff"
//!     font "Inter"
//!     size 14
//!     radius 6
//!     part button hover background accent_soft
//!     style primary background accent, text on_accent, weight bold
//! ```

use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;
use crate::lexer::TokenKind;
use crate::model::{
    Ident, Look, Project, Shadow, SpecFile, ThemeColor, ThemeDecl, ThemeMode, ThemePart, ThemeStyle, ThemeValue, Weight, Widget,
};
use crate::parser::{Cursor, Node, ParseResult, first_word, no_children};

/// The colors every theme has, which the renderer itself uses. Clarity's
/// own theme sets them all; another sets those it changes, and can name
/// colors of its own for its styles and parts.
pub const ROLES: [&str; 21] = [
    "background",
    "panel",
    "raised",
    "text",
    "muted",
    "border",
    "strong_border",
    "accent",
    "accent_soft",
    "on_accent",
    "selection",
    "focus",
    "input",
    "frame",
    "frame_text",
    "error",
    "error_soft",
    "warning",
    "warning_soft",
    "success",
    "success_soft",
];

/// The fonts, text sizes and radii a theme sets, by role.
pub const FONTS: [&str; 3] = ["text", "heading", "mono"];
pub const SIZES: [&str; 3] = ["text", "small", "heading"];
pub const RADII: [&str; 3] = ["input", "button", "panel"];

/// The kinds of part a theme gives a look, and the states they can be in.
pub const PART_KINDS: [&str; 5] = ["button", "input", "header", "tab", "menu"];
pub const STATES: [&str; 3] = ["hover", "pressed", "focus"];

/// What a style or a part look sets.
const LOOK_WORDS: [&str; 5] = ["background", "text", "border", "radius", "weight"];

/// The source of Clarity's own theme, which sets every value.
pub const BUILT_IN: &str = include_str!("../themes/clarity.clarity");

/// The name of Clarity's own theme.
pub const BUILT_IN_NAME: &str = "Clarity";

/// Clarity's own theme: the look every theme starts from.
pub fn built_in() -> ThemeDecl {
    let (file, diagnostics) = crate::parser::parse(BUILT_IN, "clarity.clarity");
    debug_assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    file.themes.into_iter().next().expect("the built-in theme")
}

pub(crate) fn parse_theme(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> ThemeDecl {
    let mut theme = ThemeDecl {
        class: class_name("theme", &name.name),
        name,
        label: String::new(),
        title: None,
        mode: None,
        colors: Vec::new(),
        fonts: Vec::new(),
        sizes: Vec::new(),
        radii: Vec::new(),
        padding: None,
        border: None,
        shadow: None,
        parts: Vec::new(),
        styles: Vec::new(),
        span: node.span(),
    };
    for child in &node.children {
        if let Err(diag) = parse_line(child, &mut theme) {
            diagnostics.push(diag);
        }
    }
    theme.label = theme.title.clone().unwrap_or_else(|| crate::model::default_label(&theme.name.name));
    theme
}

fn number(cursor: &mut Cursor, what: &str) -> ParseResult<u32> {
    let token = cursor.expect(TokenKind::Number, what)?;
    token.text.parse().map_err(|_| Diagnostic::error(format!("{what} is a whole number"), token.span))
}

/// The role before a value, when there is one: `font heading "Georgia"`
/// reads `heading`, `font "Inter"` gives `default`.
fn role(cursor: &mut Cursor, default: &str, known: &[&str], unknown: impl Fn(&str) -> String) -> ParseResult<String> {
    match cursor.peek() {
        Some(t) if t.kind == TokenKind::Ident => {
            cursor.next();
            if known.contains(&t.text.as_str()) {
                Ok(t.text.clone())
            } else {
                Err(Diagnostic::error(unknown(&t.text), t.span))
            }
        }
        _ => Ok(default.to_string()),
    }
}

fn listed_str(words: &[&str]) -> String {
    listed(&words.iter().map(|w| w.to_string()).collect::<Vec<_>>())
}

/// `background accent, text on_accent, radius 6, weight bold`.
fn parse_look(cursor: &mut Cursor, what: &str) -> ParseResult<Look> {
    let mut look = Look::default();
    loop {
        let word = cursor.expect_ident(&format!("what {what} sets: {}", listed_str(&LOOK_WORDS)))?;
        match word.name.as_str() {
            "background" | "text" | "border" => {
                let value = match cursor.peek() {
                    Some(t) if matches!(t.kind, TokenKind::Ident | TokenKind::Str) => {
                        cursor.next();
                        t.text.clone()
                    }
                    _ => return Err(cursor.error_here("expected a color: a name of the theme, or \"#rrggbb\"")),
                };
                let slot = match word.name.as_str() {
                    "background" => &mut look.background,
                    "text" => &mut look.text,
                    _ => &mut look.border,
                };
                *slot = Some(value);
            }
            "radius" => look.radius = Some(number(cursor, "the corner radius")?),
            "weight" => {
                let value = cursor.expect_ident("bold or normal")?;
                look.weight = Some(match value.name.as_str() {
                    "bold" => Weight::Bold,
                    "normal" => Weight::Normal,
                    other => return Err(Diagnostic::error(format!("unknown weight '{other}'; it is bold or normal"), value.span)),
                });
            }
            other => {
                return Err(Diagnostic::error(
                    format!("{what} sets {}, not '{other}'", listed_str(&LOOK_WORDS)),
                    word.span,
                ));
            }
        }
        if !cursor.eat(TokenKind::Comma) {
            return Ok(look);
        }
    }
}

fn parse_line(node: &Node, theme: &mut ThemeDecl) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = first_word(node).unwrap_or("");
    cursor.next();
    let span = node.line.span;
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
            theme.colors.push(ThemeColor { name, light, dark, span });
        }
        "title" => theme.title = Some(cursor.expect_string("the name people see")?),
        "font" => {
            let role = role(&mut cursor, "text", &FONTS, |r| {
                format!("unknown font '{r}'; a theme sets the fonts {}", listed_str(&FONTS))
            })?;
            let value = cursor.expect_string("a font name")?;
            theme.fonts.push(ThemeValue { role, value, span });
        }
        "size" => {
            let role = role(&mut cursor, "text", &SIZES, |r| {
                format!("unknown size '{r}'; a theme sets the sizes {}", listed_str(&SIZES))
            })?;
            let value = number(&mut cursor, "the text size")?.to_string();
            theme.sizes.push(ThemeValue { role, value, span });
        }
        "radius" => {
            let role = role(&mut cursor, "all", &RADII, |r| {
                format!(
                    "unknown radius '{r}'; a theme sets the radius of {}, or of all with 'radius n'",
                    listed_str(&RADII)
                )
            })?;
            let value = number(&mut cursor, "the corner radius")?.to_string();
            theme.radii.push(ThemeValue { role, value, span });
        }
        "padding" => theme.padding = Some(number(&mut cursor, "the padding")?),
        "border" => theme.border = Some(number(&mut cursor, "the border width")?),
        "shadow" => {
            let word = cursor.expect_ident("none, flat or raised")?;
            theme.shadow = Some(match word.name.as_str() {
                "none" => Shadow::None,
                "flat" => Shadow::Flat,
                "raised" => Shadow::Raised,
                other => {
                    return Err(Diagnostic::error(
                        format!("unknown shadow '{other}'; a shadow is none, flat or raised"),
                        word.span,
                    ));
                }
            });
        }
        "part" => {
            let kind = cursor.expect_ident("a kind of part")?;
            if !PART_KINDS.contains(&kind.name.as_str()) {
                return Err(Diagnostic::error(
                    format!("a theme sets the look of the parts {}, not '{}'", listed_str(&PART_KINDS), kind.name),
                    kind.span,
                ));
            }
            // A word that is not what a look sets is the state.
            let state = match cursor.peek() {
                Some(t) if t.kind == TokenKind::Ident && !LOOK_WORDS.contains(&t.text.as_str()) => {
                    cursor.next();
                    if !STATES.contains(&t.text.as_str()) {
                        return Err(Diagnostic::error(
                            format!("unknown state '{}'; a part looks different on {}", t.text, listed_str_or(&STATES)),
                            t.span,
                        ));
                    }
                    Some(t.text.clone())
                }
                _ => None,
            };
            let look = parse_look(&mut cursor, "a part")?;
            theme.parts.push(ThemePart { kind: kind.name, state, look, span });
        }
        "style" => {
            let name = cursor.expect_ident("a style name")?;
            let look = parse_look(&mut cursor, "a style")?;
            theme.styles.push(ThemeStyle { class: class_name("style", &name.name), name, look, span });
        }
        other => {
            return Err(Diagnostic::error(
                format!(
                    "unknown theme setting '{other}'; a theme has title, mode, color, font, size, radius, padding, border, shadow, part and style"
                ),
                span,
            ));
        }
    }
    cursor.expect_end()
}

/// `hover, pressed or focus`.
fn listed_str_or(words: &[&str]) -> String {
    match words.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
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

/// The colors a style or part of `theme` can name: the roles, then its own.
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
/// styles and parts that use colors the theme has.
pub(crate) fn check_decls(file: &SpecFile) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for theme in &file.themes {
        let name = &theme.name.name;
        for (i, color) in theme.colors.iter().enumerate() {
            if theme.colors[..i].iter().any(|c| c.name.name == color.name.name) {
                found.push(Diagnostic::error(format!("theme {name} has color '{}' twice", color.name.name), color.span));
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
        let looks = theme
            .styles
            .iter()
            .map(|s| (format!("style '{}'", s.name.name), &s.look, s.span))
            .chain(theme.parts.iter().map(|p| {
                let state = p.state.as_deref().map(|s| format!(" {s}")).unwrap_or_default();
                (format!("part {}{state}", p.kind), &p.look, p.span)
            }));
        for (what, look, span) in looks {
            for value in [&look.background, &look.text, &look.border].into_iter().flatten() {
                let known = is_hex(value) || names.contains(value) || (value == "none" && look.border.as_ref() == Some(value));
                if !known {
                    found.push(Diagnostic::error(
                        format!("{what} uses color '{value}', which theme {name} does not have; its colors are {}", listed(&names)),
                        span,
                    ));
                }
            }
        }
    }
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

/// Sets the themes the project file offers (`theme Shop Night`), the first
/// being the one the application starts with, and checks that each is
/// declared and has every style a part uses, so switching themes never
/// drops one.
pub fn use_themes(project: &mut Project, offered: &[&str]) {
    project.themes = offered.iter().map(|t| t.to_string()).collect();
    let mut found = Vec::new();
    let mut declared = Vec::new();
    for name in offered {
        match project.files.iter().flat_map(|f| &f.themes).find(|d| d.name.name == *name) {
            Some(decl) => declared.push(decl),
            None => found.push(Diagnostic::error(
                format!("the project file names theme {name}, which no module declares"),
                crate::span::Span::default(),
            )),
        }
    }
    for file in &project.files {
        let mut uses = Vec::new();
        for page in &file.pages {
            styled(&page.widgets, &mut uses);
        }
        for (style, span) in uses {
            if offered.is_empty() {
                let example = project.files.iter().flat_map(|f| &f.themes).next().map_or("Shop", |t| t.name.name.as_str());
                found.push(
                    Diagnostic::error(
                        format!("style '{style}' comes from the project's theme; name it in the project file, like: theme {example}"),
                        span,
                    )
                    .in_file(&file.path),
                );
                continue;
            }
            for decl in declared.iter().filter(|d| !d.styles.iter().any(|s| s.name.name == style)) {
                let names: Vec<String> = decl.styles.iter().map(|s| s.name.name.clone()).collect();
                let has = if names.is_empty() { "it has no styles".to_string() } else { format!("its styles are {}", listed(&names)) };
                found.push(Diagnostic::error(format!("theme {} has no style '{style}'; {has}", decl.name.name), span).in_file(&file.path));
            }
        }
    }
    project.diagnostics.extend(found);
}

/// A look as resolved: colors as `#rrggbb`, `none` for a border, or `@role`
/// for a role the theme leaves to Clarity's own.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResolvedLook {
    pub background: Option<String>,
    pub text: Option<String>,
    pub border: Option<String>,
    pub radius: Option<u32>,
    pub weight: Option<Weight>,
}

/// A theme for light or dark mode: what every stack shows, whatever its
/// means (conformance/themes.json). It holds what the theme sets; what it
/// leaves out is Clarity's own.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Resolved {
    pub dark: bool,
    pub colors: BTreeMap<String, String>,
    pub fonts: BTreeMap<String, String>,
    pub sizes: BTreeMap<String, u32>,
    pub radii: BTreeMap<String, u32>,
    pub padding: Option<u32>,
    pub border: Option<u32>,
    pub shadow: Option<Shadow>,
    /// By kind, or `kind:state`: `button`, `button:hover`.
    pub parts: BTreeMap<String, ResolvedLook>,
    pub styles: BTreeMap<String, ResolvedLook>,
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
    let colors: BTreeMap<String, String> = theme
        .colors
        .iter()
        .map(|c| (c.name.name.clone(), full_hex(c.dark.as_deref().filter(|_| dark).unwrap_or(&c.light))))
        .collect();
    let color = |written: &Option<String>| -> Option<String> {
        let written = written.as_deref()?;
        Some(match written {
            "none" => "none".to_string(),
            hex if hex.starts_with('#') => full_hex(hex),
            name => colors.get(name).cloned().unwrap_or_else(|| format!("@{name}")),
        })
    };
    let look = |l: &Look| ResolvedLook {
        background: color(&l.background),
        text: color(&l.text),
        border: color(&l.border),
        radius: l.radius,
        weight: l.weight,
    };
    let numbers = |values: &[ThemeValue]| values.iter().filter_map(|v| Some((v.role.clone(), v.value.parse().ok()?))).collect();
    let parts = theme
        .parts
        .iter()
        .map(|p| {
            let key = p.state.as_ref().map_or_else(|| p.kind.clone(), |s| format!("{}:{s}", p.kind));
            (key, look(&p.look))
        })
        .collect();
    let styles = theme.styles.iter().map(|s| (s.name.name.clone(), look(&s.look))).collect();
    Resolved {
        dark,
        colors: colors.clone(),
        fonts: theme.fonts.iter().map(|v| (v.role.clone(), v.value.clone())).collect(),
        sizes: numbers(&theme.sizes),
        radii: numbers(&theme.radii),
        padding: theme.padding,
        border: theme.border,
        shadow: theme.shadow,
        parts,
        styles,
    }
}

/// A class name for a theme or style: `theme-night-owl`, `style-primary`.
fn class_name(kind: &str, name: &str) -> String {
    format!("{kind}-{}", crate::model::snake_case(name).replace('_', "-"))
}

/// The text of a new theme called `name`: a copy of Clarity's own, so
/// every value it can set is there to change.
pub fn starter(name: &str) -> String {
    let start = BUILT_IN.find("\ntheme ").map_or(0, |at| at + 1);
    BUILT_IN[start..]
        .replacen(&format!("theme {BUILT_IN_NAME}"), &format!("theme {name}"), 1)
        .replacen(&format!("title \"{BUILT_IN_NAME}\""), &format!("title \"{name}\""), 1)
}

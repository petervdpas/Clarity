//! The web renderer's share of a project's look: one CSS file per theme
//! (design section 1.5, T1), as a web developer would write it. Clarity's
//! own theme is the base (`themes/clarity.css` on `.clarity-app`); a
//! project's theme sits on top of it under its class, so whatever it leaves
//! out keeps the base's value. A part holds the class of its style. The
//! designer gets the same CSS for its canvas.

use clarity_spec::model::{Look, Shadow, ThemeDecl, ThemeValue, Weight};
use clarity_spec::{Preview, Project};

use crate::OutFile;
use clarity_spec::themes::{built_in, full_hex, resolve};

/// The renderer's CSS variable for each color role.
const COLOR_VARIABLES: [(&str, &str); 21] = [
    ("background", "--bg"),
    ("panel", "--panel"),
    ("raised", "--panel-alt"),
    ("text", "--text"),
    ("muted", "--text-muted"),
    ("border", "--border"),
    ("strong_border", "--border-strong"),
    ("accent", "--accent"),
    ("accent_soft", "--accent-soft"),
    ("on_accent", "--on-accent"),
    ("selection", "--selection"),
    ("focus", "--focus"),
    ("input", "--input"),
    ("frame", "--frame"),
    ("frame_text", "--frame-text"),
    ("error", "--error"),
    ("error_soft", "--error-soft"),
    ("warning", "--warning"),
    ("warning_soft", "--warning-soft"),
    ("success", "--success"),
    ("success_soft", "--success-soft"),
];

const FONT_VARIABLES: [(&str, &str); 3] = [("text", "--font"), ("heading", "--font-heading"), ("mono", "--mono")];
const SIZE_VARIABLES: [(&str, &str); 3] = [("text", "--font-size"), ("small", "--font-size-small"), ("heading", "--font-size-heading")];
const RADIUS_VARIABLES: [(&str, &str); 3] = [("input", "--radius-input"), ("button", "--radius-button"), ("panel", "--radius-panel")];

/// Font families CSS knows by name, which are never quoted.
pub(crate) const GENERIC_FONTS: [&str; 8] = [
    "system-ui",
    "-apple-system",
    "sans-serif",
    "serif",
    "monospace",
    "ui-monospace",
    "ui-sans-serif",
    "ui-serif",
];

/// The class of Clarity's application root, which the base theme sets.
pub const APP_CLASS: &str = "clarity-app";

fn lookup(table: &[(&str, &str)], key: &str) -> Option<String> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string())
}

/// The CSS variable of a color: a role's own variable, or one of the
/// theme's colors (names are the theme's own, an open set).
fn color_variable(color: &str) -> String {
    lookup(&COLOR_VARIABLES, color).unwrap_or_else(|| format!("--color-{}", color.replace('_', "-")))
}

/// A color as CSS: a name as its variable, `#abc` in full, `none` as is.
fn color_value(written: &str) -> String {
    match written {
        "none" => "none".into(),
        hex if hex.starts_with('#') => full_hex(hex),
        name => format!("var({})", color_variable(name)),
    }
}

/// A font list as CSS: families with their names quoted, generic ones not.
fn font_value(families: &str) -> String {
    families
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|f| if GENERIC_FONTS.contains(&f) { f.to_string() } else { format!("\"{}\"", f.replace('"', "")) })
        .collect::<Vec<_>>()
        .join(", ")
}

fn shadow_value(shadow: Shadow) -> &'static str {
    match shadow {
        Shadow::None => "none",
        Shadow::Flat => "0 1px 2px rgb(0 0 0 / 0.15)",
        Shadow::Raised => "0 6px 18px rgb(0 0 0 / 0.25)",
    }
}

fn weight_value(weight: Weight) -> &'static str {
    match weight {
        Weight::Bold => "600",
        Weight::Normal => "400",
    }
}

/// The variables of a part look: `--button-hover-bg: var(--accent-soft)`.
fn part_variables(prefix: &str, look: &Look) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(v) = &look.background {
        lines.push(format!("--{prefix}-bg: {}", color_value(v)));
    }
    if let Some(v) = &look.text {
        lines.push(format!("--{prefix}-text: {}", color_value(v)));
    }
    if let Some(v) = &look.border {
        lines.push(format!("--{prefix}-border: {}", color_value(v)));
    }
    if let Some(v) = look.radius {
        lines.push(format!("--{prefix}-radius: {v}px"));
    }
    if let Some(v) = look.weight {
        lines.push(format!("--{prefix}-weight: {}", weight_value(v)));
    }
    lines
}

/// The CSS a style or look applies to a part directly.
fn look_rules(look: &Look) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(v) = &look.background {
        lines.push(format!("background: {}", color_value(v)));
    }
    if let Some(v) = &look.text {
        lines.push(format!("color: {}", color_value(v)));
    }
    match look.border.as_deref() {
        Some("none") => lines.push("border: none".into()),
        Some(v) => lines.push(format!("border-color: {}", color_value(v))),
        None => {}
    }
    if let Some(v) = look.radius {
        lines.push(format!("border-radius: {v}px"));
    }
    if let Some(v) = look.weight {
        lines.push(format!("font-weight: {}", weight_value(v)));
    }
    lines
}

fn rule(selector: &str, lines: &[String]) -> String {
    let body: String = lines.iter().map(|l| format!("  {l};\n")).collect();
    format!("{selector} {{\n{body}}}\n")
}

/// The variables a theme sets for light mode (or its only mode).
fn variables(theme: &ThemeDecl) -> Vec<String> {
    let light = resolve(theme, false);
    let mut lines: Vec<String> = theme
        .colors
        .iter()
        .filter_map(|c| light.colors.get(&c.name.name).map(|v| format!("{}: {v}", color_variable(&c.name.name))))
        .collect();
    let named = |values: &[ThemeValue], table: &[(&str, &str)], value: &dyn Fn(&str) -> String| -> Vec<String> {
        values
            .iter()
            .filter_map(|v| lookup(table, &v.role).map(|var| format!("{var}: {}", value(&v.value))))
            .collect()
    };
    lines.extend(named(&theme.fonts, &FONT_VARIABLES, &font_value));
    lines.extend(named(&theme.sizes, &SIZE_VARIABLES, &|v| format!("{v}px")));
    for radius in &theme.radii {
        // `radius n` sets every kind; `radius input n` one.
        let kinds: Vec<&(&str, &str)> = RADIUS_VARIABLES.iter().filter(|(k, _)| radius.role == "all" || *k == radius.role).collect();
        lines.extend(kinds.iter().map(|(_, var)| format!("{var}: {}px", radius.value)));
    }
    if let Some(padding) = theme.padding {
        lines.push(format!("--padding: {padding}px"));
    }
    if let Some(border) = theme.border {
        lines.push(format!("--border-width: {border}px"));
    }
    if let Some(shadow) = theme.shadow {
        lines.push(format!("--shadow: {}", shadow_value(shadow)));
    }
    for part in &theme.parts {
        let prefix = part.state.as_ref().map_or_else(|| part.kind.clone(), |s| format!("{}-{s}", part.kind));
        lines.extend(part_variables(&prefix, &part.look));
    }
    lines
}

/// The selector a theme's CSS applies to: the base on every application,
/// another theme only under its class.
fn selector(theme: &ThemeDecl, base: bool) -> String {
    if base { format!(".{APP_CLASS}") } else { format!(".{APP_CLASS}.{}", theme.class) }
}

fn css(theme: &ThemeDecl, base: bool) -> String {
    let at = selector(theme, base);
    let mut lines = variables(theme);
    if base {
        // The base applies its values to the application itself.
        lines.extend(
            [
                "color: var(--text)",
                "background: var(--bg)",
                "font-family: var(--font)",
                "font-size: var(--font-size)",
                "line-height: 1.4",
                "color-scheme: light",
            ]
            .map(String::from),
        );
    }
    let mut out = format!(
        "/* Generated by Clarity from theme {}. Do not edit: change the theme in Clarity instead. */\n\n",
        theme.name.name
    );
    out.push_str(&rule(&at, &lines));
    let light = resolve(theme, false);
    let dark = resolve(theme, true);
    let mut darker: Vec<String> = theme
        .colors
        .iter()
        .filter_map(|c| {
            let name = &c.name.name;
            let value = dark.colors.get(name)?;
            (light.colors.get(name) != Some(value)).then(|| format!("{}: {value}", color_variable(name)))
        })
        .collect();
    if base {
        darker.push("color-scheme: dark".into());
    }
    if !darker.is_empty() {
        out.push('\n');
        out.push_str(&rule(&format!("{at}[data-scheme=\"dark\"]"), &darker));
    }
    for style in &theme.styles {
        out.push('\n');
        out.push_str(&rule(&format!("{at} .{}", style.class), &look_rules(&style.look)));
    }
    out
}

/// The CSS file of a project's theme, on top of the base.
pub fn theme_css(theme: &ThemeDecl) -> String {
    css(theme, false)
}

/// The base CSS: Clarity's own theme, which every application starts from.
pub fn base_css() -> String {
    css(&built_in(), true)
}

/// The file of a theme in the generated application's `ui` folder:
/// `themes/night-owl.css` for the class `theme-night-owl`.
pub fn theme_file(theme: &ThemeDecl) -> String {
    format!("themes/{}.css", theme.class.trim_start_matches("theme-"))
}

/// The frontend, built from the designer's renderer (`npm run build:runtime`).
const UI_FILES: [(&str, &str); 3] = [
    ("ui/index.html", include_str!("../ui/runtime.html")),
    ("ui/runtime.js", include_str!("../ui/runtime.js")),
    ("ui/runtime.css", include_str!("../ui/runtime.css")),
];

/// The renderer of an application on a web stack, in `ui/`: its page,
/// script and styles, and a CSS file per theme the application offers,
/// linked from the page so the application can hold the class of the theme
/// it shows. Every web stack ships the same files.
pub fn renderer_files(project: &Project) -> Vec<OutFile> {
    let mut files: Vec<OutFile> = UI_FILES.iter().map(|(path, body)| OutFile { path: (*path).into(), text: (*body).into(), bytes: None }).collect();
    let themes: Vec<&ThemeDecl> = project
        .themes
        .iter()
        .filter_map(|name| project.files.iter().flat_map(|f| &f.themes).find(|t| t.name.name == *name))
        .collect();
    // Clarity's own theme first: the base the project's themes change.
    let base = built_in();
    let links: String = std::iter::once(&base)
        .chain(themes.iter().copied())
        .map(|t| format!("    <link rel=\"stylesheet\" href=\"./{}\">\n", theme_file(t)))
        .collect();
    files.push(OutFile { path: format!("ui/{}", theme_file(&base)), text: base_css(), bytes: None });
    for theme in &themes {
        files.push(OutFile { path: format!("ui/{}", theme_file(theme)), text: theme_css(theme), bytes: None });
    }
    if let Some(index) = files.iter_mut().find(|f| f.path == "ui/index.html") {
        index.text = index.text.replace("  </head>", &format!("{links}  </head>"));
    }
    files.extend(picture_files(project, "ui/images"));
    files
}

/// The pictures of the project's images folder, in `folder` of the output.
pub(crate) fn picture_files(project: &Project, folder: &str) -> Vec<OutFile> {
    project
        .images
        .iter()
        .flatten()
        .map(|image| OutFile {
            path: format!("{folder}/{}", image.name),
            text: String::new(),
            bytes: Some(std::borrow::Cow::Owned(image.bytes.clone())),
        })
        .collect()
}

/// The windows, the frame and the data dictionary, for the renderer. File
/// paths are left out: they belong to the machine the project was on.
pub fn model_json(project: &Project) -> String {
    let mut project = project.clone();
    for file in &mut project.files {
        file.path = std::path::Path::new(&file.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        file.version = String::new();
    }
    project.diagnostics.clear();
    let mut preview = Preview::new(project);
    preview.samples.clear();
    preview.displays.clear();
    serde_json::to_string(&preview).unwrap_or_default() + "\n"
}

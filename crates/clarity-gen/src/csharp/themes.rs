//! Themes for Avalonia (DESIGN 1.5, T1). Avalonia has no cascade, so the
//! generator merges Clarity's own theme with each offered theme into one
//! ResourceDictionary per theme, `Themes/Shop.axaml`, which holds every
//! value: Light and Dark theme dictionaries with the colors (as `Clarity*`
//! brushes for the application's own styles), the looks of parts as the
//! Fluent theme's per-state keys (`ButtonBackgroundPointerOver`), and the
//! values of styles; fonts, sizes, radii and spacing beside them. A style is
//! a style class (`Classes="primary"`) whose setters read the theme's values,
//! so switching themes swaps one dictionary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use clarity_spec::layout::{BUTTON_SIDES, INPUT_SIDES};
use clarity_spec::model::{Align, Shadow, ThemeDecl, ThemeMode, Weight};
use clarity_spec::themes::{ResolvedLook, built_in, is_dark, resolve};

use super::HEADER;
use super::names::{pascal, string};
use crate::web::GENERIC_FONTS;

/// A theme for one mode, merged with Clarity's own: every value set, every
/// color a `#rrggbb` (or `none` for a border).
struct Merged {
    colors: BTreeMap<String, String>,
    fonts: BTreeMap<String, String>,
    sizes: BTreeMap<String, u32>,
    /// By kind of part: input, button, panel.
    radii: BTreeMap<String, u32>,
    padding: u32,
    border: u32,
    shadow: Shadow,
    /// By kind, or `kind:state`.
    parts: BTreeMap<String, ResolvedLook>,
    styles: BTreeMap<String, ResolvedLook>,
}

/// How far a framed container's frame sits out into the space between columns, each side.
const FRAME_OUT: u32 = 3;

const RADIUS_KINDS: [&str; 3] = ["input", "button", "panel"];

fn merged(theme: &ThemeDecl, system_dark: bool) -> Merged {
    let dark = is_dark(theme, system_dark);
    let base = resolve(&built_in(), dark);
    let own = resolve(theme, dark);
    let mut colors = base.colors.clone();
    colors.extend(own.colors.clone());
    // `@role` is a role the theme leaves to Clarity's own.
    let fix = |v: &Option<String>| -> Option<String> {
        v.as_ref().map(|v| v.strip_prefix('@').map_or_else(|| v.clone(), |role| colors.get(role).cloned().unwrap_or_else(|| "none".into())))
    };
    let fixed = |l: &ResolvedLook| ResolvedLook {
        background: fix(&l.background),
        text: fix(&l.text),
        border: fix(&l.border),
        radius: l.radius,
        weight: l.weight,
        size: l.size,
        font: l.font.clone(),
        align: l.align,
    };
    let mut radii = BTreeMap::new();
    for source in [&base, &own] {
        if let Some(all) = source.radii.get("all") {
            for kind in RADIUS_KINDS {
                radii.insert(kind.to_string(), *all);
            }
        }
        for (kind, value) in source.radii.iter().filter(|(k, _)| *k != "all") {
            radii.insert(kind.clone(), *value);
        }
    }
    let mut parts: BTreeMap<String, ResolvedLook> = base.parts.iter().map(|(k, l)| (k.clone(), fixed(l))).collect();
    for (key, look) in &own.parts {
        let look = fixed(look);
        let entry = parts.entry(key.clone()).or_insert(ResolvedLook { background: None, text: None, border: None, radius: None, weight: None, size: None, font: None, align: None });
        entry.background = look.background.or(entry.background.take());
        entry.text = look.text.or(entry.text.take());
        entry.border = look.border.or(entry.border.take());
        entry.radius = look.radius.or(entry.radius);
        entry.weight = look.weight.or(entry.weight);
    }
    let overlay = |a: &BTreeMap<String, String>, b: &BTreeMap<String, String>| {
        let mut out = a.clone();
        out.extend(b.clone());
        out
    };
    let mut sizes = base.sizes.clone();
    sizes.extend(own.sizes.clone());
    Merged {
        fonts: overlay(&base.fonts, &own.fonts),
        sizes,
        radii,
        padding: own.padding.or(base.padding).unwrap_or(4),
        border: own.border.or(base.border).unwrap_or(1),
        shadow: own.shadow.or(base.shadow).unwrap_or(Shadow::None),
        styles: own.styles.iter().map(|(k, l)| (k.clone(), fixed(l))).collect(),
        parts,
        colors,
    }
}

impl Merged {
    /// A part's look in a state, falling back to its look without one.
    fn look(&self, kind: &str, state: Option<&str>) -> ResolvedLook {
        let plain = self.parts.get(kind);
        let stated = state.and_then(|s| self.parts.get(&format!("{kind}:{s}")));
        let pick = |f: fn(&ResolvedLook) -> Option<String>| stated.and_then(f).or_else(|| plain.and_then(f));
        ResolvedLook {
            background: pick(|l| l.background.clone()),
            text: pick(|l| l.text.clone()),
            border: pick(|l| l.border.clone()),
            radius: stated.and_then(|l| l.radius).or_else(|| plain.and_then(|l| l.radius)),
            weight: stated.and_then(|l| l.weight).or_else(|| plain.and_then(|l| l.weight)),
            // Only a style sets how its text shows.
            size: None,
            font: None,
            align: None,
        }
    }

    fn radius_of(&self, kind: &str) -> u32 {
        let role = if RADIUS_KINDS.contains(&kind) { kind } else { "panel" };
        self.radii.get(role).copied().unwrap_or(0)
    }
}

/// A color as Avalonia reads it: `none` is no color.
fn color(value: &str) -> &str {
    if value == "none" { "Transparent" } else { value }
}

fn brush(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "      <SolidColorBrush x:Key=\"{key}\" Color=\"{}\" />", color(value));
}

fn weight(w: Option<Weight>) -> &'static str {
    match w {
        // The web renderer's 600.
        Some(Weight::Bold) => "SemiBold",
        _ => "Normal",
    }
}

/// The Fluent theme's keys for a part in a state, by look property
/// (background, text, border). Kinds or states Fluent has no key for are
/// left to the application's own styles.
fn fluent_keys(kind: &str, state: Option<&str>) -> Vec<(&'static str, String)> {
    let controls: &[&str] = match kind {
        "button" => &["Button"],
        "input" => &["TextControl", "ComboBox", "CalendarDatePicker"],
        // A table header has no states of its own in Fluent.
        "header" if state.is_none() => {
            return vec![("background", "DataGridColumnHeaderBackgroundBrush".into()), ("text", "DataGridColumnHeaderForegroundBrush".into())];
        }
        "tab" => {
            let suffix = match state {
                None => "",
                Some("hover") => "PointerOver",
                Some("pressed") => "Pressed",
                _ => return vec![],
            };
            return vec![
                ("background", format!("TabItemHeaderBackgroundUnselected{suffix}")),
                ("text", format!("TabItemHeaderForegroundUnselected{suffix}")),
            ];
        }
        "menu" => {
            return match state {
                None => vec![
                    ("background", "MenuFlyoutPresenterBackground".into()),
                    ("border", "MenuFlyoutPresenterBorderBrush".into()),
                    ("text", "MenuFlyoutItemForeground".into()),
                ],
                Some("hover") => vec![
                    ("background", "MenuFlyoutItemBackgroundPointerOver".into()),
                    ("text", "MenuFlyoutItemForegroundPointerOver".into()),
                ],
                _ => vec![],
            };
        }
        _ => return vec![],
    };
    let suffix = match (kind, state) {
        (_, None) => "",
        (_, Some("hover")) => "PointerOver",
        ("button", Some("pressed")) => "Pressed",
        ("input", Some("focus")) => "Focused",
        _ => return vec![],
    };
    let mut keys = Vec::new();
    for control in controls {
        for (property, name) in [("background", "Background"), ("text", "Foreground"), ("border", "BorderBrush")] {
            // A ComboBox and a date picker have no focused keys of their own.
            if suffix == "Focused" && *control != "TextControl" {
                continue;
            }
            keys.push((property, format!("{control}{name}{suffix}")));
        }
    }
    keys
}

const PART_KINDS: [&str; 5] = ["button", "input", "header", "tab", "menu"];
const STATES: [&str; 3] = ["hover", "pressed", "focus"];

/// The look properties of each style used across the offered themes, so
/// every theme defines the same keys.
pub type StyleProperties = BTreeMap<String, BTreeSet<&'static str>>;

pub fn style_properties(themes: &[&ThemeDecl]) -> StyleProperties {
    let mut out: StyleProperties = BTreeMap::new();
    for theme in themes {
        for style in &theme.styles {
            let set = out.entry(style.name.name.clone()).or_default();
            let l = &style.look;
            if l.background.is_some() {
                set.insert("background");
            }
            if l.text.is_some() {
                set.insert("text");
            }
            if l.border.is_some() {
                set.insert("border");
            }
            if l.radius.is_some() {
                set.insert("radius");
            }
            if l.weight.is_some() {
                set.insert("weight");
            }
            if l.size.is_some() {
                set.insert("size");
            }
            if l.font.is_some() {
                set.insert("font");
            }
            if l.align.is_some() {
                set.insert("align");
            }
        }
    }
    out
}

/// The values of a mode: colors, part looks and styles.
fn mode(m: &Merged, styles: &StyleProperties) -> String {
    let mut out = String::new();
    for (role, value) in &m.colors {
        brush(&mut out, &format!("Clarity{}", pascal(role)), value);
    }
    let accent = &m.colors["accent"];
    for key in ["SystemAccentColor", "SystemAccentColorDark1", "SystemAccentColorDark2", "SystemAccentColorDark3", "SystemAccentColorLight1", "SystemAccentColorLight2", "SystemAccentColorLight3"] {
        let _ = writeln!(out, "      <Color x:Key=\"{key}\">{accent}</Color>");
    }
    for kind in PART_KINDS {
        for state in [None].into_iter().chain(STATES.map(Some)) {
            let look = m.look(kind, state);
            for (property, key) in fluent_keys(kind, state) {
                let value = match property {
                    "background" => look.background.clone(),
                    "text" => look.text.clone(),
                    _ => look.border.clone(),
                };
                if let Some(value) = value {
                    brush(&mut out, &key, &value);
                }
            }
        }
    }
    // A style's values; what this theme leaves out is a button's own look.
    let button = m.look("button", None);
    for (name, properties) in styles {
        let key = format!("ClarityStyle{}", pascal(name));
        let own = m.styles.get(name);
        for property in properties {
            match *property {
                "background" => brush(&mut out, &format!("{key}Background"), &own.and_then(|l| l.background.clone()).or(button.background.clone()).unwrap_or("none".into())),
                "text" => brush(&mut out, &format!("{key}Text"), &own.and_then(|l| l.text.clone()).or(button.text.clone()).unwrap_or("none".into())),
                "border" => {
                    let border = own.and_then(|l| l.border.clone()).or(button.border.clone()).unwrap_or("none".into());
                    let width = if border == "none" { 0 } else { m.border };
                    brush(&mut out, &format!("{key}Border"), &border);
                    let _ = writeln!(out, "      <Thickness x:Key=\"{key}BorderThickness\">{width}</Thickness>");
                }
                "radius" => {
                    let radius = own.and_then(|l| l.radius).unwrap_or(m.radius_of("button"));
                    let _ = writeln!(out, "      <CornerRadius x:Key=\"{key}Radius\">{radius}</CornerRadius>");
                }
                "weight" => {
                    let _ = writeln!(out, "      <FontWeight x:Key=\"{key}Weight\">{}</FontWeight>", weight(own.and_then(|l| l.weight)));
                }
                // What a theme leaves out is the text as a part shows it.
                "size" => {
                    let size = own.and_then(|l| l.size).or(m.sizes.get("text").copied()).unwrap_or(14);
                    let _ = writeln!(out, "      <x:Double x:Key=\"{key}Size\">{size}</x:Double>");
                }
                "font" => {
                    let role = own.and_then(|l| l.font.as_deref()).unwrap_or("text");
                    let families = m.fonts.get(role).or(m.fonts.get("text")).map(String::as_str).unwrap_or("Inter");
                    let _ = writeln!(out, "      <FontFamily x:Key=\"{key}Font\">{}</FontFamily>", font(families));
                }
                _ => {
                    let align = own.and_then(|l| l.align);
                    let (text, content) = match align {
                        Some(Align::Center) => ("Center", "Center"),
                        Some(Align::Right) => ("Right", "Right"),
                        Some(Align::Left) => ("Left", "Left"),
                        None => ("Left", "Center"),
                    };
                    // Aligned text keeps an input's padding from the edge.
                    let side = if align.is_some() { m.padding * INPUT_SIDES } else { 0 };
                    let _ = writeln!(out, "      <TextAlignment x:Key=\"{key}Align\">{text}</TextAlignment>");
                    let _ = writeln!(out, "      <HorizontalAlignment x:Key=\"{key}Content\">{content}</HorizontalAlignment>");
                    let _ = writeln!(out, "      <Thickness x:Key=\"{key}Padding\">{side},0</Thickness>");
                }
            }
        }
    }
    out
}

/// A font list as Avalonia reads it: the families a CSS list names, without
/// CSS's generic names, then the platform's own font.
fn font(families: &str) -> String {
    let mut named: Vec<&str> = families.split(',').map(str::trim).filter(|f| !f.is_empty() && !GENERIC_FONTS.contains(f)).collect();
    named.push("$Default");
    named.join(", ")
}

fn shadow(s: Shadow) -> &'static str {
    match s {
        Shadow::None => "0 0 0 0 #00000000",
        Shadow::Flat => "0 1 2 0 #26000000",
        Shadow::Raised => "0 6 18 0 #40000000",
    }
}

/// The values that are the same in light and dark mode.
fn shared(m: &Merged) -> String {
    let mut out = String::new();
    for (role, key) in [("text", "ClarityFont"), ("heading", "ClarityFontHeading"), ("mono", "ClarityFontMono")] {
        if let Some(families) = m.fonts.get(role) {
            let _ = writeln!(out, "  <FontFamily x:Key=\"{key}\">{}</FontFamily>", font(families));
        }
    }
    if let Some(families) = m.fonts.get("text") {
        let _ = writeln!(out, "  <FontFamily x:Key=\"ContentControlThemeFontFamily\">{}</FontFamily>", font(families));
    }
    for (role, key) in [("text", "ClarityFontSize"), ("small", "ClaritySizeSmall"), ("heading", "ClaritySizeHeading")] {
        if let Some(size) = m.sizes.get(role) {
            let _ = writeln!(out, "  <x:Double x:Key=\"{key}\">{size}</x:Double>");
        }
    }
    if let Some(size) = m.sizes.get("text") {
        let _ = writeln!(out, "  <x:Double x:Key=\"ControlContentThemeFontSize\">{size}</x:Double>");
    }
    for kind in RADIUS_KINDS {
        let _ = writeln!(out, "  <CornerRadius x:Key=\"ClarityRadius{}\">{}</CornerRadius>", pascal(kind), m.radius_of(kind));
    }
    let _ = writeln!(out, "  <CornerRadius x:Key=\"ControlCornerRadius\">{}</CornerRadius>", m.radius_of("input"));
    let _ = writeln!(out, "  <CornerRadius x:Key=\"OverlayCornerRadius\">{}</CornerRadius>", m.radius_of("panel"));
    // The radius and weight of each part in each state.
    for kind in PART_KINDS {
        for state in [None].into_iter().chain(STATES.map(Some)) {
            let look = m.look(kind, state);
            let key = format!("Clarity{}{}", pascal(kind), state.map(pascal).unwrap_or_default());
            let _ = writeln!(out, "  <CornerRadius x:Key=\"{key}Radius\">{}</CornerRadius>", look.radius.unwrap_or(m.radius_of(kind)));
            let _ = writeln!(out, "  <FontWeight x:Key=\"{key}Weight\">{}</FontWeight>", weight(look.weight));
        }
    }
    let p = m.padding;
    let _ = writeln!(out, "  <x:Double x:Key=\"ClarityPadding\">{p}</x:Double>");
    let _ = writeln!(out, "  <Thickness x:Key=\"ButtonPadding\">{},{p}</Thickness>", p * BUTTON_SIDES);
    let _ = writeln!(out, "  <Thickness x:Key=\"TextControlThemePadding\">{},{p}</Thickness>", p * INPUT_SIDES);
    let b = m.border;
    for key in ["ClarityBorderWidth", "ButtonBorderThemeThickness", "TextControlBorderThemeThickness"] {
        let _ = writeln!(out, "  <Thickness x:Key=\"{key}\">{b}</Thickness>");
    }
    // A framed container's border and padding sit out in the space between
    // columns, 3 pixels each side of its span (Border.panel's Margin), so its
    // grid has the window's column lines, as on the web.
    let _ = writeln!(out, "  <Thickness x:Key=\"ClarityFramePadding\">{side},{v},{side},{v}</Thickness>", side = FRAME_OUT.saturating_sub(b), v = p * 2);
    let _ = writeln!(out, "  <BoxShadows x:Key=\"ClarityShadow\">{}</BoxShadows>", shadow(m.shadow));
    out
}

/// The ResourceDictionary of `theme`, merged with Clarity's own, with the
/// keys of `styles`.
pub fn theme_axaml(theme: &ThemeDecl, styles: &StyleProperties) -> String {
    let light = merged(theme, false);
    let dark = merged(theme, true);
    format!(
        "<!-- Generated by Clarity from theme {name}. Do not edit: change the theme in Clarity instead. -->
<ResourceDictionary xmlns=\"https://github.com/avaloniaui\"
                    xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">
  <ResourceDictionary.ThemeDictionaries>
    <ResourceDictionary x:Key=\"Light\">
{light_values}    </ResourceDictionary>
    <ResourceDictionary x:Key=\"Dark\">
{dark_values}    </ResourceDictionary>
  </ResourceDictionary.ThemeDictionaries>
{shared}</ResourceDictionary>
",
        name = theme.name.name,
        light_values = mode(&light, styles),
        dark_values = mode(&dark, styles),
        shared = shared(&light),
    )
}

/// The ResourceDictionary of one theme on its own (its own styles' keys).
pub fn avalonia_theme(theme: &ThemeDecl) -> String {
    theme_axaml(theme, &style_properties(&[theme]))
}

/// Where a theme's dictionary is in the project.
pub fn theme_path(theme: &ThemeDecl) -> String {
    format!("Themes/{}.axaml", theme.name.name)
}

/// The style class a part with style `name` holds: `primary`, `night-owl`.
pub fn style_class(name: &str) -> String {
    clarity_spec::model::snake_case(name).replace('_', "-")
}

/// The style classes of the application: each style's setters read the
/// values of the theme shown, in every state of a button.
pub fn style_classes(styles: &StyleProperties) -> String {
    let mut out = String::new();
    for (name, properties) in styles {
        let class = style_class(name);
        let key = format!("ClarityStyle{}", pascal(name));
        let setter = |property: &str, resource: &str| format!("      <Setter Property=\"{property}\" Value=\"{{DynamicResource {key}{resource}}}\" />\n");
        let mut templated = String::new();
        let mut text = String::new();
        let mut presenter = String::new();
        let mut content = String::new();
        for p in properties {
            match *p {
                "background" => {
                    templated.push_str(&setter("Background", "Background"));
                    text.push_str(&setter("Background", "Background"));
                    presenter.push_str(&setter("Background", "Background"));
                }
                "text" => {
                    templated.push_str(&setter("Foreground", "Text"));
                    text.push_str(&setter("Foreground", "Text"));
                    presenter.push_str(&setter("Foreground", "Text"));
                }
                "border" => {
                    templated.push_str(&setter("BorderBrush", "Border"));
                    templated.push_str(&setter("BorderThickness", "BorderThickness"));
                    presenter.push_str(&setter("BorderBrush", "Border"));
                }
                "radius" => {
                    templated.push_str(&setter("CornerRadius", "Radius"));
                    presenter.push_str(&setter("CornerRadius", "Radius"));
                }
                "weight" => {
                    templated.push_str(&setter("FontWeight", "Weight"));
                    text.push_str(&setter("FontWeight", "Weight"));
                }
                "size" => {
                    templated.push_str(&setter("FontSize", "Size"));
                    text.push_str(&setter("FontSize", "Size"));
                }
                "font" => {
                    templated.push_str(&setter("FontFamily", "Font"));
                    text.push_str(&setter("FontFamily", "Font"));
                }
                _ => {
                    content.push_str(&setter("HorizontalContentAlignment", "Content"));
                    text.push_str(&setter("TextAlignment", "Align"));
                    text.push_str(&setter("Padding", "Padding"));
                }
            }
        }
        let _ = write!(out, "    <!-- Style {name}: its values come from the theme shown. -->\n    <Style Selector=\":is(TemplatedControl).{class}\">\n{templated}    </Style>\n");
        if !text.is_empty() {
            let _ = write!(out, "    <Style Selector=\"TextBlock.{class}\">\n{text}    </Style>\n");
        }
        if !content.is_empty() {
            let _ = write!(out, "    <Style Selector=\":is(ContentControl).{class}\">\n{content}    </Style>\n");
        }
        if !presenter.is_empty() {
            for state in ["pointerover", "pressed"] {
                let _ = write!(
                    out,
                    "    <Style Selector=\"Button.{class}:{state} /template/ ContentPresenter#PART_ContentPresenter\">\n{presenter}    </Style>\n"
                );
            }
        }
    }
    out
}

/// The themes the application offers, for the runtime's `Themes`.
pub fn app_themes(themes: &[&ThemeDecl], namespace: &str) -> String {
    let mut entries = String::new();
    for theme in themes {
        let variant = match theme.mode {
            Some(ThemeMode::Light) => "ThemeVariant.Light",
            Some(ThemeMode::Dark) => "ThemeVariant.Dark",
            _ => "ThemeVariant.Default",
        };
        let _ = writeln!(
            entries,
            "        new({}, {}, {variant}, () => (ResourceDictionary)AvaloniaXamlLoader.Load(new Uri({}))),",
            string(&theme.name.name),
            // A keyed title by its key, which the theme picker translates as it opens.
            super::texts::Texts::kept(&theme.label),
            string(&format!("avares://{namespace}/{}", theme_path(theme)))
        );
    }
    format!(
        "{HEADER}
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using Avalonia.Styling;
using Clarity.Avalonia;

namespace {namespace};

public static class AppThemes
{{
    /// <summary>The themes the application offers, the one it starts with first.</summary>
    public static readonly Theme[] Offered =
    [
{entries}    ];
}}
"
    )
}

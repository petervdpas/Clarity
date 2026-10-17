//! Themes (T1): named colors per light and dark mode, font, size, radius
//! and named styles; parts pick a style by name.

use clarity_spec::model::ThemeMode;
use clarity_spec::{Project, Severity, load_sources, themes};

const THEME: &str = "theme Shop
    mode system
    color accent \"#2f6fdf\" dark \"#5b8cff\"
    color brand \"#f5c400\"
    font \"Inter\"
    size 14
    radius 6
    style primary background accent, text \"#ffffff\"
    style quiet border none
    style loud background brand, text text, border accent
";

const WINDOW: &str = "page Card: window
    button \"Save\" name save at 1,1 span 2 style primary
    label \"Hello\" at 1,2 span 3
";

fn project(text: &str, theme: Option<&str>) -> Project {
    let source = format!("module shop\n\n{text}");
    let mut project = load_sources([("shop.clarity", source.as_str())]);
    themes::use_theme(&mut project, theme);
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_theme_has_colors_a_font_and_styles() {
    let project = project(&format!("{THEME}\n{WINDOW}"), Some("Shop"));
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let theme = &project.files[0].themes[0];
    assert_eq!(theme.name.name, "Shop");
    assert_eq!(theme.mode, Some(ThemeMode::System));
    assert_eq!(theme.colors[0].name.name, "accent");
    assert_eq!(theme.colors[0].light, "#2f6fdf");
    assert_eq!(theme.colors[0].dark.as_deref(), Some("#5b8cff"));
    assert_eq!((theme.font.as_deref(), theme.size, theme.radius), (Some("Inter"), Some(14), Some(6)));
    let primary = &theme.styles[0];
    assert_eq!(primary.background.as_deref(), Some("accent"));
    assert_eq!(primary.text.as_deref(), Some("#ffffff"));
    assert_eq!(theme.styles[1].border.as_deref(), Some("none"));
    // The project knows its theme, for the renderer.
    assert_eq!(project.theme.as_deref(), Some("Shop"));
    assert_eq!(project.files[0].pages[0].widgets[0].props.style.as_deref(), Some("primary"));
}

#[test]
fn colors_and_styles_are_checked() {
    let bad = THEME
        .replace("\"#f5c400\"", "\"yellow\"")
        .replace("style quiet border none", "style quiet border none, shadow big")
        .replace("text text, border accent", "text nothing");
    assert_eq!(errors(&project(&bad, Some("Shop"))), [
        "a style sets background, text and border, not 'shadow'",
        "'yellow' is not a color; write it as \"#rrggbb\" or \"#rgb\"",
        "style 'loud' uses color 'nothing', which theme Shop does not have; its colors are background, panel, text, muted, border, accent, error, warning and brand",
    ]);
    let twice = THEME.replace("    color brand", "    color accent \"#000000\"\n    color brand");
    assert_eq!(errors(&project(&twice, Some("Shop"))), ["theme Shop has color 'accent' twice"]);
    assert_eq!(errors(&project(&THEME.replace("mode system", "mode bright"), Some("Shop"))), [
        "unknown mode 'bright'; a theme is light, dark or system"
    ]);
}

#[test]
fn a_part_picks_a_style_of_the_projects_theme() {
    let unknown = WINDOW.replace("style primary", "style shiny");
    assert_eq!(errors(&project(&format!("{THEME}\n{unknown}"), Some("Shop"))), [
        "theme Shop has no style 'shiny'; its styles are primary, quiet and loud"
    ]);
    assert_eq!(errors(&project(&format!("{THEME}\n{WINDOW}"), None)), [
        "style 'primary' comes from the project's theme; name it in the project file, like: theme Shop"
    ]);
    assert_eq!(errors(&project(WINDOW, Some("Plain"))), ["the project file names theme Plain, which no module declares"]);
}

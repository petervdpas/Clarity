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

fn project(text: &str, offered: &[&str]) -> Project {
    let source = format!("module shop\n\n{text}");
    let mut project = load_sources([("shop.clarity", source.as_str())]);
    themes::use_themes(&mut project, offered);
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_theme_has_colors_a_font_and_styles() {
    let project = project(&format!("{THEME}\n{WINDOW}"), &["Shop"]);
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
    assert_eq!(project.themes, ["Shop"]);
    assert_eq!(project.files[0].pages[0].widgets[0].props.style.as_deref(), Some("primary"));
}

#[test]
fn colors_and_styles_are_checked() {
    let bad = THEME
        .replace("\"#f5c400\"", "\"yellow\"")
        .replace("style quiet border none", "style quiet border none, shadow big")
        .replace("text text, border accent", "text nothing");
    assert_eq!(errors(&project(&bad, &["Shop"])), [
        "a style sets background, text and border, not 'shadow'",
        "'yellow' is not a color; write it as \"#rrggbb\" or \"#rgb\"",
        "style 'loud' uses color 'nothing', which theme Shop does not have; its colors are background, panel, text, muted, border, accent, error, warning and brand",
    ]);
    let twice = THEME.replace("    color brand", "    color accent \"#000000\"\n    color brand");
    assert_eq!(errors(&project(&twice, &["Shop"])), ["theme Shop has color 'accent' twice"]);
    assert_eq!(errors(&project(&THEME.replace("mode system", "mode bright"), &["Shop"])), [
        "unknown mode 'bright'; a theme is light, dark or system"
    ]);
}

#[test]
fn a_part_picks_a_style_of_the_projects_theme() {
    let unknown = WINDOW.replace("style primary", "style shiny");
    assert_eq!(errors(&project(&format!("{THEME}\n{unknown}"), &["Shop"])), [
        "theme Shop has no style 'shiny'; its styles are primary, quiet and loud"
    ]);
    assert_eq!(errors(&project(&format!("{THEME}\n{WINDOW}"), &[])), [
        "style 'primary' comes from the project's theme; name it in the project file, like: theme Shop"
    ]);
    assert_eq!(errors(&project(WINDOW, &["Plain"])), ["the project file names theme Plain, which no module declares"]);
}

/// A theme as the conformance cases write it, as the parser would give it.
fn theme_of(input: &serde_json::Value) -> clarity_spec::model::ThemeDecl {
    use clarity_spec::model::{Ident, ThemeColor, ThemeDecl, ThemeStyle};
    let ident = |v: &serde_json::Value| Ident { name: v.as_str().unwrap().to_string(), span: Default::default() };
    let text = |v: &serde_json::Value| v.as_str().map(str::to_string);
    ThemeDecl {
        name: Ident { name: "Case".into(), span: Default::default() },
        class: "theme-case".into(),
        label: "Case".into(),
        title: None,
        mode: match input["mode"].as_str() {
            Some("light") => Some(ThemeMode::Light),
            Some("dark") => Some(ThemeMode::Dark),
            Some("system") => Some(ThemeMode::System),
            _ => None,
        },
        colors: input["colors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| ThemeColor { name: ident(&c["name"]), light: text(&c["light"]).unwrap(), dark: text(&c["dark"]), span: Default::default() })
            .collect(),
        font: text(&input["font"]),
        size: input["size"].as_u64().map(|n| n as u32),
        radius: input["radius"].as_u64().map(|n| n as u32),
        styles: input["styles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| ThemeStyle {
                class: format!("style-{}", s["name"].as_str().unwrap()),
                name: ident(&s["name"]),
                background: text(&s["background"]),
                text: text(&s["text"]),
                border: text(&s["border"]),
                span: Default::default(),
            })
            .collect(),
        span: Default::default(),
    }
}

#[test]
fn a_theme_resolves_as_the_conformance_cases_say() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../conformance/themes.json")).unwrap();
    for case in cases["resolves"].as_array().unwrap() {
        let resolved = themes::resolve(&theme_of(&case["theme"]), case["dark"].as_bool().unwrap());
        assert_eq!(serde_json::to_value(&resolved).unwrap(), case["gives"], "{}", case["what"]);
    }
}

#[test]
fn a_theme_and_its_styles_carry_their_class_names() {
    let project = project(&format!("{THEME}\ntheme NightOwl\n    mode dark\n"), &[]);
    let themes = &project.files[0].themes;
    assert_eq!(themes[0].class, "theme-shop");
    assert_eq!(themes[0].styles[0].class, "style-primary");
    assert_eq!(themes[1].class, "theme-night-owl");
}

const NIGHT: &str = "theme NightOwl
    mode dark
    color accent \"#5b8cff\"
    style primary background accent
    style quiet border none
    style loud background accent
";

#[test]
fn an_application_can_offer_several_themes() {
    let both = project(&format!("{THEME}\n{NIGHT}\n{WINDOW}"), &["Shop", "NightOwl"]);
    assert!(both.diagnostics.is_empty(), "{:#?}", both.diagnostics);
    assert_eq!(both.themes, ["Shop", "NightOwl"]);
    // A theme is named for people by its title, or by its name.
    let themes = &both.files[0].themes;
    assert_eq!(themes[0].label, "Shop");
    assert_eq!(themes[1].label, "Night owl");
    let titled = project(&format!("{THEME}\n{}", NIGHT.replace("    mode dark\n", "    mode dark\n    title \"Midnight\"\n")), &[]);
    assert_eq!(titled.files[0].themes[1].label, "Midnight");
}

#[test]
fn a_style_must_be_in_every_offered_theme() {
    let without = NIGHT.replace("    style primary background accent\n", "");
    assert_eq!(errors(&project(&format!("{THEME}\n{without}\n{WINDOW}"), &["Shop", "NightOwl"])), [
        "theme NightOwl has no style 'primary'; its styles are quiet and loud"
    ]);
    // A theme the application does not offer is not held to it.
    assert!(errors(&project(&format!("{THEME}\n{without}\n{WINDOW}"), &["Shop"])).is_empty());
}

#[test]
fn theme_names_are_texts_to_translate() {
    let project = project(&format!("{THEME}\n{NIGHT}"), &["Shop", "NightOwl"]);
    let texts: Vec<String> = clarity_spec::translations::texts(&project).into_iter().map(|u| u.text).collect();
    assert!(texts.contains(&"Night owl".to_string()), "{texts:?}");
}

#[test]
fn a_menu_item_can_switch_the_theme() {
    let app = "app Shop\n    main Card\n    menu\n        item \"Theme\" on click -> theme\n";
    let project = project(&format!("{THEME}\n{WINDOW}\n{app}"), &["Shop"]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(clarity_spec::Preview::new(project).commands.contains(&"theme".to_string()));
}

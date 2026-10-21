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
    label \"Hello\" name label1 at 1,2 span 3
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
    assert_eq!((theme.fonts[0].value.as_str(), theme.sizes[0].value.as_str(), theme.radii[0].value.as_str()), ("Inter", "14", "6"));
    let primary = &theme.styles[0];
    assert_eq!(primary.look.background.as_deref(), Some("accent"));
    assert_eq!(primary.look.text.as_deref(), Some("#ffffff"));
    assert_eq!(theme.styles[1].look.border.as_deref(), Some("none"));
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
        "a style sets background, text, border, radius, weight, size, font and align, not 'shadow'",
        "'yellow' is not a color; write it as \"#rrggbb\" or \"#rgb\"",
        "style 'loud' uses color 'nothing', which theme Shop does not have; its colors are background, panel, raised, text, muted, border, strong_border, accent, accent_soft, on_accent, selection, focus, input, frame, frame_text, error, error_soft, warning, warning_soft, success, success_soft and brand",
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

#[test]
fn a_theme_resolves_as_the_conformance_cases_say() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../conformance/themes.json")).unwrap();
    for case in cases["resolves"].as_array().unwrap() {
        let source = format!("module case\n\n{}", case["theme"]["source"].as_str().unwrap());
        let (file, diagnostics) = clarity_spec::parser::parse(&source, "case.clarity");
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let resolved = themes::resolve(&file.themes[0], case["dark"].as_bool().unwrap());
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

/// An offered theme's name is read by people, so its title is a key.
#[test]
fn an_offered_themes_title_is_a_key() {
    let titled = NIGHT.replace("    mode dark\n", "    mode dark\n    title @nightOwlTheme\n");
    let project = project(&format!("{THEME}\n{titled}"), &["Shop", "NightOwl"]);
    let keys: Vec<String> = clarity_spec::translations::keys(&project).into_iter().map(|u| u.key).collect();
    assert!(keys.contains(&"nightOwlTheme".to_string()), "{keys:?}");
}

#[test]
fn a_menu_item_can_switch_the_theme() {
    let app = "app Shop\n    main Card\n    menu name mainMenu\n        item \"Theme\" name themeItem on click -> theme\n";
    let project = project(&format!("{THEME}\n{WINDOW}\n{app}"), &["Shop"]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(clarity_spec::Preview::new(project).commands.contains(&"theme".to_string()));
}

const DEEP: &str = "theme Deep
    color raised \"#f6f7f9\" dark \"#232832\"
    color on_accent \"#ffffff\"
    font \"Inter\"
    font heading \"Georgia\"
    font mono \"JetBrains Mono\"
    size 14
    size small 12
    size heading 18
    radius 6
    radius input 4
    padding 5
    border 2
    shadow raised
    part button background raised, border strong_border, radius 6
    part button hover background accent_soft
    part input focus border accent
    style primary background accent, text on_accent, weight bold, radius 10
";

#[test]
fn a_theme_sets_fonts_sizes_radii_spacing_and_part_looks() {
    let project = project(DEEP, &["Deep"]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let theme = &project.files[0].themes[0];
    let pairs = |list: &[clarity_spec::model::ThemeValue]| list.iter().map(|v| (v.role.clone(), v.value.clone())).collect::<Vec<_>>();
    assert_eq!(pairs(&theme.fonts), [("text".into(), "Inter".into()), ("heading".into(), "Georgia".into()), ("mono".into(), "JetBrains Mono".into())]);
    assert_eq!(pairs(&theme.sizes), [("text".into(), "14".into()), ("small".into(), "12".into()), ("heading".into(), "18".into())]);
    assert_eq!(pairs(&theme.radii), [("all".into(), "6".into()), ("input".into(), "4".into())]);
    assert_eq!((theme.padding, theme.border, theme.shadow), (Some(5), Some(2), Some(clarity_spec::model::Shadow::Raised)));
    let button = &theme.parts[0];
    assert_eq!((button.kind.as_str(), button.state.as_deref()), ("button", None));
    assert_eq!(button.look.background.as_deref(), Some("raised"));
    assert_eq!(button.look.radius, Some(6));
    assert_eq!(theme.parts[1].state.as_deref(), Some("hover"));
    assert_eq!(theme.styles[0].look.weight, Some(clarity_spec::model::Weight::Bold));
    assert_eq!(theme.styles[0].look.radius, Some(10));
}

#[test]
fn deeper_theme_lines_are_checked() {
    let bad = DEEP
        .replace("font heading", "font fancy")
        .replace("size small 12", "size huge 40")
        .replace("radius input 4", "radius knob 4")
        .replace("shadow raised", "shadow deep")
        .replace("part input focus", "part slider focus")
        .replace("part button hover", "part button wobble");
    assert_eq!(errors(&project(&bad, &["Deep"])), [
        "unknown font 'fancy'; a theme sets the fonts text, heading and mono",
        "unknown size 'huge'; a theme sets the sizes text, small and heading",
        "unknown radius 'knob'; a theme sets the radius of input, button and panel, or of all with 'radius n'",
        "unknown shadow 'deep'; a shadow is none, flat or raised",
        "unknown state 'wobble'; a part looks different on hover, pressed or focus",
        "a theme sets the look of the parts button, input, header, tab and menu, not 'slider'",
    ]);
}

#[test]
fn clarity_s_own_theme_sets_every_value() {
    let (file, diagnostics) = clarity_spec::parser::parse(themes::BUILT_IN, "clarity.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let checked = load_sources([("clarity.clarity", themes::BUILT_IN)]);
    assert!(checked.diagnostics.is_empty(), "{:#?}", checked.diagnostics);
    let theme = themes::built_in();
    assert_eq!(theme.name.name, themes::BUILT_IN_NAME);
    assert_eq!(file.themes[0], theme);
    for role in themes::ROLES {
        let color = theme.colors.iter().find(|c| c.name.name == role).unwrap_or_else(|| panic!("no color {role}"));
        assert!(color.dark.is_some(), "color {role} has no dark value");
    }
    let has = |values: &[clarity_spec::model::ThemeValue], role: &str| values.iter().any(|v| v.role == role);
    for role in themes::FONTS {
        assert!(has(&theme.fonts, role), "no font {role}");
    }
    for role in themes::SIZES {
        assert!(has(&theme.sizes, role), "no size {role}");
    }
    for role in themes::RADII {
        assert!(has(&theme.radii, role), "no radius {role}");
    }
    assert!(theme.padding.is_some() && theme.border.is_some() && theme.shadow.is_some());
    for kind in themes::PART_KINDS {
        assert!(theme.parts.iter().any(|p| p.kind == kind && p.state.is_none()), "no look for part {kind}");
    }
}

#[test]
fn a_new_theme_starts_as_a_copy_of_clarity_s_own() {
    let text = themes::starter("Ocean");
    assert!(text.starts_with("theme Ocean\n    title \"Ocean\"\n    mode system\n"), "{text}");
    assert!(!text.contains("module"), "{text}");
    let source = format!("module shop\n\n{text}");
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let ocean = &project.files[0].themes[0];
    let own = themes::built_in();
    assert_eq!((ocean.colors.len(), ocean.parts.len()), (own.colors.len(), own.parts.len()));
    // The designer shows Clarity's own theme too.
    let preview = clarity_spec::Preview::new(project);
    assert_eq!(preview.base_theme.name.name, "Clarity");
}

/// A style sets how its text shows too: its size, a font of the theme and
/// where it sits, as a calculator's display: large, monospace, on the right.
#[test]
fn a_style_sets_the_size_font_and_alignment_of_its_text() {
    use clarity_spec::model::Align;
    let theme = "theme Calc
    style display background \"#c9d8b0\", size 36, font mono, align right
";
    let project = project(theme, &["Calc"]);
    assert!(errors(&project).is_empty(), "{:#?}", errors(&project));
    let decl = &project.files[0].themes[0];
    let look = &decl.styles[0].look;
    assert_eq!((look.size, look.font.as_deref(), look.align), (Some(36), Some("mono"), Some(Align::Right)));
    let resolved = themes::resolve(decl, false);
    let display = &resolved.styles["display"];
    assert_eq!((display.size, display.font.as_deref(), display.align), (Some(36), Some("mono"), Some(Align::Right)));

    let bad = |line: &str| errors(&project_of(&format!("theme Calc\n    {line}\n")));
    assert_eq!(bad("style display font fancy"), ["unknown font 'fancy'; a style takes a font of the theme: text, heading or mono"]);
    assert_eq!(bad("style display align middle"), ["unknown alignment 'middle'; text aligns left, center or right"]);
    assert_eq!(bad("style display size 0"), ["a text size is 6 to 200 pixels"]);
    // A part keeps to its kind's own text; a style sets it.
    assert_eq!(bad("part button size 20"), ["a part sets background, text, border, radius and weight; size, font and align belong in a style"]);
}

fn project_of(text: &str) -> Project {
    project(text, &["Calc"])
}

/// A new theme can start from each of Clarity's own looks: light and dark
/// as the system is, light, dark, or cream and green.
#[test]
fn a_new_theme_starts_from_a_look_of_its_choice() {
    use themes::ThemeStart;
    let theme = |start| {
        let source = format!("module shop\n\n{}", themes::starter_from("Ocean", start));
        let project = load_sources([("shop.clarity", source.as_str())]);
        assert!(project.diagnostics.is_empty(), "{start:?}: {:#?}", project.diagnostics);
        project.files[0].themes[0].clone()
    };
    let own = themes::built_in();
    let color = |t: &clarity_spec::model::ThemeDecl, name: &str| t.colors.iter().find(|c| c.name.name == name).cloned().unwrap();

    let system = theme(ThemeStart::System);
    assert_eq!(system.mode, Some(ThemeMode::System));
    assert_eq!((color(&system, "panel").light, color(&system, "panel").dark), (color(&own, "panel").light, color(&own, "panel").dark));

    let light = theme(ThemeStart::Light);
    assert_eq!(light.mode, Some(ThemeMode::Light));
    assert_eq!(color(&light, "panel").light, color(&own, "panel").light);
    assert!(light.colors.iter().all(|c| c.dark.is_none()));

    let dark = theme(ThemeStart::Dark);
    assert_eq!(dark.mode, Some(ThemeMode::Dark));
    assert_eq!(Some(color(&dark, "panel").light), color(&own, "panel").dark);

    let cream = theme(ThemeStart::CreamGreen);
    assert_eq!(cream.mode, Some(ThemeMode::Light));
    assert_eq!(color(&cream, "panel").light, "#fbf8ee");
    assert_eq!(color(&cream, "accent").light, "#2e6b3a");
    // Every color of Clarity's own has its cream and green value.
    assert_eq!(cream.colors.len(), own.colors.len());
    assert!(cream.colors.iter().all(|c| c.dark.is_none()));
}

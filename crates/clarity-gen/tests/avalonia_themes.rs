//! Themes on Avalonia (T1 for G2c, DESIGN 1.5): the generator merges
//! Clarity's own theme with each offered one (Avalonia has no cascade) into
//! a ResourceDictionary with Light and Dark theme dictionaries; parts take
//! their looks through the Fluent theme's per-state keys, and styles are
//! style classes. Held to the same cases as every stack: conformance/themes.json.

use clarity_gen::{OutFile, avalonia_theme, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::themes::{built_in, resolve, use_themes};
use clarity_spec::{load_sources, parser};

/// The theme dictionary for `mode` (`Light` or `Dark`).
fn section<'a>(axaml: &'a str, mode: &str) -> &'a str {
    let start = axaml.find(&format!("<ResourceDictionary x:Key=\"{mode}\">")).unwrap_or_else(|| panic!("no {mode} in\n{axaml}"));
    let rest = &axaml[start..];
    &rest[..rest.find("</ResourceDictionary>").unwrap()]
}

fn pascal(name: &str) -> String {
    name.split('_').map(|w| w[..1].to_uppercase() + &w[1..]).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

/// The Fluent key of a part's look: `button:hover` background is `ButtonBackgroundPointerOver`.
fn fluent(part: &str, property: &str) -> String {
    let (kind, state) = part.split_once(':').unwrap_or((part, ""));
    let control = match kind {
        "button" => "Button",
        "input" => "TextControl",
        other => panic!("no Fluent keys for {other} in this test"),
    };
    let property = match property {
        "background" => "Background",
        "text" => "Foreground",
        _ => "BorderBrush",
    };
    let state = match state {
        "hover" => "PointerOver",
        "pressed" => "Pressed",
        "focus" => "Focused",
        _ => "",
    };
    format!("{control}{property}{state}")
}

#[test]
fn a_theme_resolves_on_avalonia_as_the_conformance_cases_say() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/themes.json")).unwrap();
    for case in cases["resolves"].as_array().unwrap() {
        let what = case["what"].as_str().unwrap();
        let source = format!("module case\n\n{}", case["theme"]["source"].as_str().unwrap());
        let (file, diagnostics) = parser::parse(&source, "case.clarity");
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let axaml = avalonia_theme(&file.themes[0]);
        let gives = &case["gives"];
        let dark = gives["dark"].as_bool().unwrap();
        let mode = section(&axaml, if dark { "Dark" } else { "Light" });
        // What the theme leaves out is Clarity's own, in the same mode.
        let base = resolve(&built_in(), dark);
        let color = |v: &str| v.strip_prefix('@').map_or_else(|| v.to_string(), |role| base.colors[role].clone());
        let brush = |key: &str, v: &str| format!("<SolidColorBrush x:Key=\"{key}\" Color=\"{}\" />", color(v));

        for (role, hex) in gives["colors"].as_object().unwrap() {
            assert!(mode.contains(&brush(&format!("Clarity{}", pascal(role)), hex.as_str().unwrap())), "{what}: color {role}\n{mode}");
        }
        for (part, look) in gives["parts"].as_object().unwrap() {
            for property in ["background", "text", "border"] {
                if let Some(v) = look[property].as_str() {
                    assert!(mode.contains(&brush(&fluent(part, property), v)), "{what}: {part} {property}\n{mode}");
                }
            }
            if let Some(radius) = look["radius"].as_u64() {
                let key = format!("Clarity{}Radius", part.split(':').map(pascal).collect::<String>());
                has(&axaml, &format!("<CornerRadius x:Key=\"{key}\">{radius}</CornerRadius>"));
            }
        }
        for (name, look) in gives["styles"].as_object().unwrap() {
            let key = format!("ClarityStyle{}", pascal(name));
            for (property, suffix) in [("background", "Background"), ("text", "Text"), ("border", "Border")] {
                match look[property].as_str() {
                    Some("none") => has(mode, &format!("<Thickness x:Key=\"{key}BorderThickness\">0</Thickness>")),
                    Some(v) => assert!(mode.contains(&brush(&format!("{key}{suffix}"), v)), "{what}: style {name} {property}\n{mode}"),
                    None => {}
                }
            }
            if look["weight"].as_str() == Some("bold") {
                has(mode, &format!("<FontWeight x:Key=\"{key}Weight\">SemiBold</FontWeight>"));
            }
            // How its text shows: a size, a font of the theme, where it sits.
            if let Some(size) = look["size"].as_u64() {
                has(mode, &format!("<x:Double x:Key=\"{key}Size\">{size}</x:Double>"));
            }
            if look["font"].as_str().is_some() {
                has(mode, &format!("<FontFamily x:Key=\"{key}Font\">"));
            }
            if let Some(align) = look["align"].as_str() {
                has(mode, &format!("<TextAlignment x:Key=\"{key}Align\">{}</TextAlignment>", pascal(align)));
                has(mode, &format!("<HorizontalAlignment x:Key=\"{key}Content\">{}</HorizontalAlignment>", pascal(align)));
            }
        }
        if let Some(font) = gives["fonts"]["text"].as_str() {
            has(&axaml, &format!("<FontFamily x:Key=\"ClarityFont\">{font}, $Default</FontFamily>"));
        }
        if let Some(size) = gives["sizes"]["text"].as_u64() {
            has(&axaml, &format!("<x:Double x:Key=\"ClarityFontSize\">{size}</x:Double>"));
            has(&axaml, &format!("<x:Double x:Key=\"ControlContentThemeFontSize\">{size}</x:Double>"));
        }
        if let Some(size) = gives["sizes"]["small"].as_u64() {
            has(&axaml, &format!("<x:Double x:Key=\"ClaritySizeSmall\">{size}</x:Double>"));
        }
        for (kind, radius) in gives["radii"].as_object().unwrap() {
            let kinds: Vec<&str> = if kind == "all" { vec!["input", "button", "panel"] } else { vec![kind.as_str()] };
            for k in kinds {
                has(&axaml, &format!("<CornerRadius x:Key=\"ClarityRadius{}\">{radius}</CornerRadius>", pascal(k)));
            }
        }
        if let Some(padding) = gives["padding"].as_u64() {
            // Beside a button's text three times the padding, beside an input's two (clarity_spec::layout).
            has(&axaml, &format!("<Thickness x:Key=\"ButtonPadding\">{},{padding}</Thickness>", padding * 3));
            has(&axaml, &format!("<Thickness x:Key=\"TextControlThemePadding\">{},{padding}</Thickness>", padding * 2));
        }
        if let Some(border) = gives["border"].as_u64() {
            has(&axaml, &format!("<Thickness x:Key=\"ClarityBorderWidth\">{border}</Thickness>"));
        }
        if gives["shadow"].as_str() == Some("flat") {
            has(&axaml, "<BoxShadows x:Key=\"ClarityShadow\">0 1 2 0 #26000000</BoxShadows>");
        }
    }
}

#[test]
fn every_theme_holds_every_value_so_nothing_falls_back_to_fluent() {
    let (file, _) = parser::parse("module case\n\ntheme Bare\n", "case.clarity");
    let axaml = avalonia_theme(&file.themes[0]);
    for (mode, dark) in [("Light", false), ("Dark", true)] {
        let base = resolve(&built_in(), dark);
        let section = section(&axaml, mode);
        for (role, hex) in &base.colors {
            has(section, &format!("<SolidColorBrush x:Key=\"Clarity{}\" Color=\"{hex}\" />", pascal(role)));
        }
        has(section, &format!("<Color x:Key=\"SystemAccentColor\">{}</Color>", base.colors["accent"]));
        has(section, &format!("<SolidColorBrush x:Key=\"ButtonBorderBrushPointerOver\" Color=\"{}\" />", base.colors["accent"]));
        has(section, &format!("<SolidColorBrush x:Key=\"TextControlBorderBrushFocused\" Color=\"{}\" />", base.colors["focus"]));
    }
    has(&axaml, "<CornerRadius x:Key=\"ClarityRadiusButton\">6</CornerRadius>");
    has(&axaml, "<x:Double x:Key=\"ClaritySizeHeading\">18</x:Double>");
    has(&axaml, "<Thickness x:Key=\"ButtonPadding\">12,4</Thickness>");
}

const SHOP: &str = "module shop

entity Customer
    name: text(80)

app Shop
    main Card
    menu name mainMenu
        item \"Theme\" name themeItem on click -> theme

page Card: window
    data customer: Customer
    button \"Save\" name save at 1,1 span 2 style primary

theme Shop
    color accent \"#1f6f5c\" dark \"#4fb393\"
    style primary background accent, text \"#fff\", weight bold

theme NightOwl
    mode dark
    title \"Night owl\"
    style primary background accent
";

fn settings() -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    settings
}

fn generated(offered: &[&str]) -> Vec<OutFile> {
    // A part's style comes from a theme; without one, parts have none.
    let source = if offered.is_empty() { SHOP.replace(" style primary\n", "\n") } else { SHOP.to_string() };
    let mut project = load_sources([("shop.clarity", source.as_str())]);
    use_themes(&mut project, offered);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

#[test]
fn the_application_offers_its_themes_by_title_and_mode() {
    let files = generated(&["Shop", "NightOwl"]);
    assert!(file(&files, "Themes/Shop.axaml").starts_with("<!-- Generated by Clarity from theme Shop."));
    assert!(files.iter().any(|f| f.path == "Themes/NightOwl.axaml"));
    let offered = file(&files, "AppThemes.cs");
    // Each theme is a class of its own, compiled from its file, so a trimmed
    // program keeps it without loading it by its address.
    has(offered, "new(\"Shop\", \"Shop\", ThemeVariant.Default, () => new ShopTheme()),");
    has(offered, "new(\"NightOwl\", \"Night owl\", ThemeVariant.Dark, () => new NightOwlTheme()),");
    assert!(!offered.contains("AvaloniaXamlLoader"), "{offered}");
    has(file(&files, "Themes/Shop.axaml"), "<ResourceDictionary x:Class=\"Shop.ShopTheme\"");
    let code = file(&files, "Themes/Shop.axaml.cs");
    has(code, "namespace Shop;");
    has(code, "public partial class ShopTheme : ResourceDictionary");
    has(code, "public ShopTheme() => AvaloniaXamlLoader.Load(this);");
    // Only offered themes ship.
    let one = generated(&["Shop"]);
    assert!(!one.iter().any(|f| f.path == "Themes/NightOwl.axaml"));
}

#[test]
fn without_a_theme_the_application_looks_as_clarity_does() {
    let files = generated(&[]);
    assert!(file(&files, "Themes/Clarity.axaml").starts_with("<!-- Generated by Clarity from theme Clarity."));
    has(file(&files, "AppThemes.cs"), "new(\"Clarity\", \"Clarity\", ThemeVariant.Default,");
}

#[test]
fn a_style_is_a_style_class_whose_values_come_from_the_theme() {
    let files = generated(&["Shop", "NightOwl"]);
    has(file(&files, "Windows/CardView.axaml"), "Classes=\"primary\"");
    let app = file(&files, "App.axaml");
    has(app, "<Style Selector=\":is(TemplatedControl).primary\">");
    has(app, "<Setter Property=\"Background\" Value=\"{DynamicResource ClarityStylePrimaryBackground}\" />");
    has(app, "<Setter Property=\"FontWeight\" Value=\"{DynamicResource ClarityStylePrimaryWeight}\" />");
    // The style holds in every state, over Fluent's own looks.
    has(app, "<Style Selector=\"Button.primary:pointerover /template/ ContentPresenter#PART_ContentPresenter\">");
    // A theme that leaves a value of a style out shows the part's own look.
    let night = file(&files, "Themes/NightOwl.axaml");
    has(night, "<FontWeight x:Key=\"ClarityStylePrimaryWeight\">Normal</FontWeight>");
    // Parts read the theme's values.
    has(app, "Value=\"{DynamicResource ClarityBackground}\"");
}

/// Avalonia refuses a dictionary with a key twice, at start.
#[test]
fn every_key_is_in_a_dictionary_once() {
    let (file, _) = parser::parse("module case\n\ntheme Bare\n    part header hover background accent\n", "case.clarity");
    let axaml = avalonia_theme(&file.themes[0]);
    let light = section(&axaml, "Light");
    let dark = section(&axaml, "Dark");
    let shared = &axaml[axaml.find("</ResourceDictionary.ThemeDictionaries>").unwrap()..];
    for part in [light, dark, shared] {
        let mut seen = std::collections::BTreeSet::new();
        for key in part.split("x:Key=\"").skip(1).map(|k| &k[..k.find('"').unwrap()]) {
            assert!(seen.insert(key), "{key} twice in\n{part}");
        }
    }
}

/// A style's text: its size, font and alignment reach a text part and a button alike.
#[test]
fn a_style_sets_how_its_text_shows() {
    let source = SHOP.replace("style primary background accent, text \"#fff\", weight bold", "style primary background accent, text \"#fff\", weight bold, size 36, font mono, align right");
    let mut project = load_sources([("shop.clarity", source.as_str())]);
    use_themes(&mut project, &["Shop", "NightOwl"]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let app = file(&files, "App.axaml");
    for (property, resource) in [("FontSize", "Size"), ("FontFamily", "Font"), ("TextAlignment", "Align"), ("Padding", "Padding")] {
        has(app, &format!("<Setter Property=\"{property}\" Value=\"{{DynamicResource ClarityStylePrimary{resource}}}\" />"));
    }
    has(app, "<Style Selector=\":is(ContentControl).primary\">\n      <Setter Property=\"HorizontalContentAlignment\" Value=\"{DynamicResource ClarityStylePrimaryContent}\" />");
    // A theme that leaves them out shows the text as a part does.
    let night = file(&files, "Themes/NightOwl.axaml");
    has(night, "<TextAlignment x:Key=\"ClarityStylePrimaryAlign\">Left</TextAlignment>");
    has(night, "<HorizontalAlignment x:Key=\"ClarityStylePrimaryContent\">Center</HorizontalAlignment>");
}

#[test]
fn a_theme_title_shows_in_the_language_shown() {
    // A keyed title is kept by its key, which the theme picker translates as it opens.
    let mut project = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
    use_themes(&mut project, &["Shop", "NightOwl"]);
    let night = project.files[0].themes.iter().find(|t| t.name.name == "NightOwl").unwrap();
    let key = night.label.strip_prefix('@').unwrap_or_else(|| panic!("the title is a key: {}", night.label)).to_string();
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let offered = file(&files, "AppThemes.cs");
    has(offered, &format!("new(\"NightOwl\", \"{key}\", ThemeVariant.Dark,"));
    assert!(!offered.contains("\"@"), "no key is shown as written:\n{offered}");
    has(file(&files, "Texts.cs"), &format!("Locale.Translate(\"{key}\")"));
}

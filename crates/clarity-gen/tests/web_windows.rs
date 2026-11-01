//! The windows of a web stack as markup (DESIGN 7.8): each part an element
//! with what differs from its kind's defaults, written in Clarity's own
//! words, in an XML file the screen Clarity ships reads.

use std::path::Path;

fn calculator() -> clarity_spec::Project {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/calculator");
    clarity_project::load_modules(&dir).unwrap()
}

#[test]
fn the_calculator_s_window_is_markup_in_clarity_s_words() {
    let project = calculator();
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Main").unwrap();
    let markup = clarity_gen::web_windows::window_markup(page);
    for line in [
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<Window name=\"Main\">",
        "  <Display name=\"screen\" at=\"1,1\" span=\"4\" rows=\"2\" bind=\"shown\" digits=\"12\" style=\"display\" />",
        "  <Button name=\"clearKey\" at=\"1,3\" span=\"1\" caption=\"@clearKeyCaption\" key=\"Escape\" onclick=\"clear\" />",
        "  <Button name=\"digit7\" at=\"1,4\" span=\"1\" caption=\"7\" key=\"7\" onclick=\"digit(7)\" />",
        "  <Button name=\"equalsKey\" at=\"4,7\" span=\"1\" caption=\"=\" style=\"equals\" key=\"=\" default=\"true\" onclick=\"equals\" />",
        "</Window>",
    ] {
        assert!(markup.contains(line), "{line}\nin\n{markup}");
    }
}

#[test]
fn every_window_of_the_calculator_is_written() {
    let files = clarity_gen::web_windows::window_files(&calculator());
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["ui/windows/Main.cxaml", "ui/windows/Preferences.cxaml"]);
}

/// Text that XML gives a meaning, and line ends, survive in an attribute.
#[test]
fn text_is_escaped_for_xml() {
    let source = "module m\n\npage Main: window\n    label \"Fish & <chips> \\\"now\\\"\" name fish at 1,1 span 4\n";
    let (file, diagnostics) = clarity_spec::parser::parse(source, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let markup = clarity_gen::web_windows::window_markup(&file.pages[0]);
    assert!(markup.contains("caption=\"Fish &amp; &lt;chips&gt; &quot;now&quot;\""), "{markup}");
}

/// A field its kind gives a value and the part has none of is listed in
/// `none`, so it is never taken for the text "null".
#[test]
fn a_field_with_no_value_is_listed_as_none() {
    let source = "module m\n\npage Main: window\n    data shown: integer\n    display name screen at 1,1 span 4 bind shown\n";
    let (mut file, diagnostics) = clarity_spec::parser::parse(source, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    file.pages[0].widgets[0].props.caption = None;
    let markup = clarity_gen::web_windows::window_markup(&file.pages[0]);
    assert!(!markup.contains("\"null\""), "{markup}");
}

/// The screen reads each kind's defaults, a new menu item's, and how to read
/// each field, from files this writes, so Rust's catalog and ts-rs's types
/// stay the one source of them.
/// `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test web_windows` writes them again.
#[test]
fn the_screen_reads_the_catalog_s_defaults_and_the_fields_types() {
    let generated = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web/src/generated");
    let read = |name: &str| std::fs::read_to_string(generated.join(name)).unwrap_or_default();
    let wanted = [
        ("part-defaults.json", clarity_gen::web_windows::part_defaults()),
        ("command-defaults.json", clarity_gen::web_windows::command_defaults()),
        ("part-fields.json", clarity_gen::web_windows::field_kinds(&read("WidgetProps.ts"))),
        ("command-fields.json", clarity_gen::web_windows::field_kinds(&read("CommandProps.ts"))),
        ("sidebar-defaults.json", clarity_gen::web_windows::sidebar_defaults()),
        ("sidebar-fields.json", clarity_gen::web_windows::field_kinds(&read("SidebarProps.ts"))),
    ];
    for (name, text) in &wanted {
        if std::env::var("CLARITY_WRITE_DEFAULTS").is_ok() {
            std::fs::write(generated.join(name), text).unwrap();
        }
        assert!(read(name) == *text, "{name} is behind; write it with CLARITY_WRITE_DEFAULTS=1");
    }
    let fields: serde_json::Value = serde_json::from_str(&wanted[2].1).unwrap();
    assert_eq!((fields["caption"].as_str(), fields["digits"].as_str(), fields["cards"].as_str(), fields["order"].as_str()), (Some("text"), Some("number"), Some("bool"), Some("json")));
}

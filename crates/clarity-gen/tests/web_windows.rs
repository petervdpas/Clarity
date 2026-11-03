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
        "<Window name=\"Main\" columns=\"4\">",
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

/// Clarity.UI's markup reader takes each kind's defaults, a new menu item's,
/// and how to read each field, from files this writes (runtimes/rust-ui/catalog),
/// so Rust's catalog and the model's types (ts-rs) stay the one source of them.
/// `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test web_windows` writes them again.
#[test]
fn the_screen_reads_the_catalog_s_defaults_and_the_fields_types() {
    let types = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web/src/generated");
    let generated = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust-ui/catalog");
    let read_type = |name: &str| std::fs::read_to_string(types.join(name)).unwrap_or_default();
    let read = |name: &str| std::fs::read_to_string(generated.join(name)).unwrap_or_default();
    let wanted = [
        ("part-defaults.json", clarity_gen::web_windows::part_defaults()),
        ("command-defaults.json", clarity_gen::web_windows::command_defaults()),
        ("part-fields.json", clarity_gen::web_windows::field_kinds(&read_type("WidgetProps.ts"))),
        ("command-fields.json", clarity_gen::web_windows::field_kinds(&read_type("CommandProps.ts"))),
        ("sidebar-defaults.json", clarity_gen::web_windows::sidebar_defaults()),
        ("sidebar-fields.json", clarity_gen::web_windows::field_kinds(&read_type("SidebarProps.ts"))),
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

/// Where each part's edits are undone (DESIGN 5.29), as the project works it
/// out, goes into the markup, so a screen that reads the markup alone (C# on
/// Clarity.UI) has it: a field in a panel undoes with the panel, a part outside
/// one with itself, and a part with `no undo` has none.
#[test]
fn a_part_s_undo_scope_is_in_the_markup() {
    let source = "module m\n\nentity Note\n    title: text\n\npage Main: window\n    data note: Note\n    panel name notePanel at 1,1 span 12 rows 3\n        textbox name titleInput at 1,1 span 6 bind note.title\n    textbox name loose at 1,4 span 6 bind note.title\n    textbox name plain at 1,5 span 6 bind note.title no undo\n";
    let project = clarity_spec::keyed::load_keyed([("m.clarity", source)], &[], "en");
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::diagnostic::Severity::Error), "{:#?}", project.diagnostics);
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Main").unwrap();
    let markup = clarity_gen::web_windows::window_markup(page);
    for line in [
        "<Textbox name=\"titleInput\" at=\"1,1\" span=\"6\" bind=\"note.title\" undoScope=\"notePanel\" />",
        "<Textbox name=\"loose\" at=\"1,4\" span=\"6\" bind=\"note.title\" undoScope=\"loose\" />",
        "<Textbox name=\"plain\" at=\"1,5\" span=\"6\" bind=\"note.title\" noUndo=\"true\" />",
    ] {
        assert!(markup.contains(line), "{line}\nin\n{markup}");
    }
}

/// A window's grid has its columns on the root element when they are not
/// the twelve every window starts with.
#[test]
fn a_window_s_columns_are_on_its_root_when_not_twelve() {
    let source = "module m\n\npage Main: window\n    grid 6 columns\n    label \"Hello\" name hello at 1,1 span 6\n\npage Other: window\n    label \"Hello\" name hello at 1,1 span 6\n";
    let (file, diagnostics) = clarity_spec::parser::parse(source, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let six = clarity_gen::web_windows::window_markup(&file.pages[0]);
    assert!(six.contains("<Window name=\"Main\" columns=\"6\">"), "{six}");
    let twelve = clarity_gen::web_windows::window_markup(&file.pages[1]);
    assert!(twelve.contains("<Window name=\"Other\">"), "{twelve}");
}

/// A custom part declared with `grows` (DESIGN 5.19) says so in the markup,
/// so every Clarity.UI lays it out as a growing part; one without it keeps
/// its rows and writes nothing.
#[test]
fn a_custom_part_that_grows_says_so_in_the_markup() {
    let source = "module m\n\npart Board\n    crate \"parts/Board\"\n    size 6 x 4\n    grows\n\npart Dial\n    crate \"parts/Dial\"\n\npage Main: window\n    Board name board at 1,1 span 6 rows 4\n    Dial name dial at 7,1 span 6 rows 2\n";
    let project = clarity_spec::keyed::load_keyed([("m.clarity", source)], &[], "en");
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Main").unwrap();
    let markup = clarity_gen::web_windows::window_markup(page);
    assert!(markup.contains("<Custom name=\"board\" at=\"1,1\" span=\"6\" rows=\"4\" undoScope=\"board\" part=\"Board\" grows=\"true\" />"), "{markup}");
    assert!(markup.contains("<Custom name=\"dial\" at=\"7,1\" span=\"6\" rows=\"2\" undoScope=\"dial\" part=\"Dial\" />"), "{markup}");
}

/// A web program's browser files (round B, DESIGN 7.9): the page, titled as
/// the app, and files.json with what the library draws from, the same as a
/// desktop program carries, in the project's order.
#[test]
fn a_web_program_s_browser_files_carry_the_page_and_what_the_library_draws_from() {
    let project = calculator();
    let mut settings = clarity_project::starter("Calc", "calc");
    settings.title = Some("Calc <one>".into());
    let files = clarity_gen::web_browser::site(&project, &settings);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths[..2], ["browser/index.html", "browser/clarity/files.json"]);
    let page = &files[0].text;
    assert!(page.contains("<title>Calc &lt;one&gt;</title>"), "{page}");
    assert!(page.contains("<canvas id=\"clarity-ui\"></canvas>") && page.contains("<script src=\"clarity-ui-page.js\"></script>"), "{page}");
    let json: serde_json::Value = serde_json::from_str(&files[1].text).unwrap();
    assert_eq!(json["title"], "Calc <one>");
    let windows: Vec<&String> = json["windows"].as_object().unwrap().keys().collect();
    assert_eq!(windows, ["Main", "Preferences"]);
    assert!(json["windows"]["Main"].as_str().unwrap().contains("<Window name=\"Main\" columns=\"4\">"));
    assert!(json["model"].is_object(), "the model is JSON: {}", json["model"]);
    assert!(json["width"].as_u64().unwrap() > 0 && json["height"].as_u64().unwrap() > 0);
    // The light theme before the dark, as the desktop programs carry them.
    let text = &files[1].text;
    let first_light = text.find(".light\":").unwrap();
    let first_dark = text.find(".dark\":").unwrap();
    assert!(first_light < first_dark, "{}", &text[..text.len().min(400)]);
    assert!(json["themes"].as_object().unwrap().values().all(serde_json::Value::is_object));
    assert_eq!(json["resources"], serde_json::json!([]));
}

/// The resources are named in files.json and served beside it; the
/// library's four files are added to a program that is served, from the
/// browser folder, and to no other.
#[test]
fn the_library_s_browser_files_join_the_programs_served_to_a_browser() {
    let mut project = calculator();
    project.resources = Some(clarity_spec::resources::Resources { files: vec![clarity_spec::resources::ResourceFile { name: "logo.png".into(), bytes: vec![1, 2, 3] }], ..Default::default() });
    let settings = clarity_project::starter("Calc", "calc");
    let site = clarity_gen::web_browser::site(&project, &settings);
    let json: serde_json::Value = serde_json::from_str(&site.iter().find(|f| f.path == "browser/clarity/files.json").unwrap().text).unwrap();
    assert_eq!(json["resources"], serde_json::json!(["logo.png"]));
    let logo = site.iter().find(|f| f.path == "browser/clarity/resources/logo.png").expect("the resource is served");
    assert_eq!(logo.bytes.as_deref(), Some(&[1u8, 2, 3][..]));

    let folder = std::env::temp_dir().join(format!("clarity-browser-files-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    for name in clarity_gen::clarity_ui::BROWSER_FILES {
        std::fs::write(folder.join(name), name.as_bytes()).unwrap();
    }
    let mut programs = vec![
        clarity_gen::Program { target: clarity_project::Target::Web, files: site.clone() },
        clarity_gen::Program { target: clarity_project::Target::Cli, files: vec![] },
    ];
    assert!(clarity_gen::web_browser::needs_library(&programs));
    clarity_gen::web_browser::with_library(&mut programs, &folder).unwrap();
    // Twice is the same as once.
    clarity_gen::web_browser::with_library(&mut programs, &folder).unwrap();
    for name in clarity_gen::clarity_ui::BROWSER_FILES {
        let copied: Vec<_> = programs[0].files.iter().filter(|f| f.path == format!("browser/{name}")).collect();
        assert_eq!(copied.len(), 1, "{name}");
        assert_eq!(copied[0].bytes.as_deref(), Some(name.as_bytes()));
    }
    assert!(programs[1].files.is_empty());
    let all = clarity_gen::web_browser::files(&project, &settings, &folder).unwrap();
    assert_eq!(all.len(), site.len() + 4);
    std::fs::remove_dir_all(&folder).unwrap();
    let missing = clarity_gen::web_browser::library(&folder).unwrap_err();
    assert!(missing.contains("clarity-ui.js"), "{missing}");
}

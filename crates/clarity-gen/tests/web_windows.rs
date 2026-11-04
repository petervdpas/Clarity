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

/// The project's row height (`row height 30`) is on every window's root,
/// where Clarity.UI reads it, and in the preview's and designer's files;
/// the standard 36 writes nothing, so a project without the line keeps its markup.
#[test]
fn a_project_s_row_height_is_on_every_window_s_root() {
    let (_, mut project) = todo();
    assert_eq!(project.row_height, 36);
    for file in clarity_gen::web_windows::window_files(&project) {
        assert!(!file.text.contains("rowHeight"), "{}", file.text);
    }
    project.row_height = 30;
    let files_written = clarity_gen::web_windows::window_files(&project);
    assert!(!files_written.is_empty());
    for file in &files_written {
        let root = file.text.lines().find(|l| l.starts_with("<Window ")).unwrap();
        assert!(root.contains(" rowHeight=\"30\""), "{root}");
    }
    let designed = files(&preview::files_json(&project, "Todo", &Shown::Window("Main".into()), true, &Default::default()));
    assert!(designed["windows"]["Main"].as_str().unwrap().contains("<Window name=\"Main\" rowHeight=\"30\" designItem="), "{designed}");
    let app = files(&preview::files_json(&project, "Todo", &Shown::App("Todo".into()), true, &Default::default()));
    assert!(app["windows"]["Todo"].as_str().unwrap().starts_with("<Window name=\"Todo\" rowHeight=\"30\" designItem="), "{app}");
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

/// A custom part's declared description (DESIGN 7.9, A1) is in its
/// markup, so every Clarity.UI gives it to screen readers.
#[test]
fn a_custom_part_s_description_is_in_the_markup() {
    let source = "module m\n\npart Dial\n    crate \"parts/Dial\"\n    description @dialAbout\n\npage Main: window\n    Dial name dial at 1,1 span 6 rows 2\n";
    let project = clarity_spec::keyed::load_keyed([("m.clarity", source)], &[], "en");
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Main").unwrap();
    let markup = clarity_gen::web_windows::window_markup(page);
    assert!(markup.contains("part=\"Dial\" description=\"@dialAbout\""), "{markup}");
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

// The IDE's preview and design canvas on Clarity.UI (DESIGN 8.1, clarity_gen::preview).

use clarity_gen::preview::{self, PreviewProgram, Shown};
use serde_json::{json, Value};

fn todo() -> (std::path::PathBuf, clarity_spec::Project) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/todo");
    let project = clarity_project::load_modules(&dir).unwrap();
    (dir, project)
}

fn files(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn the_designer_draws_markup_that_says_where_each_part_is_written() {
    let (_, project) = todo();
    let main = project.pages().find(|p| p.name.name == "Main").unwrap();
    let designed = files(&preview::files_json(&project, "Todo", &Shown::Window("Main".into()), true, &Default::default()));
    let markup = designed["windows"]["Main"].as_str().unwrap();
    assert!(markup.contains(&format!("<Window name=\"Main\" designItem=\"{}\">", main.span.start)), "{markup}");
    let add = main.widgets.iter().find(|w| w.props.name.as_deref() == Some("addButton")).unwrap();
    assert!(markup.contains(&format!("onclick=\"addClicked\" designItem=\"{}\"", add.span.start)), "{markup}");
    // The app's menu items too.
    assert!(designed["apps"]["Todo"].as_str().unwrap().contains("designItem="));
    // The preview's markup is the program's own: no design items.
    let shown = files(&preview::files_json(&project, "Todo", &Shown::Window("Users".into()), false, &Default::default()));
    assert!(!shown["windows"].to_string().contains("designItem"));
    assert_eq!(shown["title"], json!("Todo"));
    assert_eq!((shown["width"].clone(), shown["height"].clone()), (json!(1100), json!(760)));
    // A program's own markup never carries them either.
    assert!(!clarity_gen::web_windows::window_markup(main).contains("designItem"));
}

#[test]
fn an_app_is_designed_as_an_empty_window_in_its_frame() {
    let (_, project) = todo();
    let app = project.apps().next().unwrap();
    let designed = files(&preview::files_json(&project, "Todo", &Shown::App("Todo".into()), true, &Default::default()));
    assert_eq!(designed["windows"]["Todo"], json!(format!("<Window name=\"Todo\" designItem=\"{}\" />\n", app.span.start)));
    let program = PreviewProgram::new(project, "Todo");
    assert_eq!(program.answer("started", &json!({})), json!({ "value": { "main": "Todo" } }));
    assert!(program.answer("open_window", &json!({ "window": "Todo", "with": {} }))["value"]["state"]["window"] == json!("Todo"));
}

#[test]
fn the_preview_program_opens_windows_with_sample_data_and_runs_no_handler() {
    let (_, project) = todo();
    let program = PreviewProgram::new(project, "Main");
    assert_eq!(program.answer("started", &json!({})), json!({ "value": { "main": "Main" } }));
    assert_eq!(program.answer("start_app", &json!({}))["value"]["state"]["window"], json!(""));
    let opened = program.answer("open_window", &json!({ "window": "Users", "with": { "extra": 1 } }));
    let data = &opened["value"]["state"]["data"];
    // A list every sample record, a record the first, each with its number; what `with` gives is kept.
    assert_eq!(data["people"].as_array().unwrap().len(), clarity_spec::sample::ROWS_PER_ENTITY);
    assert_eq!(data["person"], data["people"][0]);
    assert_eq!(data["people"][1]["id"], json!(2));
    assert_eq!(data["extra"], json!(1));
    // A reference names its record by number, as a program holds it; the dropdown's choices by the same numbers.
    let tasks = program.answer("open_window", &json!({ "window": "Main", "with": {} }))["value"]["state"]["data"]["tasks"].clone();
    assert!(tasks.as_array().unwrap().iter().any(|t| t["finished_by"].is_number()), "{tasks}");
    let options = program.answer("ref_options", &json!({ "entity": "Person" }))["value"].clone();
    assert_eq!(options[0]["id"], json!(1));
    // A handler is not run: the window stays as it was and says where it runs.
    let state = opened["value"]["state"].clone();
    let clicked = program.answer("dispatch_event", &json!({ "state": state, "handler": "personSelected" }));
    assert_eq!(clicked["value"]["state"], state);
    // In Clarity's own words, by their key, the handler in its slot.
    let said = &clicked["value"]["messages"][0]["text"];
    assert_eq!(said["key"], json!("previewRunsNoHandler"));
    assert_eq!(said["text"], json!(clarity_spec::translations::PREVIEW_KEYS[0].1));
    assert_eq!(said["values"]["handler"]["value"], json!("personSelected"));
    // What the person changes stays as the screen sent it.
    assert_eq!(program.answer("edit_event", &json!({ "state": state, "part": "name" }))["value"]["state"], state);
    assert!(program.answer("open_window", &json!({ "window": "Nowhere" }))["error"].is_string());
    assert!(program.answer("facts", &json!({}))["value"].is_object());
}

/// A request to the preview server: its status line and body.
fn ask(address: &str, method: &str, path: &str, body: &str) -> (String, Vec<u8>) {
    use std::io::{Read, Write};
    let rest = address.strip_prefix("http://").unwrap();
    let (host, _) = rest.split_once('/').unwrap();
    let mut stream = std::net::TcpStream::connect(host).unwrap();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let split = answer.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&answer[..split]).to_string();
    (head, answer[split + 4..].to_vec())
}

#[test]
fn the_preview_server_serves_the_library_the_windows_and_the_wire_below_its_secret() {
    let (dir, _) = todo();
    let file = dir.with_extension("clarity");
    let library = Path::new(env!("CARGO_TARGET_TMPDIR")).join("preview-library");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("clarity-ui-page.js"), "// page").unwrap();
    let (loaded_file, loaded_dir) = (file.clone(), dir.clone());
    let server = preview::Server::start(
        std::sync::Arc::new(move || preview::load(&loaded_file, &loaded_dir)),
        std::sync::Arc::new(move || Ok(library.clone())),
        "s3cret",
    )
    .unwrap();
    let address = server.address().to_string();
    assert!(address.starts_with("http://127.0.0.1:") && address.ends_with("/s3cret/"), "{address}");
    let path = |rest: &str| format!("/s3cret/{rest}");
    let (head, body) = ask(&address, "GET", &path("clarity-ui-page.js"), "");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("text/javascript") && head.contains("Access-Control-Allow-Origin: *"), "{head}");
    assert_eq!(body, b"// page");
    // The wire answers once the drawing read the project.
    let (_, body) = ask(&address, "POST", &path("invoke"), r#"{"command":"started","args":{}}"#);
    assert!(String::from_utf8_lossy(&body).contains("error"));
    let (head, body) = ask(&address, "GET", &path("files.json?window=Users&design=1&revision=3"), "");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(files(&String::from_utf8_lossy(&body))["windows"]["Users"].as_str().unwrap().contains("designItem"));
    let (_, body) = ask(&address, "POST", &path("invoke"), r#"{"command":"started","args":{},"browser":"x","settings":{}}"#);
    assert_eq!(files(&String::from_utf8_lossy(&body)), json!({ "value": { "main": "Users" } }));
    // The unsaved Design text's project is drawn when asked for.
    let mut draft = clarity_project::load_modules(&dir).unwrap();
    draft.files.iter_mut().flat_map(|f| &mut f.pages).find(|p| p.name.name == "Users").unwrap().title = Some("Drafted".into());
    server.set_draft(Some(draft));
    let (_, body) = ask(&address, "GET", &path("files.json?window=Users&design=1&draft=1"), "");
    assert!(files(&String::from_utf8_lossy(&body))["model"].to_string().contains("Drafted"));
    let (_, body) = ask(&address, "GET", &path("files.json?window=Users&design=1"), "");
    assert!(!files(&String::from_utf8_lossy(&body))["model"].to_string().contains("Drafted"));
    // A page's preflight, and nothing outside the secret or the resources folder.
    let (head, _) = ask(&address, "OPTIONS", &path("invoke"), "");
    assert!(head.starts_with("HTTP/1.1 204") && head.contains("Access-Control-Allow-Headers: Content-Type"), "{head}");
    assert!(ask(&address, "GET", "/other/clarity-ui-page.js", "").0.starts_with("HTTP/1.1 404"));
    assert!(ask(&address, "GET", &path("resources/../todo.clarity"), "").0.starts_with("HTTP/1.1 404"));
    assert!(ask(&address, "GET", &path("files.json"), "").0.starts_with("HTTP/1.1 400"));
    assert_eq!(preview::query_of("window=Ma%20in&design=1"), [("design".to_string(), "1".to_string()), ("window".to_string(), "Ma in".to_string())].into_iter().collect());
}

/// A copy of a project's file and folder, without what was built or kept.
fn copy_project(from_file: &Path, to: &Path) -> std::path::PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap().flatten() {
            let name = entry.file_name();
            if name == "out" || name == "data" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                copy(&path, &to.join(&name));
            } else {
                std::fs::copy(&path, to.join(&name)).unwrap();
            }
        }
    }
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    let file = to.join(from_file.file_name().unwrap());
    std::fs::copy(from_file, &file).unwrap();
    copy(&from_file.with_extension(""), &file.with_extension(""));
    file
}

/// The IDE's own backend as far as its preview and designer use it, for a
/// page of the IDE run by Vite in a browser without Tauri: each command of
/// the page's `invoke` posted as `{cmd, args}`, answered `{value}` or `{error}`.
fn ide_backend(file: std::path::PathBuf, preview_address: String) -> String {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut length = 0;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let dir = clarity_project::module_dir(&file);
            let load = || clarity_spec::Preview::new(clarity_project::load_modules(&dir).unwrap());
            let args = &asked["args"];
            let answer = match asked["cmd"].as_str().unwrap_or_default() {
                "initial_project" => json!({ "value": file.display().to_string() }),
                "open_project" | "check_project" => json!({ "value": load() }),
                "project_file" => json!({ "value": clarity_project::load(&file) }),
                "preview_server_address" => json!({ "value": preview_address }),
                "history_counts" => json!({ "value": { "undo": 0, "redo": 0 } }),
                "apply_edit" => {
                    let path = dir.join(Path::new(args["path"].as_str().unwrap_or_default()).file_name().unwrap_or_default());
                    let source = std::fs::read_to_string(&path).unwrap_or_default();
                    let edit: Result<clarity_spec::edit::Edit, _> = serde_json::from_value(args["edit"].clone());
                    match edit.map_err(|e| e.to_string()).and_then(|edit| clarity_spec::edit::apply_in(&load().project, &source, &edit).map_err(|e| e.to_string())) {
                        Ok(outcome) => {
                            std::fs::write(&path, &outcome.text).unwrap();
                            json!({ "value": { "preview": load(), "focus": outcome.focus } })
                        }
                        Err(why) => json!({ "error": why }),
                    }
                }
                _ => json!({ "value": null }),
            };
            let text = answer.to_string();
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
        }
    });
    address
}

/// Stops a program the test started, however the test ends.
struct Started(std::process::Child);

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The IDE's preview and design canvas in a real browser (DESIGN 8.1): the
/// IDE's page run by Vite, its backend answered by this test, the window
/// drawn by Clarity.UI's browser files (target/clarity-ui-browser, or
/// CLARITY_UI_BROWSER) from the preview server. In the preview a click on a
/// drawn button reaches the preview's program; on the design canvas a click
/// on a part picks it and a drag moves it in the project's file.
///
/// Ignored: it needs Node with the IDE's packages (app/node_modules),
/// Chromium and the browser files. Run with `cargo test -p clarity-gen --test
/// web_windows the_ide -- --ignored`; CLARITY_IDE_SHOTS names the folder for
/// the browser's profile and the screenshots.
#[test]
#[ignore]
fn the_ide_previews_and_designs_on_clarity_ui_in_a_browser() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shots = std::env::var_os("CLARITY_IDE_SHOTS").map(std::path::PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_TARGET_TMPDIR")).join("ide-live"));
    std::fs::create_dir_all(&shots).unwrap();
    let file = copy_project(&repo.join("examples/todo.clarity"), &shots.join("project"));
    let library = match std::env::var_os("CLARITY_UI_BROWSER") {
        Some(folder) => std::path::PathBuf::from(folder),
        None => repo.join("target/clarity-ui-browser"),
    };
    let (loaded_file, loaded_dir) = (file.clone(), clarity_project::module_dir(&file));
    let server = preview::Server::start(std::sync::Arc::new(move || preview::load(&loaded_file, &loaded_dir)), std::sync::Arc::new(move || Ok(library.clone())), &preview::secret()).unwrap();
    let backend = ide_backend(file.clone(), server.address().to_string());
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let app = repo.join("app");
    let _vite = Started(std::process::Command::new("node").args(["node_modules/vite/bin/vite.js", "--port", &port.to_string(), "--strictPort", "--host", "127.0.0.1"]).current_dir(&app).stdout(std::process::Stdio::null()).spawn().unwrap());
    let end = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(std::time::Instant::now() < end, "Vite did not serve");
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let driven = std::process::Command::new("node")
        .arg(repo.join("crates/clarity-gen/tests/live/ide.mjs"))
        .args([format!("http://127.0.0.1:{port}/"), backend, shots.display().to_string()])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&driven.stdout).to_string();
    let result: Value = serde_json::from_str(said.lines().last().unwrap_or("null")).unwrap_or_else(|_| panic!("{said}\n{}", String::from_utf8_lossy(&driven.stderr)));
    assert!(result["error"].is_null(), "{result:#}");
    // The preview: drawn by Clarity.UI, and a click on a drawn button answered by the preview's program.
    assert_eq!(result["preview"]["window"], json!("Main"), "{result:#}");
    assert!(result["preview"]["said"].to_string().contains("addClicked runs in the program itself"), "{result:#}");
    // The designer: the layer has a box for each part, a click picks one, a drag moves it in the file.
    assert!(result["design"]["parts"].as_u64().unwrap() >= 6, "{result:#}");
    assert_eq!(result["design"]["picked"], json!(true), "{result:#}");
    let text = std::fs::read_to_string(clarity_project::module_dir(&file).join("todo.clarity")).unwrap();
    assert!(text.contains("button @deleteButtonCaption name deleteButton at 7,11 span 2"), "the drag moved the button:\n{text}");
    // A column's width dragged on the canvas, and a ribbon button picked there.
    assert!(result["design"]["column"].as_str().is_some_and(|line| line.contains("title")), "{result:#}");
    assert_eq!(result["design"]["command"], json!(true), "{result:#}");
}

#[test]
fn the_designer_shows_the_page_it_switched_to_and_opens_the_popover_picked() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/records");
    let project = clarity_project::load_modules(&dir).unwrap();
    // A container with pages, wherever the example has one.
    let page = project.pages().find(|p| p.widgets.iter().any(|w| !w.pages.is_empty())).expect("a window with tabs, steps or sections");
    let container = page.widgets.iter().find(|w| !w.pages.is_empty()).unwrap();
    let query = preview::query_of(&format!("window={}&pages={}.1,bad", page.name.name, container.span.start));
    let showing = preview::showing_of(&query);
    assert_eq!(showing, [(container.span.start, 1)].into_iter().collect());
    let designed = files(&preview::files_json(&project, "Records", &Shown::Window(page.name.name.clone()), true, &showing));
    let markup = designed["windows"][&page.name.name].as_str().unwrap();
    assert!(markup.contains(&format!("designItem=\"{}\" designShown=\"2\"", container.span.start)), "{markup}");
    // A popover picked shows open: its state says so when its window opens.
    let program = PreviewProgram::new(project.clone(), &page.name.name).opening(vec!["nowhere".into()]);
    let opened = program.answer("open_window", &json!({ "window": page.name.name }));
    assert_eq!(opened["value"]["state"]["parts"], json!({}), "a name the window has not opens nothing");
    let named = container.props.name.clone().unwrap();
    let program = PreviewProgram::new(project.clone(), &page.name.name).opening(vec![named.clone()]);
    assert_eq!(program.answer("open_window", &json!({ "window": page.name.name }))["value"]["state"]["parts"][&named], json!({ "visible": true }));
}

/// A part stuck to the bottom of its grid says so in the markup, which
/// Clarity.UI lays out by (conformance/grid-layout.json).
#[test]
fn a_part_stuck_to_the_bottom_says_so_in_the_markup() {
    let source = "module m\n\npage Main: window\n    button \"Save\" name save at 1,11 span 2 bottom\n    label \"Note\" name note at 3,11 span 2\n";
    let (file, diagnostics) = clarity_spec::parser::parse(source, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let markup = clarity_gen::web_windows::window_markup(&file.pages[0]);
    assert!(markup.contains("<Button name=\"save\" at=\"1,11\" span=\"2\" caption=\"Save\" bottom=\"true\" />"), "{markup}");
    assert!(!markup.contains("<Label name=\"note\" at=\"3,11\" span=\"2\" caption=\"Note\" bottom"), "{markup}");
}

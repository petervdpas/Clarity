//! Custom parts (DESIGN 5.19): a part an outside library draws, declared
//! in Clarity with `part Name`, its library (`native`, `nuget`, `dll`),
//! what it `binds`, typed properties, `tells`, `hears` and `size`, and
//! placed in a window by its name like a built-in part.

use clarity_spec::code::PartLibrary;
use clarity_spec::keyed::load_keyed;
use clarity_spec::model::Literal;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, widget_line};
use clarity_spec::{Project, Severity};

const GAME: &str = "module game

entity Player
    name: text(80), required
    best: integer

part Arena
    native \"parts/Arena\"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    tells scored(points: integer)
    tells finished
    hears restart(level: integer)
    size 8 x 6
    preview \"arena.png\"

page Game: window
    data player: Player
    Arena name arena at 1,1 span 8 rows 6 bind player.best speed 5 title \"Level one\" on change: bestChanged on scored: arenaScored
    button \"Again\" name again at 1,7 span 2 on click: againClicked
";

const CODE: &str = "
code Game
    handler bestChanged
        save player

    handler arenaScored(points: integer)
        message \"Scored {points}\"

    handler againClicked
        arena.speed = arena.speed + 1
        arena.restart(2)
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("game.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn has(errors: &[String], part: &str) -> bool {
    errors.iter().any(|e| e.contains(part))
}

#[test]
fn a_part_is_declared_with_its_library_and_what_it_says() {
    let (file, diagnostics) = parse(GAME, "game.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let part = &file.parts[0];
    assert_eq!(part.name.name, "Arena");
    assert_eq!(part.library, Some(PartLibrary::Native { folder: "parts/Arena".into() }));
    assert!(part.binds.is_some());
    let properties: Vec<&str> = part.properties.iter().map(|p| p.name.name.as_str()).collect();
    assert_eq!(properties, ["speed", "title"]);
    let tells: Vec<(&str, usize)> = part.tells.iter().map(|t| (t.name.name.as_str(), t.params.len())).collect();
    assert_eq!(tells, [("scored", 1), ("finished", 0)]);
    assert_eq!(part.hears[0].name.name, "restart");
    assert_eq!(part.size, Some((8, 6)));
    assert_eq!(part.preview.as_deref(), Some("arena.png"));
}

#[test]
fn a_dotnet_part_comes_from_a_package_or_a_dll() {
    let text = "module notes

part Notes
    nuget \"AvaloniaEdit\" 11.1.0
    binds text

part Plot
    dll \"libs/Plot.dll\"
";
    let (file, diagnostics) = parse(text, "notes.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(file.parts[0].library, Some(PartLibrary::Nuget { package: "AvaloniaEdit".into(), version: "11.1.0".into() }));
    assert_eq!(file.parts[1].library, Some(PartLibrary::Dll { file: "libs/Plot.dll".into() }));
    assert!(file.parts[1].binds.is_none());
}

#[test]
fn a_part_is_placed_by_its_name_with_its_values_and_tells() {
    let (file, diagnostics) = parse(GAME, "game.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.kind, WidgetKind::Custom);
    assert_eq!(props.part.as_deref(), Some("Arena"));
    assert_eq!(props.bind.as_deref(), Some("player.best"));
    assert_eq!(props.values.get("speed"), Some(&Literal::Number("5".into())));
    assert_eq!(props.values.get("title"), Some(&Literal::Str("Level one".into())));
    assert_eq!(props.handlers.len(), 1);
    let tells: Vec<(&str, &str)> = props.tells.iter().map(|t| (t.tell.as_str(), t.handler.as_str())).collect();
    assert_eq!(tells, [("scored", "arenaScored")]);
}

#[test]
fn the_line_of_a_placed_part_is_written_back_as_read() {
    let (file, _) = parse(GAME, "game.clarity");
    let props = &file.pages[0].widgets[0].props;
    let line = widget_line(props);
    assert_eq!(line, "Arena name arena at 1,1 span 8 rows 6 bind player.best speed 5 title \"Level one\" on change: bestChanged on scored: arenaScored");
}

#[test]
fn a_built_in_word_still_needs_lower_case_and_the_custom_kind_is_never_typed() {
    let (_, diagnostics) = parse("module m\n\npage P: window\n    gadget at 1,1\n", "m.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("unknown widget 'gadget'")), "{diagnostics:?}");
    assert_eq!(widgets::keyword(WidgetKind::Custom), "custom part");
}

#[test]
fn a_good_part_and_its_use_have_no_errors() {
    assert_eq!(errors(GAME, CODE), Vec::<String>::new());
}

#[test]
fn a_part_that_is_not_declared_is_an_error() {
    let page = GAME.replace("    Arena name arena", "    Arean name arena");
    assert!(has(&errors(&page, ""), "unknown part 'Arean'"), "{:?}", errors(&page, ""));
}

#[test]
fn values_in_the_window_are_declared_and_fit() {
    let e = errors(&GAME.replace("speed 5", "colour 5"), CODE);
    assert!(has(&e, "part Arena has no property 'colour'"), "{e:?}");
    let e = errors(&GAME.replace("speed 5", "speed \"fast\""), CODE);
    assert!(has(&e, "speed"), "{e:?}");
    let e = errors(&GAME.replace("speed 5", "speed 12"), CODE);
    assert!(has(&e, "speed is at most 10"), "{e:?}");
}

#[test]
fn a_wired_tell_is_declared_and_its_handler_takes_its_values() {
    let e = errors(&GAME.replace("on scored:", "on bounced:"), CODE);
    assert!(has(&e, "part Arena tells no 'bounced'"), "{e:?}");
    let code = CODE.replace("arenaScored(points: integer)", "arenaScored(points: text)");
    let e = errors(GAME, &code);
    assert!(has(&e, "arenaScored"), "{e:?}");
    let code = CODE.replace("arenaScored(points: integer)", "arenaScored").replace("message \"Scored {points}\"", "message \"Scored\"");
    let e = errors(GAME, &code);
    assert!(has(&e, "arenaScored"), "{e:?}");
}

#[test]
fn what_it_binds_is_the_type_of_the_field() {
    let e = errors(&GAME.replace("bind player.best", "bind player.name"), CODE);
    assert!(has(&e, "part Arena binds integer"), "{e:?}");
}

#[test]
fn code_sets_its_properties_and_calls_what_it_hears() {
    let e = errors(GAME, &CODE.replace("arena.speed = arena.speed + 1", "arena.speed = \"quick\""));
    assert!(has(&e, "cannot set integer to text"), "{e:?}");
    let e = errors(GAME, &CODE.replace("arena.restart(2)", "arena.jump(2)"));
    assert!(has(&e, "part Arena hears no 'jump'"), "{e:?}");
    let e = errors(GAME, &CODE.replace("arena.restart(2)", "arena.restart(\"two\")"));
    assert!(has(&e, "restart"), "{e:?}");
    let e = errors(GAME, &CODE.replace("arena.restart(2)", "arena.restart()"));
    assert!(has(&e, "restart"), "{e:?}");
    // Every part is shown and enabled by code.
    assert_eq!(errors(GAME, &CODE.replace("arena.restart(2)", "arena.visible = false\n        arena.enabled = true")), Vec::<String>::new());
}

#[test]
fn a_declaration_is_checked() {
    let twice = GAME.replace("    title: text\n", "    title: text\n    speed: decimal\n");
    assert!(has(&errors(&twice, CODE), "part Arena has two properties, tells or hears named 'speed'"));
    let built_in = GAME.replace("    title: text\n", "    name: text\n").replace(" title \"Level one\"", "");
    assert!(has(&errors(&built_in, CODE), "'name' is a word every part has"), "{:?}", errors(&built_in, CODE));
    let record = GAME.replace("    title: text\n", "    who: Player\n").replace(" title \"Level one\"", "");
    assert!(has(&errors(&record, CODE), "a property is text, a number, a date, a duration, an enum or bool"), "{:?}", errors(&record, CODE));
    let bad_default = GAME.replace("default 3", "default \"three\"");
    assert!(has(&errors(&bad_default, CODE), "the default of speed"));
    let no_library = GAME.replace("    native \"parts/Arena\"\n", "");
    assert!(has(&errors(&no_library, CODE), "part Arena needs its library"), "{:?}", errors(&no_library, CODE));
    let twice = format!("{GAME}\npart Arena\n    native \"x\"\n");
    assert!(has(&errors(&twice, CODE), "'Arena' is already declared"), "{:?}", errors(&twice, CODE));
}

#[test]
fn a_text_property_is_shown_text_with_a_key() {
    let p = project(GAME, CODE);
    let props = &p.files[0].pages[0].widgets[0].props;
    assert_eq!(props.values.get("title"), Some(&Literal::Str("@arenaTitle".into())));
    assert_eq!(props.values.get("speed"), Some(&Literal::Number("5".into())));
}

#[test]
fn a_native_part_needs_its_folder_with_its_build_file() {
    use clarity_spec::native::{self, PartFile};
    let file = |path: &str| PartFile { part: "Arena".into(), path: path.into(), bytes: b"x".to_vec() };
    let checked = |files: Vec<PartFile>| {
        let mut p = project(GAME, CODE);
        native::attach(&mut p, files);
        p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect::<Vec<_>>()
    };
    assert_eq!(checked(vec![file("CMakeLists.txt"), file("Arena.cpp")]), Vec::<String>::new());
    assert!(has(&checked(vec![]), "part Arena: its folder 'parts/Arena' is not there"));
    assert!(has(&checked(vec![file("Arena.cpp")]), "'parts/Arena' has no CMakeLists.txt"));
}

#[test]
fn a_native_parts_folder_is_read_without_what_its_build_leaves() {
    let dir = std::env::temp_dir().join(format!("clarity-native-parts-{}", std::process::id()));
    let folder = dir.join("parts/Arena");
    std::fs::create_dir_all(folder.join("build")).unwrap();
    std::fs::create_dir_all(folder.join("clarity")).unwrap();
    std::fs::create_dir_all(folder.join("src")).unwrap();
    std::fs::write(folder.join("CMakeLists.txt"), "x").unwrap();
    std::fs::write(folder.join("src/Arena.cpp"), "x").unwrap();
    std::fs::write(folder.join("build/junk.o"), "x").unwrap();
    std::fs::write(folder.join("clarity/Arena.h"), "x").unwrap();
    let p = project(GAME, CODE);
    let files = clarity_spec::native::native_files(&dir, &p).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["CMakeLists.txt", "src/Arena.cpp"]);
}

#[test]
fn the_designer_drops_a_declared_part_and_moves_it() {
    use clarity_spec::edit::{Edit, apply};
    let page_at = GAME.find("page Game").unwrap() as u32;
    let dropped = apply(
        GAME,
        &Edit::CreateWidget { container: page_at, kind: WidgetKind::Custom, col: 1, row: 9, width: Some(8), caption: None, bind: None, part: Some("Arena".into()), height: Some(6) },
    )
    .unwrap();
    assert!(dropped.text.contains("    Arena name arena2 at 1,9 span 8 rows 6\n"), "{}", dropped.text);
    // Moved and given a value in the designer, it keeps its kind and its line.
    let (file, _) = parse(&dropped.text, "game.clarity");
    let placed = &file.pages[0].widgets.iter().find(|w| w.props.name.as_deref() == Some("arena2")).unwrap();
    let mut props = placed.props.clone();
    props.col = 9;
    props.values.insert("speed".into(), Literal::Number("4".into()));
    let moved = apply(&dropped.text, &Edit::UpdateWidget { item: placed.span.start, container: None, props }).unwrap();
    assert!(moved.text.contains("    Arena name arena2 at 9,9 span 8 rows 6 speed 4\n"), "{}", moved.text);
}

#[test]
fn wiring_a_tell_writes_a_handler_that_takes_its_values() {
    use clarity_spec::edit::{Edit, apply};
    let page = GAME.replace(" on scored: arenaScored", "");
    let code = CODE.replace("    handler arenaScored(points: integer)\n        message \"Scored {points}\"\n\n", "");
    let text = format!("{page}{code}");
    let wired = apply(&text, &Edit::AddHandlerWith { owner: "Game".into(), name: "arenaScored".into(), params: "points: integer".into() }).unwrap();
    assert!(wired.text.contains("    handler arenaScored(points: integer)\n"), "{}", wired.text);
}

#[test]
fn a_canvas_has_no_renderer_any_more() {
    let (_, diagnostics) = parse("module m\n\npage P: window\n    canvas name c at 1,1 renderer Game\n", "m.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("graphics an outside library draws are a custom part")), "{diagnostics:?}");
}

#[test]
fn a_dotnet_part_needs_its_adapter_and_a_dll_its_assembly() {
    use clarity_spec::native::{self, PartFile};
    let text = "module notes\n\npart Notes\n    nuget \"Newtonsoft.Json\" 13.0.1\n\npart Plot\n    dll \"libs/Plot.dll\"\n";
    let checked = |files: Vec<PartFile>| {
        let mut p = load_keyed([("notes.clarity", text)], &[], "en");
        native::attach(&mut p, files);
        p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect::<Vec<_>>()
    };
    let file = |part: &str, path: &str| PartFile { part: part.into(), path: path.into(), bytes: b"x".to_vec() };
    assert_eq!(checked(vec![file("Notes", "Notes.cs"), file("Plot", "Plot.cs"), file("Plot", "Plot.dll")]), Vec::<String>::new());
    assert!(has(&checked(vec![file("Plot", "Plot.cs"), file("Plot", "Plot.dll")]), "'parts/Notes/Notes.cs' is not there"));
    assert!(has(&checked(vec![file("Notes", "Notes.cs"), file("Plot", "Plot.cs")]), "its assembly 'libs/Plot.dll' is not there"));
}

#[test]
fn sample_text_shows_no_label_key() {
    let p = load_keyed([("game.clarity", "module game\n\nentity Score\n    player: text(40), label @scorePlayerLabel\n")], &[], "en");
    let rows = &clarity_spec::sample::sample_data(&p)["Score"];
    let text = rows[0]["player"].as_str().unwrap();
    assert!(!text.contains('@'), "{text}");
    assert!(text.starts_with("Player "), "{text}");
}

#[test]
fn a_rust_or_go_part_names_its_crate_or_package() {
    let text = "module game\n\npart Arena\n    crate \"parts/Arena\"\n\npart Field\n    go \"parts/field\"\n";
    let (file, diagnostics) = parse(text, "game.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(file.parts[0].library, Some(PartLibrary::Crate { folder: "parts/Arena".into() }));
    assert_eq!(file.parts[1].library, Some(PartLibrary::Go { folder: "parts/field".into() }));
}

#[test]
fn a_crate_or_go_part_needs_its_folder_with_its_manifest() {
    use clarity_spec::native::{self, PartFile};
    let text = "module game\n\npart Arena\n    crate \"parts/Arena\"\n\npart Field\n    go \"parts/field\"\n";
    let checked = |files: Vec<PartFile>| {
        let mut p = load_keyed([("game.clarity", text)], &[], "en");
        native::attach(&mut p, files);
        p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect::<Vec<_>>()
    };
    let file = |part: &str, path: &str| PartFile { part: part.into(), path: path.into(), bytes: b"x".to_vec() };
    assert_eq!(checked(vec![file("Arena", "Cargo.toml"), file("Field", "go.mod")]), Vec::<String>::new());
    assert!(has(&checked(vec![file("Field", "go.mod")]), "part Arena: its folder 'parts/Arena' is not there"));
    assert!(has(&checked(vec![file("Arena", "src/lib.rs"), file("Field", "go.mod")]), "'parts/Arena' has no Cargo.toml"));
    assert!(has(&checked(vec![file("Arena", "Cargo.toml"), file("Field", "field.go")]), "'parts/field' has no go.mod"));
}

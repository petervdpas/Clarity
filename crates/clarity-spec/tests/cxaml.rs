//! Windows, views and the app as `.cxaml` (docs/CXAML-SOURCE-PLAN.md):
//! every page and app of every example, and of the fixture with every part,
//! written as its `.cxaml` and read back is the same; each part's span is
//! its element's place in the file.

use std::path::{Path, PathBuf};

use clarity_spec::cxaml::{app_source, is_app_markup, page_source, read_app, read_page};
use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every folder of `.clarity` modules: each example's, and the fixture's.
fn module_folders() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(repo().join("examples"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    out.push(repo().join("crates/clarity-gen/tests/fixtures/everything"));
    out.sort();
    out
}

/// Every page of every example and of the fixture with every part: those
/// in their `design/` files, and any still written in a module's file.
fn all_pages() -> Vec<(String, clarity_spec::model::Page)> {
    let mut out = Vec::new();
    for folder in module_folders() {
        for path in clarity_spec::spec_files(&folder).unwrap() {
            let text = std::fs::read_to_string(&path).unwrap();
            let (file, _) = clarity_spec::parser::parse(&text, "");
            out.extend(file.pages.into_iter().map(|p| (path.display().to_string(), p)));
        }
        for path in clarity_spec::cxaml::page_files(&folder, &[], &[]).unwrap() {
            let text = std::fs::read_to_string(&path).unwrap();
            if is_app_markup(&text) {
                continue;
            }
            let (page, d) = read_page(&text, "");
            assert!(d.is_empty(), "{}: {d:#?}", path.display());
            out.push((path.display().to_string(), page.unwrap()));
        }
    }
    out
}

/// A page as JSON without where things are, which differs between a file and its markup.
fn without_places(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(fields.iter().filter(|(k, _)| !k.ends_with("span") && !k.ends_with("Span")).map(|(k, v)| (k.clone(), without_places(v))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(without_places).collect()),
        other => other.clone(),
    }
}

#[test]
fn every_page_reads_back_from_its_cxaml_as_it_was() {
    let pages = all_pages();
    for (path, page) in &pages {
        let markup = page_source(page, "m");
        let (read, diagnostics) = read_page(&markup, "Page.cxaml");
        assert!(diagnostics.is_empty(), "{path} {}: {diagnostics:#?}\n{markup}", page.name.name);
        same(path, page, &read.expect("a page"), &markup);
    }
    assert!(pages.len() > 80, "{} pages", pages.len());
}

/// `got` is `want` read back, places aside; else where they first differ.
fn same(path: &str, want: &clarity_spec::model::Page, got: &clarity_spec::model::Page, written: &str) {
    let mut want = want.clone();
    want.data.retain(|d| !d.made);
    let (want_json, got_json) = (without_places(&serde_json::to_value(&want).unwrap()), without_places(&serde_json::to_value(got).unwrap()));
    if want_json != got_json {
        let (w, g) = (serde_json::to_string_pretty(&want_json).unwrap(), serde_json::to_string_pretty(&got_json).unwrap());
        let first = w.lines().zip(g.lines()).position(|(a, b)| a != b).unwrap_or(0);
        panic!("{path} {} differs from line {first}:\nwant: {}\ngot:  {}\n{written}", want.name.name, w.lines().skip(first.saturating_sub(3)).take(8).collect::<Vec<_>>().join("\n"), g.lines().skip(first.saturating_sub(3)).take(8).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn each_part_is_where_its_element_is() {
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Window class=\"dashboard.Board\" title=\"@boardTitle\" width=\"1200\" height=\"760\" onopen=\"boardOpened\">\n  <Data name=\"total\" type=\"decimal\" />\n  <Content name=\"totalSquare\" at=\"1,8\" span=\"4\" rows=\"7\">\n    <View name=\"TotalView\" onsaved=\"totalSaved\" />\n  </Content>\n</Window>\n";
    let (page, diagnostics) = read_page(markup, "Board.cxaml");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let page = page.unwrap();
    assert_eq!(page.title.as_deref(), Some("@boardTitle"));
    assert_eq!(page.size.map(|s| (s.width, s.height)), Some((1200, 760)));
    assert_eq!(page.handlers[0].handler, "boardOpened");
    let content = &page.widgets[0];
    assert_eq!((content.props.col, content.props.row, content.props.width, content.props.height), (1, 8, 4, 7));
    assert!(markup[content.span.start as usize..].starts_with("<Content name=\"totalSquare\""));
    assert_eq!(content.pages[0].view.as_deref(), Some("TotalView"));
    assert_eq!(content.pages[0].tells[0].handler, "totalSaved");
}

#[test]
fn what_is_wrong_in_the_markup_is_said_where() {
    let (_, d) = read_page("<Window class=\"m.M\">\n  <Buton at=\"1,1\" span=\"2\" />\n</Window>\n", "M.cxaml");
    assert!(d.iter().any(|d| d.message.contains("<Buton> is not a part Clarity knows") && d.span.line == 2 && d.path == "M.cxaml"), "{d:#?}");
    let (_, d) = read_page("<Window class=\"m.M\">\n  <Button at=\"1,1\" span=\"2\" colour=\"red\" />\n</Window>\n", "M.cxaml");
    assert!(d.iter().any(|d| d.message.contains("<Button> has no 'colour'")), "{d:#?}");
    let (_, d) = read_page("<Window class=\"m.M\">\n  <Data name=\"x\" type=\"lst<\" />\n</Window>\n", "M.cxaml");
    assert!(d.iter().any(|d| d.span.line == 2), "a data line's error points at its element: {d:#?}");
}

/// Every example with its pages moved into `.cxaml` loads as it did: the
/// same diagnostics, and the same windows once its views are placed.
#[test]
fn every_example_with_its_pages_in_cxaml_loads_as_before() {
    for folder in module_folders() {
        let paths = clarity_spec::spec_files(&folder).unwrap();
        if paths.is_empty() {
            continue;
        }
        let files: Vec<(String, String)> = paths.iter().map(|p| (p.display().to_string(), std::fs::read_to_string(p).unwrap())).collect();
        let before = clarity_spec::load_converted(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
        let moved = clarity_spec::cxaml::moved_out(&files).unwrap();
        let mut after_files = files.clone();
        for (path, text) in &moved {
            match after_files.iter_mut().find(|(p, _)| p == path) {
                Some(file) => file.1 = text.clone(),
                None => after_files.push((path.clone(), text.clone())),
            }
        }
        let after = clarity_spec::load_converted(after_files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
        let messages = |p: &clarity_spec::Project| {
            let mut m: Vec<String> = p.diagnostics.iter().map(|d| format!("{:?} {}", d.severity, d.message)).collect();
            m.sort();
            m
        };
        assert_eq!(messages(&before), messages(&after), "{}", folder.display());
        let pages = |p: &clarity_spec::Project| {
            let placed = clarity_spec::views::placed(p);
            let mut pages: Vec<Value> = placed.files.iter().flat_map(|f| &f.pages).map(|page| without_places(&serde_json::to_value(page).unwrap())).collect();
            pages.sort_by_key(|v| v.to_string());
            pages
        };
        assert!(pages(&before) == pages(&after), "{}: the placed windows differ", folder.display());
        assert!(moved.iter().all(|(p, t)| !p.ends_with(".clarity") || !t.contains("\npage ")), "{}: a page block was left", folder.display());
        assert!(moved.iter().all(|(p, t)| !p.ends_with(".clarity") || !t.contains("\napp ")), "{}: an app block was left", folder.display());
    }
}

mod designer_edits {
    use clarity_spec::cxaml::{apply, read_page};
    use clarity_spec::edit::Edit;
    use clarity_spec::widgets::{new_props, WidgetKind};

    const TOTAL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- What all sales bring in. -->\n<View class=\"dashboard.TotalView\">\n  <Data name=\"total\" type=\"decimal\" />\n  <Heading name=\"totalHeading\" at=\"1,1\" span=\"12\" caption=\"@totalHeading\" />\n  <Display name=\"totalBox\" at=\"1,2\" span=\"12\" rows=\"3\" bind=\"total\" digits=\"9\" />\n</View>\n";

    fn page(markup: &str) -> clarity_spec::model::Page {
        let (page, d) = read_page(markup, "");
        assert!(d.is_empty(), "{d:#?}");
        page.unwrap()
    }

    #[test]
    fn a_part_resized_in_the_designer_changes_its_element() {
        let display = page(TOTAL).widgets[1].clone();
        let mut props = display.props.clone();
        props.width = 6;
        props.height = 4;
        let done = apply(None, TOTAL, &Edit::UpdateWidget { item: display.span.start, container: None, props }).unwrap();
        assert!(done.text.contains("<Display name=\"totalBox\" at=\"1,2\" span=\"6\" rows=\"4\""), "{}", done.text);
        assert!(done.text.contains("<!-- What all sales bring in. -->"), "the comment stays: {}", done.text);
        let focus = done.focus.expect("a focus") as usize;
        assert!(done.text[focus..].starts_with("<Display"), "the focus is the part: {}", &done.text[focus..]);
    }

    #[test]
    fn a_part_placed_removed_and_the_title_set() {
        let view = page(TOTAL);
        let mut label = new_props(WidgetKind::Label, 1, 6);
        label.caption = Some("Total so far".into());
        let placed = apply(None, TOTAL, &Edit::PlaceWidget { container: view.span.start, props: label }).unwrap();
        let after = page(&placed.text);
        assert_eq!(after.widgets.len(), 3);
        assert_eq!(after.widgets[2].props.row, 6);
        let removed = apply(None, &placed.text, &Edit::Remove { item: after.widgets[0].span.start }).unwrap();
        assert_eq!(page(&removed.text).widgets.iter().map(|w| w.props.name.clone()).collect::<Vec<_>>()[0].as_deref(), Some("totalBox"));
        let titled = apply(None, TOTAL, &Edit::SetTitle { target: view.span.start, title: "@totalTitle".into() }).unwrap();
        assert_eq!(page(&titled.text).title.as_deref(), Some("@totalTitle"));
    }

    #[test]
    fn what_belongs_in_the_module_is_refused() {
        let err = apply(None, TOTAL, &Edit::AddEntity { name: "Thing".into() }).unwrap_err();
        assert!(err.0.contains("belongs in the module's file"), "{err}");
    }
}

/// Renaming an entity renames it in the windows' and views' `.cxaml` too.
#[test]
fn a_rename_reaches_the_cxaml_pages() {
    let module = "module shop\n\nentity Sale\n    amount: decimal(10, 2), label @amountLabel\n\ncode Sales\n    handler opened\n        sales = find s in Sale\n";
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- Every sale. -->\n<Window class=\"shop.Sales\" onopen=\"opened\">\n  <Data name=\"sales\" type=\"list&lt;Sale&gt;\" />\n  <Table name=\"salesGrid\" at=\"1,1\" span=\"12\" rows=\"6\" bind=\"sales\" columns=\"amount\" />\n</Window>\n";
    let files = [("shop/shop.clarity", module), ("shop/design/Sales.cxaml", markup)];
    let project = clarity_spec::load_converted(files);
    let source = |path: &str| files.iter().find(|(p, _)| *p == path).map(|(_, t)| t.to_string());
    let renamed = clarity_spec::rename::rename(&project, source, clarity_spec::rename::RenameKind::Entity, "Sale", "Purchase").unwrap();
    let page = renamed.iter().find(|r| r.path.ends_with("Sales.cxaml")).expect("the window's file is renamed too");
    assert!(page.text.contains("type=\"list&lt;Purchase&gt;\""), "{}", page.text);
    assert!(page.text.contains("<!-- Every sale. -->"), "{}", page.text);
}

/// Every example's windows and views, in `design/`, read and
/// written again are the same file: what the designer writes back is what
/// was there.
#[test]
fn every_example_cxaml_writes_back_as_it_was() {
    let mut count = 0;
    for folder in module_folders() {
        let Ok(files) = std::fs::read_dir(folder.join(clarity_spec::cxaml::DESIGN_DIR)) else { continue };
        for entry in files.filter_map(Result::ok) {
            let path = entry.path();
            let text = std::fs::read_to_string(&path).unwrap();
            let designed = clarity_spec::cxaml::read_designed(&text, &path.display().to_string()).unwrap();
            assert_eq!(clarity_spec::cxaml::written_again(&text, &designed), text, "{}", path.display());
            count += 1;
        }
    }
    assert!(count > 95, "{count} files");
}

/// The Design text of a `.cxaml` is colored as markup, in Clarity's own kinds.
#[test]
fn a_cxaml_is_colored_as_markup() {
    use clarity_spec::highlight::{highlight_markup, PieceKind};
    let text = "<!-- The board. -->\n<Window class=\"dashboard.Board\" title=\"@boardTitle\" width=\"1200\" resizable=\"true\">\n  <Label caption=\"Hi {name}\" />\n</Window>";
    let pieces = highlight_markup(text);
    let kinds = |line: u32| pieces.iter().filter(|p| p.line == line).map(|p| (p.kind, p.from, p.to)).collect::<Vec<_>>();
    assert_eq!(kinds(1), [(PieceKind::Comment, 0, 19)]);
    let second = kinds(2);
    assert_eq!(second[0], (PieceKind::Type, 1, 7), "the element");
    assert!(second.contains(&(PieceKind::Name, 8, 13)), "class: {second:?}");
    assert!(second.contains(&(PieceKind::Key, 38, 51)), "the key, with its quotes: {second:?}");
    assert!(second.iter().any(|p| p.0 == PieceKind::Number), "{second:?}");
    assert!(second.iter().any(|p| p.0 == PieceKind::Literal), "{second:?}");
    assert!(kinds(3).iter().any(|p| p.0 == PieceKind::Slot), "{:?}", kinds(3));
}

/// Make texts keys reaches the windows' `.cxaml`: a caption written as text
/// becomes a key there, and the language file gets its text.
#[test]
fn make_texts_keys_reaches_the_cxaml_pages() {
    let module = "module shop\n\nentity Sale\n    amount: decimal(10, 2), label @amountLabel\n";
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- Every sale. -->\n<Window class=\"shop.Sales\" title=\"Sales\">\n  <Label name=\"totalLabel\" at=\"1,1\" span=\"4\" caption=\"Total so far\" />\n</Window>\n";
    let english = "translation en\n    amountLabel \"Amount\"\n";
    let keyed = clarity_spec::cxaml::make_keys_with_pages(&[("shop/shop.clarity", module)], &[("shop/design/Sales.cxaml", markup)], &[("shop/translations/en.clarity", english)], "en");
    let (path, page) = keyed.modules.iter().find(|(p, _)| p.ends_with("Sales.cxaml")).expect("the window's file");
    assert_eq!(path, "shop/design/Sales.cxaml");
    assert!(page.contains("<!-- Every sale. -->"), "{page}");
    assert!(!page.contains("\"Total so far\""), "the caption is a key now: {page}");
    let (read, d) = read_page(page, "");
    assert!(d.is_empty(), "{d:#?}");
    let read = read.unwrap();
    let caption = read.widgets[0].props.caption.clone().unwrap();
    let title = read.title.clone().unwrap();
    assert!(caption.starts_with('@') && title.starts_with('@'), "{caption} {title}");
    let en = &keyed.languages["en"];
    assert_eq!(en.get(&caption[1..]).map(String::as_str), Some("Total so far"));
    assert_eq!(en.get(&title[1..]).map(String::as_str), Some("Sales"));
}

/// Every app of every example and of the fixture: in its `design/` file,
/// or still written in a module's file.
fn all_apps() -> Vec<(String, clarity_spec::model::App)> {
    let mut out = Vec::new();
    for folder in module_folders() {
        for path in clarity_spec::spec_files(&folder).unwrap() {
            let text = std::fs::read_to_string(&path).unwrap();
            let (file, _) = clarity_spec::parser::parse(&text, "");
            out.extend(file.apps.into_iter().map(|a| (path.display().to_string(), a)));
        }
        for path in clarity_spec::cxaml::page_files(&folder, &[], &[]).unwrap() {
            let text = std::fs::read_to_string(&path).unwrap();
            if !is_app_markup(&text) {
                continue;
            }
            let (app, d) = read_app(&text, "");
            assert!(d.is_empty(), "{}: {d:#?}", path.display());
            out.push((path.display().to_string(), app.unwrap()));
        }
    }
    out
}

/// `got` is `want` read back, places aside; else where they first differ.
fn same_app(path: &str, want: &clarity_spec::model::App, got: &clarity_spec::model::App, written: &str) {
    let (want_json, got_json) = (without_places(&serde_json::to_value(want).unwrap()), without_places(&serde_json::to_value(got).unwrap()));
    if want_json != got_json {
        let (w, g) = (serde_json::to_string_pretty(&want_json).unwrap(), serde_json::to_string_pretty(&got_json).unwrap());
        let first = w.lines().zip(g.lines()).position(|(a, b)| a != b).unwrap_or(0);
        panic!("{path} {} differs from line {first}:\nwant: {}\ngot:  {}\n{written}", want.name.name, w.lines().skip(first.saturating_sub(3)).take(8).collect::<Vec<_>>().join("\n"), g.lines().skip(first.saturating_sub(3)).take(8).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn every_app_reads_back_from_its_cxaml_as_it_was() {
    let apps = all_apps();
    for (path, app) in &apps {
        let markup = app_source(app, "m");
        assert!(markup.contains(&format!("<App class=\"m.{}\"", app.name.name)), "{markup}");
        let (read, diagnostics) = read_app(&markup, "App.cxaml");
        assert!(diagnostics.is_empty(), "{path} {}: {diagnostics:#?}\n{markup}", app.name.name);
        same_app(path, app, &read.expect("an app"), &markup);
    }
    assert!(apps.len() > 15, "{} apps", apps.len());
}

const SHOP: &str = "module shop\n\n// The shop: its frame and where it starts.\napp Shop\n    title @shopTitle\n    main Desk\n    splash Welcome for 1.5 seconds\n    data session: Session\n    on start: appStarted\n    menu name mainMenu\n        item @fileItem name fileItem\n            item @quitItem name quitItem key \"Ctrl+Q\" on click -> quit\n    footer\n        status @shopStatus\n\nentity Session\n    user: text, label @userLabel\n\npage Desk: window\n    title @deskTitle\n\npage Welcome: splash\n    label @welcomeLabel name welcomeLabel at 1,1 span 4\n\ncode Shop\n    handler appStarted\n        session.user = \"\"\n";

/// An app moves out of its module into `design/<App>.cxaml`, as a window
/// does, with the comment above it; the module keeps its code.
#[test]
fn an_app_moves_into_its_own_cxaml() {
    let moved = clarity_spec::cxaml::moved_out(&[("shop/shop.clarity".to_string(), SHOP.to_string())]).unwrap();
    let (_, app) = moved.iter().find(|(p, _)| p == "shop/design/Shop.cxaml").expect("the app's file");
    assert!(app.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- The shop: its frame and where it starts. -->\n<App class=\"shop.Shop\" title=\"@shopTitle\" main=\"Desk\" splash=\"Welcome\" splashFor=\"1.5\" onstart=\"appStarted\">\n"), "{app}");
    assert!(app.contains("  <Data name=\"session\" type=\"Session\" />\n"), "{app}");
    assert!(app.contains("<Menu name=\"mainMenu\">") && app.contains("<Footer status=\"@shopStatus\""), "{app}");
    let (_, module) = moved.iter().find(|(p, _)| p == "shop/shop.clarity").expect("the module");
    assert!(!module.contains("app Shop") && !module.contains("The shop: its frame"), "{module}");
    assert!(module.contains("code Shop\n    handler appStarted"), "the app's code stays: {module}");
    let project = clarity_spec::load_converted(moved.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let before = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    let messages = |p: &clarity_spec::Project| p.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>();
    assert_eq!(messages(&project), messages(&before));
    let app = project.apps().next().expect("the app is loaded");
    assert_eq!((app.main.as_ref().unwrap().name.as_str(), app.splash_for), ("Desk", Some(1500)));
    assert_eq!(clarity_spec::cxaml::folded(&project).files.iter().filter(|f| !f.apps.is_empty()).map(|f| f.path.as_str()).collect::<Vec<_>>(), ["shop/shop.clarity"], "folded, the app is its module's again");
}

/// An app left in a module's file is an error, as a page is.
#[test]
fn an_app_left_in_a_module_is_an_error() {
    let project = clarity_spec::load_sources([("shop/shop.clarity", SHOP)]);
    let left = clarity_spec::cxaml::pages_left_in_modules(&project);
    assert!(left.iter().any(|d| d.message.starts_with("Shop ") && d.message.contains(clarity_spec::cxaml::APP_LEFT_IN_MODULE)), "{left:#?}");
}

/// What is wrong in an app's markup is said where.
#[test]
fn what_is_wrong_in_an_app_is_said_where() {
    let (_, d) = read_app("<App title=\"@t\">\n</App>\n", "Shop.cxaml");
    assert!(d.iter().any(|d| d.message.contains("class=\"shop.Shop\"")), "{d:#?}");
    let (_, d) = read_app("<App class=\"shop.Shop\" splash=\"Welcome\" splashFor=\"90\">\n</App>\n", "Shop.cxaml");
    assert!(d.iter().any(|d| d.message.contains("0.1 to 30")), "{d:#?}");
    let (_, d) = read_app("<App class=\"shop.Shop\" colour=\"red\">\n  <Buton at=\"1,1\" />\n</App>\n", "Shop.cxaml");
    assert!(d.iter().any(|d| d.message.contains("<App> has no 'colour'")), "{d:#?}");
    assert!(d.iter().any(|d| d.message.contains("<Buton>") && d.span.line == 2), "{d:#?}");
    let (_, d) = read_page("<App class=\"shop.Shop\">\n</App>\n", "Shop.cxaml");
    assert!(d.iter().any(|d| d.message.contains("<App>")), "a page's reader says an app is no page: {d:#?}");
}


/// Make texts keys reaches the app's `.cxaml` too: a menu item written as
/// text becomes a key there, and the language file gets its text.
#[test]
fn make_texts_keys_reaches_the_app_cxaml() {
    let module = "module shop\n\ncode Shop\n    handler appStarted\n        return\n";
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- The frame. -->\n<App class=\"shop.Shop\" title=\"Shop\" onstart=\"appStarted\">\n  <Menu name=\"mainMenu\">\n    <Item caption=\"File\" name=\"fileItem\" />\n  </Menu>\n</App>\n";
    let keyed = clarity_spec::cxaml::make_keys_with_pages(&[("shop/shop.clarity", module)], &[("shop/design/Shop.cxaml", markup)], &[("shop/translations/en.clarity", "translation en\n")], "en");
    let (_, app) = keyed.modules.iter().find(|(p, _)| p == "shop/design/Shop.cxaml").expect("the app's file");
    assert!(app.contains("<!-- The frame. -->") && app.contains("<App class=\"shop.Shop\""), "{app}");
    let (read, d) = read_app(app, "");
    assert!(d.is_empty(), "{d:#?}");
    let read = read.unwrap();
    let caption = read.frame.menu.unwrap().items[0].props.caption.clone().unwrap();
    assert!(caption.starts_with('@') && read.title.as_deref().is_some_and(|t| t.starts_with('@')), "{app}");
    assert_eq!(keyed.languages["en"].get(&caption[1..]).map(String::as_str), Some("File"));
}

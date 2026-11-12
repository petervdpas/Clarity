//! The designer's edits made on the model (`design_edit`) do what they did
//! when made through the page's text of Clarity: every edit the designer
//! makes, on every thing of every example's windows and views, gives the
//! same file and the same focus both ways.

use std::path::{Path, PathBuf};

use clarity_spec::cxaml::{apply, read_app, read_page, is_app_markup};
use clarity_spec::edit::{Edit, FrameItem};
use clarity_spec::frame::RegionKind;
use clarity_spec::model::{Command, Frame, Side, Widget};
use clarity_spec::widgets::{Event, WidgetKind};

fn design_files() -> Vec<(PathBuf, String)> {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&examples).unwrap().flatten() {
        let design = entry.path().join("design");
        let Ok(files) = std::fs::read_dir(&design) else { continue };
        for file in files.flatten() {
            if file.path().extension().is_some_and(|e| e == "cxaml") {
                out.push((file.path(), std::fs::read_to_string(file.path()).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn frame_edits(frame: &Frame, owner: u32, out: &mut Vec<Edit>) {
    fn commands(items: &[Command], out: &mut Vec<Edit>) {
        for c in items {
            out.push(Edit::Remove { item: c.span.start });
            let mut props = c.props.clone();
            props.caption = props.caption.map(|c| format!("{c} more"));
            out.push(Edit::UpdateCommand { item: c.span.start, props });
            let mut nameless = c.props.clone();
            nameless.name = None;
            out.push(Edit::UpdateCommand { item: c.span.start, props: nameless });
            out.push(Edit::MoveSibling { item: c.span.start, delta: -1 });
            out.push(Edit::MoveSibling { item: c.span.start, delta: 1 });
            out.push(Edit::AddFrameItem { parent: c.span.start, item: FrameItem::Item, caption: Some("Sub".into()) });
            commands(&c.items, out);
        }
    }
    for region in [RegionKind::Menu, RegionKind::Toolbar, RegionKind::Ribbon, RegionKind::Footer] {
        out.push(Edit::AddRegion { owner, region, side: None });
    }
    for side in [Side::Left, Side::Right] {
        out.push(Edit::AddRegion { owner, region: RegionKind::Sidebar, side: Some(side) });
    }
    if let Some(menu) = &frame.menu {
        out.push(Edit::Remove { item: menu.span.start });
        out.push(Edit::SetName { item: menu.span.start, name: "otherMenu".into() });
        out.push(Edit::AddFrameItem { parent: menu.span.start, item: FrameItem::Item, caption: None });
        out.push(Edit::AddFrameItem { parent: menu.span.start, item: FrameItem::Separator, caption: None });
        commands(&menu.items, out);
    }
    if let Some(toolbar) = &frame.toolbar {
        out.push(Edit::Remove { item: toolbar.span.start });
        out.push(Edit::SetName { item: toolbar.span.start, name: "otherToolbar".into() });
        out.push(Edit::AddFrameItem { parent: toolbar.span.start, item: FrameItem::Button, caption: Some("Go".into()) });
        commands(&toolbar.items, out);
    }
    if let Some(ribbon) = &frame.ribbon {
        out.push(Edit::Remove { item: ribbon.span.start });
        out.push(Edit::UpdateRibbon { item: ribbon.span.start, name: None, collapsible: true, collapsed: true });
        out.push(Edit::AddFrameItem { parent: ribbon.span.start, item: FrameItem::Tab, caption: Some("View".into()) });
        for tab in &ribbon.tabs {
            out.push(Edit::Remove { item: tab.span.start });
            out.push(Edit::SetTitle { target: tab.span.start, title: "Start".into() });
            out.push(Edit::SetName { item: tab.span.start, name: "startTab".into() });
            out.push(Edit::MoveSibling { item: tab.span.start, delta: 1 });
            out.push(Edit::AddFrameItem { parent: tab.span.start, item: FrameItem::Group, caption: None });
            for group in &tab.groups {
                out.push(Edit::Remove { item: group.span.start });
                out.push(Edit::SetTitle { target: group.span.start, title: "Edit".into() });
                out.push(Edit::AddFrameItem { parent: group.span.start, item: FrameItem::Button, caption: None });
                commands(&group.items, out);
            }
        }
    }
    for sidebar in &frame.sidebars {
        out.push(Edit::Remove { item: sidebar.span.start });
        let mut props = sidebar.props.clone();
        props.name = None;
        out.push(Edit::UpdateSidebar { item: sidebar.span.start, props });
        out.push(Edit::CreateWidget { container: sidebar.span.start, kind: WidgetKind::Button, col: 1, row: 20, width: None, caption: None, bind: None, part: None, height: None, input: None, strategy: None, shows: None });
        widget_edits(&sidebar.widgets, out);
    }
    if let Some(footer) = &frame.footer {
        out.push(Edit::Remove { item: footer.span.start });
        widget_edits(&footer.widgets, out);
    }
    out.push(Edit::NameParts);
    out.push(Edit::UpdateSidebars);
}

fn widget_edits(widgets: &[Widget], out: &mut Vec<Edit>) {
    for w in widgets {
        out.push(Edit::Remove { item: w.span.start });
        let mut moved = w.props.clone();
        moved.col = 1;
        out.push(Edit::UpdateWidget { item: w.span.start, container: None, props: moved });
        let mut nameless = w.props.clone();
        nameless.name = None;
        out.push(Edit::UpdateWidget { item: w.span.start, container: None, props: nameless });
        out.push(Edit::AddMenu { item: w.span.start });
        out.push(Edit::AddTab { item: w.span.start, title: "ExtraView".into() });
        out.push(Edit::CreateWidget { container: w.span.start, kind: WidgetKind::Label, col: 1, row: 20, width: Some(2), caption: Some("@extraCaption".into()), bind: None, part: None, height: None, input: None, strategy: None, shows: None });
        if let Some(menu) = &w.menu {
            out.push(Edit::Remove { item: menu.span.start });
            out.push(Edit::AddFrameItem { parent: menu.span.start, item: FrameItem::Item, caption: None });
        }
        for inner in &w.pages {
            out.push(Edit::Remove { item: inner.span.start });
            out.push(Edit::SetTitle { target: inner.span.start, title: "Renamed".into() });
            out.push(Edit::SetPaneSpan { target: inner.span.start, columns: Some(3) });
            out.push(Edit::SetPageView { target: inner.span.start, view: Some("OtherView".into()) });
            out.push(Edit::SetViewTell { target: inner.span.start, tell: "saved".into(), handler: Some("otherSaved".into()) });
            out.push(Edit::SetPageColor { target: inner.span.start, color: Some("#ff0000".into()) });
            out.push(Edit::CreateWidget { container: inner.span.start, kind: WidgetKind::Button, col: 1, row: 20, width: None, caption: None, bind: None, part: None, height: None, input: None, strategy: None, shows: None });
            // A part moved into the page of another.
            if let Some(first) = widgets.iter().find(|o| o.span.start != w.span.start) {
                out.push(Edit::UpdateWidget { item: first.span.start, container: Some(inner.span.start), props: first.props.clone() });
            }
            widget_edits(&inner.children, out);
        }
        widget_edits(&w.children, out);
    }
}

/// Every edit the designer makes, on every thing of what the file holds.
fn edits_of(markup: &str) -> Vec<Edit> {
    let mut out = Vec::new();
    if is_app_markup(markup) {
        let app = read_app(markup, "").0.unwrap();
        let at = app.span.start;
        out.push(Edit::SetTitle { target: at, title: "Other".into() });
        out.push(Edit::SetTitle { target: at, title: String::new() });
        out.push(Edit::SetMain { app: at, main: "Other".into() });
        out.push(Edit::SetSplash { app: at, splash: "Welcome".into(), ms: Some(1500) });
        out.push(Edit::SetSplash { app: at, splash: String::new(), ms: None });
        out.push(Edit::SetOwnerEvent { owner: at, event: Event::Start, handler: Some("started".into()) });
        out.push(Edit::SetOwnerEvent { owner: at, event: Event::Start, handler: None });
        out.push(Edit::AddData { page: at, name: "extra".into(), entity: "Thing".into(), list: true });
        frame_edits(&app.frame, at, &mut out);
        return out;
    }
    let page = read_page(markup, "").0.unwrap();
    let at = page.span.start;
    out.push(Edit::SetTitle { target: at, title: "Other".into() });
    out.push(Edit::SetTitle { target: at, title: String::new() });
    out.push(Edit::SetGrid { page: at, columns: 10, rows: None });
    out.push(Edit::SetGrid { page: at, columns: 10, rows: Some(12) });
    out.push(Edit::SetSize { page: at, width: Some(800), height: Some(600) });
    out.push(Edit::SetSize { page: at, width: None, height: None });
    out.push(Edit::SetPageStyle { page: at, style: "accent".into() });
    out.push(Edit::SetPageStyle { page: at, style: String::new() });
    out.push(Edit::SetPageTell { page: at, name: "saved".into(), message: Some("saved(name: text)".into()) });
    for tell in &page.tells {
        out.push(Edit::SetPageTell { page: at, name: tell.name.name.clone(), message: None });
    }
    out.push(Edit::AddData { page: at, name: "extra".into(), entity: "Thing".into(), list: true });
    out.push(Edit::SetOwnerEvent { owner: at, event: Event::Open, handler: Some("opened".into()) });
    out.push(Edit::SetOwnerEvent { owner: at, event: Event::Open, handler: None });
    out.push(Edit::CreateWidget { container: at, kind: WidgetKind::Tabs, col: 1, row: 30, width: Some(4), caption: None, bind: None, part: None, height: None, input: None, strategy: None, shows: None });
    out.push(Edit::CreateWidget { container: at, kind: WidgetKind::Button, col: 3, row: 1, width: None, caption: Some("Go".into()), bind: None, part: None, height: None, input: None, strategy: None, shows: None });
    for data in page.data.iter().filter(|d| !d.made) {
        out.push(Edit::Remove { item: data.span.start });
    }
    frame_edits(&page.frame, at, &mut out);
    widget_edits(&page.widgets, &mut out);
    out
}

/// Every edit the designer makes, on every thing of every example's windows,
/// views and apps, is refused with a reason or writes a file that reads back
/// clean, the same each time, its focus on a thing of the file. (Until windows
/// moved into `.cxaml` files each gave what the edit through the page's
/// text gave, 7764 of them.)
#[test]
fn every_edit_writes_a_file_that_reads_back_clean() {
    let (mut made, mut refused) = (0, 0);
    for (path, markup) in design_files() {
        for edit in edits_of(&markup) {
            let Ok(done) = apply(None, &markup, &edit) else {
                refused += 1;
                continue;
            };
            made += 1;
            let again = apply(None, &markup, &edit).unwrap();
            assert_eq!((&done.text, done.focus), (&again.text, again.focus), "{}: {edit:?} twice", path.display());
            let designed = clarity_spec::cxaml::read_designed(&done.text, "");
            let read = if is_app_markup(&done.text) { read_app(&done.text, "").1 } else { read_page(&done.text, "").1 };
            assert!(designed.is_ok() && read.is_empty(), "{}: {edit:?}\n{read:#?}\n{}", path.display(), done.text);
            if let Some(at) = done.focus {
                let keys = match designed.unwrap() {
                    clarity_spec::design_edit::Designed::Page(page) => clarity_spec::design_keys::page_keys("", &page),
                    clarity_spec::design_edit::Designed::App(app) => clarity_spec::design_keys::app_keys("", &app),
                };
                assert!(keys.iter().any(|(start, _)| *start == at), "{}: {edit:?} focuses {at}, no thing of the file\n{}", path.display(), done.text);
            }
        }
    }
    assert!(made > 5000 && refused > 100, "{made} made, {refused} refused");
}

/// A footer's status is set on the model; through the text it was never found.
#[test]
fn a_footers_status_takes_its_new_text() {
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Window class=\"m.Main\">\n  <Footer status=\"@readyStatus\" columns=\"12\" />\n</Window>\n";
    let page = read_page(markup, "").0.unwrap();
    let status = page.frame.footer.unwrap().status_span.unwrap();
    let done = apply(None, markup, &Edit::SetTitle { target: status.start, title: "@busyStatus".into() }).unwrap();
    assert!(done.text.contains("status=\"@busyStatus\""), "{}", done.text);
    // The status is known by its value's place, as it is read.
    assert!(done.focus.is_some_and(|at| done.text[at as usize..].starts_with("@busyStatus")), "{}", done.text);
}

/// With the project: the texts an edit writes become keys, and a field of
/// the window's data is placed with its label, each file reading back clean
/// with its new keys.
#[test]
fn with_the_project_texts_become_keys_and_fields_are_placed() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let (mut keyed, mut fields) = (0, 0);
    for example in ["todo", "records", "shop"] {
        let dir = examples.join(example);
        let project = clarity_project_load(&dir);
        for (path, markup) in design_files().into_iter().filter(|(p, _)| p.starts_with(&dir)) {
            if is_app_markup(&markup) {
                let app = read_app(&markup, "").0.unwrap();
                // A region the app has not yet: its starting texts become keys.
                let free = [(RegionKind::Menu, app.frame.menu.is_none()), (RegionKind::Toolbar, app.frame.toolbar.is_none()), (RegionKind::Ribbon, app.frame.ribbon.is_none()), (RegionKind::Footer, app.frame.footer.is_none())];
                let Some((region, _)) = free.into_iter().find(|(_, free)| *free) else { continue };
                let done = apply(Some(&project), &markup, &Edit::AddRegion { owner: app.span.start, region, side: None }).unwrap();
                assert!(!done.keys.is_empty() && !done.text.contains("caption=\"File\"") && !done.text.contains("caption=\"Button\""), "{}: the region's texts are keys\n{}", path.display(), done.text);
                keyed += 1;
                continue;
            }
            let page = read_page(&markup, "").0.unwrap();
            let at = page.span.start;
            let done = apply(Some(&project), &markup, &Edit::CreateWidget { container: at, kind: WidgetKind::Button, col: 1, row: 30, width: None, caption: Some("Go there".into()), bind: None, part: None, height: None, input: None, strategy: None, shows: None }).unwrap();
            let (key, text) = done.keys.first().unwrap_or_else(|| panic!("{}: the caption is a key\n{}", path.display(), done.text));
            assert!(text == "Go there" && done.text.contains(&format!("caption=\"@{key}\"")), "{}: {key} {text}\n{}", path.display(), done.text);
            keyed += 1;
            for data in &page.data {
                let clarity_spec::model::DataKind::Record { entity } = &data.of else { continue };
                for field in project.entity(&entity.name).map(|e| e.fields.clone()).unwrap_or_default() {
                    let bind = format!("{}.{}", data.name.name, field.name.name);
                    let Ok(done) = apply(Some(&project), &markup, &Edit::PlaceField { container: at, col: 1, row: 40, room: 12, bind: bind.clone() }) else { continue };
                    assert!(done.text.contains(&format!("bind=\"{bind}\"")) && read_page(&done.text, "").1.is_empty(), "{}: {bind}\n{}", path.display(), done.text);
                    fields += 1;
                }
            }
        }
    }
    assert!(keyed > 10 && fields > 10, "{keyed} keyed, {fields} fields placed");
}

fn clarity_project_load(dir: &Path) -> clarity_spec::Project {
    let mut sources = Vec::new();
    let mut add = |folder: &Path| {
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "clarity" || e == "cxaml") {
                sources.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    };
    add(dir);
    add(&dir.join("design"));
    let translations: Vec<(String, String)> = std::fs::read_dir(dir.join("translations")).into_iter().flatten().flatten().map(|e| (e.path().display().to_string(), std::fs::read_to_string(e.path()).unwrap())).collect();
    let mut project = clarity_spec::load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    clarity_spec::translations::attach(&mut project, translations.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    project
}

/// Wiring an event on a window's part is two edits: the part's handler in
/// its `.cxaml`, the handler's code in the module's file.
#[test]
fn a_wiring_is_split_between_the_design_file_and_the_module() {
    use clarity_spec::design_edit::split;
    let wire = Edit::Batch { edits: vec![Edit::SetOwnerEvent { owner: 10, event: Event::Open, handler: Some("opened".into()) }, Edit::AddHandler { owner: "Main".into(), name: "opened".into() }] };
    let (design, code) = split(&wire);
    assert!(matches!(design, Some(Edit::SetOwnerEvent { .. })), "{design:?}");
    assert!(matches!(code, Some(Edit::AddHandler { .. })), "{code:?}");
    assert_eq!(split(&Edit::AddEntity { name: "Thing".into() }).0, None);
    assert!(split(&Edit::NameParts).1.is_none());
}

/// What the project works out when it loads (a part's undo scope, a graph's
/// model) comes back with the part's properties in a designer's edit, and
/// is never written into the source.
#[test]
fn an_edit_writes_no_worked_out_property_into_the_source() {
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Window class=\"m.Main\">\n  <Graph name=\"map\" at=\"1,1\" span=\"6\" rows=\"6\" lists=\"people\" />\n</Window>\n";
    let page = read_page(markup, "").0.unwrap();
    let graph = &page.widgets[0];
    let mut props = graph.props.clone();
    props.undo_scope = Some("map".into());
    props.graph_model = Some(clarity_spec::model::GraphModel { lists: vec![clarity_spec::model::GraphList { list: "people".into(), entity: "Person".into(), label: None, refs: vec![] }] });
    props.legend = true;
    let done = apply(None, markup, &Edit::UpdateWidget { item: graph.span.start, container: None, props }).unwrap();
    assert!(done.text.contains("legend=\"true\""), "{}", done.text);
    assert!(!done.text.contains("undoScope") && !done.text.contains("graphModel"), "{}", done.text);
}

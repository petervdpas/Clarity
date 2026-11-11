//! The frame's edits (regions, menu items, ribbon tabs and groups, the
//! app's main window and splash) made on a window's or the app's `.cxaml`.

use clarity_spec::cxaml::{apply, read_app};
use clarity_spec::edit::{Edit, FrameItem};
use clarity_spec::frame::RegionKind;
use clarity_spec::model::{CommandKind, CommandProps, Side};

const PAGE: &str = "module m

page Card: window
    title \"Card\"
    data customer: Customer

    label \"Name\" name label1 at 1,1 span 2
";

const APP: &str = "module m

app Shop
    title \"Shop\"
    menu name mainMenu
        item \"File\"
            item \"Open\"
            item \"Quit\" on click -> quit
    footer
        status \"Ready\"
";

/// The `.cxaml` the page or app `name` of `source` becomes.
fn design(source: &str, name: &str) -> String {
    let file = format!("design/{name}.cxaml");
    clarity_spec::converted([("m/m.clarity", source)])
        .into_iter()
        .find(|(p, _)| p.ends_with(&file))
        .map(|(_, t)| t)
        .unwrap_or_else(|| panic!("no {file}"))
}

/// Where the element that starts with `needle` starts in `markup`.
fn at(markup: &str, needle: &str) -> u32 {
    markup.find(needle).unwrap_or_else(|| panic!("no {needle:?} in {markup}")) as u32
}

fn edit(markup: &str, edit: Edit) -> String {
    apply(None, markup, &edit).unwrap_or_else(|e| panic!("edit failed: {e}")).text
}

fn error(markup: &str, edit: Edit) -> String {
    apply(None, markup, &edit).expect_err("edit should fail").0
}

#[test]
fn docks_regions_after_the_data_in_catalog_order() {
    let page = design(PAGE, "Card");
    let text = edit(&page, Edit::AddRegion { owner: at(&page, "<Window"), region: RegionKind::Footer, side: None });
    let text = edit(&text, Edit::AddRegion { owner: at(&text, "<Window"), region: RegionKind::Menu, side: None });
    assert!(
        text.contains(
            "  <Data name=\"customer\" type=\"Customer\" />\n  <Menu name=\"mainMenu\">\n    <Item caption=\"File\" name=\"fileItem\">\n      <Item caption=\"Quit\" name=\"quitItem\" opens=\"quit\" />\n    </Item>\n  </Menu>\n  <Footer status=\"Ready\" columns=\"12\" />\n  <Label name=\"label1\""
        ),
        "{text}"
    );
}

#[test]
fn adds_a_sidebar_per_side() {
    let page = design(PAGE, "Card");
    let text = edit(&page, Edit::AddRegion { owner: at(&page, "<Window"), region: RegionKind::Sidebar, side: Some(Side::Right) });
    // A new sidebar is named after its side, with the default width and grid.
    assert!(text.contains("  <Sidebar side=\"right\" name=\"rightSidebar\">\n"), "{text}");
    let again = Edit::AddRegion { owner: at(&text, "<Window"), region: RegionKind::Sidebar, side: Some(Side::Right) };
    assert!(error(&text, again).contains("already has a right sidebar"));
    let left = edit(&text, Edit::AddRegion { owner: at(&text, "<Window"), region: RegionKind::Sidebar, side: Some(Side::Left) });
    let mut props = clarity_spec::frame::new_sidebar(Side::Right, Some("leftSidebar".into()));
    let moved = Edit::UpdateSidebar { item: at(&left, "<Sidebar side=\"left\""), props: props.clone() };
    assert!(error(&left, moved).contains("already a right sidebar"));
    props.side = Side::Left;
    props.width = 320;
    props.resizable = true;
    props.min = Some(200);
    props.columns = 4;
    let wider = edit(&left, Edit::UpdateSidebar { item: at(&left, "<Sidebar side=\"left\""), props });
    assert!(wider.contains("  <Sidebar side=\"left\" name=\"leftSidebar\" width=\"320\" min=\"200\" resizable=\"true\" columns=\"4\">\n"), "{wider}");
}

#[test]
fn adds_menu_items_and_submenus() {
    let app = design(APP, "Shop");
    let text = edit(&app, Edit::AddFrameItem { parent: at(&app, "<Item caption=\"Open\""), item: FrameItem::Item, caption: Some("Recent".into()) });
    assert!(text.contains("      <Item caption=\"Open\">\n        <Item caption=\"Recent\" name=\"recentItem\" />\n      </Item>\n"), "{text}");
    let menu = at(&app, "<Menu");
    let text = edit(&app, Edit::AddFrameItem { parent: menu, item: FrameItem::Separator, caption: None });
    assert!(text.contains("      <Item caption=\"Quit\" opens=\"quit\" />\n    </Item>\n    <Item kind=\"separator\" />\n  </Menu>\n"), "{text}");
    let wrong = Edit::AddFrameItem { parent: menu, item: FrameItem::Button, caption: None };
    assert!(error(&app, wrong).contains("cannot go there"));
}

#[test]
fn rewrites_a_command() {
    let app = design(APP, "Shop");
    let props = CommandProps {
        kind: CommandKind::Item,
        caption: Some("Open...".into()),
        name: None,
        icon: None,
        shortcut: Some("Ctrl+O".into()),
        large: false,
        on_click: Some("Card".into()),
        new_window: false,
        open_with: Vec::new(),
        handler: None,
        allow: None,
        allow_condition: None,
    };
    let text = edit(&app, Edit::UpdateCommand { item: at(&app, "<Item caption=\"Open\""), props });
    // An item written without a name gets one, after its caption.
    assert!(text.contains("      <Item caption=\"Open...\" name=\"openItem\" key=\"Ctrl+O\" opens=\"Card\" />\n"), "{text}");
}

#[test]
fn moves_commands_among_their_siblings() {
    let app = design(APP, "Shop");
    let text = edit(&app, Edit::MoveSibling { item: at(&app, "<Item caption=\"Quit\""), delta: -1 });
    assert!(text.contains("      <Item caption=\"Quit\" opens=\"quit\" />\n      <Item caption=\"Open\" />\n"), "{text}");
    // Already first: nothing changes.
    assert_eq!(edit(&app, Edit::MoveSibling { item: at(&app, "<Item caption=\"File\""), delta: -1 }), app);
}

#[test]
fn renames_status_and_removes_regions() {
    let app = design(APP, "Shop");
    let (read, _) = read_app(&app, "");
    let status = read.unwrap().frame.footer.unwrap().status_span.expect("the status's place").start;
    let text = edit(&app, Edit::SetTitle { target: status, title: "Idle".into() });
    assert!(text.contains("  <Footer status=\"Idle\" columns=\"12\" />\n"), "{text}");
    let text = edit(&app, Edit::Remove { item: at(&app, "<Footer") });
    assert!(!text.contains("<Footer"), "{text}");
}

#[test]
fn builds_a_ribbon() {
    let app = design(APP, "Shop");
    let text = edit(&app, Edit::AddRegion { owner: at(&app, "<App"), region: RegionKind::Ribbon, side: None });
    assert!(
        text.contains("  <Ribbon name=\"mainRibbon\">\n    <Tab title=\"Home\" name=\"homeTab\">\n      <Group title=\"Actions\" name=\"actionsGroup\">\n        <Item kind=\"button\" caption=\"Button\" name=\"buttonButton\" />\n      </Group>\n    </Tab>\n  </Ribbon>\n  <Footer"),
        "{text}"
    );
    let text = edit(&text, Edit::AddFrameItem { parent: at(&text, "<Tab title=\"Home\""), item: FrameItem::Group, caption: Some("Print".into()) });
    assert!(text.contains("      </Group>\n      <Group title=\"Print\" name=\"printGroup\">\n      </Group>\n    </Tab>\n  </Ribbon>\n  <Footer"), "{text}");
}

/// A new app is a file of its own in design/, titled after its name; its
/// main window is set on its `.cxaml`.
#[test]
fn adds_an_app_and_sets_its_main_window() {
    let module = std::path::Path::new("m.clarity");
    let project = clarity_spec::load_sources([("m.clarity", "module m\n")]);
    let added = clarity_spec::cxaml::added(&project, module, "module m\n", &Edit::AddApp { name: "shop".into() }).unwrap().unwrap();
    assert_eq!(added.files[0].0, std::path::Path::new("design/shop.cxaml"));
    let app = added.files[0].1.replace("@shopTitle", "Shop");
    assert!(app.contains("<App class=\"m.shop\" title=\"Shop\">"), "{app}");
    let with_app = clarity_spec::load_sources([("m.clarity", "module m\n"), ("design/shop.cxaml", app.as_str())]);
    assert!(clarity_spec::cxaml::added(&with_app, module, "module m\n", &Edit::AddApp { name: "x".into() }).unwrap().unwrap_err().contains("already has an app"));
    let set = edit(&app, Edit::SetMain { app: at(&app, "<App"), main: "Card".into() });
    assert!(set.contains("<App class=\"m.shop\" title=\"Shop\" main=\"Card\">"), "{set}");
}

/// The designer names the app's splash page and how long it shows at least,
/// or takes the splash away: `splash="Welcome" splashFor="2"` on the app.
#[test]
fn sets_and_removes_the_app_s_splash() {
    let source = design("module m\n\napp shop\n    title \"Shop\"\n    main Card\n", "shop");
    let app = at(&source, "<App");
    let text = edit(&source, Edit::SetSplash { app, splash: "Welcome".into(), ms: Some(2000) });
    assert!(text.contains("<App class=\"m.shop\" title=\"Shop\" main=\"Card\" splash=\"Welcome\" splashFor=\"2\">"), "{text}");
    let text = edit(&text, Edit::SetSplash { app, splash: "Welcome".into(), ms: Some(1500) });
    assert!(text.contains(" splash=\"Welcome\" splashFor=\"1.5\">"), "{text}");
    let text = edit(&text, Edit::SetSplash { app, splash: "Welcome".into(), ms: None });
    assert!(text.contains(" main=\"Card\" splash=\"Welcome\">"), "{text}");
    let text = edit(&text, Edit::SetSplash { app, splash: String::new(), ms: None });
    assert_eq!(text, source, "no splash, no attribute");
    assert!(error(&source, Edit::SetSplash { app, splash: "Welcome".into(), ms: Some(40_000) }).contains("from 0.1 to 30"));
}

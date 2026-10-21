use clarity_spec::edit::{Edit, FrameItem, apply};
use clarity_spec::frame::RegionKind;
use clarity_spec::model::{CommandKind, CommandProps, Side};

const PAGE: &str = "module m

page Card: window
    title \"Card\"
    data customer: Customer

    // The name.
    label \"Name\" name label1 at 1,1 span 2
";

const APP: &str = "module m

app Shop
    title \"Shop\"
    menu
        item \"File\"
            item \"Open\" // opens a file
            item \"Quit\" on click -> quit
    footer
        status \"Ready\"
";

fn at(source: &str, needle: &str) -> u32 {
    let mut offset = 0;
    for line in source.split('\n') {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(needle) {
            return (offset + indent) as u32;
        }
        offset += line.len() + 1;
    }
    panic!("no line starts with {needle:?}")
}

fn edit(source: &str, edit: Edit) -> String {
    apply(source, &edit)
        .unwrap_or_else(|e| panic!("edit failed: {e}"))
        .text
}

fn error(source: &str, edit: Edit) -> String {
    apply(source, &edit).expect_err("edit should fail").0
}

#[test]
fn docks_regions_after_the_settings_in_catalog_order() {
    let page = at(PAGE, "page ");
    let text = edit(
        PAGE,
        Edit::AddRegion {
            owner: page,
            region: RegionKind::Footer,
            side: None,
        },
    );
    let text = edit(
        &text,
        Edit::AddRegion {
            owner: page,
            region: RegionKind::Menu,
            side: None,
        },
    );
    assert!(
        text.contains(
            "    data customer: Customer\n    menu\n        item \"File\"\n            item \"Quit\" on click -> quit\n    footer\n        status \"Ready\"\n\n    // The name.\n"
        ),
        "{text}"
    );
}

#[test]
fn adds_a_sidebar_per_side() {
    let page = at(PAGE, "page ");
    let text = edit(
        PAGE,
        Edit::AddRegion {
            owner: page,
            region: RegionKind::Sidebar,
            side: Some(Side::Right),
        },
    );
    assert!(text.contains("    sidebar right span 3\n"));
    let again = Edit::AddRegion {
        owner: page,
        region: RegionKind::Sidebar,
        side: Some(Side::Right),
    };
    assert!(error(&text, again).contains("already has a right sidebar"));
    let left = edit(
        &text,
        Edit::AddRegion {
            owner: page,
            region: RegionKind::Sidebar,
            side: Some(Side::Left),
        },
    );
    let moved = Edit::SetSidebar {
        item: at(&left, "sidebar left"),
        side: Side::Right,
        width: 4,
    };
    assert!(error(&left, moved).contains("already a right sidebar"));
    let wider = edit(
        &left,
        Edit::SetSidebar {
            item: at(&left, "sidebar left"),
            side: Side::Left,
            width: 4,
        },
    );
    assert!(wider.contains("    sidebar left span 4\n"));
}

#[test]
fn adds_menu_items_and_submenus() {
    let open = at(APP, "item \"Open\"");
    let text = edit(
        APP,
        Edit::AddFrameItem {
            parent: open,
            item: FrameItem::Item,
            caption: Some("Recent".into()),
        },
    );
    assert!(
        text.contains(
            "            item \"Open\" // opens a file\n                item \"Recent\" name recentItem\n"
        ),
        "{text}"
    );
    let menu = at(APP, "menu");
    let text = edit(
        APP,
        Edit::AddFrameItem {
            parent: menu,
            item: FrameItem::Separator,
            caption: None,
        },
    );
    assert!(text.contains("            item \"Quit\" on click -> quit\n        separator\n"));
    let wrong = Edit::AddFrameItem {
        parent: menu,
        item: FrameItem::Button,
        caption: None,
    };
    assert!(error(APP, wrong).contains("cannot go there"));
}

#[test]
fn rewrites_a_command_and_keeps_its_comment() {
    let props = CommandProps {
        kind: CommandKind::Item,
        caption: Some("Open...".into()),
        name: None,
        icon: None,
        shortcut: Some("Ctrl+O".into()),
        large: false,
        on_click: Some("Card".into()),
        new_window: false,
        handler: None,
    };
    let text = edit(
        APP,
        Edit::UpdateCommand {
            item: at(APP, "item \"Open\""),
            props,
        },
    );
    // An item written without a name gets one, after its caption.
    assert!(
        text.contains("            item \"Open...\" name openItem key \"Ctrl+O\" on click -> Card // opens a file\n"),
        "{text}"
    );
}

#[test]
fn moves_commands_among_their_siblings() {
    let text = edit(
        APP,
        Edit::MoveSibling {
            item: at(APP, "item \"Quit\""),
            delta: -1,
        },
    );
    assert!(text.contains(
        "            item \"Quit\" on click -> quit\n            item \"Open\" // opens a file\n"
    ));
    // Already first: nothing changes.
    assert_eq!(
        edit(
            APP,
            Edit::MoveSibling {
                item: at(APP, "item \"File\""),
                delta: -1
            }
        ),
        APP
    );
}

#[test]
fn renames_status_and_removes_regions() {
    let text = edit(
        APP,
        Edit::SetTitle {
            target: at(APP, "status"),
            title: "Idle".into(),
        },
    );
    assert!(text.contains("        status \"Idle\"\n"));
    let text = edit(
        APP,
        Edit::Remove {
            item: at(APP, "footer"),
        },
    );
    assert!(!text.contains("footer"));
}

#[test]
fn builds_a_ribbon() {
    let app = at(APP, "app ");
    let text = edit(
        APP,
        Edit::AddRegion {
            owner: app,
            region: RegionKind::Ribbon,
            side: None,
        },
    );
    assert!(text.contains("    ribbon\n        tab \"Home\"\n            group \"Group\"\n                button \"Button\"\n    footer"), "{text}");
    let tab = at(&text, "tab \"Home\"");
    let text = edit(
        &text,
        Edit::AddFrameItem {
            parent: tab,
            item: FrameItem::Group,
            caption: Some("Print".into()),
        },
    );
    assert!(text.contains("            group \"Print\"\n    footer"));
}

#[test]
fn adds_an_app_and_sets_its_main_window() {
    let text = edit(
        "module m\n",
        Edit::AddApp {
            name: "shop".into(),
        },
    );
    assert_eq!(text, "module m\n\napp shop\n    title \"Shop\"\n");
    let app = at(&text, "app ");
    let text = edit(
        &text,
        Edit::SetMain {
            app,
            main: "Card".into(),
        },
    );
    assert!(text.ends_with("    title \"Shop\"\n    main Card\n"));
    assert!(error(&text, Edit::AddApp { name: "x".into() }).contains("already has an app"));
}

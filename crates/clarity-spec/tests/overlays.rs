//! Context menus and popovers (DESIGN 5.16): a `menu` block under a part is
//! its context menu, written as the frame's menu items are; a `popover` is a
//! container part outside the window's grid, opened next to the part in its
//! `for` by `on click -> name` or by code through `name.visible`.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::CommandKind;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, ContainerKind, WidgetKind};
use clarity_spec::{Project, Severity};

const MUSIC: &str = "module music

entity Album
    title: text(120), required

page AlbumBrowse: window
    data albums: list<Album>
    data onlyVinyl: bool
    table name grid at 1,1 span 12 rows 6 bind albums columns title
        menu
            item \"Open\" name openItem on click: openClicked
            separator
            item \"Delete\" name deleteItem on click: deleteClicked
    button \"Filter\" name filterButton at 1,7 span 2 on click -> filters
    popover name filters for filterButton span 4 rows 2
        checkbox \"Only vinyl\" name vinylBox at 1,1 span 4 bind onlyVinyl on change: refilter
";

const CODE: &str = "
code AlbumBrowse
    handler openClicked
        filters.visible = true

    handler deleteClicked
        delete grid.selected
        reload albums

    handler refilter
        filters.visible = false
        deleteItem.enabled = onlyVinyl
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("music.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_menu_under_a_part_is_its_context_menu() {
    let (file, diagnostics) = parse(MUSIC, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let grid = &file.pages[0].widgets[0];
    let menu = grid.menu.as_ref().expect("the table has a context menu");
    let kinds: Vec<CommandKind> = menu.items.iter().map(|c| c.props.kind).collect();
    assert_eq!(kinds, vec![CommandKind::Item, CommandKind::Separator, CommandKind::Item]);
    assert_eq!(menu.items[0].props.handler.as_deref(), Some("openClicked"));
    assert_eq!(menu.items[2].props.name.as_deref(), Some("deleteItem"));
    assert_eq!(widgets::widget_line(&grid.props), "table name grid at 1,1 span 12 rows 6 bind albums columns title");
    assert!(file.pages[0].widgets[1].menu.is_none());
}

#[test]
fn a_popover_is_a_container_without_a_place_in_the_grid() {
    let (file, diagnostics) = parse(MUSIC, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let filters = &file.pages[0].widgets[2];
    assert_eq!(filters.props.kind, WidgetKind::Popover);
    assert_eq!(widgets::container(WidgetKind::Popover), Some(ContainerKind::Plain));
    assert_eq!(filters.props.for_part.as_deref(), Some("filterButton"));
    assert_eq!((filters.props.width, filters.props.height), (4, 2));
    assert_eq!(filters.children.len(), 1);
    assert_eq!(widgets::widget_line(&filters.props), "popover name filters for filterButton span 4 rows 2");
}

#[test]
fn a_window_with_both_checks_clean_and_their_texts_are_keys() {
    let p = project(MUSIC, CODE);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    for text in ["Open", "Delete", "Only vinyl"] {
        assert!(en.values().any(|v| v == text), "{text} is a key: {en:?}");
    }
    // The keys of a context menu are used keys, which the stacks write.
    let used: Vec<String> = clarity_spec::translations::keys(&p).into_iter().map(|k| k.key).collect();
    assert!(used.iter().any(|k| k == "openItem") && used.iter().any(|k| k == "deleteItem"), "{used:?}");
}

#[test]
fn the_rules_of_menus_and_popovers() {
    // A menu item runs a handler of the window, which is written in time.
    let unknown: Vec<String> = project(&MUSIC.replace("on click: openClicked", "on click: nothingHere"), CODE).diagnostics.into_iter().map(|d| d.message).collect();
    assert!(unknown.iter().any(|m| m.contains("handler 'nothingHere' is wired but not written yet")), "{unknown:#?}");
    // A part has one context menu.
    let twice = MUSIC.replace("            item \"Delete\" name deleteItem on click: deleteClicked\n", "            item \"Delete\" name deleteItem on click: deleteClicked\n        menu\n            item \"Again\" on click: openClicked\n");
    assert!(errors(&twice, CODE).iter().any(|m| m.contains("one context menu")), "{:#?}", errors(&twice, CODE));
    // A popover opens next to a part of the window.
    let nowhere = MUSIC.replace("for filterButton", "for nothingHere");
    assert!(errors(&nowhere, CODE).iter().any(|m| m.contains("no part 'nothingHere'")), "{:#?}", errors(&nowhere, CODE));
    // It has no place in the grid, and stands on the window, not in another part.
    let placed = MUSIC.replace("popover name filters for filterButton span 4 rows 2", "popover name filters for filterButton at 1,1 span 4 rows 2");
    assert!(errors(&placed, CODE).iter().any(|m| m.contains("a popover has no 'at'")), "{:#?}", errors(&placed, CODE));
    let nested = MUSIC.replace(
        "    popover name filters for filterButton span 4 rows 2\n        checkbox",
        "    panel name box at 1,8 span 12 rows 3\n        popover name filters for filterButton span 4 rows 2\n            checkbox",
    );
    assert!(errors(&nested, CODE).iter().any(|m| m.contains("a popover stands on the window")), "{:#?}", errors(&nested, CODE));
    // It needs a name, as every part does, so a click and code can open it.
    let unnamed = MUSIC.replace("popover name filters for", "popover for").replace("on click -> filters", "on click: refilter");
    let unnamed_code = CODE.replace("filters.visible = true", "deleteItem.visible = true").replace("filters.visible = false\n", "");
    assert!(errors(&unnamed, &unnamed_code).iter().any(|m| m.contains("popover at 1,1 has no name")), "{:#?}", errors(&unnamed, &unnamed_code));
    // And a part to open next to.
    let alone = MUSIC.replace("popover name filters for filterButton span", "popover name filters span");
    assert!(errors(&alone, CODE).iter().any(|m| m.contains("write 'for'")), "{:#?}", errors(&alone, CODE));
}

#[test]
fn name_them_names_a_popover_and_the_items_of_a_context_menu() {
    use clarity_spec::edit::Edit;
    let unnamed = MUSIC.replace(" name openItem", "").replace(" name deleteItem", "").replace("popover name filters for", "popover for");
    // The window as the designer edits it: its own file.
    let files = clarity_spec::converted([("music.clarity", unnamed.as_str())]);
    let (_, markup) = files.iter().find(|(p, _)| p.ends_with("design/AlbumBrowse.cxaml")).expect("the window's file");
    let named = clarity_spec::cxaml::apply(None, markup, &Edit::NameParts).unwrap_or_else(|e| panic!("{e}")).text;
    assert!(named.contains("<Item caption=\"Open\" name=\"openItem\" onclick=\"openClicked\" />"), "{named}");
    assert!(named.contains("<Item caption=\"Delete\" name=\"deleteItem\" onclick=\"deleteClicked\" />"), "{named}");
    let popover = named.lines().find(|l| l.trim_start().starts_with("<Popover ")).expect("the popover");
    assert!(popover.trim_start().starts_with("<Popover name=\"popover1\" ") && popover.contains(" span=\"4\" rows=\"2\"") && popover.contains(" for=\"filterButton\""), "{named}");
    assert!(named.contains("    <Menu>\n"), "the menu itself keeps no name: {named}");
}

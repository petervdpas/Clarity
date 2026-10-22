//! Every part has a name, and parts and handlers are named in camelCase:
//! letters and digits, starting with a small letter. The generators turn
//! them into each stack's own style.

use clarity_spec::names::{camel_case, is_camel_case};
use clarity_spec::{Severity, load_sources};

fn errors(source: &str) -> Vec<String> {
    load_sources([("game.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

const GAME: &str = "module game

page Main: window
    data status: text
    text name statusText at 1,1 span 6 bind status
    button \"New game\" name newGame at 1,2 span 6 on click: newGameClick

code Main
    handler newGameClick
        status = \"new\"
";

#[test]
fn a_name_is_camel_case() {
    for good in ["newGame", "square1", "x", "boardClick"] {
        assert!(is_camel_case(good), "{good}");
    }
    for bad in ["NewGame", "new_game", "new game", "1st", "", "naïve", "a-b"] {
        assert!(!is_camel_case(bad), "{bad}");
    }
    assert_eq!(camel_case("board_clicked"), "boardClicked");
    assert_eq!(camel_case("Square_1"), "square1");
    assert_eq!(camel_case("new game"), "newGame");
    assert_eq!(camel_case("newGame"), "newGame");
}

#[test]
fn parts_and_handlers_named_in_camel_case_check_clean() {
    assert!(errors(GAME).is_empty(), "{:#?}", errors(GAME));
}

#[test]
fn every_part_has_a_name() {
    let unnamed = GAME.replace("text name statusText at", "text at");
    assert_has(&errors(&unnamed), "text at 1,1 has no name; every part has one");
}

#[test]
fn a_part_and_a_handler_are_named_in_camel_case() {
    let part = GAME.replace("name newGame", "name new_game");
    assert_has(&errors(&part), "part 'new_game' is not camelCase: letters and digits, starting with a small letter, such as 'newGame'");
    let handler = GAME.replace("newGameClick", "new_game_click");
    assert_has(&errors(&handler), "handler 'new_game_click' is not camelCase");
}

/// Make them camelCase: a file's parts and handlers renamed, everywhere the
/// file uses them; fields after a dot and text keep theirs.
#[test]
fn names_are_made_camel_case_everywhere_they_are_used() {
    let old = "module game

entity Customer
    unit_price: money

code Customer
    before save
        if customer.unit_price < 0
            refuse \"no\"

page Main: window
    data c: Customer
    data list: list<Customer>
    textbox name unit_price at 1,1 span 6 bind c.unit_price
    label \"unit_price\" name price_label at 1,2 for unit_price
    button \"Go\" name go_button at 1,3 on click: go_clicked(1)
    table name unit_grid at 1,4 span 6 bind list columns unit_price, name edit unit_price on open: go_clicked(2)

code Main
    handler go_clicked(n: integer)
        unit_price.caption = \"go_clicked {unit_price.caption} {unit_grid}\"
        show_it()

    handler show_it
        go_button.enabled = false
";
    let new = clarity_spec::names::camel_names(old);
    assert_eq!(
        new,
        "module game

entity Customer
    unit_price: money

code Customer
    before save
        if customer.unit_price < 0
            refuse \"no\"

page Main: window
    data c: Customer
    data list: list<Customer>
    textbox name unitPrice at 1,1 span 6 bind c.unit_price
    label \"unit_price\" name priceLabel at 1,2 for unitPrice
    button \"Go\" name goButton at 1,3 on click: goClicked(1)
    table name unitGrid at 1,4 span 6 bind list columns unit_price, name edit unit_price on open: goClicked(2)

code Main
    handler goClicked(n: integer)
        unitPrice.caption = \"go_clicked {unitPrice.caption} {unitGrid}\"
        showIt()

    handler showIt
        goButton.enabled = false
"
    );
    assert_eq!(clarity_spec::names::camel_names(&new), new);
}

/// A translation's texts follow the slots the module's texts name.
#[test]
fn translations_follow_the_renamed_slots() {
    let renames = vec![("loan_grid".to_string(), "loanGrid".to_string())];
    assert_eq!(
        clarity_spec::names::rename_slots("    \"{loan_grid.selected.title} is back\" = \"{loan_grid.selected.title} ist zur\u{fc}ck\"\n", &renames),
        "    \"{loanGrid.selected.title} is back\" = \"{loanGrid.selected.title} ist zur\u{fc}ck\"\n"
    );
}

const MENU: &str = "module game

app Game
    title \"Game\"
    main Main
    menu name mainMenu
        item \"Game\" name gameMenu
            item \"New game\" name newGameItem key \"Ctrl+N\" on click: newGameClicked
            separator
            item \"Quit\" name quitItem on click -> quit
    toolbar name mainToolbar
        button \"New\" name newButton on click: newGameClicked

code Game
    handler newGameClicked
        message \"new\"

page Main: window
    text name statusText at 1,1 span 6
";

/// Menu items and toolbar and ribbon buttons are named like parts; a separator is not.
#[test]
fn menu_items_and_buttons_have_camel_case_names() {
    assert!(errors(MENU).is_empty(), "{:#?}", errors(MENU));
    let unnamed = MENU.replace("item \"New game\" name newGameItem key", "item \"New game\" key");
    assert_has(&errors(&unnamed), "item \"New game\" has no name; every menu item and button has one");
    let snake = MENU.replace("button \"New\" name newButton", "button \"New\" name new_button");
    assert_has(&errors(&snake), "item or button 'new_button' is not camelCase");
}

/// Name them names a command after its caption, and Make them camelCase renames it.
#[test]
fn menu_items_are_named_after_their_captions() {
    let unnamed = MENU
        .replace("item \"Game\" name gameMenu", "item \"Game\"")
        .replace("item \"New game\" name newGameItem key", "item \"New game\" key")
        .replace("button \"New\" name newButton on", "button \"New\" on");
    let named = clarity_spec::edit::apply(&unnamed, &clarity_spec::edit::Edit::NameParts).unwrap().text;
    assert!(named.contains("        item \"Game\" name gameItem\n"), "{named}");
    assert!(named.contains("            item \"New game\" name newGameItem key \"Ctrl+N\""), "{named}");
    assert!(named.contains("        button \"New\" name newButton on click"), "{named}");
    assert!(named.contains("            separator\n"), "{named}");
    assert!(errors(&named).is_empty(), "{:#?}", errors(&named));
    let snake = MENU.replace("name newGame key", "name new_game key");
    assert!(clarity_spec::names::camel_names(&snake).contains("item \"New game\" name newGameItem key"));
}

/// A menu item or button of the app shows in every window, so it cannot have
/// the name of a window's data.
#[test]
fn a_menu_item_is_not_named_like_a_windows_data() {
    let clash = MENU.replace("page Main: window\n", "page Main: window\n    data quitItem: text\n");
    assert_has(&errors(&clash), "item or button 'quitItem' has the name of data of Main; give one of them another name");
}

/// A new part is named after what it already knows of its purpose: what it
/// is for, what it shows, its source or its caption. Only a part that knows
/// none of these gets a numbered name, `button3`.
#[test]
fn a_part_is_named_after_its_purpose() {
    use clarity_spec::names::suggested_name;
    use clarity_spec::widgets::{WidgetKind, new_props};
    let part = |kind: WidgetKind, set: &dyn Fn(&mut clarity_spec::model::WidgetProps)| {
        let mut props = new_props(kind, 1, 1);
        props.caption = None;
        set(&mut props);
        suggested_name(&props)
    };
    assert_eq!(part(WidgetKind::Label, &|p| p.for_part = Some("vatNumber".into())), Some("vatNumberLabel".into()));
    assert_eq!(part(WidgetKind::Help, &|p| p.for_part = Some("vatNumber".into())), Some("vatNumberHelp".into()));
    assert_eq!(part(WidgetKind::Textbox, &|p| p.bind = Some("customer.vat_number".into())), Some("vatNumber".into()));
    assert_eq!(part(WidgetKind::Text, &|p| p.bind = Some("customer.notes".into())), Some("notesText".into()));
    assert_eq!(part(WidgetKind::Number, &|p| p.bind = Some("count".into())), Some("count".into()));
    assert_eq!(part(WidgetKind::Table, &|p| p.bind = Some("orders".into())), Some("ordersTable".into()));
    assert_eq!(part(WidgetKind::List, &|p| p.bind = Some("recent".into())), Some("recentList".into()));
    assert_eq!(part(WidgetKind::Form, &|p| p.bind = Some("customer".into())), Some("customerForm".into()));
    assert_eq!(part(WidgetKind::Image, &|p| p.bind = Some("album.cover".into())), Some("cover".into()));
    assert_eq!(part(WidgetKind::Image, &|p| p.source = Some("logo.png".into())), Some("logoImage".into()));
    assert_eq!(part(WidgetKind::Markdown, &|p| p.source = Some("rules-en.md".into())), Some("rulesEnText".into()));
    assert_eq!(part(WidgetKind::Markdown, &|p| p.caption = Some("@gameRules".into())), Some("gameRules".into()));
    // A caption names it, as a key or as written; a word code uses gets the part's kind.
    assert_eq!(part(WidgetKind::Button, &|p| p.caption = Some("@addLineCaption".into())), Some("addLine".into()));
    assert_eq!(part(WidgetKind::Button, &|p| p.caption = Some("Add line".into())), Some("addLine".into()));
    assert_eq!(part(WidgetKind::Button, &|p| p.caption = Some("@saveCaption".into())), Some("saveButton".into()));
    assert_eq!(part(WidgetKind::Heading, &|p| p.caption = Some("@collectionHeadingCaption".into())), Some("collectionHeading".into()));
    // A label, help text or heading named after its text says what it is, so it never takes an input's name.
    assert_eq!(part(WidgetKind::Label, &|p| p.caption = Some("E-mail".into())), Some("eMailLabel".into()));
    assert_eq!(part(WidgetKind::Label, &|p| p.caption = Some("@customerNameLabel".into())), Some("customerNameLabel".into()));
    assert_eq!(part(WidgetKind::Heading, &|p| p.caption = Some("Albums".into())), Some("albumsHeading".into()));
    assert_eq!(part(WidgetKind::Panel, &|p| p.caption = Some("@identityPanelCaption".into())), Some("identityPanel".into()));
    assert_eq!(part(WidgetKind::Panel, &|p| p.caption = Some("Credit".into())), Some("creditPanel".into()));
    // Nothing known yet: the default caption a new part gets says nothing.
    assert_eq!(part(WidgetKind::Button, &|p| p.caption = Some("Button".into())), None);
    assert_eq!(part(WidgetKind::Button, &|_| {}), None);
}

const FORM: &str = "module m

page Card: window
    data customer: Customer
    data orders: list<Customer>
    textbox name vatNumber at 4,1 span 4 bind customer.vat_number
    label \"VAT number\" name label1 at 1,1 span 3
    label \"Used\" name label2 at 1,2 span 3
    button \"Add line\" at 1,3 span 2

code Card
    handler cardOpened
        label2.caption = \"x\"

entity Customer
    vat_number: text(20)
";

fn part_line(text: &str, start: &str) -> String {
    text.lines().find(|l| l.trim_start().starts_with(start)).unwrap_or_else(|| panic!("no {start} in {text}")).trim().to_string()
}

/// Placing a part names it after what it shows; one with nothing to go by is numbered.
#[test]
fn a_placed_part_is_named_after_what_it_shows() {
    use clarity_spec::edit::{Edit, apply};
    use clarity_spec::widgets::WidgetKind;
    let page = FORM.find("page Card").unwrap() as u32;
    let create = |kind, bind: Option<&str>| Edit::CreateWidget { container: page, kind, col: 1, row: 5, width: None, caption: None, bind: bind.map(String::from) };
    let text = apply(FORM, &create(WidgetKind::Table, Some("orders"))).unwrap().text;
    assert!(part_line(&text, "table").contains("name ordersTable "), "{text}");
    let text = apply(FORM, &create(WidgetKind::Button, None)).unwrap().text;
    assert!(text.contains("button \"Button\" name button1 at 1,5"), "{text}");
}

/// A part that still has its numbered name takes a better one once it knows
/// its purpose, unless code goes by the number.
#[test]
fn a_numbered_part_takes_its_name_when_it_knows_its_purpose() {
    use clarity_spec::edit::{Edit, apply};
    use clarity_spec::parser::parse;
    let (file, _) = parse(FORM, "m.clarity");
    let widgets = &file.pages[0].widgets;
    let update = |name: &str| {
        let widget = widgets.iter().find(|w| w.props.name.as_deref() == Some(name)).unwrap();
        let mut props = widget.props.clone();
        props.for_part = Some("vatNumber".into());
        Edit::UpdateWidget { item: widget.span.start, container: None, props }
    };
    let text = apply(FORM, &update("label1")).unwrap().text;
    assert_eq!(part_line(&text, "label \"VAT"), "label \"VAT number\" name vatNumberLabel at 1,1 span 3 for vatNumber");
    // `label2` is what the code changes, so it keeps its name.
    let text = apply(FORM, &update("label2")).unwrap().text;
    assert_eq!(part_line(&text, "label \"Used"), "label \"Used\" name label2 at 1,1 span 3 for vatNumber".replace("1,1", "1,2"));
}

/// Name them names an unnamed part after its purpose too.
#[test]
fn name_them_names_a_part_after_its_purpose() {
    use clarity_spec::edit::{Edit, apply};
    let text = apply(FORM, &Edit::NameParts).unwrap().text;
    assert!(text.contains("button \"Add line\" name addLine at 1,3 span 2"), "{text}");
}

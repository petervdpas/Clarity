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
    menu
        item \"Game\" name gameMenu
            item \"New game\" name newGameItem key \"Ctrl+N\" on click: newGameClicked
            separator
            item \"Quit\" name quitItem on click -> quit
    toolbar
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

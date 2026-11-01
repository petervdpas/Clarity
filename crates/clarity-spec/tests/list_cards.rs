//! A list's rows as cards (`cards`): each row a small card, the first field
//! of `show` its title and the others lines below it, on every stack.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets;
use clarity_spec::Severity;

const PLAN: &str = "module plan

entity Item
    title: text(80)
    kind: text(20)

page Main: window
    data items: list<Item>
    list name itemList at 1,1 span 4 rows 6 bind items show title, kind cards
";

fn errors(page: &str) -> Vec<String> {
    load_keyed([("plan.clarity", page)], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_list_of_cards_reads_and_writes_back() {
    let (file, diagnostics) = parse(PLAN, "plan.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let list = &file.pages[0].widgets[0].props;
    assert!(list.cards);
    assert_eq!(widgets::widget_line(list), "list name itemList at 1,1 span 4 rows 6 bind items show title, kind cards");
    let found = errors(PLAN);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn only_a_list_has_cards() {
    let page = PLAN.replace("    list name itemList at 1,1 span 4 rows 6 bind items show title, kind cards\n", "    table name itemList at 1,1 span 4 rows 6 bind items columns title cards\n");
    let found = errors(&page);
    assert!(found.iter().any(|m| m.contains("'cards' does not apply to a table")), "{found:#?}");
}

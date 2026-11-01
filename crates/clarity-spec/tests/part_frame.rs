//! A list, table or tree in a container (a tab page, a pane, a panel, a form,
//! a section, a step, a scroll box) shows no border of its own, its
//! container framing it; `frame` gives it one anyway (DESIGN 5.6, 7.8).

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::Severity;

const PLAN: &str = "module plan

entity Item
    title: text(80)

page Main: window
    data items: list<Item>
    tabs name sheets at 1,1 span 12 rows 6
        tab \"Items\"
            list name itemList at 1,1 span 12 rows 5 bind items show title frame
";

fn errors(page: &str) -> Vec<String> {
    load_keyed([("plan.clarity", page)], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_framed_list_reads_and_writes_back() {
    assert!(errors(PLAN).is_empty(), "{:#?}", errors(PLAN));
    let (file, _) = parse(PLAN, "plan.clarity");
    let list = &file.pages[0].widgets[0].pages[0].children[0];
    assert!(list.props.frame);
    assert!(widgets::widget_line(&list.props).ends_with(" frame"), "{}", widgets::widget_line(&list.props));
}

#[test]
fn a_list_a_table_and_the_trees_have_a_frame_and_nothing_else() {
    for kind in [WidgetKind::List, WidgetKind::Table, WidgetKind::Tree, WidgetKind::TreeTable] {
        assert!(widgets::has_property(kind, WidgetProperty::Frame), "{kind:?}");
    }
    assert!(!widgets::has_property(WidgetKind::Button, WidgetProperty::Frame));
    let button = PLAN.replace("list name itemList at 1,1 span 12 rows 5 bind items show title frame", "button \"Go\" name go at 1,1 span 2 frame");
    assert!(errors(&button).iter().any(|m| m.contains("'frame' does not apply to a button")), "{:#?}", errors(&button));
}

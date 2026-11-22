//! `link chapter: toChapter`: a table column whose cells are links that run a handler.

use clarity_spec::{Severity, load_converted};

const DESK: &str = "module m

entity Chapter
    title: text

entity Place
    chapter: ref Chapter
    note: text

page Desk: window
    data places: list<Place>
    table name placeTable at 1,1 span 6 rows 5 bind places columns chapter, note link chapter: toChapter

code Desk
    handler toChapter
        let p = placeTable.selected
";

fn diagnostics(text: &str) -> Vec<(Severity, String)> {
    load_converted([("m.clarity", text)]).diagnostics.into_iter().map(|d| (d.severity, d.message)).collect()
}

#[test]
fn a_link_column_runs_its_handler() {
    let all = diagnostics(DESK);
    assert!(all.iter().all(|(s, _)| *s != Severity::Error), "{all:#?}");
    let project = load_converted([("m.clarity", DESK)]);
    let table = &project.pages().nth(0).unwrap().widgets[0];
    assert_eq!(table.props.links[0].handler, "toChapter");
    assert!(clarity_spec::widgets::widget_line(&table.props).contains("link chapter: toChapter"));
    let e = diagnostics(&DESK.replace("link chapter: toChapter", "link other: toChapter"));
    assert!(e.iter().any(|(s, m)| *s == Severity::Error && m.contains("'other' is no column of this table")), "{e:?}");
    let w = diagnostics(&DESK.replace("link chapter: toChapter", "link chapter: missing"));
    assert!(w.iter().any(|(_, m)| m.contains("handler 'missing' is wired but not written yet")), "{w:?}");
}

const FILING: &str = "module m

entity Fragment
    text: text

entity Chapter
    title: text

page Desk: window
    data fragments: list<Fragment>
    data chapters: list<Chapter>
    list name fragmentList at 1,1 span 4 rows 5 bind fragments show text
    list name chapterList at 5,1 span 4 rows 5 bind chapters show title on drop from fragmentList: fileFragment

code Desk
    handler fileFragment
        let f = fragmentList.selected
        let c = chapterList.selected
";

/// `on drop from fragmentList: fileFragment`: rows dragged from that part onto this one.
#[test]
fn a_list_takes_rows_dragged_from_another_part() {
    let all = diagnostics(FILING);
    assert!(all.iter().all(|(s, _)| *s != Severity::Error), "{all:#?}");
    let project = load_converted([("m.clarity", FILING)]);
    let list = &project.pages().nth(0).unwrap().widgets[1];
    assert_eq!((list.props.drops[0].from.as_str(), list.props.drops[0].handler.as_str()), ("fragmentList", "fileFragment"));
    assert!(clarity_spec::widgets::widget_line(&list.props).contains("on drop from fragmentList: fileFragment"));
    let e = diagnostics(&FILING.replace("on drop from fragmentList", "on drop from nowhere"));
    assert!(e.iter().any(|(s, m)| *s == Severity::Error && m.contains("there is no part 'nowhere' in this window")), "{e:?}");
    let e = diagnostics(&FILING.replace("on drop from fragmentList", "on drop from chapterList"));
    assert!(e.iter().any(|(s, m)| *s == Severity::Error && m.contains("own rows move with `reorder by`")), "{e:?}");
}

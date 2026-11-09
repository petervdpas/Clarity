//! Code works where the person types (Writer gap 21): a memo's caret, its
//! selected text, text put in at the caret, and rows dropped onto it.

use clarity_spec::{Severity, load_converted};

const DESK: &str = "module desk

entity Fragment
    heading: text, label @fragmentHeading
    text: text, label @fragmentText

entity Chapter
    text: text, label @chapterText

page Desk: window
    data chapter: Chapter
    data fragments: list<Fragment>
    data at: integer
    data picked: text
    list name fragmentList at 1,1 span 3 rows 10 bind fragments show heading
    memo name manuscriptBox at 4,1 span 9 rows 10 bind chapter.text markdown on drop from fragmentList: fragmentDropped
    button @quoteCaption name quoteButton at 4,11 span 2 on click: quoteClicked

code Desk
    handler fragmentDropped
        at = manuscriptBox.caret
        manuscriptBox.insert(fragmentList.selected.text)

    handler quoteClicked
        picked = manuscriptBox.selectedText
        manuscriptBox.insert(\"> \" + picked)
        manuscriptBox.caret = at + 2
";

fn errors(text: &str) -> Vec<String> {
    load_converted([("desk.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn code_reads_and_sets_the_caret_reads_the_selection_and_inserts() {
    assert!(errors(DESK).is_empty(), "{:#?}", errors(DESK));
    let e = errors(&DESK.replace("picked = manuscriptBox.selectedText", "manuscriptBox.selectedText = picked"));
    assert!(e.iter().any(|m| m.contains("manuscriptBox.selectedText is read, not set")), "{e:?}");
    let e = errors(&DESK.replace("manuscriptBox.insert(fragmentList.selected.text)", "manuscriptBox.insert(3)"));
    assert!(!e.is_empty(), "insert takes a text: {e:?}");
    let e = errors(&DESK.replace("manuscriptBox.insert(\"> \" + picked)", "manuscriptBox.append(picked)"));
    assert!(e.iter().any(|m| m.contains("hears insert")), "{e:?}");
}

#[test]
fn rows_dropped_on_a_memo_run_its_handler() {
    assert!(errors(DESK).is_empty());
    let e = errors(&DESK.replace("button @quoteCaption name quoteButton at 4,11 span 2 on click: quoteClicked", "button @quoteCaption name quoteButton at 4,11 span 2 on drop from fragmentList: quoteClicked"));
    assert!(e.iter().any(|m| m.contains("takes no rows dragged onto it")), "a button takes no drop: {e:?}");
}

/// A memo's toolbox: a page of its own whose coded tools work on its text.
#[test]
fn a_memo_has_a_toolbox_of_coded_tools() {
    let tools = "
page WritingTools: toolbox
    tool name quoteTool icon \"quote\" at 1,1 on click: quoteClicked
";
    let with = DESK.replace("on drop from fragmentList: fragmentDropped", "toolbox WritingTools on drop from fragmentList: fragmentDropped") + tools;
    assert!(errors(&with).is_empty(), "{:#?}", errors(&with));
    let plain = DESK.replace("on drop from fragmentList: fragmentDropped", "toolbox on drop from fragmentList: fragmentDropped");
    assert!(errors(&plain).iter().any(|m| m.contains("a memo's toolbox is a page of its own")), "{:?}", errors(&plain));
    let painting = with.replace("tool name quoteTool icon \"quote\" at 1,1 on click: quoteClicked", "tool name penTool at 1,1 does pen");
    assert!(errors(&painting).iter().any(|m| m.contains("a memo's toolbox holds tools with 'on click'")), "{:?}", errors(&painting));
}

/// A link into the application in the memo's preview or a markdown part
/// (`[the passage](clarity:fragment/412)`): `on link` runs, and `link` says where.
#[test]
fn a_link_into_the_application_runs_on_link() {
    let followed = DESK.replace("on drop from fragmentList: fragmentDropped", "on drop from fragmentList: fragmentDropped on link: linkFollowed")
        + "\n    handler linkFollowed\n        picked = manuscriptBox.link\n";
    assert!(errors(&followed).is_empty(), "{:#?}", errors(&followed));
    let set = followed.replace("picked = manuscriptBox.link", "manuscriptBox.link = picked");
    assert!(errors(&set).iter().any(|m| m.contains("manuscriptBox.link is read, not set")), "{:?}", errors(&set));
}

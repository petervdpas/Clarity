//! Code reads and sets the open tab of a `tabs` part as `part.tab`, counted
//! from 1, as it does a ribbon's: a workspace opens the tab of what was
//! chosen in its tree.

use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

page Work: window
    tabs name workTabs at 1,1 span 12 rows 6 on change: tabChanged
        tab \"Details\"
            text name detailsText at 1,1 span 6
        tab \"Diagram\"
            text name diagramText at 1,1 span 6
    button \"Diagram\" name diagramButton at 1,7 span 2 on click: showDiagram

code Work
    handler showDiagram
        workTabs.tab = 2

    handler tabChanged
        if workTabs.tab == 1
            workTabs.tab = 1
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("shop.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn code_reads_and_sets_the_open_tab_of_tabs() {
    assert!(errors(SHOP).is_empty(), "{:#?}", errors(SHOP));
    let wrong = SHOP.replace("workTabs.tab = 2", "workTabs.tab = \"two\"");
    assert!(errors(&wrong).iter().any(|e| e.contains("cannot set integer to text")), "{:#?}", errors(&wrong));
}

//! A window shows views, and each view's data and code become the window's
//! (views::placed). A local or parameter in a view's code may not take the
//! name of data another view (or the window) brings into the same window:
//! there it would mean that data on every stack. The checker says so, as it
//! does for a local named like its own page's data.

use clarity_spec::{Severity, load_sources};

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

const OWN: &str = "module m

page Main: window
    data values: list<text>
    button \"Go\" name goButton at 1,1 span 2 on click: go

code Main
    handler go
        let values = [\"a\"]
        let n = count(values)
";

const VIEWS: &str = "module m

page Main: window
    slideout name leftPanel side left view Values width 300 span 4 rows 4
    slideout name rightPanel side right view Counter width 300 span 4 rows 4

page Values: view
    data values: list<text>
    label \"Values\" name valuesLabel at 1,1 span 2

page Counter: view
    data total: integer
    button \"Count\" name countButton at 1,1 span 2 on click: counted

code Counter
    handler counted
        let values = [\"a\", \"b\"]
        total = count(values)
";

#[test]
fn a_local_named_like_its_own_page_s_data_is_refused() {
    let found = errors(OWN);
    assert!(found.iter().any(|e| e.contains("values")), "{found:#?}");
}

#[test]
fn a_local_named_like_data_another_view_brings_into_the_window_is_refused() {
    let found = errors(VIEWS);
    assert!(found.iter().any(|e| e.contains("'values'") && e.contains("Values") && e.contains("Main")), "{found:#?}");
    // Renamed, it is fine.
    let renamed = errors(&VIEWS.replace("let values = ", "let picked = ").replace("count(values)", "count(picked)"));
    assert!(renamed.is_empty(), "{renamed:#?}");
}

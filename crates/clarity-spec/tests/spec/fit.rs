//! A design edit never leaves parts reaching past what their window holds
//! (docs/ledger/app-mcp.md, 2026-11-13): a part is cut to end by the last
//! row, a slide-out to the rows the window holds, a view's parts to its
//! content area's rows. The window keeps its size.

use clarity_spec::fit;

fn page(markup: &str) -> clarity_spec::model::Page {
    clarity_spec::cxaml::read_page(markup, "design/Main.cxaml").0.expect("the page reads")
}

fn rows_of(page: &clarity_spec::model::Page, name: &str) -> (u32, u32) {
    let w = page.widgets.iter().find(|w| w.props.name.as_deref() == Some(name)).unwrap();
    (w.props.row, w.props.height)
}

#[test]
fn a_window_s_parts_and_slide_out_are_cut_to_the_rows_it_holds() {
    let mut main = page(r#"<Window class="shop.Main" title="Main">
  <Content name="main" at="1,1" span="12" rows="16"><View name="MainView" /></Content>
  <Button name="far" at="1,20" span="2" caption="Far" />
  <Slideout name="review" at="1,1" span="6" rows="22" caption="Review"><Label name="note" at="1,1" span="6" caption="Note" /></Slideout>
</Window>"#);
    assert!(fit::fit_window(&mut main, 15));
    assert_eq!(rows_of(&main, "main"), (1, 15));
    assert_eq!(rows_of(&main, "review").1, 15);
    // A part that starts below the last row stays where it is: moving it could cover another.
    assert_eq!(rows_of(&main, "far"), (20, 1));
    assert!(!fit::fit_window(&mut main, 15), "a fitted window is left as it is");
}

/// A content area grows to fill the window, so its view has the rows from its top to the window's last.
#[test]
fn a_view_has_the_rows_from_its_content_area_s_top() {
    let main = page(r#"<Window class="shop.Main" title="Main">
  <Label name="top" at="1,1" span="2" caption="Top" />
  <Content name="main" at="1,3" span="12" rows="1"><View name="MainView" /></Content>
  <Slideout name="review" at="1,1" span="6" rows="10" caption="Review"><Content name="inner" at="1,2" span="6" rows="1"><View name="Inner" /></Content></Slideout>
</Window>"#);
    assert_eq!(fit::shown_views(&main, 15), vec![("MainView".to_string(), 13), ("Inner".to_string(), 9)]);
}

#[test]
fn a_view_s_parts_are_cut_to_its_content_area() {
    let mut view = page(r#"<View class="shop.MainView">
  <Memo name="entryText" at="1,1" span="12" rows="16" />
</View>"#);
    assert!(fit::fit_view(&mut view, 15));
    assert_eq!(rows_of(&view, "entryText"), (1, 15));
}

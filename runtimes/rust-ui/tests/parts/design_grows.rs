//! What you see is what you get: on the design canvas (each part with its
//! `designItem`) a part that grows takes the height to spare as it does in
//! the running program, so the designer shows the window as it runs (user,
//! 2026-11-13: "that isnt wysiwyg... memo in design is not the same height
//! as in preview!").

use clarity_ui::testing::{part_named, window_of, TestHost};

fn heights(designed: bool) -> (f64, f64) {
    let item = |key: &str| if designed { format!(r#" designItem="design/Main.cxaml#Main/{key}""#) } else { String::new() };
    let markup = format!(
        r#"<Label at="1,1" span="12" name="title" caption="Title"{} /><Memo at="1,2" span="12" rows="3" name="notes"{} /><List at="1,5" span="12" rows="2" name="items" bind="items"{} />"#,
        item("title"),
        item("notes"),
        item("items")
    );
    let mut screen = window_of(&markup, TestHost::new());
    screen.resize(808.0, 700.0, 1.0);
    let _ = clarity_ui::testing::Shot::of(&mut screen, 808, 700, 1.0);
    let height = |name: &str| screen.element(part_named(&screen, name)).unwrap().node().bounds.height;
    (height("notes"), height("items"))
}

#[test]
fn a_part_that_grows_grows_on_the_design_canvas_as_when_it_runs() {
    let running = heights(false);
    let designed = heights(true);
    assert!(running.0 > 3.0 * 36.0 + 2.0 * 8.0, "the memo takes the height to spare when it runs: {running:?}");
    assert_eq!(running, designed, "the designer shows the window as it runs");
}

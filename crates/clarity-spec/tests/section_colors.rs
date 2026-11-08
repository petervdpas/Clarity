//! An accordion's section headers in colors of their own (`section "Chapters"
//! color "#2e86de"`); one without keeps the theme's look.

use clarity_spec::edit::{apply, Edit};
use clarity_spec::parser::parse;

const DESK: &str = "module m

page Notes: view
    label \"Notes\" name notesLabel at 1,1

page Desk: window
    accordion name parts at 1,1 span 12 rows 10
        section \"Chapters\" color \"#2e86de\"
            label \"One\" name oneLabel at 1,1
        section \"Notes\" color accent view Notes on saved: noted
        section \"Plain\"
            label \"Two\" name twoLabel at 1,1

code Desk
    handler noted
        message \"Saved\"
";

#[test]
fn a_section_takes_a_color_written_out_or_of_the_theme() {
    let (file, diagnostics) = parse(DESK, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let pages = &file.pages.iter().find(|p| p.name.name == "Desk").unwrap().widgets[0].pages;
    assert_eq!(Some("#2e86de"), pages[0].color.as_deref());
    assert_eq!(Some("accent"), pages[1].color.as_deref());
    assert_eq!(Some("Notes"), pages[1].view.as_deref());
    assert_eq!(None, pages[2].color);
    let (_, wrong) = parse(&DESK.replace("\"#2e86de\"", "\"blue-ish\""), "m.clarity");
    assert!(wrong.iter().any(|d| d.message.contains("'blue-ish' is not a color")), "{wrong:#?}");
    let (_, on_tab) = parse(&DESK.replace("accordion name parts", "tabs name parts").replace("section \"", "tab \""), "m.clarity");
    assert!(!on_tab.is_empty(), "a tab has no color");
}

#[test]
fn the_designer_sets_and_clears_it_keeping_the_rest_of_the_line() {
    let (file, _) = parse(DESK, "m.clarity");
    let pages = &file.pages.iter().find(|p| p.name.name == "Desk").unwrap().widgets[0].pages;
    let set = apply(DESK, &Edit::SetPageColor { target: pages[2].span.start, color: Some("#c0392b".into()) }).unwrap().text;
    assert!(set.contains("        section \"Plain\" color \"#c0392b\"\n"), "{set}");
    let cleared = apply(DESK, &Edit::SetPageColor { target: pages[1].span.start, color: None }).unwrap().text;
    assert!(cleared.contains("        section \"Notes\" view Notes on saved: noted\n"), "{cleared}");
    let viewed = apply(DESK, &Edit::SetPageView { target: pages[0].span.start, view: Some("Notes".into()) }).unwrap().text;
    assert!(viewed.contains("        section \"Chapters\" color \"#2e86de\" view Notes\n"), "{viewed}");
}

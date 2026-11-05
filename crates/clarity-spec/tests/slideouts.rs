//! Slide-outs: a `slideout` is a container part outside the window's grid,
//! as a popover is, that slides in over the window from its right edge, or
//! its left, top or bottom. A click with `on click -> name` or code with `name.visible` opens
//! it; a pinnable one is pinned open beside the window's parts with
//! `name.pinned`. It runs `on open` and `on close`.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::SlideSide;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, ContainerKind, Event, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

page Orders: window
    data note: text
    button \"Details\" name detailsButton at 1,1 span 2 on click -> details
    slideout \"Order\" name details side left width 480 min 300 max 900 resizable pinnable span 6 rows 8 on open: loadDetails on close: dropDetails
        textbox name noteBox at 1,1 span 6 bind note
    menu name mainMenu
        item \"Show details\" name showItem on click -> details
";

const CODE: &str = "
code Orders
    handler loadDetails
        note = \"open\"

    handler dropDetails
        details.pinned = false
        details.visible = false
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn warnings(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message).collect()
}

#[test]
fn a_slideout_is_a_container_at_a_side_of_the_window() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let details = &file.pages[0].widgets[1];
    let p = &details.props;
    assert_eq!(p.kind, WidgetKind::Slideout);
    assert_eq!(widgets::container(WidgetKind::Slideout), Some(ContainerKind::Plain));
    assert!(widgets::off_grid(WidgetKind::Slideout));
    assert_eq!((p.side, p.slide_width, p.min.as_deref(), p.max.as_deref()), (Some(SlideSide::Left), Some(480), Some("300"), Some("900")));
    assert!(p.resizable && p.pinnable);
    assert_eq!(p.caption.as_deref(), Some("Order"));
    assert_eq!(p.handlers.iter().map(|h| h.event).collect::<Vec<_>>(), vec![Event::Open, Event::Close]);
    assert_eq!(details.children.len(), 1);
    assert_eq!(
        widgets::widget_line(p),
        "slideout \"Order\" name details side left width 480 pinnable span 6 rows 8 min 300 max 900 resizable on open: loadDetails on close: dropDetails"
    );
    // The right side and the standard width are left out.
    let plain = parse(&SHOP.replace(" side left width 480", ""), "shop.clarity").0;
    assert!(widgets::widget_line(&plain.pages[0].widgets[1].props).starts_with("slideout \"Order\" name details pinnable span 6"));
}

#[test]
fn a_window_with_a_slideout_checks_clean() {
    let p = project(SHOP, CODE);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    // A menu item opens it as a button does: no word that it is no window.
    assert!(!warnings(SHOP, CODE).iter().any(|m| m.contains("'details' is not a window")), "{:#?}", warnings(SHOP, CODE));
}

#[test]
fn the_rules_of_slideouts() {
    let has = |page: &str, code: &str, words: &str| {
        let found = errors(page, code);
        assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
    };
    // It has no place in the grid and opens next to no part.
    has(&SHOP.replace("name details side", "name details at 1,2 side"), CODE, "a slide-out has no 'at'");
    has(&SHOP.replace("name details side", "name details for detailsButton side"), CODE, "a slide-out has no 'for'");
    has(&SHOP.replace("side left", "side middle"), CODE, "the left, the right, the top or the bottom");
    // At the top or bottom it has a height, at the left or right a width.
    has(&SHOP.replace("side left", "side top"), CODE, "has a height, not a width");
    has(&SHOP.replace("width 480", "height 480"), CODE, "has a width, not a height");
    has(&SHOP.replace("side left width 480", "side bottom height 40"), CODE, "a slide-out is 100 to 1600 pixels high");
    has(&SHOP.replace("side left width 480", "side bottom height 1000"), CODE, "height lies between its min and max");
    // Its widths are a sidebar's, the width between the limits.
    has(&SHOP.replace("width 480", "width 40"), CODE, "a slide-out is 100 to 1600 pixels wide");
    has(&SHOP.replace("width 480", "width 1000"), CODE, "between its min and max");
    // It stands on the window, not in another part.
    let nested = SHOP.replace(
        "    slideout \"Order\" name details side left width 480 min 300 max 900 resizable pinnable span 6 rows 8 on open: loadDetails on close: dropDetails\n        textbox",
        "    panel name box at 1,2 span 12 rows 9\n        slideout \"Order\" name details side left width 480 min 300 max 900 resizable pinnable span 6 rows 8 on open: loadDetails on close: dropDetails\n            textbox",
    );
    has(&nested, CODE, "a slide-out stands on the window");
    // Only a pinnable one is pinned by code.
    has(&SHOP.replace(" pinnable", ""), CODE, "pinned");
    // Its events are open and close.
    has(&SHOP.replace("on close: dropDetails", "on change: dropDetails"), CODE, "a slideout has no change event");
}

#[test]
fn name_them_names_a_slideout_after_its_caption() {
    use clarity_spec::edit::{Edit, apply};
    let unnamed = SHOP.replace("slideout \"Order\" name details side", "slideout \"Order\" side").replace("on click -> details", "on click: loadDetails");
    let named = apply(&unnamed, &Edit::NameParts).unwrap().text;
    assert!(named.contains("    slideout \"Order\" name "), "{named}");
}

#[test]
fn a_slideout_comes_in_from_the_top_or_the_bottom_with_a_height() {
    for side in ["top", "bottom"] {
        let page = SHOP.replace("side left width 480", &format!("side {side} height 320"));
        assert!(errors(&page, CODE).is_empty(), "{side}: {:#?}", errors(&page, CODE));
        let (file, _) = parse(&page, "shop.clarity");
        let p = &file.pages[0].widgets[1].props;
        assert_eq!((p.side.map(SlideSide::keyword), p.slide_width, p.slide_height), (Some(side), None, Some(320)));
        assert!(widgets::widget_line(p).starts_with(&format!("slideout \"Order\" name details side {side} height 320 pinnable")), "{}", widgets::widget_line(p));
    }
}

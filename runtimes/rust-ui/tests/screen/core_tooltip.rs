//! A part's designed tooltip (`tooltip "@saveTooltip"`) shows when the
//! pointer rests on it, in the person's language, as the web screen shows it.

use std::time::Instant;

use clarity_ui::testing::{TestHost, part_named, window_of};

#[test]
fn a_parts_tooltip_shows_after_the_pointer_rests() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("saveTooltip".into(), "Saves the customer".into());
    let mut screen = window_of(r#"<Button name="save" at="1,1" span="3" caption="Save" tooltip="@saveTooltip" />"#, host);
    screen.resize(400.0, 200.0, 1.0);
    let start = Instant::now();
    screen.now = Box::new(move || start);
    let mut surface = skia_safe::surfaces::raster_n32_premul((400, 200)).unwrap();
    screen.draw(surface.canvas());
    let b = screen.element(part_named(&screen, "save")).unwrap().node().bounds;
    screen.pointer_moved(b.x + 5.0, b.y + 5.0);
    let due = screen.next_wake().expect("the tooltip waits for the pause");
    screen.wake_due(due);
    assert_eq!("Saves the customer", screen.tip().expect("the tooltip shows").text);
}

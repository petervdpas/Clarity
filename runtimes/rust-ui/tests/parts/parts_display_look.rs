//! A display draws as Avalonia's segment display did, as the C#
//! DisplayLookTests check it: no background or frame of its own, its
//! segments in the text color over what is behind it.

use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use serde_json::json;

#[test]
fn a_display_has_no_background_or_frame_of_its_own() {
    let host = TestHost::new();
    host.set("shown", json!("8"));
    let mut screen = window_of(r#"<Display name="lcd" at="1,1" span="6" rows="3" bind="shown" />"#, host);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let b = screen.element(part_named(&screen, "lcd")).unwrap().node().bounds;
    // Its corner and edge show what is behind it, as the window beside it does.
    let behind = shot.pixel(b.right() as i32 + 20, b.y as i32 + 2);
    assert_eq!(behind, shot.pixel(b.x as i32 + 1, b.y as i32 + 1));
    assert_eq!(behind, shot.pixel(b.x as i32 + 3, (b.y + b.height / 2.0) as i32));
}

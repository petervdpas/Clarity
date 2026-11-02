//! A radio option's label shows whole: its last letter is not cut off by
//! the place measured for it (a glyph may reach past its advance, as k does).

use clarity_ui::painter::TextAlign;
use clarity_ui::parts::inputs::radio::{CIRCLE, RadioPart};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::testing::{part_named, window_of};
use serde_json::json;

#[test]
fn a_radio_label_is_drawn_as_the_same_text_alone() {
    let host = InputsHost::new();
    host.set("customer.kind", json!("b"));
    host.offer("customer.kind", &[(json!("b"), "Zakelijk"), (json!("p"), "Particulier")]);
    let mut screen = window_of(r#"<Radio name="kind" at="1,1" span="12" bind="customer.kind" />"#, host);
    screen.resize(800.0, 200.0, 1.0);
    let mut radio_surface = skia_safe::surfaces::raster_n32_premul((800, 200)).unwrap();
    screen.draw(radio_surface.canvas());
    let places = {
        let radio = screen.element(part_named(&screen, "kind")).unwrap().as_any().downcast_ref::<RadioPart>().unwrap();
        radio.lay_out(screen.fonts(), screen.look())
    };
    let first = places[0];
    let text_x = first.x + 1.0 + CIRCLE + 6.0;
    // The same label drawn alone in a wide place, on the same background.
    let mut alone = skia_safe::surfaces::raster_n32_premul((800, 200)).unwrap();
    screen.draw(alone.canvas());
    {
        let fonts = clarity_ui::fonts::Fonts::new(screen.look().clone());
        let p = clarity_ui::Painter::new(alone.canvas(), screen.look(), &fonts);
        let look = screen.look();
        let cover = clarity_ui::Rect::new(text_x - 1.0, first.y, 200.0, first.height);
        p.fill(cover, look.color("background"), 0.0);
        p.text("Zakelijk", clarity_ui::Rect::new(text_x, first.y, 400.0, first.height), look.color("text"), look.size("text"), "text", false, TextAlign::Left);
    }
    let end = (text_x + screen.fonts().measure("Zakelijk", "text", screen.look().size("text"), false)).ceil() as i32;
    let read = |s: &mut skia_safe::Surface, x: i32, y: i32| {
        let mut out = [0u8; 4];
        let info = skia_safe::ImageInfo::new((1, 1), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Unpremul, None);
        s.read_pixels(&info, &mut out, 4, (x, y));
        out
    };
    // The last letter's columns, up to a few pixels past the measured end, are the same.
    for x in end - 6..end + 2 {
        for y in first.y as i32..first.bottom() as i32 {
            assert_eq!(read(&mut radio_surface, x, y), read(&mut alone, x, y), "at {x},{y}");
        }
    }
}

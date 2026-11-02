//! Clarity's icons as Clarity.UI draws them, as the C# IconTests check them:
//! every path of crates/clarity-spec/icons/lucide.json parses inside its 24
//! by 24 box, and an icon's line takes the foreground of the button it is
//! on (a disabled one's dimmed).

use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_ui::markup;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::theme::ThemeLook;
use clarity_ui::window::WindowView;
use clarity_ui::Screen;

#[test]
fn every_icon_parses_as_a_geometry_in_its_box() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/clarity-spec/icons/lucide.json");
    let icons: BTreeMap<String, String> = serde_json::from_str(&std::fs::read_to_string(path).expect("lucide.json reads")).expect("lucide.json is JSON");
    assert!(icons.len() > 1800);
    let mut outside = Vec::new();
    for (name, data) in &icons {
        let Some(path) = skia_safe::Path::from_svg(data) else {
            outside.push(format!("{name} does not parse"));
            continue;
        };
        // The drawn shape's bounds, without the curves' control points.
        let b = path.compute_tight_bounds();
        if b.left < -1.0 || b.top < -1.0 || b.right > 25.0 || b.bottom > 25.0 {
            outside.push(format!("{name} {b:?}"));
        }
    }
    assert!(outside.is_empty(), "{}", outside.iter().take(10).cloned().collect::<Vec<_>>().join("; "));
}

#[test]
fn an_icons_line_takes_the_foreground_of_its_button() {
    // Three lines across the middle: a solid band where the icon is.
    const BAND: &str = "M 2 10 L 22 10 M 2 12 L 22 12 M 2 14 L 22 14";
    for dark in [false, true] {
        let host = TestHost::new();
        host.icons.borrow_mut().insert("band".into(), BAND.into());
        let spec = markup::read(
            r#"<Window name="Main"><Toolbar name="tools">
                 <Item name="save" caption="Save" icon="band" onclick="save" />
                 <Item name="mail" caption="Mail" icon="band" onclick="mail" />
               </Toolbar></Window>"#,
        )
        .expect("the window's markup reads");
        let as_host: Rc<dyn PartHost> = host.clone();
        let mut screen = Screen::new(Box::new(WindowView::new(&spec, as_host).with_frame(None)), ThemeLook::clarity(false));
        host.set_state("mail", |s| s.enabled = Some(false));
        screen.show(ThemeLook::clarity(dark));
        let mut shot = Shot::of(&mut screen, 2000, 1200, 2.0);
        let look = screen.look().clone();
        let text = look.part("button", None).text.unwrap_or_else(|| look.color("text"));
        let view = screen.root().as_any().downcast_ref::<WindowView>().unwrap();
        let place = |name: &str| view.toolbar().unwrap().placed().into_iter().find(|(item, _)| item.text("name") == Some(name)).map(|(_, r)| r).unwrap();
        let (save, mail) = (place("save"), place("mail"));
        // The small icon sits 10 in from the left, 16 square, in the middle of the button's height.
        let middle = |shot: &mut Shot, b: clarity_ui::Rect| shot.pixel(((b.x + 10.0 + 8.0) * 2.0) as i32, ((b.y + b.height / 2.0) * 2.0) as i32);
        assert_eq!(text, middle(&mut shot, save), "dark {dark}");
        // A disabled button's icon is dimmed with it.
        assert_ne!(text, middle(&mut shot, mail), "dark {dark}");
    }
}

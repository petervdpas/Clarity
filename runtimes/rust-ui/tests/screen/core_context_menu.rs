//! A part's own menu (`table ... menu / item ...`) opens at the pointer on a
//! right press, as the web screen opens it; picking an item runs it.

use clarity_ui::frame::menus::FrameMenuList;
use clarity_ui::testing::{TestHost, part_named, window_of};
use clarity_ui::PointerButton;

const PARTS: &str = r#"<Label name="box" at="1,1" span="6" rows="3" caption="Right click here">
    <Menu>
      <Item caption="Pick" name="pickItem" onclick="picked" />
      <Item kind="separator" />
      <Item caption="Filter" name="filterItem" onclick="filtered" />
    </Menu>
  </Label>"#;

#[test]
fn a_right_press_opens_the_parts_menu_and_an_item_runs() {
    let host = TestHost::new();
    let mut screen = window_of(PARTS, host.clone());
    screen.resize(600.0, 300.0, 1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((600, 300)).unwrap();
    screen.draw(surface.canvas());
    let b = screen.element(part_named(&screen, "box")).unwrap().node().bounds;
    let (x, y) = (b.x + 10.0, b.y + 10.0);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    screen.pointer_released(x, y, PointerButton::Right);
    let menu = screen.overlays().last().and_then(|o| o.content.as_any().downcast_ref::<FrameMenuList>()).expect("the menu opens");
    let captions: Vec<String> = menu.entries().iter().filter(|e| !e.separator).map(|e| e.caption.clone()).collect();
    assert_eq!(captions, ["Pick", "Filter"]);
    let row = menu.row_of(0);
    assert!(row.y >= y, "it opens at the pointer");
    screen.click(row.x + 5.0, row.y + row.height / 2.0);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().iter().any(|l| l.contains("picked")), "{:?}", host.logged());
}

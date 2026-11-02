//! The calendar's month arrows point the way they turn: the previous
//! month's to the left, the next month's to the right (the same case as the
//! C# DatePartTests.The_month_arrows_point_the_way_they_turn).

use clarity_ui::geometry::Rect;
use clarity_ui::input::PointerButton;
use clarity_ui::parts::inputs::date::DatePart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use serde_json::json;
use skia_safe::Color;

const DUE: &str = r#"<Date at="1,1" span="4" name="due" bind="task.due" />"#;

fn far(a: Color, b: Color) -> bool {
    let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
    d(a.r(), b.r()) + d(a.g(), b.g()) + d(a.b(), b.b()) > 90
}

/// Whether the arrow's ink farthest to the left (or right) in `area` lies near the area's middle row.
fn points(shot: &mut Shot, area: Rect, left: bool) -> bool {
    let ground = shot.pixel(area.x as i32 + 1, area.y as i32 + 1);
    let middle = area.y + area.height / 2.0;
    let mut columns: Vec<i32> = (area.x as i32..(area.x + area.width) as i32).collect();
    if !left {
        columns.reverse();
    }
    for x in columns {
        let ink: Vec<i32> = (area.y as i32..(area.y + area.height) as i32).filter(|y| far(shot.pixel(x, *y), ground)).collect();
        if !ink.is_empty() {
            return ink.iter().all(|y| (*y as f64 + 0.5 - middle).abs() < 2.5);
        }
    }
    false
}

#[test]
fn the_month_arrows_point_the_way_they_turn() {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.set("task.due", json!("2026-01-20"));
    let mut screen = window_of(DUE, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    let button = screen.element(part_named(&screen, "due")).and_then(|e| e.as_any().downcast_ref::<DatePart>()).unwrap().button();
    let (x, y) = (button.x + button.width / 2.0, button.y + button.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    let calendar = screen.element(part_named(&screen, "due")).and_then(|e| e.as_any().downcast_ref::<DatePart>()).unwrap().calendar().expect("the calendar is open");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(points(&mut shot, calendar.previous(), true), "the previous month's arrow points left");
    assert!(points(&mut shot, calendar.next(), false), "the next month's arrow points right");
}

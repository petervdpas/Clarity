//! A scrolling place, as the C# ScrollViewTests check it: content taller or
//! wider than its place scrolls with the wheel and a thin bar the person
//! drags, and shows only inside its place.

use clarity_ui::element::{find_type, GridPanel, Plate};
use clarity_ui::geometry::Rect;
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Modifiers, PointerButton};
use clarity_ui::parts::containers::scroll_view::{ScrollView, BAR_THICKNESS};
use clarity_ui::testing::Shot;
use clarity_ui::theme::ThemeLook;
use clarity_ui::{Element, Screen};

fn first<T: Element>(screen: &Screen) -> &T {
    find_type::<T>(screen.root(), |_| true).expect("the part is on the screen")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn tall(content_width: f64, content_height: f64) -> Screen {
    let mut view = ScrollView::new(Box::new(Plate::new("accent", None)));
    view.measure = Some(Box::new(move |_, _, _, _| (content_width, content_height)));
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, true));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(200.0, 200.0, 1.0);
    screen
}

/// The wheel turned on the view itself: whether it took the turn.
fn wheel_on_view(screen: &mut Screen, dy: f64) -> bool {
    let id = view(screen).node().id();
    screen.act(id, |e, cx| e.on_wheel(0.0, dy, Modifiers::NONE, cx)).unwrap()
}

fn view(screen: &Screen) -> &ScrollView {
    first::<ScrollView>(screen)
}

#[test]
fn content_taller_than_its_place_scrolls_with_the_wheel_and_stops_at_its_ends() {
    let mut screen = tall(300.0, 1000.0);
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::NONE);
    assert_eq!(40.0, view(&screen).scroll_y());
    assert_eq!(-40.0, view(&screen).content().node().bounds.y);
    assert_eq!(1000.0, view(&screen).content().node().bounds.height);
    screen.wheel(50.0, 50.0, 0.0, 5000.0, Modifiers::NONE);
    assert_eq!(800.0, view(&screen).scroll_y());
    assert_eq!(800.0, view(&screen).max_scroll_y());
    // At its end it takes no more, so what is around it may scroll.
    assert!(!wheel_on_view(&mut screen, 40.0));
    screen.wheel(50.0, 50.0, 0.0, -5000.0, Modifiers::NONE);
    assert_eq!(0.0, view(&screen).scroll_y());
    assert!(!wheel_on_view(&mut screen, -40.0));
}

#[test]
fn content_that_fits_does_not_scroll_and_shows_no_bar() {
    let mut screen = tall(200.0, 100.0);
    assert!(!wheel_on_view(&mut screen, 40.0));
    assert_eq!(0.0, view(&screen).scroll_y());
    assert!(view(&screen).bars().iter().all(|b| !b.is_shown()));
    assert_eq!(Some(view(&screen).content().node().id()), screen.at(195.0, 50.0));
}

#[test]
fn content_wider_than_its_place_scrolls_across() {
    let mut screen = tall(500.0, 100.0);
    screen.wheel(50.0, 50.0, 30.0, 0.0, Modifiers::NONE);
    assert_eq!(30.0, view(&screen).scroll_x());
    assert_eq!(-30.0, view(&screen).content().node().bounds.x);
    assert_eq!(300.0, view(&screen).max_scroll_x());
}

#[test]
fn dragging_the_bar_scrolls_as_far_as_the_bar_went() {
    let mut screen = tall(200.0, 1000.0);
    let bar = screen.at(196.0, 10.0);
    assert!(bar.is_some());
    assert_ne!(Some(view(&screen).content().node().id()), bar);
    screen.pointer_pressed(196.0, 10.0, PointerButton::Left, 1);
    screen.pointer_moved(196.0, 90.0);
    screen.pointer_released(196.0, 90.0, PointerButton::Left);
    // 80 of the 160 the thumb can move is half of the 800 the content can.
    assert!((view(&screen).scroll_y() - 400.0).abs() < 0.001);
    // A press on the track below the thumb pages down.
    click(&mut screen, 196.0, 190.0);
    assert!((view(&screen).scroll_y() - 600.0).abs() < 0.001);
}

#[test]
fn keeping_room_for_the_bar_narrows_the_content_while_the_bar_shows() {
    let made = |height: f64| {
        let mut view = ScrollView::new(Box::new(Plate::new("accent", None))).keeping_bar_room();
        view.measure = Some(Box::new(move |_, w, _, _| (w, height)));
        let mut grid = GridPanel::new(1);
        grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, true));
        let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
        screen.resize(200.0, 200.0, 1.0);
        screen
    };
    let screen = made(1000.0);
    assert_eq!(200.0 - BAR_THICKNESS, view(&screen).content().node().bounds.width);
    assert_eq!(0.0, view(&screen).max_scroll_x());
    let fits = made(100.0);
    assert_eq!(200.0, view(&fits).content().node().bounds.width);
}

#[test]
fn it_shows_its_content_only_inside_its_place() {
    let mut view = ScrollView::new(Box::new(Plate::new("accent", None)));
    view.measure = Some(Box::new(|_, w, _, _| (w, 1000.0)));
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    let (accent, background) = (screen.look().color("accent"), screen.look().color("background"));
    let mut shot = Shot::of(&mut screen, 200, 200, 1.0);
    assert_eq!(accent, shot.pixel(20, 20));
    assert_eq!(background, shot.pixel(20, 100));
}

#[test]
fn a_box_of_the_content_scrolls_into_view() {
    let mut screen = tall(300.0, 1000.0);
    let id = view(&screen).node().id();
    let scrolled = |screen: &mut Screen, area: Rect| {
        screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<ScrollView>().unwrap().scroll_into(area, cx));
        view(screen).scroll_y()
    };
    assert_eq!(350.0, scrolled(&mut screen, Rect::new(0.0, 500.0, 10.0, 50.0)));
    assert_eq!(100.0, scrolled(&mut screen, Rect::new(0.0, 100.0, 10.0, 50.0)));
    assert_eq!(100.0, scrolled(&mut screen, Rect::new(0.0, 150.0, 10.0, 50.0)));
}

#[test]
fn a_grid_scrolls_by_its_own_height() {
    let mut inner = GridPanel::new(1);
    for row in 1..=10 {
        inner.add(Box::new(Plate::new("panel", None)), GridItem::new(1, row, 1, 1, false));
    }
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(ScrollView::new(Box::new(inner))), GridItem::new(1, 1, 1, 3, true));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(300.0, 124.0, 1.0);
    // Ten rows of 36 with 8 between them.
    assert_eq!(10.0 * 36.0 + 9.0 * 8.0 - 124.0, view(&screen).max_scroll_y());
}

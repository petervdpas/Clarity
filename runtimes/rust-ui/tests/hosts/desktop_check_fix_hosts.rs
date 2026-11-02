//! Rows of the hosting area C# checks in CheckFixHostsTests that the desktop
//! host has: a desktop window opens at the standard size, the size its
//! window names, or a dialog's, with a dialog's first part focused and the
//! top window's title.

#![cfg(feature = "desktop")]

use super::core_windows_fake as fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::desktop::grow::{first_focus, open_size, top_size};
use clarity_ui::desktop::STANDARD_SIZE;
use clarity_ui::window::{Apart, WindowStack};
use clarity_ui::{AppFiles, Screen};
use fake::*;

fn stack_of(files: AppFiles) -> WindowStack {
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    WindowStack::start(program, files)
}

#[test]
fn a_desktop_window_opens_at_the_standard_size_the_size_its_window_names_or_a_dialog_s() {
    assert_eq!((1100.0, 760.0), STANDARD_SIZE);
    assert_eq!(STANDARD_SIZE, open_size(&stack_of(AppFiles { width: 0.0, height: 0.0, ..FILES })));
    assert_eq!((800.0, 500.0), open_size(&stack_of(FILES)));
    let (mut screen, _) = started();
    screen.act_root::<WindowStack, _>(|s, _| s.running().set_apart(true));
    click(&mut screen, "openPick");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("the dialog opens apart") };
    assert!(request.dialog);
    let pick = WindowStack::apart(stack(&screen).running().clone(), request);
    // A dialog in a window of its own is as large as its content needs.
    let size = open_size(&pick);
    assert_eq!(pick.wanted_size(), Some(size));
    assert_ne!(STANDARD_SIZE, size);
}

#[test]
fn a_dialog_in_a_window_of_its_own_starts_on_its_first_part_and_the_title_is_the_top_window_s() {
    let (mut screen, _) = started();
    screen.act_root::<WindowStack, _>(|s, _| s.running().set_apart(true));
    click(&mut screen, "openPick");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("the dialog opens apart") };
    let pick = WindowStack::apart(stack(&screen).running().clone(), request);
    assert_eq!("Pick a color", pick.title());
    let mut dialog = Screen::new(Box::new(pick), FILES.look(false));
    dialog.resize(480.0, 200.0, 1.0);
    let first = first_focus(dialog.root()).expect("a part takes the focus");
    assert_eq!(named(&dialog, "choose"), first);

    // A window opened in place: the desktop window's title is the top window's.
    let (mut main, _) = started();
    assert_eq!("Orders", stack(&main).title());
    click(&mut main, "openConnection");
    assert_eq!("Connection", stack(&main).title());
    // It names no size of its own: the desktop window keeps its size.
    assert_eq!(None, top_size(stack(&main)));
}

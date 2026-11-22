//! The catalog's timer.

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    Entry {
        kind: WidgetKind::Timer,
        keyword: "timer",
        name: "Timer",
        category: Category::Time,
        // Never in the grid: the designer keeps it in the tray under the window, with the sounds.
        size: (1, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Tick],
        properties: &[WidgetProperty::Every],
    },
];

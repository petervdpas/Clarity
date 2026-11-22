//! The catalog's inputs: the text inputs, and the numbers, choices, dates, files,
//! tags, colors and buttons.

use super::*;

pub(super) const WRITTEN: &[Entry] = &[
    input(WidgetKind::Textbox, "textbox", "Text box", (4, 1), &["text", "email", "url"], INPUT_EVENTS, TEXT_BOX),
    input(WidgetKind::Memo, "memo", "Memo", (6, 3), &["text"], MEMO_EVENTS, MEMO),
    input(WidgetKind::Password, "password", "Password", (4, 1), &["text", "secret"], INPUT_EVENTS, TYPED),
];

pub(super) const CHOSEN: &[Entry] = &[
    input(WidgetKind::Number, "number", "Number", (3, 1), &["integer", "decimal", "money", "id", "duration"], INPUT_EVENTS, TYPED),
    Entry {
        caption: Some("Check box"),
        ..input(WidgetKind::Checkbox, "checkbox", "Check box", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    Entry {
        caption: Some("Toggle"),
        ..input(WidgetKind::Toggle, "toggle", "Toggle button", (2, 1), &["bool"], &[Event::Change], &[WidgetProperty::Readonly, WidgetProperty::NoUndo, WidgetProperty::TabOrder, WidgetProperty::Icon])
    },
    Entry {
        caption: Some("Switch"),
        ..input(WidgetKind::Switch, "switch", "Switch", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    // Also the language and the theme of the settings: the project's languages, the offered themes.
    // `typed`, a text with suggestions; `search show name, city`, a reference among many records.
    input(
        WidgetKind::Dropdown,
        "dropdown",
        "Dropdown",
        (4, 1),
        &["enum", "ref", "language", "theme", "text"],
        &[Event::Change],
        &[WidgetProperty::Typed, WidgetProperty::Search, WidgetProperty::Show, WidgetProperty::Placeholder, WidgetProperty::Readonly, WidgetProperty::NoUndo, WidgetProperty::TabOrder],
    ),
    input(
        WidgetKind::Radio,
        "radio",
        "Radio buttons",
        (6, 1),
        &["enum"],
        &[Event::Change],
        &[WidgetProperty::Orientation, WidgetProperty::Readonly, WidgetProperty::NoUndo, WidgetProperty::TabOrder],
    ),
    input(WidgetKind::Date, "date", "Date", (3, 1), &["date", "dateTime"], &[Event::Change], TYPED),
    // A file of a record (U1c): its name, with buttons to choose, open, save a copy of and remove it.
    input(WidgetKind::File, "file", "File", (6, 1), &["blob", "file"], &[Event::Change], FILE),
    input(WidgetKind::Folder, "folder", "Folder", (6, 1), &["folder"], INPUT_EVENTS, FOLDER),
    // A list field of values (U1c): its items as chips, a typed item or an enum's value added, one removed.
    input(WidgetKind::Tags, "tags", "Tags", (6, 1), &["values"], &[Event::Change], TYPED),
    // A color (U1c): typed as #rrggbb, or picked from the palette every renderer shares.
    input(WidgetKind::Color, "color", "Color", (3, 1), &["color"], &[Event::Change], TYPED),
    // A number between its lowest and highest value (U1c): the range is written, or the field's own.
    input(
        WidgetKind::Slider,
        "slider",
        "Slider",
        (6, 1),
        &["integer", "decimal", "money"],
        &[Event::Change],
        &[WidgetProperty::Min, WidgetProperty::Max, WidgetProperty::Step, WidgetProperty::Readonly, WidgetProperty::NoUndo, WidgetProperty::TabOrder],
    ),
    input(
        WidgetKind::Spinbox,
        "spinbox",
        "Spin box",
        (3, 1),
        &["integer", "decimal", "money"],
        INPUT_EVENTS,
        &[
            WidgetProperty::Min,
            WidgetProperty::Max,
            WidgetProperty::Step,
            WidgetProperty::Placeholder,
            WidgetProperty::Readonly, WidgetProperty::NoUndo,
            WidgetProperty::TabOrder,
        ],
    ),
    Entry {
        kind: WidgetKind::Button,
        keyword: "button",
        name: "Button",
        category: Category::BasicInput,
        size: (2, 1),
        caption: Some("Button"),
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: true,
        events: &[Event::Click],
        properties: &[
            WidgetProperty::TabOrder,
            WidgetProperty::Shortcut,
            WidgetProperty::Default,
            WidgetProperty::Cancel,
            WidgetProperty::Icon,
            WidgetProperty::Split,
        ],
    },
];

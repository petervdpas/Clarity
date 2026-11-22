//! The catalog's graphics: displays, images and canvases, and a custom part.

use super::*;

pub(super) const DRAWN: &[Entry] = &[
    Entry {
        kind: WidgetKind::Display,
        keyword: "display",
        name: "Display",
        category: Category::Graphics,
        size: (4, 2),
        caption: None,
        // A number in seven-segment digits, as on a calculator.
        bind: BindTarget::Field,
        field_types: &["integer", "decimal", "money"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Digits],
    },
    Entry {
        kind: WidgetKind::Image,
        keyword: "image",
        name: "Image",
        category: Category::Graphics,
        size: (3, 3),
        caption: None,
        // A picture of a record (`bind album.cover`), or a fixed one of the
        // resources folder (`source "logo.png"`).
        bind: BindTarget::Field,
        field_types: &["image"],
        // Bound, it takes a picture from a file: a label or help is for it.
        input: true,
        container: None,
        clickable: true,
        events: &[Event::Click],
        // A picture loaded or removed is an edit Ctrl+Z takes back (manual/code.handlers.md).
        properties: &[WidgetProperty::Source, WidgetProperty::NoUndo],
    },
    Entry {
        kind: WidgetKind::Canvas,
        keyword: "canvas",
        name: "Canvas",
        category: Category::Graphics,
        size: (6, 4),
        caption: None,
        // A painted canvas binds its picture, a vector one its drawing as SVG text (manual/parts.graphics.md).
        bind: BindTarget::Field,
        field_types: &["image", "text"],
        input: false,
        container: None,
        clickable: false,
        // Drawn by its draw handler (`on draw`);
        // a click runs a handler, which reads where as `canvas.x` and `canvas.y`;
        // a drag and a drop also read where it was pressed, `fromX` and `fromY`.
        // `on change`: an edit of a canvas with a mode ended (manual/parts.graphics.md).
        events: &[Event::Draw, Event::Click, Event::Drag, Event::Drop, Event::Bend, Event::Connect, Event::Resize, Event::Pointer, Event::Key, Event::Change],
        properties: &[
            WidgetProperty::Units,
            WidgetProperty::NoUndo,
            WidgetProperty::Zoom,
            WidgetProperty::Grid,
            WidgetProperty::Snap,
            WidgetProperty::Background,
            WidgetProperty::Dots,
            WidgetProperty::Runs,
            WidgetProperty::Surface,
            WidgetProperty::Mode,
            WidgetProperty::Tools,
            WidgetProperty::Toolbox,
            WidgetProperty::Brush,
            WidgetProperty::Ink,
        ],
    },
];

pub(super) const CUSTOM: &[Entry] = &[
    Entry {
        kind: WidgetKind::Custom,
        keyword: "custom part",
        name: "Custom part",
        category: Category::Graphics,
        size: (4, 3),
        caption: None,
        bind: BindTarget::Field,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Change],
        // What it binds the person changes through it: an edit, which Ctrl+Z takes back (manual/code.handlers.md).
        properties: &[WidgetProperty::TabOrder, WidgetProperty::NoUndo],
    },
];

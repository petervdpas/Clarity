//! The catalog's media: sounds, videos and PDF reports.

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    // A custom part is written by its declared name; its keyword is two
    // words, so it is never typed and reads well in messages. The Parts
    // palette lists the declared parts in its place.
    Entry {
        kind: WidgetKind::Sound,
        keyword: "sound",
        name: "Sound",
        category: Category::Media,
        // Never in the grid: the designer keeps it in a tray under the window.
        size: (1, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::End, Event::Fail],
        properties: &[WidgetProperty::Source, WidgetProperty::Volume, WidgetProperty::Repeat],
    },
    Entry {
        kind: WidgetKind::Video,
        keyword: "video",
        name: "Video",
        category: Category::Media,
        size: (8, 6),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::End, Event::Fail],
        properties: &[WidgetProperty::Source, WidgetProperty::Volume, WidgetProperty::Repeat],
    },
    Entry {
        kind: WidgetKind::PdfReport,
        keyword: "pdfReport",
        name: "PDF report",
        category: Category::Media,
        size: (8, 10),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Gather],
        properties: &[WidgetProperty::Report, WidgetProperty::Save, WidgetProperty::Print, WidgetProperty::Zoom, WidgetProperty::Pages],
    },
];

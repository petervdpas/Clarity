//! The catalog's text parts: labels, help, headings, icons, badges, links and Markdown.

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    text_part(WidgetKind::Label, "label", "Label", (3, 1), Some("Label"), BindTarget::Nothing, &[WidgetProperty::For]),
    text_part(WidgetKind::Help, "help", "Help text", (4, 1), None, BindTarget::Nothing, &[WidgetProperty::For]),
    text_part(WidgetKind::Heading, "heading", "Heading", (6, 1), Some("Heading"), BindTarget::Field, &[]),
    // An icon of its own; what it shows is its first word: `icon "triangle-alert"`.
    text_part(WidgetKind::Icon, "icon", "Icon", (1, 1), None, BindTarget::Nothing, &[]),
    // A short text or number in a pill: written, or what it binds.
    Entry {
        kind: WidgetKind::Badge,
        keyword: "badge",
        name: "Badge",
        category: Category::Text,
        size: (1, 1),
        caption: Some("New"),
        bind: BindTarget::Field,
        field_types: &["text", "integer", "decimal", "money", "enum"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[],
    },
    // Text that opens an address, the address it binds, or runs a handler.
    Entry {
        kind: WidgetKind::Link,
        keyword: "link",
        name: "Link",
        category: Category::Text,
        size: (2, 1),
        caption: Some("Link"),
        bind: BindTarget::Field,
        field_types: &["url", "email", "text"],
        input: false,
        container: None,
        clickable: true,
        events: &[Event::Click],
        properties: &[WidgetProperty::To, WidgetProperty::TabOrder],
    },
    text_part(WidgetKind::Text, "text", "Text", (4, 1), None, BindTarget::Field, &[]),
    // Text written in Markdown: a file of the resources folder
    // (`source "rules.md"`, one per language) or a text it binds.
    Entry {
        field_types: &["text"],
        events: &[Event::Link],
        ..text_part(WidgetKind::Markdown, "markdown", "Markdown", (6, 4), None, BindTarget::Field, &[WidgetProperty::Source, WidgetProperty::ReadingWidth])
    },
];

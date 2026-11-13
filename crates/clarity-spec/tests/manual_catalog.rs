//! The manual's parts pages follow Clarity's part catalog: one page per
//! group of the Parts palette (`manual/parts.<group>.md`), each with a table
//! written from the catalog (crates/clarity-spec/src/widgets.rs) of every
//! part of its group: its line, what it binds, its size, whether it grows,
//! its properties and its events. The table is written, never edited: run
//! `CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog`
//! after a part, property or event changes. Every part also has a section
//! of its own on its page (`### \`keyword\``), so none goes undescribed.

use std::path::{Path, PathBuf};

use clarity_spec::widgets::{self, Category, WidgetInfo};

const START: &str = "<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->";
const END: &str = "<!-- /catalog -->";

/// Each palette group and its page.
const PAGES: [(Category, &str); 9] = [
    (Category::Text, "parts.text"),
    (Category::BasicInput, "parts.input"),
    (Category::Containers, "parts.containers"),
    (Category::Data, "parts.data"),
    (Category::Graphics, "parts.graphics"),
    (Category::Media, "parts.media"),
    (Category::Time, "parts.time"),
    (Category::Tools, "parts.toolbox"),
    (Category::Signin, "parts.signin"),
];

fn manual() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manual")
}

/// A value of the catalog as its JSON names it (`basicInput`, `click`).
fn named<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value).unwrap() {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }
}

fn listed<T: serde::Serialize>(values: &[T]) -> String {
    if values.is_empty() { "none".to_string() } else { values.iter().map(|v| format!("`{}`", named(v))).collect::<Vec<_>>().join(", ") }
}

/// The table of one group, as the page holds it between its markers.
fn table(parts: &[WidgetInfo]) -> String {
    let mut out = format!("{START}\n\n| Part | Line | Binds | Size | Grows | Properties | Events |\n|---|---|---|---|---|---|---|\n");
    for p in parts {
        let binds = named(&p.bind);
        let types = if p.field_types.is_empty() { String::new() } else { format!(" ({})", p.field_types.join(", ")) };
        out.push_str(&format!(
            "| {} | `{}` | {binds}{types} | {} x {} | {} | {} | {} |\n",
            p.name,
            p.keyword,
            p.width,
            p.height,
            if p.grows { "yes" } else { "no" },
            listed(&p.properties),
            listed(&p.events),
        ));
    }
    out.push('\n');
    out.push_str(END);
    out
}

#[test]
fn every_parts_page_holds_its_group_of_the_catalog() {
    let write = std::env::var_os("CLARITY_WRITE_MANUAL").is_some();
    let catalog = widgets::catalog();
    let mut stale = Vec::new();
    for (category, page) in PAGES {
        let parts: Vec<WidgetInfo> = catalog.iter().filter(|p| p.category == category).cloned().collect();
        let path = manual().join(format!("{page}.md"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("manual/{page}.md is the page of {category:?}"));
        let wanted = table(&parts);
        let now = match (text.find(START), text.find(END)) {
            (Some(a), Some(b)) => text[a..b + END.len()].to_string(),
            _ => String::new(),
        };
        if now != wanted {
            if write {
                let updated = match (text.find(START), text.find(END)) {
                    (Some(a), Some(b)) => format!("{}{wanted}{}", &text[..a], &text[b + END.len()..]),
                    _ => format!("{}\n\n## The parts at a glance\n\n{wanted}\n", text.trim_end()),
                };
                std::fs::write(&path, updated).unwrap();
            } else {
                stale.push(page);
            }
        }
    }
    assert!(stale.is_empty(), "the catalog tables of {stale:?} are behind the catalog: run CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog");
}

#[test]
fn every_part_has_a_section_on_its_page() {
    let mut missing = Vec::new();
    for info in widgets::catalog() {
        let page = PAGES.iter().find(|(c, _)| *c == info.category).map(|(_, p)| *p).expect("every group has its page");
        let text = std::fs::read_to_string(manual().join(format!("{page}.md"))).unwrap_or_default();
        if !text.lines().any(|l| l.trim() == format!("### `{}`", info.keyword)) {
            missing.push(format!("{} on {page}", info.keyword));
        }
    }
    assert!(missing.is_empty(), "parts with no section (### `keyword`) of their own: {missing:#?}");
}

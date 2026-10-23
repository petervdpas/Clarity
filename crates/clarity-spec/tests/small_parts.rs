//! The small parts of U1b: a separator line, a link (to an address, to what
//! it binds, or running a handler), a badge, a progress bar for a number,
//! and a busy indicator that code shows and hides.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::Orientation;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const MUSIC: &str = "module music

entity Artist
    name: text(80), required
    website: url
    formed: date

page ArtistCard: window
    data artist: Artist
    data unread: integer
    data copied: integer
    separator name lineA at 1,4 span 12
    separator name lineB at 6,1 span 1 rows 3 vertical
    link \"Lucide\" name lucideLink at 1,5 span 2 to \"https://lucide.dev\"
    link name siteLink at 3,5 span 4 bind artist.website
    link \"Details\" name moreLink at 7,5 span 2 on click: showDetails
    badge name unreadBadge at 10,1 span 1 bind unread
    badge \"New\" name newBadge at 11,1 span 1
    progress name copyBar at 1,7 span 6 bind copied max 250
    busy \"Saving...\" name savingBusy at 8,7 span 3
";

const CODE: &str = "
code ArtistCard
    handler showDetails
        savingBusy.visible = true
        copied = copied + 1
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("music.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_small_parts_read_and_write_back_as_written() {
    let (file, diagnostics) = parse(MUSIC, "music.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    let kinds: Vec<WidgetKind> = parts.iter().map(|w| w.props.kind).collect();
    use WidgetKind::*;
    assert_eq!(kinds, vec![Separator, Separator, Link, Link, Link, Badge, Badge, Progress, Busy]);
    let page = &MUSIC[MUSIC.find("page ArtistCard").unwrap()..];
    let lines = page.lines().filter(|l| l.starts_with("    ") && !l.trim_start().starts_with("data "));
    for (widget, line) in parts.iter().zip(lines) {
        assert_eq!(widgets::widget_line(&widget.props), line.trim(), "{line}");
    }
    assert_eq!(parts[0].props.orientation, None, "a separator is a level line unless written vertical");
    assert_eq!(parts[1].props.orientation, Some(Orientation::Vertical));
    assert_eq!(parts[2].props.to.as_deref(), Some("https://lucide.dev"));
    assert_eq!(parts[7].props.max.as_deref(), Some("250"));
    assert_eq!(widgets::new_props(Separator, 1, 1).orientation, None);
}

#[test]
fn a_window_with_them_checks_clean_and_their_texts_are_keys() {
    let p = project(MUSIC);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    for text in ["Lucide", "Details", "New", "Saving..."] {
        assert!(en.values().any(|v| v == text), "{text} is a key: {en:?}");
    }
    assert!(!en.values().any(|v| v.contains("lucide.dev")), "an address is no text people read: {en:?}");
}

#[test]
fn the_rules_of_the_small_parts() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    // A link opens an address, opens what it binds, or runs a handler: one of them.
    says(MUSIC.replace(" to \"https://lucide.dev\"", ""), "a link opens an address ('to'), what it binds or runs a handler");
    says(MUSIC.replace(" on click: showDetails", " to \"https://lucide.dev\" on click: showDetails"), "a link opens an address, what it binds or runs a handler: one of them");
    says(MUSIC.replace("to \"https://lucide.dev\"", "to \"lucide.dev\""), "'lucide.dev' is no address; write it whole, as in \"https://lucide.dev\"");
    says(MUSIC.replace("bind artist.website", "bind artist.formed"), "a link shows url, email, text");
    // A progress bar shows a number it binds, from 0 to its max.
    says(MUSIC.replace(" bind copied max 250", ""), "a progress bar shows a number: bind one");
    says(MUSIC.replace("max 250", "max 0"), "'max' is a number above 0");
    // A separator is only a line.
    says(MUSIC.replace("separator name lineA", "separator \"Line\" name lineA"), "a separator has no caption");
}

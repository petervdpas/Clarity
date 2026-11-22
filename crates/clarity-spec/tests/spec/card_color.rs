//! `colored by`: a list's cards each take the colour of a colour field of
//! their record, as a chip around one of its lines (`chip category`), a
//! small band at its start (`band`) or the whole card (`cards colored by
//! paint band`; in the window's file `cardColor="paint" cardBand="true"`,
//! `cardChip="category"`). Options, off by default: without them the cards
//! keep the theme's `card` look.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

record Card
    name: text
    category: text
    paint: color

page Main: window
    data cards: list<Card>
    list name cardList at 1,1 span 6 rows 6 bind cards show name, category cards colored by paint band
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_colouring_and_its_band_read_and_write_back() {
    let (file, diagnostics) = parse(WINDOW, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.card_color.as_deref(), Some("paint"));
    assert!(props.card_band);
    assert!(widgets::widget_line(props).ends_with("cards colored by paint band"), "{}", widgets::widget_line(props));
    assert!(widgets::has_property(WidgetKind::List, WidgetProperty::CardColor));
    assert!(!widgets::has_property(WidgetKind::Table, WidgetProperty::CardColor), "a table has no cards");
    let files = clarity_spec::converted([("m.clarity", WINDOW)]);
    let design = files.iter().find(|(p, _)| p.ends_with("Main.cxaml")).map(|(_, t)| t.clone()).unwrap();
    assert!(design.contains("cardColor=\"paint\"") && design.contains("cardBand=\"true\""), "{design}");
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn whole_cards_or_none_coloured() {
    for line in ["cards colored by paint", "cards", "cards colored by paint chip category"] {
        let source = WINDOW.replace("cards colored by paint band", line);
        assert!(errors(&source).is_empty(), "{line}: {:#?}", errors(&source));
        let (file, _) = parse(&source, "m.clarity");
        let props = &file.pages[0].widgets[0].props;
        assert!(!props.card_band);
        assert_eq!(props.card_chip.is_some(), line.ends_with("chip category"));
        assert!(widgets::widget_line(props).ends_with(line));
    }
}

#[test]
fn the_checker_refuses_a_colouring_it_cannot_draw() {
    let refused = |line: &str, says: &str| {
        let source = WINDOW.replace("cards colored by paint band", line);
        assert!(errors(&source).iter().any(|m| m.contains(says)), "{line}: {:#?}", errors(&source));
    };
    refused("colored by paint", "'colored by' colours a list's cards: write 'cards colored by paint'");
    refused("cards colored by name", "'name' is text; the cards are colored by a color field");
    refused("cards colored by missing", "'Card' has no field 'missing'");
    refused("cards colored by paint chip paint", "the chip is one of the shown fields (name, category), not 'paint'");
}

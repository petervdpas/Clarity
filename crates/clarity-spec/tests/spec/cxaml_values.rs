//! A window's data, tells and handlers in its `.cxaml` are read by
//! Clarity's own rules on each attribute's value, where it is in the file:
//! what is wrong in one is said there.

use clarity_spec::cxaml::read_page;
use clarity_spec::model::DataKind;

fn at<'a>(markup: &'a str, from: usize) -> &'a str {
    &markup[from..]
}

#[test]
fn data_tells_and_handlers_are_read_from_their_values() {
    let markup = "<Window class=\"m.Main\" onopen=\"opened\">\n  <Data name=\"orders\" type=\"list&lt;Order&gt;\" />\n  <Data name=\"count\" type=\"integer\" default=\"3\" />\n  <Tells message=\"saved(name: text)\" />\n  <Button name=\"go\" at=\"1,1\" span=\"2\" onclick=\"digit(7, true)\" />\n</Window>\n";
    let (page, diagnostics) = read_page(markup, "design/Main.cxaml");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let page = page.unwrap();
    assert!(matches!(&page.data[0].of, DataKind::List { entity } if entity.name == "Order"));
    assert!(matches!(&page.data[1].of, DataKind::Value { default: Some(_), .. }));
    assert_eq!(page.tells[0].name.name, "saved");
    assert_eq!(page.tells[0].params.len(), 1);
    assert_eq!(page.handlers[0].handler, "opened");
    assert_eq!(page.widgets[0].props.handlers[0].args.len(), 2);
}

#[test]
fn what_is_wrong_in_a_value_is_said_where_the_value_is() {
    let markup = "<Window class=\"m.Main\">\n  <Data name=\"count\" type=\"integer\" default=\"(\" />\n</Window>\n";
    let (_, diagnostics) = read_page(markup, "design/Main.cxaml");
    let d = diagnostics.first().expect("the default is wrong");
    let value = markup.find("default=\"(").unwrap() + "default=\"".len();
    assert_eq!(d.span.start as usize, value, "{d:#?} at {:?}", at(markup, d.span.start as usize));
    assert_eq!(d.span.line, 2);
    // A window has its own events only.
    let (_, diagnostics) = read_page("<Window class=\"m.Main\" onstart=\"x\" />\n", "");
    assert!(diagnostics.iter().any(|d| d.message == "a window has the events open, close and return, not start"), "{diagnostics:#?}");
}

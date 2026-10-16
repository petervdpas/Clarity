use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::Handler;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{Event, WidgetKind, new_props, validate_props, widget_line};
use clarity_spec::{Severity, load_sources};

const PAGE: &str = "module m

page Card: window
    title \"Card\"
    textbox name customer_name at 1,1 span 6 on change: name_changed on leave: name_left
    button \"Save\" name save at 1,2 span 2 on click: save_clicked
    button \"Back\" name button1 at 3,2 span 2 on click -> Card
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn parses_names_and_handlers() {
    let (file, diags) = parse(PAGE, "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let widgets = &file.pages[0].widgets;
    let input = &widgets[0].props;
    assert_eq!(input.name.as_deref(), Some("customer_name"));
    assert_eq!(
        input.handlers,
        vec![
            Handler {
                event: Event::Change,
                handler: "name_changed".into()
            },
            Handler {
                event: Event::Leave,
                handler: "name_left".into()
            },
        ]
    );
    assert_eq!(widgets[1].props.handlers[0].event, Event::Click);
    assert_eq!(widgets[2].props.on_click.as_deref(), Some("Card"));
    assert!(errors(PAGE).is_empty(), "{:#?}", errors(PAGE));
}

#[test]
fn writes_name_after_the_caption_and_handlers_last() {
    let mut props = new_props(WidgetKind::Button, 1, 1);
    props.name = Some("save".into());
    props.handlers.push(Handler {
        event: Event::Click,
        handler: "save_clicked".into(),
    });
    assert_eq!(
        widget_line(&props),
        "button \"Button\" name save at 1,1 span 2 on click: save_clicked"
    );
}

#[test]
fn rejects_events_a_part_does_not_have() {
    let (_, diags) = parse(
        "module m\npage P: window\n    label \"A\" at 1,1 on change: x\n",
        "m.clarity",
    );
    assert!(diags[0].message.contains("a label has no change event"), "{diags:#?}");

    let mut props = new_props(WidgetKind::Button, 1, 1);
    props.handlers.push(Handler {
        event: Event::Click,
        handler: "a".into(),
    });
    props.on_click = Some("Card".into());
    assert!(validate_props(&props).unwrap_err().contains("wired twice"));
}

#[test]
fn an_event_is_wired_once() {
    let (_, diags) = parse(
        "module m\npage P: window\n    button \"A\" at 1,1 on click: a on click -> P\n",
        "m.clarity",
    );
    assert!(diags[0].message.contains("duplicate 'on click'"), "{diags:#?}");
}

#[test]
fn names_are_unique_in_a_window() {
    let source = "module m\npage P: window\n    button \"A\" name go at 1,1\n    panel at 1,2 span 6 rows 3\n        button \"B\" name go at 1,1\n";
    assert!(errors(source).iter().any(|e| e.contains("duplicate part name 'go'")));
    // Another window may use the same name.
    let two = "module m\npage P: window\n    button \"A\" name go at 1,1\npage Q: window\n    button \"A\" name go at 1,1\n";
    assert!(errors(two).is_empty());
}

#[test]
fn new_parts_get_the_first_free_name() {
    let page = PAGE.find("page Card").unwrap() as u32;
    let edit = Edit::CreateWidget {
        container: page,
        kind: WidgetKind::Button,
        col: 5,
        row: 2,
        width: None,
        caption: None,
        bind: None,
    };
    let text = apply(PAGE, &edit).unwrap().text;
    // `button1` is taken, so the new button is `button2`.
    assert!(text.contains("button \"Button\" name button2 at 5,2 span 2"), "{text}");
}

use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::PageKind;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{WidgetKind, new_props};

const PAGE: &str = "module p

page Card: window
    title \"Card\"
    data c: Customer

    label \"Name\" at 1,1 span 2
    textbox at 3,1 span 6 bind c.name // the name
    panel \"Box\" at 1,3 span 6 rows 3
        label \"Inside\" at 1,1
    tabs at 1,7 span 12 rows 4
        tab \"One\"
            label \"First\" at 1,1
";

/// Offset of the last line whose content starts with `needle`.
fn at(source: &str, needle: &str) -> u32 {
    let mut offset = 0;
    let mut found = None;
    for line in source.split('\n') {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(needle) {
            found = Some((offset + indent) as u32);
        }
        offset += line.len() + 1;
    }
    found.unwrap_or_else(|| panic!("no line starts with {needle:?}"))
}

fn edit(source: &str, edit: Edit) -> (String, Option<u32>) {
    let outcome = apply(source, &edit).unwrap_or_else(|e| panic!("edit failed: {e}"));
    let (_, diags) = parse(&outcome.text, "p.clarity");
    assert!(
        diags.is_empty(),
        "edit produced parse errors: {diags:#?}\n{}",
        outcome.text
    );
    (outcome.text, outcome.focus)
}

fn error(source: &str, edit: Edit) -> String {
    apply(source, &edit).expect_err("edit should fail").0
}

#[test]
fn places_a_widget_in_reading_order() {
    let mut props = new_props(WidgetKind::Checkbox, 9, 1);
    props.bind = Some("c.active".into());
    let (text, focus) = edit(
        PAGE,
        Edit::PlaceWidget {
            container: at(PAGE, "page Card"),
            props,
        },
    );
    assert!(text.contains(
        "    textbox at 3,1 span 6 bind c.name // the name\n    checkbox \"Check box\" name active at 9,1 span 3 bind c.active\n    panel"
    ), "{text}");
    assert!(text[focus.unwrap() as usize..].starts_with("checkbox"));
}

#[test]
fn places_widgets_inside_containers_and_tabs() {
    let (text, _) = edit(
        PAGE,
        Edit::PlaceWidget {
            container: at(PAGE, "panel"),
            props: new_props(WidgetKind::Button, 1, 2),
        },
    );
    assert!(
        text.contains("        label \"Inside\" at 1,1\n        button \"Button\" name button1 at 1,2 span 2\n")
    );

    let (text, _) = edit(
        PAGE,
        Edit::PlaceWidget {
            container: at(PAGE, "tab \"One\""),
            props: new_props(WidgetKind::Date, 1, 2),
        },
    );
    assert!(
        text.ends_with("            label \"First\" at 1,1\n            date name date1 at 1,2 span 3\n"),
        "{text}"
    );
}

#[test]
fn new_containers_get_their_blocks() {
    let (text, _) = edit(
        PAGE,
        Edit::PlaceWidget {
            container: at(PAGE, "page Card"),
            props: new_props(WidgetKind::Splitter, 1, 12),
        },
    );
    assert!(
        text.ends_with(
            "    splitter name splitter1 at 1,12 span 12 rows 6 horizontal\n        pane\n        pane\n"
        ),
        "{text}"
    );

    let (text, _) = edit(
        PAGE,
        Edit::PlaceWidget {
            container: at(PAGE, "page Card"),
            props: new_props(WidgetKind::Tabs, 1, 12),
        },
    );
    assert!(
        text.ends_with("    tabs name tabs1 at 1,12 span 8 rows 5\n        tab \"Tab 1\"\n"),
        "{text}"
    );
}

#[test]
fn updates_a_widget_and_keeps_its_comment() {
    let mut props = new_props(WidgetKind::Textbox, 3, 2);
    props.caption = None;
    props.width = 8;
    props.bind = Some("c.email".into());
    let (text, focus) = edit(
        PAGE,
        Edit::UpdateWidget {
            item: at(PAGE, "textbox"),
            container: None,
            props,
        },
    );
    assert!(
        text.contains("    textbox name textbox1 at 3,2 span 8 bind c.email // the name\n"),
        "{text}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("textbox"));
}

#[test]
fn moves_a_widget_into_a_container() {
    let props = new_props(WidgetKind::Label, 1, 2);
    let mut props = props;
    props.caption = Some("Name".into());
    props.width = 2;
    let (text, _) = edit(
        PAGE,
        Edit::UpdateWidget {
            item: at(PAGE, "label \"Name\""),
            container: Some(at(PAGE, "panel")),
            props,
        },
    );
    assert!(
        text.contains("        label \"Inside\" at 1,1\n        label \"Name\" name label1 at 1,2 span 2\n"),
        "{text}"
    );
    assert!(!text.contains("    label \"Name\" at 1,1"));
}

#[test]
fn moves_a_container_with_its_children() {
    let mut props = new_props(WidgetKind::Panel, 7, 3);
    props.caption = Some("Box".into());
    props.width = 6;
    props.height = 3;
    let (text, _) = edit(
        PAGE,
        Edit::UpdateWidget {
            item: at(PAGE, "panel"),
            container: Some(at(PAGE, "tab \"One\"")),
            props,
        },
    );
    assert!(text.contains(
        "            label \"First\" at 1,1\n            panel \"Box\" name panel1 at 7,3 span 6 rows 3\n                label \"Inside\" at 1,1\n"
    ), "{text}");
}

#[test]
fn rejects_bad_widget_edits() {
    let page = at(PAGE, "page Card");
    let mut bad = new_props(WidgetKind::Button, 1, 1);
    bad.bind = Some("c.name".into());
    assert!(
        error(
            PAGE,
            Edit::PlaceWidget {
                container: page,
                props: bad
            }
        )
        .contains("cannot be bound")
    );

    let mut bad = new_props(WidgetKind::Label, 1, 1);
    bad.columns = vec!["x".into()];
    assert!(
        error(
            PAGE,
            Edit::PlaceWidget {
                container: page,
                props: bad
            }
        )
        .contains("'columns' does not apply")
    );

    let zero = new_props(WidgetKind::Label, 0, 1);
    assert!(
        error(
            PAGE,
            Edit::PlaceWidget {
                container: page,
                props: zero
            }
        )
        .contains("start at 1")
    );

    let inside_label = new_props(WidgetKind::Label, 1, 1);
    assert!(
        error(
            PAGE,
            Edit::PlaceWidget {
                container: at(PAGE, "textbox"),
                props: inside_label
            }
        )
        .contains("is not a page, panel, tab or pane")
    );

    let wrong_kind = new_props(WidgetKind::Button, 3, 1);
    assert!(
        error(
            PAGE,
            Edit::UpdateWidget {
                item: at(PAGE, "textbox"),
                container: None,
                props: wrong_kind
            }
        )
        .contains("cannot be turned into")
    );

    let mut into_itself = new_props(WidgetKind::Panel, 1, 1);
    into_itself.caption = Some("Box".into());
    assert!(
        error(
            PAGE,
            Edit::UpdateWidget {
                item: at(PAGE, "panel"),
                container: Some(at(PAGE, "panel")),
                props: into_itself
            }
        )
        .contains("into itself")
    );
}

#[test]
fn removes_widgets_tabs_and_data() {
    let (text, _) = edit(
        PAGE,
        Edit::Remove {
            item: at(PAGE, "panel"),
        },
    );
    assert!(!text.contains("panel") && !text.contains("Inside"));

    let (text, _) = edit(
        PAGE,
        Edit::Remove {
            item: at(PAGE, "data c"),
        },
    );
    assert!(!text.contains("data c"));

    let (text, _) = edit(
        PAGE,
        Edit::Remove {
            item: at(PAGE, "tab \"One\""),
        },
    );
    assert!(text.ends_with("    tabs at 1,7 span 12 rows 4\n"), "{text}");

    assert!(
        error(
            PAGE,
            Edit::Remove {
                item: at(PAGE, "title")
            }
        )
        .contains("is not a window, app, entity")
    );
}

#[test]
fn adds_tabs_data_grid_and_pages() {
    let (text, focus) = edit(
        PAGE,
        Edit::AddTab {
            item: at(PAGE, "tabs"),
            title: "Two".into(),
        },
    );
    assert!(text.ends_with("        tab \"Two\"\n"), "{text}");
    assert!(text[focus.unwrap() as usize..].starts_with("tab \"Two\""));

    let page = at(PAGE, "page Card");
    let (text, _) = edit(
        PAGE,
        Edit::AddData {
            page,
            name: "orders".into(),
            entity: "Order".into(),
            list: true,
        },
    );
    assert!(text.contains("    data c: Customer\n    data orders: list<Order>\n"));

    let (text, _) = edit(PAGE, Edit::SetGrid { page, columns: 16 });
    assert!(text.contains("    data c: Customer\n    grid 16 columns\n"));
    let (text, _) = edit(
        &text,
        Edit::SetGrid {
            page: at(&text, "page Card"),
            columns: 8,
        },
    );
    assert!(text.contains("    grid 8 columns\n") && !text.contains("16"));
    assert!(error(PAGE, Edit::SetGrid { page, columns: 40 }).contains("1 to 24"));

    // The window's size: set, changed, and removed for the standard one.
    let (text, _) = edit(PAGE, Edit::SetSize { page, width: Some(320), height: Some(460) });
    assert!(text.contains("    data c: Customer\n    size 320 x 460\n"), "{text}");
    let (text, _) = edit(&text, Edit::SetSize { page: at(&text, "page Card"), width: Some(400), height: Some(300) });
    assert!(text.contains("    size 400 x 300\n") && !text.contains("320"), "{text}");
    let (text, _) = edit(&text, Edit::SetSize { page: at(&text, "page Card"), width: None, height: None });
    assert!(!text.contains("size"), "{text}");

    let (text, focus) = edit(
        PAGE,
        Edit::AddPage {
            name: "Settings".into(),
            kind: PageKind::Dialog,
        },
    );
    assert!(
        text.ends_with(
            "            label \"First\" at 1,1\n\npage Settings: dialog\n    title \"Settings\"\n"
        ),
        "{text:?}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("page Settings"));
    assert!(
        error(
            PAGE,
            Edit::AddPage {
                name: "Card".into(),
                kind: PageKind::View
            }
        )
        .contains("already declared")
    );
}

#[test]
fn renames_pages_and_widget_tabs() {
    let (text, _) = edit(
        PAGE,
        Edit::SetTitle {
            target: at(PAGE, "page Card"),
            title: "Customer".into(),
        },
    );
    assert!(text.contains("    title \"Customer\"\n"));
    let (text, _) = edit(
        PAGE,
        Edit::SetTitle {
            target: at(PAGE, "tab \"One\""),
            title: "Orders".into(),
        },
    );
    assert!(text.contains("        tab \"Orders\"\n"));
}

#[test]
fn edits_keep_the_example_page_valid() {
    let source = include_str!("../../../examples/shop/desk.clarity");
    let mut props = new_props(WidgetKind::Number, 8, 3);
    props.bind = Some("customer.credit_limit".into());
    props.width = 2;
    let (text, _) = edit(
        source,
        Edit::UpdateWidget {
            item: at(source, "number"),
            container: None,
            props,
        },
    );
    let customers = include_str!("../../../examples/shop/customers.clarity");
    let sales = include_str!("../../../examples/shop/sales.clarity");
    let project = clarity_spec::load_sources([
        ("customers.clarity", customers),
        ("sales.clarity", sales),
        ("desk.clarity", text.as_str()),
    ]);
    let errors: Vec<_> = project
        .diagnostics
        .iter()
        .filter(|d| d.severity == clarity_spec::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn creates_widgets_with_catalog_defaults() {
    let (text, _) = edit(
        PAGE,
        Edit::CreateWidget {
            container: at(PAGE, "panel"),
            kind: WidgetKind::Textbox,
            col: 1,
            row: 2,
            width: Some(5),
            caption: None,
            bind: Some("c.email".into()),
            part: None,
            height: None,
        },
    );
    assert!(
        text.contains(
            "        label \"Inside\" at 1,1\n        textbox name email at 1,2 span 5 bind c.email\n"
        ),
        "{text}"
    );
}

#[test]
fn applies_a_batch_as_one_edit() {
    let page = at(PAGE, "page Card");
    let (text, focus) = edit(
        PAGE,
        Edit::Batch {
            edits: vec![
                Edit::CreateWidget {
                    container: page,
                    kind: WidgetKind::Label,
                    col: 1,
                    row: 2,
                    width: Some(2),
                    caption: Some("E-mail".into()),
                    bind: None,
                    part: None,
                    height: None,
                },
                Edit::CreateWidget {
                    container: page,
                    kind: WidgetKind::Textbox,
                    col: 3,
                    row: 2,
                    width: None,
                    caption: None,
                    bind: Some("c.email".into()),
                    part: None,
                    height: None,
                },
            ],
        },
    );
    assert!(
        text.contains(
            "    label \"E-mail\" name eMailLabel at 1,2 span 2\n    textbox name email at 3,2 span 4 bind c.email\n    panel"
        ),
        "{text}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("textbox name email at 3,2"));

    // One failing edit refuses the whole batch.
    let failing = Edit::Batch {
        edits: vec![
            Edit::CreateWidget {
                container: page,
                kind: WidgetKind::Label,
                col: 1,
                row: 2,
                width: None,
                caption: None,
                bind: None,
                part: None,
                height: None,
            },
            Edit::Remove { item: 9999 },
        ],
    };
    assert!(apply(PAGE, &failing).is_err());
}

#[test]
fn no_op_edit_round_trips_exactly() {
    let sources = [
        PAGE,
        "module m\r\npage P: window\r\n    title \"T\"\r\n    label \"A\" at 1,1 // c\r\n",
        "module m\n\n\n   \npage P: window\n\t\n    title \"T\"",
        include_str!("../../../examples/shop/desk.clarity"),
        include_str!("../../../examples/shop/customers.clarity"),
        include_str!("../../../examples/shop/sales.clarity"),
    ];
    // Every edit rewrites the whole file from the line tree, so an edit that
    // changes nothing must give back the input byte for byte.
    for source in sources {
        let pages: Vec<u32> = source
            .match_indices("page ")
            .filter(|(i, _)| *i == 0 || source.as_bytes()[i - 1] == b'\n')
            .map(|(i, _)| i as u32)
            .collect();
        assert!(!pages.is_empty());
        for target in pages {
            // Setting a page's title to what it already is changes nothing.
            // A title is a key, or a text without letters in quotes.
            let title = source[target as usize..]
                .lines()
                .find_map(|l| {
                    let rest = l.trim().strip_prefix("title ")?;
                    Some(rest.strip_prefix('"').map_or(rest.to_string(), |t| t.trim_end_matches('"').to_string()))
                })
                .unwrap_or_default();
            let outcome = apply(source, &Edit::SetTitle { target, title }).unwrap();
            assert_eq!(outcome.text, source);
        }
    }
}

#[test]
fn rejects_stale_offsets() {
    let msg = apply(PAGE, &Edit::Remove { item: 9999 }).unwrap_err().0;
    assert!(msg.contains("may have changed"));
}

#[test]
fn removes_a_whole_window() {
    let source = format!("{PAGE}\npage Other: dialog\n    title \"Other\"\n");
    let text = apply(&source, &Edit::Remove { item: at(&source, "page Card") })
        .unwrap()
        .text;
    assert!(!text.contains("page Card"), "{text}");
    assert!(text.contains("page Other: dialog"), "{text}");
}

#[test]
fn a_part_keeps_the_style_it_is_given() {
    let mut props = new_props(WidgetKind::Textbox, 3, 2);
    props.caption = None;
    props.style = Some("loud".into());
    let (text, _) = edit(PAGE, Edit::UpdateWidget { item: at(PAGE, "textbox"), container: None, props });
    assert!(text.contains("    textbox name textbox1 at 3,2 span 4 style loud"), "{text}");
    let (file, diagnostics) = parse(&text, "p.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let textbox = file.pages[0].widgets.iter().find(|w| w.props.kind == WidgetKind::Textbox).unwrap();
    assert_eq!(textbox.props.style.as_deref(), Some("loud"));
}

/// The designer resizes a container and scales its parts in one batch: the
/// container's line changes, and so do the lines inside it.
#[test]
fn a_batch_resizes_a_container_and_moves_its_parts() {
    let (file, _) = parse(PAGE, "");
    let page = &file.pages[0];
    let panel = page.widgets.iter().find(|w| w.props.kind == WidgetKind::Panel).unwrap();
    let inside = &panel.children[0];
    let mut wider = panel.props.clone();
    wider.width = 12;
    let mut moved = inside.props.clone();
    moved.col = 3;
    moved.width = 4;
    let (text, _) = edit(
        PAGE,
        Edit::Batch {
            edits: vec![
                Edit::UpdateWidget { item: panel.span.start, container: None, props: wider },
                Edit::UpdateWidget { item: inside.span.start, container: None, props: moved },
            ],
        },
    );
    assert!(text.contains("    panel \"Box\" name panel1 at 1,3 span 12 rows 3\n        label \"Inside\" name label1 at 3,1 span 4\n"), "{text}");
}

/// Every part has a name, as in an IDE: a part written without one (by hand
/// or by the agent) gets one the first time the designer changes it, so its
/// events can be wired and its properties set from code.
#[test]
fn a_part_without_a_name_gets_one_when_it_is_changed() {
    let mut props = new_props(WidgetKind::Label, 1, 1);
    props.caption = Some("Name:".into());
    props.width = 2;
    let (text, _) = edit(PAGE, Edit::UpdateWidget { item: at(PAGE, "label \"Name\""), container: None, props: props.clone() });
    assert!(text.contains("    label \"Name:\" name label1 at 1,1 span 2\n"), "{text}");
    // The next one gets the next free name.
    let (text, _) = edit(&text, Edit::UpdateWidget { item: at(&text, "label \"Inside\""), container: None, props: { let mut p = props.clone(); p.caption = Some("In".into()); p } });
    assert!(text.contains("label \"In\" name label2 at 1,1"), "{text}");
}

/// Name them: every part of every window and the app that has no name gets
/// one after its purpose, or the first free one of its kind when its line
/// says none; the rest of each line stays as written.
#[test]
fn names_every_part_that_has_none() {
    let (text, _) = edit(PAGE, Edit::NameParts);
    assert!(text.contains("    label \"Name\" name nameLabel at 1,1 span 2\n"), "{text}");
    assert!(text.contains("    textbox name name at 3,1 span 6 bind c.name // the name\n"), "{text}");
    assert!(text.contains("    panel \"Box\" name boxPanel at 1,3 span 6 rows 3\n        label \"Inside\" name insideLabel at 1,1\n"), "{text}");
    assert!(text.contains("            label \"First\" name firstLabel at 1,1\n"), "{text}");
    // Named already: nothing changes.
    assert_eq!(edit(&text, Edit::NameParts).0, text);
}

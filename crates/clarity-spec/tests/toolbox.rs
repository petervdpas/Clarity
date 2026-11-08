//! `toolbox popup` / `toolbox slideout`: a painting canvas's tools in a toolbox (docs/IMAGE-PLAN.md).

use clarity_spec::{Severity, load_sources};

const SKETCH: &str = "module m

entity Sheet
    picture: image

page Desk: window
    data sheet: Sheet
    canvas name pad at 1,1 span 6 rows 8 bind sheet.picture mode paint toolbox on change: painted

code Desk
    handler painted
        save sheet
";

fn errors(text: &str) -> Vec<String> {
    load_sources([("m.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_painting_canvas_has_a_toolbox_as_a_popup_or_a_slideout() {
    assert!(errors(SKETCH).is_empty(), "{:#?}", errors(SKETCH));
    let project = load_sources([("m.clarity", SKETCH)]);
    let pad = &project.files[0].pages[0].widgets[0];
    assert_eq!(Some(clarity_spec::model::ToolboxKind::Popup), pad.props.toolbox);
    assert!(clarity_spec::widgets::widget_line(&pad.props).contains("mode paint toolbox"));
    let slide = load_sources([("m.clarity", SKETCH.replace("mode paint toolbox", "mode paint toolbox slideout").as_str())]);
    let pad = &slide.files[0].pages[0].widgets[0];
    assert_eq!(Some(clarity_spec::model::ToolboxKind::Slideout), pad.props.toolbox);
    assert!(clarity_spec::widgets::widget_line(&pad.props).contains("toolbox slideout"));
    let e = errors(&SKETCH.replace("mode paint toolbox", "toolbox"));
    assert!(e.iter().any(|m| m.contains("a toolbox holds the tools of a canvas in mode objects, paint or vector")), "{e:?}");
    // The new paint tools are known.
    assert!(errors(&SKETCH.replace("mode paint toolbox", "mode paint toolbox tools pen, fill, picker")).is_empty());
}

const DESIGNED: &str = "module m

entity Sheet
    picture: image

page Painting: toolbox
    toolgroup name drawGroup at 1,1
        tool name penTool icon \"paintbrush\" does pen
        tool name pencilTool icon \"pencil\" does pencil
    tool name fillTool at 2,1 icon \"paint-bucket\" does fill
    tool name starTool at 3,1 icon \"star\" on click: stamp
    toolsize name sizeOption at 1,2 span 4
    toolcolors name colorOption at 1,3 span 2

page Desk: window
    data sheet: Sheet
    canvas name pad at 1,1 span 6 rows 8 bind sheet.picture mode paint toolbox Painting on change: painted

code Painting
    handler stamp
        let n = 1

code Desk
    handler painted
        save sheet
";

/// `page Painting: toolbox`: a toolbox designed as a view, its parts on its grid, a
/// tool built in (`does`) or coded (`on click`), placed in the canvas that names it.
#[test]
fn a_toolbox_is_designed_as_a_view_and_placed_in_its_canvas() {
    assert!(errors(DESIGNED).is_empty(), "{:#?}", errors(DESIGNED));
    let project = load_sources([("m.clarity", DESIGNED)]);
    let desk = project.files[0].pages.iter().find(|p| p.name.name == "Desk").unwrap();
    assert!(clarity_spec::widgets::widget_line(&desk.widgets[0].props).contains("toolbox Painting"));
    let placed = clarity_spec::views::placed(&project);
    let desk = placed.files[0].pages.iter().find(|p| p.name.name == "Desk").expect("the window stays");
    assert!(placed.files[0].pages.iter().all(|p| p.name.name != "Painting"), "the toolbox is the window's now");
    let pad = &desk.widgets[0];
    assert_eq!(5, pad.children.len(), "its parts are the canvas's");
    assert!(pad.children.iter().all(|c| c.props.for_part.as_deref() == Some("pad")), "each works on the canvas");
    assert_eq!(Some("pad"), pad.children[0].children[0].props.for_part.as_deref(), "a group's tools too");
    assert!(placed.files[0].code.iter().any(|c| c.name.name == "Desk" && c.handlers.iter().any(|h| h.name.name == "stamp")), "its code is the window's");
}

#[test]
fn a_toolbox_s_parts_and_tools_are_checked() {
    let e = errors(&DESIGNED.replace("toolbox Painting", "toolbox Nothing"));
    assert!(e.iter().any(|m| m.contains("there is no toolbox 'Nothing'")), "{e:?}");
    let e = errors(&DESIGNED.replace("does fill", "does path"));
    assert!(e.iter().any(|m| m.contains("Painting's tool 'path' is no tool of a canvas in mode paint")), "{e:?}");
    let e = errors(&DESIGNED.replace("does fill", "does nothing"));
    assert!(e.iter().any(|m| m.contains("a tool does one of")), "{e:?}");
    let e = errors(&DESIGNED.replace("    tool name fillTool at 2,1 icon \"paint-bucket\" does fill\n", "    tool name fillTool at 2,1 icon \"paint-bucket\"\n"));
    assert!(e.iter().any(|m| m.contains("does a built-in action (does pencil) or runs a handler")), "{e:?}");
    let e = errors(&DESIGNED.replace("    data sheet: Sheet\n", "    data sheet: Sheet\n    toolsize name stray at 7,1 span 4\n"));
    assert!(e.iter().any(|m| m.contains("belongs in a toolbox")), "{e:?}");
}

/// New window offers a toolbox to start from: a painting canvas's tools on a grid of 4, which checks clean.
#[test]
fn a_new_toolbox_starts_from_the_painting_tools() {
    let project = load_sources([("m.clarity", "module m\n")]);
    let page = clarity_spec::templates::page_from_template(&project, "Painting", &clarity_spec::templates::PageTemplate::Toolbox).unwrap();
    let text = format!("module m\n\n{page}\n");
    assert!(errors(&text).is_empty(), "{:#?}\n{text}", errors(&text));
    let loaded = load_sources([("m.clarity", text.as_str())]);
    assert_eq!(clarity_spec::model::PageKind::Toolbox, loaded.files[0].pages[0].kind);
}

/// Layers and the colour dialog are a picture's: a vector canvas's toolbox has neither.
#[test]
fn layers_belong_to_a_painting_canvas() {
    let text = DESIGNED.replace("    toolcolors name colorOption at 1,3 span 2\n", "    toolcolors name colorOption at 1,3 span 2\n    toollayers name layerList at 1,4 span 4 rows 3\n    tool name adjustTool at 4,1 does adjust\n");
    assert!(errors(&text).is_empty(), "{:#?}", errors(&text));
    let vector = text.replace("picture: image", "drawing: text").replace("bind sheet.picture mode paint", "bind sheet.drawing mode vector").replace("does pen", "does line").replace("does pencil", "does rect").replace("does fill", "does path");
    let e = errors(&vector);
    assert!(e.iter().any(|m| m.contains("layers are a picture's")), "{e:?}");
    assert!(e.iter().any(|m| m.contains("'adjust' is no tool of a canvas in mode vector")), "{e:?}");
}

/// A toolbox on a canvas of a view (a diagram in a slide-out) is placed too:
/// its parts are the canvas's in the window that shows the view, and its
/// tools are parts that code reaches.
#[test]
fn a_toolbox_inside_a_view_is_placed_in_the_window() {
    let source = "module m

record Box
    id: integer
    x: integer
    y: integer

page Main: window
    slideout name diagramPanel side bottom view Diagram height 300 span 12 rows 6

page Diagram: view
    data boxes: list<Box>
    canvas name sheet at 1,1 span 12 rows 5 units 4000 x 3000 mode objects boxes at x, y toolbox DiagramTools on change: moved on connect: connected

code Diagram
    handler moved
        let b = sheet.selected

    handler connected
        hideTool.visible = false

page DiagramTools: toolbox
    grid 2 columns
    tool name selectTool at 1,1 icon \"mouse-pointer-2\" does select
    tool name hideTool at 2,1 icon \"plus\" on click: connected
";
    let project = load_sources([("m.clarity", source)]);
    let found: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(found.is_empty(), "{found:#?}");
    let placed = clarity_spec::views::placed(&project);
    let main = placed.files[0].pages.iter().find(|p| p.name.name == "Main").unwrap();
    let panel = &main.widgets[0];
    let sheet = panel.children.iter().find(|w| w.props.name.as_deref() == Some("sheet")).expect("the view's canvas is the window's");
    let tools: Vec<&str> = sheet.children.iter().filter_map(|w| w.props.name.as_deref()).collect();
    assert_eq!(tools, ["selectTool", "hideTool"], "the toolbox's parts are the canvas's");
}

/// A memo's toolbox inside a view that an accordion section shows (as
/// Writer's chapters do): its tools are the memo's in the window.
#[test]
fn a_memo_toolbox_inside_a_view_of_an_accordion_section_is_placed() {
    let source = "module m

entity Chapter
    text: text

page Main: window
    accordion name parts at 1,1 span 12 rows 8
        section \"Chapters\" view ChaptersPart

page ChaptersPart: view
    data chapter: Chapter
    memo name manuscriptBox at 1,1 span 12 rows 6 bind chapter.text markdown toolbox ManuscriptTools

page ManuscriptTools: toolbox
    grid 2 columns
    tool name boldTool at 1,1 icon \"bold\" on click: bolded

code ChaptersPart
    handler bolded
        manuscriptBox.insert(\"**\")
";
    let project = load_sources([("m.clarity", source)]);
    let found: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(found.is_empty(), "{found:#?}");
    let placed = clarity_spec::views::placed(&project);
    fn find<'w>(widgets: &'w [clarity_spec::model::Widget], name: &str) -> Option<&'w clarity_spec::model::Widget> {
        widgets.iter().find_map(|w| {
            if w.props.name.as_deref() == Some(name) {
                return Some(w);
            }
            find(&w.children, name).or_else(|| w.pages.iter().find_map(|p| find(&p.children, name)))
        })
    }
    let main = placed.files[0].pages.iter().find(|p| p.name.name == "Main").unwrap();
    let memo = find(&main.widgets, "manuscriptBox").expect("the view's memo is the window's");
    let tools: Vec<&str> = memo.children.iter().filter_map(|w| w.props.name.as_deref()).collect();
    assert_eq!(tools, ["boldTool"], "the toolbox's tools are the memo's");
}

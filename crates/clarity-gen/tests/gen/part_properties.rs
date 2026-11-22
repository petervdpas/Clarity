//! Every property of Clarity's own parts that code reads or sets
//! (`clarity_spec::code::check::PART_PROPERTIES`) is written by every
//! generator: a window that reads each one, and sets each that may be set,
//! generates on all five stacks without a property left out.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};
use clarity_spec::code::check::PART_PROPERTIES;

const SWEEP: &str = "module sweep

entity Folder
    name: text, label \"Name\"
    parentId: integer

entity Row
    name: text, label \"Name\"

entity Note
    drawing: text

page Main: window
    data folders: list<Folder>
    data rows: list<Row>
    data note: Note
    data shown: text
    data done: bool
    data level: integer
    data seen: text
    data count: integer
    data on: bool
    data route: list<integer>
    sidebar left name nav width 280 min 200 max 480 resizable collapsible collapsed maximizable grid 4 columns
        text name navText at 1,1 span 4 bind shown
    ribbon name mainRibbon collapsible collapsed
        tab \"Home\" name homeTab
            group \"Clipboard\" name clipboardGroup
                button \"Paste\" name pasteButton large on click: sweep
        tab \"View\" name viewTab
            group \"Zoom\" name zoomGroup
                button \"Zoom in\" name zoomInButton on click: sweep
    button \"Go\" name goButton at 1,1 span 2 on click: sweep
    textbox name nameBox at 3,1 span 2
    checkbox \"Done\" name doneBox at 5,1 span 2
    spinbox name levelBox at 7,1 span 2 bind level min 1 max 99
    panel \"Details\" name details at 1,2 span 6 rows 2 foldable folded
    tree name folderTree at 1,4 span 6 rows 4 bind folders show name key id parent parentId fold buttons
    table name rowTable at 7,4 span 6 rows 4 bind rows
    tabs name pages at 1,8 span 6 rows 3
        tab \"One\"
        tab \"Two\"
    steps name wizard at 7,8 span 6 rows 3 on next: sweep on finish: sweep
        step \"First\"
        step \"Second\"
    accordion name faq at 1,11 span 6 rows 3
        section \"Shipping\"
    slideout \"Order\" name orderSlide side left width 480 min 300 max 900 resizable pinnable span 6 rows 8
    canvas name sheet at 7,11 span 6 rows 4 units 3000 x 2000 zoom grid 20 snap background panel dots 3 muted on draw: sweep on connect: sweep on resize: sweep on bend: sweep on drag: sweep
    canvas name sketch at 1,15 span 6 rows 4 units 400 x 300 bind note.drawing mode vector tools select, line, rect brush 4 ink accent on change: sweep

code Main
    handler sweep
        on = goButton.visible
        on = goButton.enabled
        seen = goButton.caption
        seen = nameBox.text
        on = doneBox.checked
        seen = nameBox.error
        on = nameBox.readonly
        on = details.folded
        on = nav.collapsed
        on = nav.maximized
        on = orderSlide.pinned
        count = pages.tab
        count = mainRibbon.tab
        on = mainRibbon.collapsed
        count = wizard.step
        count = faq.open
        let folder = folderTree.selected
        let row = rowTable.selected
        count = sketch.selected
        count = sheet.x
        count = sheet.y
        count = sheet.fromX
        count = sheet.fromY
        count = sheet.grid
        on = sheet.snap
        on = folderTree.foldButtons
        on = sheet.connecting
        on = sheet.resizable
        seen = sheet.bent
        route = sheet.via
        seen = sheet.background
        count = sheet.dotSize
        seen = sheet.dotColor
        count = levelBox.min
        count = levelBox.max
        seen = sketch.tool
        count = sketch.brush
        seen = sketch.ink
        goButton.visible = true
        goButton.enabled = true
        goButton.caption = \"Go on\"
        nameBox.text = \"Ada\"
        doneBox.checked = true
        nameBox.error = \"Wrong\"
        nameBox.readonly = false
        details.folded = false
        nav.collapsed = false
        nav.maximized = false
        orderSlide.pinned = true
        pages.tab = 2
        mainRibbon.tab = 2
        mainRibbon.collapsed = false
        wizard.step = 2
        faq.open = 1
        folderTree.selected = folder
        rowTable.selected = row
        sheet.grid = 40
        sheet.snap = false
        folderTree.foldButtons = false
        sheet.connecting = true
        sheet.resizable = false
        sheet.background = \"#f5f0e1\"
        sheet.dotSize = 4
        sheet.dotColor = \"text\"
        levelBox.min = 2
        levelBox.max = 50
        sketch.tool = \"line\"
        sketch.brush = 6
        sketch.ink = \"text\"
";

#[test]
fn the_sweep_reads_every_part_property() {
    for property in PART_PROPERTIES {
        assert!(SWEEP.contains(&format!(".{property}\n")) || SWEEP.contains(&format!(".{property} ")), "the sweep reads no part's {property}");
    }
}

#[test]
fn every_stack_writes_every_part_property() {
    let project = clarity_spec::keyed::load_keyed([("sweep.clarity", SWEEP)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let mut settings = starter("Sweep", "sweep");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        if let Err(e) = generate(&project, &settings) {
            panic!("{backend:?}: {e:#?}");
        }
    }
}

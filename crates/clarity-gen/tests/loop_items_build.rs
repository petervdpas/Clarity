//! A loop whose body changes its item, or needs the window while it goes
//! through one of the window's lists, builds on the Rust stack: the item is
//! the list's own (as on Go, C, C++ and C#), a list a function is given is
//! its own copy, and a handler run inside the loop gets the whole window.
//! Found while building examples/maintenance (2026-11-13): the generated Rust
//! did not pass the borrow checker. A whole build, as the IDE builds it and
//! without a warning, so it is ignored:
//! `cargo test -p clarity-gen --test loop_items_build -- --ignored`.

mod ide_build;

const MODULE: &str = "module ledger

record Entry
    key: text
    body: text
    progress: integer
    stale: bool
    flag: bool

// A list given to a function is its own copy: its items change, and so does the list.
function marked(rows: list<Entry>) -> list<Entry>
    let out: list<Entry> = []
    for e in rows
        e.stale = true
        out = out + [e]
    rows = out
    return rows

// An action changes the items of the list it is given, and gives it back.
action counted(rows: list<Entry>) -> list<Entry>
    for e in rows
        e.progress = count(rows)
    return rows

// A local list whose items change while the body reads the list.
function numbered(rows: list<Entry>) -> list<Entry>
    let all: list<Entry> = []
    all = all + rows
    for e in all
        e.progress = count(all)
    return all

page Main: window
    on open: loadEntries
    data entries: list<Entry>
    data piece: text
    data entryBody: text
    data shown: integer
    data picked: Entry
    list name entryList at 1,1 span 4 rows 6 bind entries show key
    label @ledgerCaption name footer at 5,1 span 4
    button @saveCaption name saveEntry at 5,2 span 2 on click: saveEntryClicked
    button @touchCaption name touch at 5,3 span 2 on click: touchClicked
    button @dropCaption name drop at 5,4 span 2 on click: dropClicked
    button @flagCaption name flag at 5,5 span 2 on click: flagClicked

code Main
    handler loadEntries
        let e = new Entry
        e.key = \"a\"
        entries = marked([e])
        entries = counted(entries)
        entries = numbered(entries)

    handler showFooter(e: Entry)
        shown = e.progress
        footer.caption = e.key

    // The item changes and a handler of the window runs with it.
    handler saveEntryClicked
        let rows: list<Entry> = []
        for e in entries
            if e.key == piece
                e.body = entryBody
                showFooter(e)
            rows = rows + [e]
        entries = rows

    // The item changes, the body sets the window's data and reads the list,
    // and a return in the body keeps what changed.
    handler touchClicked
        for e in entries
            e.progress = count(entries)
            shown = shown + 1
            picked = e
            if e.key == piece
                return
        entries = entries + [picked]

    // The item is only read while the body changes the list it goes through.
    handler dropClicked
        for e in entries
            if e.stale
                entries = []
                showFooter(e)

    handler flagEntry(key: text)
        for f in entries
            if f.key == key
                f.flag = true

    // A handler changes the item the loop holds, and the body changes it too:
    // both changes stay, as the item is the list's own.
    handler flagClicked
        for e in entries
            e.progress = 5
            flagEntry(e.key)
            e.body = entryBody
";

/// The Rust of the project's module, generated as the IDE generates it.
fn ledger_rust() -> String {
    let project = clarity_spec::load_converted([("ledger.clarity", MODULE)]);
    let files = clarity_gen::generate(&project, &clarity_project::starter("Ledger", "ledger")).unwrap_or_else(|e| panic!("{e:#?}"));
    files.into_iter().find(|f| f.path == "src/modules/ledger.rs").expect("the module's Rust").text
}

/// The quick half of the build below: what each loop holds its item as.
#[test]
fn a_loop_holds_its_item_as_the_borrow_checker_allows() {
    let rust = ledger_rust();
    // A list a function changes is its own copy.
    assert!(rust.contains("pub fn marked(db: &Db, rows: &[Entry]) -> Result<Vec<Entry>> {\n    let mut rows = rows.to_vec();"), "{rust}");
    assert!(rust.contains("pub fn counted(db: &Db, rows: &[Entry]) -> Result<Vec<Entry>> {\n    let mut rows = rows.to_vec();"), "{rust}");
    // A body that only reads the window beside its list borrows the item in the list.
    assert!(rust.contains("for e in rows.iter_mut() {"), "{rust}");
    // A body that needs the window holds the item by its place in the list, so a
    // handler it runs and the body change the same item; no copy is written back.
    assert!(rust.contains("while at_e < w.entries.len() {"), "{rust}");
    assert!(rust.contains("w.entries[at_e].progress = 5i64;"), "{rust}");
    assert!(rust.contains("main__show_footer(db, w, ui, w.entries[at_e].clone())?;"), "{rust}");
    assert!(!rust.contains("let mut e = w.entries"), "{rust}");
    assert!(!rust.contains("*item = e"), "{rust}");
    assert!(!rust.contains("w.entries.clone().iter()"), "{rust}");
    // The pass ends when a handler or the body leaves the item out of the list.
    assert!(rust.contains("if at_e >= w.entries.len() {\n"), "{rust}");
}

#[test]
#[ignore = "builds a generated Rust program; run with --ignored"]
fn loops_that_change_their_items_build_in_rust() {
    let root = ide_build::repo().join("target/generated/loop-items-rust");
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("ledger");
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    clarity_spec::cxaml::write_with_pages_moved_out(&dir.join("ledger.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    ledgerCaption \"Ledger\"\n    saveCaption \"Save\"\n    touchCaption \"Touch\"\n    dropCaption \"Drop\"\n    flagCaption \"Flag\"\n").unwrap();
    let file = root.join("ledger.clarity");
    std::fs::write(&file, "project Ledger\n    title \"Ledger\"\n    stack rust desktop\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/ledger.db\"\n").unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:#?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

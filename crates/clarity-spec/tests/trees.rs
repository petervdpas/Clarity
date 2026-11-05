//! Trees of U1c: `tree` and `treetable` show a list of records the window
//! holds as a tree, arranged by the reference to their own kind that the
//! list field `children` stands for (`parent: ref Folder`,
//! `children: list<Folder>`). A tree shows each record's text or the fields
//! of `show`; a tree table shows `columns`, the first one indented.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Severity, load_sources};

const FILES: &str = "module files

entity Folder
    name: text(80), required
    size: integer
    parent: ref Folder
    children: list<Folder>

page Files: window
    data folders: list<Folder>
    tree name folderTree at 1,1 span 4 rows 10 bind folders children children on select: picked
    treetable name grid at 5,1 span 8 rows 10 bind folders children children columns name 200, size
";

const CODE: &str = "
code Files
    handler picked
        grid.visible = folderTree.selected != none and folderTree.selected.size > 0
";

fn errors(page: &str) -> Vec<String> {
    let text = format!("{page}{CODE}");
    load_sources([("files.clarity", text.as_str())]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn trees_read_and_write_back() {
    let (file, diagnostics) = parse(FILES, "files.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert_eq!(parts[0].props.kind, WidgetKind::Tree);
    assert_eq!(parts[0].props.children.as_deref(), Some("children"));
    assert_eq!(widgets::widget_line(&parts[0].props), "tree name folderTree at 1,1 span 4 rows 10 bind folders children children on select: picked");
    assert_eq!(parts[1].props.kind, WidgetKind::TreeTable);
    assert_eq!(widgets::widget_line(&parts[1].props), "treetable name grid at 5,1 span 8 rows 10 bind folders children children columns name 200, size");
    let found = errors(FILES);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_rules_of_a_tree() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(FILES.replace("bind folders children children on select", "bind folders on select"), "a tree names the list of its own kind its rows have, as in 'children children'");
    says(FILES.replace("bind folders children children on select", "bind folders children name on select"), "'name' is no list of Folder records");
    says(FILES.replace("bind folders children children on select", "bind folders children branches on select"), "Folder has no field 'branches'");
    says(FILES.replace("    parent: ref Folder\n", ""), "a tree of Folder needs one field 'ref Folder' to know a row's parent");
    says(FILES.replace("    parent: ref Folder\n", "    parent: ref Folder\n    moved_from: ref Folder\n"), "a tree of Folder needs one field 'ref Folder' to know a row's parent");
}

#[test]
fn a_tree_has_clarity_s_own_words() {
    let text = format!("{FILES}{CODE}");
    let p = clarity_spec::keyed::load_keyed([("files.clarity", text.as_str())], &[], "en");
    for (key, word) in clarity_spec::translations::TREE_KEYS {
        assert_eq!(p.translations["en"].get(key).map(String::as_str), Some(word), "{key}");
    }
}

#[test]
fn sample_records_of_a_tree_form_one_for_the_designer() {
    let p = load_sources([("files.clarity", FILES)]);
    let rows = &clarity_spec::sample::sample_data(&p)["Folder"];
    let name = |i: usize| rows[i]["name"].clone();
    assert!(rows[0]["parent"].is_null() && rows[1]["parent"].is_null(), "two rows at the top");
    assert_eq!(rows[2]["parent"], name(0));
    assert_eq!(rows[3]["parent"], name(0));
    assert_eq!(rows[4]["parent"], name(1));
    assert_eq!(rows[6]["parent"], name(2), "a third level");
}

/// A tree by key: rows of any kind, records among them, arranged by a key
/// field and the field that holds the key of their parent.
const NODES: &str = "module files

record Node
    key: text
    parent: text
    label: text
    rank: integer
    up: integer

page Files: window
    data nodes: list<Node>
    tree name nav at 1,1 span 4 rows 10 bind nodes key key parent parent show label on select: picked
    treetable name grid at 5,1 span 8 rows 10 bind nodes key key parent parent columns label 200, rank

code Files
    handler picked
        grid.selected = nav.selected
";

fn node_errors(page: &str) -> Vec<String> {
    load_sources([("files.clarity", page)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_tree_by_key_reads_and_writes_back() {
    let (file, diagnostics) = parse(NODES, "files.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let tree = &file.pages[0].widgets[0].props;
    assert_eq!((tree.row_key.as_deref(), tree.parent_key.as_deref()), (Some("key"), Some("parent")));
    assert_eq!(widgets::widget_line(tree), "tree name nav at 1,1 span 4 rows 10 bind nodes key key parent parent show label on select: picked");
    assert_eq!(widgets::widget_line(&file.pages[0].widgets[1].props), "treetable name grid at 5,1 span 8 rows 10 bind nodes key key parent parent columns label 200, rank");
    let found = node_errors(NODES);
    assert!(found.is_empty(), "{found:#?}");
    // Whole numbers name rows too, and a stored record by its id.
    let numbers = NODES.replace("key key parent parent show", "key rank parent up show");
    assert!(node_errors(&numbers).is_empty(), "{:#?}", node_errors(&numbers));
    let ids = format!("{FILES}").replace("bind folders children children on select", "bind folders key id parent size on select");
    assert!(errors(&ids).is_empty(), "{:#?}", errors(&ids));
}

#[test]
fn the_rules_of_a_tree_by_key() {
    let says = |page: String, wanted: &str| {
        let found = node_errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(NODES.replace("key key parent parent show", "key key show"), "a tree by key names both fields, as in 'key id parent parentId'");
    says(NODES.replace("key key parent parent show", "parent parent show"), "a tree by key names both fields");
    says(NODES.replace("key key parent parent show", "key name parent parent show"), "Node has no field 'name'");
    says(NODES.replace("key key parent parent show", "key key parent parent children kids show"), "a tree is arranged by 'children' or by 'key' and 'parent', not both");
    says(NODES.replace("    rank: integer\n", "    rank: decimal\n").replace("key key parent parent show", "key rank parent up show"), "'rank' is decimal; a tree's key and parent are text or integer");
    says(NODES.replace("key key parent parent show", "key key parent up show"), "'key' is text and 'up' is integer");
    says(NODES.replace("key key parent parent show", "key id parent parent show"), "Node has no field 'id'");
    says(NODES.replace("tree name nav at 1,1 span 4 rows 10 bind nodes key key parent parent", "table name nav at 1,1 span 4 rows 10 bind nodes parent parent columns label"), "'parent' does not apply to a table");
}

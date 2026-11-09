//! A tree of several kinds of records (DESIGN 5.33): `level` lines, each
//! the records of an entity under the row of the level around it, `group`
//! headings shown when the levels inside have rows, `on select Entity:
//! handler` with the record picked, and `tree.reload()`. Placed, it is a
//! tree of records arranged by key, which every stack draws already.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Project
    name: text(80), label @projectNameLabel

entity Domain
    project: ref Project, label @domainProjectLabel
    name: text(80), label @domainNameLabel

entity Version
    domain: ref Domain, label @versionDomainLabel
    number: integer, label @versionNumberLabel

page Main: window
    on open: mainOpened
    content name main at 1,1 span 12 rows 10
        view Explorer

page Explorer: view
    data picked: text
    tree name modelTree at 1,1 span 4 rows 10
        level Project show name order by name
            level Domain under project show name order by name
                group @versionsNode
                    level Version under domain show @versionNode order by number desc
        on select Project: projectPicked
        on select Version: versionPicked
    button @saveCaption name saveButton at 5,1 span 2 on click: saveClicked

code Main
    handler mainOpened
        picked = \"\"

code Explorer
    handler projectPicked(p: Project)
        picked = p.name

    handler versionPicked(v: Version)
        picked = \"v\"

    handler saveClicked
        modelTree.reload()
";

const EN: &str = "translation en
    projectNameLabel \"Name\"
    domainProjectLabel \"Project\"
    domainNameLabel \"Name\"
    versionDomainLabel \"Domain\"
    versionNumberLabel \"Number\"
    versionsNode \"Versions\"
    versionNode \"Version {number}\"
    saveCaption \"Save\"
";

fn project(text: &str) -> Project {
    let mut project = clarity_spec::load_sources([("shop.clarity", text)]);
    project.locale.language = Some("en".into());
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    project
}

fn errors(text: &str) -> Vec<String> {
    project(text).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn has(text: &str, words: &str) {
    let found = errors(text);
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn a_tree_holds_levels_of_several_entities() {
    assert!(errors(SHOP).is_empty(), "{:#?}", errors(SHOP));
    let p = project(SHOP);
    let explorer = p.files[0].pages.iter().find(|p| p.name.name == "Explorer").unwrap();
    let tree = &explorer.widgets[0];
    assert_eq!(tree.levels[0].entity.as_deref(), Some("Project"));
    let domain = &tree.levels[0].levels[0];
    assert_eq!((domain.entity.as_deref(), domain.under.as_deref()), (Some("Domain"), Some("project")));
    let group = &domain.levels[0];
    assert_eq!(group.group.as_deref(), Some("@versionsNode"));
    assert_eq!(group.levels[0].show.as_deref(), Some("@versionNode"));
    assert!(group.levels[0].descending);
    assert_eq!(tree.selects.iter().map(|s| (s.entity.as_str(), s.handler.as_str())).collect::<Vec<_>>(), [("Project", "projectPicked"), ("Version", "versionPicked")]);
}

#[test]
fn the_rules_of_a_tree_of_levels() {
    has(&SHOP.replace("level Domain under project", "level Domain under name"), "Domain.name does not refer to Project");
    has(&SHOP.replace("level Domain under project", "level Domain under nothing"), "Domain has no field 'nothing'");
    has(&SHOP.replace("level Domain under project show name", "level Domain show name"), "say which field of Domain refers to Project");
    has(&SHOP.replace("level Project show name order by name", "level Nope show name"), "there is no entity 'Nope'");
    has(&SHOP.replace("order by number desc", "order by size"), "Version has no field 'size' to order by");
    has(&SHOP.replace("level Project show name", "level Project show title"), "Project has no field 'title' to show");
    has(&SHOP.replace("on select Version: versionPicked", "on select Domainx: versionPicked"), "the tree shows no Domainx rows");
    has(&SHOP.replace("handler versionPicked(v: Version)", "handler versionPicked(v: Project)"), "versionPicked takes the Version picked");
    has(&SHOP.replace("modelTree.reload()", "modelTree.clear()"), "it loads its rows again with modelTree.reload()");
    has(&SHOP.replace("tree name modelTree at", "tree at"), "a tree of levels needs a name");
}

#[test]
fn placed_the_tree_is_a_tree_of_its_rows_loaded_when_the_window_opens() {
    let placed = clarity_spec::views::placed(&project(SHOP));
    let file = &placed.files[0];
    let window = file.pages.iter().find(|p| p.name.name == "Main").unwrap();
    // The rows: a record with a field per entity, and a list of them as the window's data.
    let row = file.records.iter().find(|r| r.name.name == "ModelTreeRow").expect("the rows' record");
    let fields: Vec<&str> = row.fields.iter().map(|f| f.name.name.as_str()).collect();
    assert_eq!(fields, ["key", "parent", "label", "kind", "ofProject", "ofDomain", "ofVersion"]);
    assert!(window.data.iter().any(|d| d.name.name == "modelTreeRows"));
    fn find<'w>(widgets: &'w [clarity_spec::model::Widget], name: &str) -> Option<&'w clarity_spec::model::Widget> {
        widgets.iter().find_map(|w| if w.props.name.as_deref() == Some(name) { Some(w) } else { find(&w.children, name).or_else(|| w.pages.iter().find_map(|p| find(&p.children, name))) })
    }
    let tree = find(&window.widgets, "modelTree").unwrap();
    assert_eq!((tree.props.bind.as_deref(), tree.props.row_key.as_deref(), tree.props.parent_key.as_deref()), (Some("modelTreeRows"), Some("key"), Some("parent")));
    assert!(tree.levels.is_empty());
    let code = file.code.iter().find(|c| c.name.name == "Main").unwrap();
    let calls = |handler: &str| -> Vec<String> {
        let h = code.handlers.iter().find(|h| h.name.name == handler).unwrap_or_else(|| panic!("no handler {handler}"));
        h.body.iter().filter_map(|s| match &s.kind { StmtKind::Call { call } => match &call.kind { ExprKind::Call { function, .. } => Some(function.name.clone()), _ => None }, _ => None }).collect()
    };
    // The window's open handler loads the rows first; reload loads them again.
    assert_eq!(calls("mainOpened"), ["modelTreeLoad"]);
    assert_eq!(calls("saveClicked"), ["modelTreeLoad"]);
    assert!(code.handlers.iter().any(|h| h.name.name == "modelTreePicked"));
    // The placed project is clean: the generators check it again.
    let types = clarity_spec::code::check::types(&placed.files);
    let _ = types;
}

/// A tree of levels in a window without an open handler gets one that loads its rows.
#[test]
fn a_window_without_an_open_handler_loads_its_tree_too() {
    let text = SHOP.replace("    on open: mainOpened\n", "").replace("code Main\n    handler mainOpened\n        picked = \\\"\\\"\n\n", "");
    let text = text.replace("code Main\n    handler mainOpened\n        picked = \"\"\n\n", "");
    assert!(errors(&text).is_empty(), "{:#?}", errors(&text));
    let placed = clarity_spec::views::placed(&project(&text));
    let window = placed.files[0].pages.iter().find(|p| p.name.name == "Main").unwrap();
    assert!(window.handlers.iter().any(|h| h.handler == "modelTreeOpened"));
}

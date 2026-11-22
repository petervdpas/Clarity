use clarity_spec::dictionary::FieldSpec;
use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::{Literal, Modifiers};
use clarity_spec::parser::parse;

const FILE: &str = "module shop

use customers

enum Status
    Open
    Closed label \"Done\"

entity Order
    number: text(12), required // the order number
    total: money

page P: window
    title \"P\"
";

fn at(source: &str, needle: &str) -> u32 {
    let mut offset = 0;
    for line in source.split('\n') {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(needle) {
            return (offset + indent) as u32;
        }
        offset += line.len() + 1;
    }
    panic!("no line starts with {needle:?}")
}

fn edit(source: &str, edit: Edit) -> String {
    let text = apply(source, &edit).unwrap_or_else(|e| panic!("{e}")).text;
    let (_, diags) = parse(&text, "x.clarity");
    assert!(diags.is_empty(), "{diags:#?}\n{text}");
    text
}

fn error(source: &str, edit: Edit) -> String {
    apply(source, &edit).expect_err("edit should fail").0
}

fn spec(name: &str, type_text: &str) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        type_text: type_text.into(),
        modifiers: Modifiers::default(),
    }
}

#[test]
fn adds_entities_and_enums_after_the_data() {
    let text = edit(FILE, Edit::AddEntity { name: "Customer".into() });
    assert!(
        text.contains("    total: money\n\nentity Customer\n    name: text(80)\n\npage P"),
        "{text}"
    );
    let text = edit(&text, Edit::AddEnum { name: "Kind".into() });
    assert!(text.contains("entity Customer\n    name: text(80)\n\nenum Kind\n    First\n"), "{text}");
    assert!(error(FILE, Edit::AddEntity { name: "Order".into() }).contains("already declared"));
}

#[test]
fn adds_updates_and_reorders_fields() {
    let order = at(FILE, "entity Order");
    let mut s = spec("placed_on", "date");
    s.modifiers.required = true;
    let text = edit(FILE, Edit::AddField { entity: order, spec: s });
    assert!(text.contains("    total: money\n    placed_on: date, required\n"), "{text}");

    let mut s = spec("number", "text(20)");
    s.modifiers.unique = true;
    s.modifiers.label = Some("Number".into());
    s.modifiers.default = Some(Literal::Str("N-1".into()));
    let text = edit(&text, Edit::UpdateField { item: at(&text, "number:"), spec: s });
    assert!(
        text.contains("    number: text(20), unique, default \"N-1\", label \"Number\" // the order number\n"),
        "{text}"
    );

    let text = edit(&text, Edit::MoveSibling { item: at(&text, "total:"), delta: -1 });
    assert!(text.contains("entity Order\n    total: money\n    number:"), "{text}");
}

#[test]
fn rejects_bad_fields() {
    let order = at(FILE, "entity Order");
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("total", "money") }).contains("already has a field"));
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("x", "decimal") }).contains("precision"));
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("two words", "text") }).contains("not a valid field name"));
}

#[test]
fn edits_variants() {
    let status = at(FILE, "enum Status");
    let text = edit(FILE, Edit::AddVariant { owner: status, name: "Shipped".into(), label: None });
    assert!(text.contains("    Closed label \"Done\"\n    Shipped\n"), "{text}");
    let text = edit(&text, Edit::UpdateVariant { item: at(&text, "Open"), name: "New".into(), label: Some("Newly opened".into()) });
    assert!(text.contains("    New label \"Newly opened\"\n"), "{text}");
}

#[test]
fn removes_fields_and_declarations() {
    let text = edit(FILE, Edit::Remove { item: at(FILE, "total:") });
    assert!(!text.contains("total"));
    let text = edit(&text, Edit::Remove { item: at(&text, "enum Status") });
    assert!(!text.contains("Status"), "{text}");
}

#[test]
fn sets_the_uses_of_a_module() {
    let text = edit(FILE, Edit::SetUses { uses: vec!["customers".into(), "sales".into()] });
    assert!(text.starts_with("module shop\n\nuse customers\nuse sales\n\nenum Status"), "{text:?}");
    let text = edit(&text, Edit::SetUses { uses: vec![] });
    assert!(text.starts_with("module shop\n\nenum Status"), "{text:?}");
    let text = edit(&text, Edit::SetUses { uses: vec!["customers".into()] });
    assert!(text.starts_with("module shop\n\nuse customers\n\nenum Status"), "{text:?}");
}

/// The engine editor writes an engine's declaration anew from its inspector,
/// and removes it, at the offset the engine's span starts at.
#[test]
fn replaces_and_removes_an_engine_declaration() {
    let file = "module game\n\nengine Board from \"board.wasm\"\n    function play(square: integer) -> text\n\nenum Level\n    Easy\n";
    let item = parse(file, "game.clarity").0.engines[0].span.start;
    let replaced = edit(
        file,
        Edit::ReplaceDecl { item, text: "engine Board from \"board.wasm\"\n    uses random\n    state per window\n    function play(square: integer) -> text\n".into() },
    );
    assert!(replaced.contains("engine Board from \"board.wasm\"\n    uses random\n    state per window\n    function play"), "{replaced}");
    assert!(replaced.contains("enum Level"), "{replaced}");
    let removed = edit(&replaced, Edit::Remove { item });
    assert!(!removed.contains("engine") && removed.contains("enum Level"), "{removed}");
}

/// Delete theme in the designer removes the theme's declaration, at the
/// offset its span starts at, and leaves the rest of the module.
#[test]
fn removes_a_theme_declaration() {
    let file = "module game\n\ntheme Night\n    title \"Night\"\n    mode dark\n    color panel \"#1d2129\"\n\nenum Level\n    Easy\n";
    let item = parse(file, "game.clarity").0.themes[0].span.start;
    let removed = edit(file, Edit::Remove { item });
    assert!(!removed.contains("Night") && removed.contains("enum Level"), "{removed}");
}

/// One of Clarity's own settings (`language`, `theme`, and the agents'
/// `agentReadsWhileClosed`, `agentChangesWhileClosed`) goes into the file's
/// settings block as its bare word, or starts the block; once is enough.
#[test]
fn adds_one_of_clarity_s_own_settings() {
    let module = "module shop\n\nsettings\n    language\n    pageSize: integer, default 50\n";
    let text = edit(module, Edit::AddOwnSetting { word: "agentReadsWhileClosed".into() });
    assert!(text.contains("    pageSize: integer, default 50\n    agentReadsWhileClosed\n"), "{text}");
    assert_eq!(edit(&text, Edit::AddOwnSetting { word: "agentReadsWhileClosed".into() }), text, "added once");
    let fresh = edit("module shop\n", Edit::AddOwnSetting { word: "agentChangesWhileClosed".into() });
    assert!(fresh.contains("settings\n    agentChangesWhileClosed\n"), "{fresh}");
    assert_eq!(error(module, Edit::AddOwnSetting { word: "pageSize".into() }), "'pageSize' is none of Clarity's own settings: language, theme, agentReadsWhileClosed, agentChangesWhileClosed");
}

/// The settings block is edited as an entity's fields are, by hand in the
/// IDE: a setting is added, rewritten and removed, and so is one of
/// Clarity's own bare words.
#[test]
fn the_settings_block_is_edited_as_fields() {
    let module = "module shop\n\nsettings\n    language\n    pageSize: integer, default 50\n";
    let block = at(module, "settings");
    let text = edit(module, Edit::AddField { entity: block, spec: spec("exportFolder", "folder") });
    assert!(text.contains("    pageSize: integer, default 50\n    exportFolder: folder\n"), "{text}");
    let mut modifiers = Modifiers::default();
    modifiers.min = Some(Literal::Number("10".into()));
    let text = edit(&text, Edit::UpdateField { item: at(&text, "pageSize"), spec: FieldSpec { name: "pageSize".into(), type_text: "integer".into(), modifiers } });
    assert!(text.contains("    pageSize: integer, min 10\n"), "{text}");
    let text = edit(&text, Edit::Remove { item: at(&text, "language") });
    assert!(!text.contains("language"), "{text}");
    // A setting added by hand goes into the block, or starts it.
    let text = edit(&text, Edit::AddSetting { spec: spec("showToolbar", "bool") });
    assert!(text.ends_with("    exportFolder: folder\n    showToolbar: bool\n"), "{text}");
    assert!(error(&text, Edit::AddSetting { spec: spec("showToolbar", "bool") }).contains("already"));
    let fresh = edit("module shop\n", Edit::AddSetting { spec: spec("pageSize", "integer") });
    assert!(fresh.contains("settings\n    pageSize: integer\n"), "{fresh}");
}

/// Every type the IDE offers a setting is one a setting takes, and every
/// other simple type is refused, so the type list and the check agree.
#[test]
fn the_setting_types_are_the_ones_a_setting_takes() {
    use clarity_spec::dictionary::{SETTING_TYPES, SIMPLE_TYPES};
    let errors = |ty: &str| {
        let source = format!("module shop\n\nentity Customer\n    name: text\n\nenum Level\n    Low\n\nsettings\n    one: {ty}\n");
        let project = clarity_spec::keyed::load_keyed([("shop.clarity", source.as_str())], &[], "en");
        project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error && d.message.contains("a setting is a plain value")).count()
    };
    for ty in SETTING_TYPES {
        let written = match *ty {
            "ref" => "ref Customer",
            "enum" => "Level",
            other => other,
        };
        assert_eq!(errors(written), 0, "{ty}");
    }
    for ty in SIMPLE_TYPES.iter().filter(|t| !SETTING_TYPES.contains(t) && !["language", "theme"].contains(t)) {
        assert_eq!(errors(ty), 1, "{ty}");
    }
}

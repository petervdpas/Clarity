//! The manual follows the code and the IDE: every statement the parser
//! takes has its section on the statements page, every declaration word of
//! a module and every section of the IDE's project tree has its page, and
//! each such page names the word it is about. A new statement, declaration
//! or tree section without its page fails here. Every other word list of
//! the language (line words, reserved words, types, theme roles, tool
//! actions, built-in functions...) has each word somewhere in the manual.
//! The parts pages are checked by manual_catalog.rs.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn page(name: &str) -> String {
    std::fs::read_to_string(root().join("manual").join(format!("{name}.md"))).unwrap_or_else(|_| panic!("manual/{name}.md is not there"))
}

/// The page that teaches each declaration word of a module.
const DECLARATIONS: [(&str, &str); 21] = [
    ("module", "code.modules"),
    ("use", "code.modules"),
    ("source", "data.sources"),
    ("enum", "data.entities"),
    ("entity", "data.entities"),
    ("record", "data.entities"),
    ("settings", "data.settings"),
    ("page", "design.windows"),
    ("app", "design.frame"),
    ("function", "code.functions"),
    ("action", "code.functions"),
    ("engine", "code.engines"),
    ("const", "code.modules"),
    ("cli", "code.commands"),
    ("command", "code.commands"),
    ("part", "design.custom-parts"),
    ("library", "code.libraries"),
    ("mapping", "code.formats"),
    ("toast", "code.toasts"),
    ("theme", "themes"),
    ("code", "code.handlers"),
];

/// The page of each section of the IDE's project tree (app/src/lib/ProjectTree.svelte).
const TREE: [(&str, &str); 16] = [
    ("Windows", "design.windows"),
    ("Views", "design.windows"),
    ("Data", "data.entities"),
    ("Mappings", "code.formats"),
    ("Web APIs", "code.web-apis"),
    ("Diagrams", "data.diagrams"),
    ("Themes", "themes"),
    ("i18n", "languages"),
    ("Resources", "data.pictures-files"),
    ("Engines", "code.engines"),
    ("Custom parts", "design.custom-parts"),
    ("Libraries", "code.libraries"),
    ("Toasts", "code.toasts"),
    ("Functions", "code.functions"),
    ("Commands", "code.commands"),
    ("Modules", "code.modules"),
];

#[test]
fn every_statement_has_its_section() {
    let text = page("code.statements");
    let missing: Vec<&str> = clarity_spec::code::STATEMENT_WORDS.iter().copied().filter(|w| !text.lines().any(|l| l.trim() == format!("### `{w}`"))).collect();
    assert!(missing.is_empty(), "statements with no section (### `word`) on code.statements: {missing:?}");
}

#[test]
fn every_declaration_word_has_its_page() {
    assert_eq!(DECLARATIONS.map(|(w, _)| w), clarity_spec::parser::DECLARATION_WORDS, "each declaration word of the parser has its page here, in its order");
    // Named in the prose (`toast`) or opening a line of an example (toast saved).
    let names = |text: &str, w: &str| text.contains(&format!("`{w}")) || text.lines().any(|l| l.trim_start().starts_with(&format!("{w} ")));
    let missing: Vec<String> = DECLARATIONS.iter().filter(|(w, p)| !names(&page(p), w)).map(|(w, p)| format!("{w} on {p}")).collect();
    assert!(missing.is_empty(), "pages that do not name their declaration word: {missing:?}");
}

#[test]
fn every_section_of_the_project_tree_has_its_page() {
    let tree = std::fs::read_to_string(root().join("app/src/lib/ProjectTree.svelte")).unwrap();
    let sections: Vec<String> = tree.split("<summary>").skip(1).map(|s| s.split(" <span").next().unwrap_or("").split('<').next().unwrap_or("").trim().to_string()).filter(|s| !s.is_empty()).collect();
    for section in &sections {
        let (_, p) = TREE.iter().find(|(name, _)| name == section).unwrap_or_else(|| panic!("the project tree's section {section} has no page in this test's list"));
        page(p);
    }
    for (name, _) in TREE {
        assert!(sections.iter().any(|s| s == name), "{name} is no longer a section of the project tree");
    }
}

#[test]
fn no_page_links_to_the_old_documents() {
    let dir = root().join("manual");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        if text.contains("LANGUAGE.md") || text.contains("DESIGN.md") {
            found.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    assert!(found.is_empty(), "pages that still link the old documents: {found:?}");
}

/// Every page of the manual as one text.
fn whole_manual() -> String {
    let mut all = String::new();
    for entry in std::fs::read_dir(root().join("manual")).unwrap().flatten() {
        all.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
        all.push('\n');
    }
    all
}

/// A word the manual shows: in code (`word`, `word(`), or as a whole word.
/// A theme role's `_soft` tint counts when its role is named with "`_soft` tints".
fn shown(manual: &str, word: &str) -> bool {
    if manual.contains(&format!("`{word}")) {
        return true;
    }
    if let Some(role) = word.strip_suffix("_soft") {
        if manual.contains("`_soft` tints") && manual.contains(&format!("`{role}`")) {
            return true;
        }
    }
    manual.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).any(|w| w == word)
}

#[test]
fn every_word_of_the_language_is_in_the_manual() {
    use clarity_spec::{code, engines, highlight, model, names, parser, themes};
    let manual = whole_manual();
    let functions: Vec<&str> = code::check::PICTURE_FUNCTIONS.iter().map(|f| f.0).chain(code::check::TEXT_FUNCTIONS.iter().map(|f| f.0)).collect();
    let lists: Vec<(&str, Vec<&str>)> = vec![
        ("reserved words", code::RESERVED_WORDS.to_vec()),
        ("keywords", highlight::KEYWORDS.to_vec()),
        ("types", highlight::TYPES.to_vec()),
        ("values", highlight::LITERALS.to_vec()),
        ("line words", parser::LINE_WORDS.to_vec()),
        ("the app's own settings", parser::OWN_SETTINGS.to_vec()),
        ("built-in functions", functions),
        ("effects", code::EFFECTS.to_vec()),
        ("app slots", code::APP_SLOTS.to_vec()),
        ("file members", code::check::FILE_MEMBERS.to_vec()),
        ("reply members", code::check::REPLY_MEMBERS.to_vec()),
        ("diagram line ends", code::check::LINK_ENDS.to_vec()),
        ("diagram line styles", code::check::LINK_STYLES.to_vec()),
        ("diagram text styles", code::check::TEXT_STYLES.to_vec()),
        ("diagram text aligns", code::check::TEXT_ALIGNS.to_vec()),
        ("tool actions", model::TOOL_ACTIONS.to_vec()),
        ("engine services", engines::SERVICES.to_vec()),
        ("engine hooks", names::HOOKS.to_vec()),
        ("theme roles", themes::ROLES.to_vec()),
        ("theme fonts", themes::FONTS.to_vec()),
        ("theme sizes", themes::SIZES.to_vec()),
        ("theme radii", themes::RADII.to_vec()),
        ("theme part looks", themes::PART_KINDS.to_vec()),
        ("theme states", themes::STATES.to_vec()),
    ];
    let missing: Vec<String> = lists
        .iter()
        .flat_map(|(list, words)| words.iter().filter(|w| !w.is_empty() && !shown(&manual, w)).map(move |w| format!("{w} ({list})")))
        .collect();
    assert!(missing.is_empty(), "words of the language the manual never shows: {missing:#?}");
}

#[test]
fn every_operator_is_in_the_operators_table() {
    use clarity_spec::code::{BinaryOp, UnaryOp};
    use BinaryOp::*;
    // A new operator does not compile here until it is listed.
    fn listed(op: BinaryOp) {
        match op {
            Or | And | Eq | NotEq | Lt | LtEq | Gt | GtEq | Add | Sub | Mul | Div | WholeDiv | Mod | Contains | Matches => {}
        }
    }
    fn listed_unary(op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Not => "`not a`",
            UnaryOp::Neg => "`-x`",
        }
    }
    let ops = [Or, And, Eq, NotEq, Lt, LtEq, Gt, GtEq, Add, Sub, Mul, Div, WholeDiv, Mod, Contains, Matches];
    ops.iter().for_each(|op| listed(*op));
    let text = page("code.expressions");
    let table = text.split("## Operators").nth(1).and_then(|t| t.split("\n## ").next()).expect("code.expressions has its Operators section");
    let mut missing: Vec<String> = ops.iter().map(|op| op.symbol()).filter(|s| !table.contains(&format!("a {s} "))).map(String::from).collect();
    missing.extend([UnaryOp::Not, UnaryOp::Neg].map(listed_unary).into_iter().filter(|row| !table.contains(row)).map(String::from));
    missing.extend(["`x in [", "`a to b`"].into_iter().filter(|row| !table.contains(row)).map(String::from));
    assert!(missing.is_empty(), "operators missing from the Operators table of code.expressions: {missing:?}");
}

/// The words on code.expressions that teach each kind of expression.
const EXPRESSIONS: [(&str, &str); 24] = [
    ("Number", "Numbers are written"),
    ("Str", "Text is written in double quotes"),
    ("Text", "## Text with slots"),
    ("Bool", "true   false"),
    ("Empty", "`none` is the empty value"),
    ("Name", "A name is one of these"),
    ("Member", "A path goes from a record"),
    ("Unary and Binary", "## Operators"),
    ("Range", "`a to b`"),
    ("In", "`x in ["),
    ("Call", "## Built-in functions"),
    ("EngineCall", "calls a function of an engine"),
    ("Ask", "`ask @question"),
    ("Find", "`find c in Customer"),
    ("Files", "`files f in photos"),
    ("ReadText", "`read text of f`"),
    ("ReadBytes", "`read bytes of f`"),
    ("New", "`new Order`"),
    ("Key", "### Texts by key"),
    ("ListOf", "`[1, 2, 3]`"),
    ("Index", "is an item, counted from 1"),
    ("FormatOf", "`json of order`"),
    ("ReadFormat", "`read list<Order> from json"),
    ("Http", "`http get url`"),
];

#[test]
fn every_expression_and_built_in_function_is_on_the_expressions_page() {
    use clarity_spec::code::{check, ExprKind};
    // A new kind of expression does not compile here until it is in EXPRESSIONS.
    fn listed(kind: &ExprKind) {
        match kind {
            ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Text { .. } | ExprKind::Bool { .. } | ExprKind::Empty | ExprKind::Name { .. }
            | ExprKind::Member { .. } | ExprKind::Unary { .. } | ExprKind::Binary { .. } | ExprKind::Range { .. } | ExprKind::In { .. }
            | ExprKind::Call { .. } | ExprKind::EngineCall { .. } | ExprKind::Ask { .. } | ExprKind::Find { .. } | ExprKind::Files { .. }
            | ExprKind::ReadText { .. } | ExprKind::ReadBytes { .. } | ExprKind::New { .. } | ExprKind::Key { .. } | ExprKind::ListOf { .. }
            | ExprKind::Index { .. } | ExprKind::FormatOf { .. } | ExprKind::ReadFormat { .. } | ExprKind::Http { .. } => {}
        }
    }
    listed(&ExprKind::Empty);
    let text = page("code.expressions");
    let mut missing: Vec<String> = EXPRESSIONS.iter().filter(|(_, words)| !text.contains(words)).map(|(kind, words)| format!("{kind}: {words}")).collect();
    // Every list of built-in functions the checker has, and an engine's `progress`.
    let functions = check::BUILT_IN_FUNCTIONS.into_iter().chain(check::UNLESS_DECLARED).chain([check::PROGRESS]);
    let named = check::PICTURE_FUNCTIONS.iter().map(|f| f.0).chain(check::TEXT_FUNCTIONS.iter().map(|f| f.0));
    missing.extend(functions.chain(named).filter(|f| !text.contains(&format!("`{f}("))).map(|f| format!("{f}()")));
    assert!(missing.is_empty(), "missing from code.expressions: {missing:?}");
}

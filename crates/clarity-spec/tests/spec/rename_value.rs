//! Renaming a value of an enum across the project: its declaration and
//! every place that names it, found through the parsed model, never by
//! searching text: a field's, a setting's and a command option's default,
//! code that names it bare (`r.kind = Private`) or with its enum
//! (`Kind.Private`), a window's data default and a button's click value.
//! A value of the same name in another enum is never touched.

use clarity_spec::value_rename::rename_value;
use clarity_spec::{converted, load_sources};

const SHOP: &str = "module shop

enum Kind
    Private
    Business

enum Other
    Private
    Public

entity Customer
    name: text
    kind: Kind, default Private
    other: Other, default Private

settings
    startKind: Kind, default Private

action markPrivate(c: Customer)
    effects db.write
    c.kind = Private
    if c.kind == Kind.Private
        c.other = Private
    save c

command customer add(name: text)
    option kind: Kind, default Private
    let c = new Customer
    c.name = name
    c.kind = kind
    save c

page Card: window
    title \"Card\"
    data kind: Kind, default Private
    button \"Private\" name privateButton at 1,1 span 2 on click: pick(Private)

code Card
    handler pick(chosen: Kind)
        kind = chosen
";

fn sources() -> Vec<(String, String)> {
    converted([("shop.clarity", SHOP)])
}

fn load(sources: &[(String, String)]) -> clarity_spec::Project {
    load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())))
}

fn errors(project: &clarity_spec::Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn renames_a_value_everywhere_it_is_named() {
    let all = sources();
    let project = load(&all);
    assert!(errors(&project).is_empty(), "{:#?}", errors(&project));
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    let files = rename_value(&project, source, "Kind", "Private", "Personal").unwrap();
    let text = |path: &str| files.iter().find(|f| f.path == path).map(|f| f.text.clone()).unwrap_or_default();
    let shop = text("shop.clarity");
    assert!(shop.contains("enum Kind\n    Personal\n    Business\n"), "{shop}");
    assert!(shop.contains("enum Other\n    Private\n    Public\n"), "another enum's value stays: {shop}");
    assert!(shop.contains("kind: Kind, default Personal\n    other: Other, default Private\n"), "{shop}");
    assert!(shop.contains("startKind: Kind, default Personal\n"), "{shop}");
    assert!(shop.contains("    c.kind = Personal\n    if c.kind == Kind.Personal\n        c.other = Private\n"), "{shop}");
    assert!(shop.contains("option kind: Kind, default Personal\n"), "{shop}");
    let card = text("design/Card.cxaml");
    assert!(card.contains("default=\"Personal\"") || card.contains("Personal"), "{card}");
    assert!(card.contains("pick(Personal)"), "the click's value: {card}");
    // The project reads clean afterwards.
    let after: Vec<(String, String)> = all.iter().map(|(p, t)| (p.clone(), files.iter().find(|f| f.path == *p).map_or(t.clone(), |f| f.text.clone()))).collect();
    let after = load(&after);
    assert!(errors(&after).is_empty(), "{:#?}", errors(&after));
}

#[test]
fn refuses_what_it_cannot_rename() {
    let all = sources();
    let project = load(&all);
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    assert!(rename_value(&project, source, "Kind", "Private", "Business").unwrap_err().contains("Kind already has a value Business"));
    assert!(rename_value(&project, source, "Kind", "Secret", "Hidden").unwrap_err().contains("Kind has no value Secret"));
    assert!(rename_value(&project, source, "Sort", "Private", "Hidden").unwrap_err().contains("there is no enum Sort"));
    assert!(rename_value(&project, source, "Kind", "Private", "my value").unwrap_err().contains("is not a valid value name"));
}

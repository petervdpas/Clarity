//! The Translate engine: a type of the model as C# code, and the types of a
//! Formidable export (the `export.ndjson` of its `fcdm-entities` collection,
//! one entry a line) as types and parameters of the model.
//!
//! As C# code: A class, record or
//! struct gets a constructor that takes each parameter and sets the property
//! of the same name, and a property per parameter that is set once
//! (`{ get; init; }`); an interface gets a property per parameter to read.
//! A parameter is written in camelCase and its property in PascalCase,
//! whichever way the model writes the name.

use serde::{Deserialize, Serialize};
use serde_json::Value;

clarity_engine::interface!();

/// What the engine reads of a type of the model.
#[derive(Deserialize, Serialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct ClassType {
    pub name: String,
    pub namespace: String,
    /// Class, Record, Struct or Interface.
    pub kind: String,
    /// Entity or Event.
    pub ontology: String,
    pub interfaces: Vec<String>,
    pub summary: String,
}

/// What the engine reads of a parameter of a type.
#[derive(Deserialize, Serialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Parameter {
    pub position: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
}

clarity_engine::function!(csharp, |(t, base, parameters): (ClassType, String, Vec<Parameter>)| Ok(written(&t, &base, parameters)));
clarity_engine::function!(types_of, |(export,): (String,)| Ok(types(&export)));
clarity_engine::function!(base_of, |(export, name): (String, String)| Ok(entry(&export, &name).map(|data| text(&data, "base-entity")).unwrap_or_default()));
clarity_engine::function!(parameters_of, |(export, name): (String, String)| Ok(entry(&export, &name).map(|data| parameters(&data)).unwrap_or_default()));

/// The `data` of each entry of an export, one JSON object a line; a line that is no entry is left out.
fn entries(export: &str) -> impl Iterator<Item = Value> + '_ {
    export.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).filter_map(|entry| entry.get("data").cloned())
}

/// The entry of the type called name.
fn entry(export: &str, name: &str) -> Option<Value> {
    entries(export).find(|data| text(data, "name") == name)
}

fn text(data: &Value, key: &str) -> String {
    data.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The types of an export, as the model has them: their kind and ontology by the model's names.
pub fn types(export: &str) -> Vec<ClassType> {
    entries(export)
        .map(|data| ClassType {
            name: text(&data, "name"),
            namespace: text(&data, "namespace"),
            kind: first(&text(&data, "kind"), true),
            ontology: first(&text(&data, "ontology"), true),
            interfaces: data.get("interfaces").and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default(),
            summary: String::new(),
        })
        .filter(|t| !t.name.is_empty())
        .collect()
}

/// A type's constructor parameters, in their order: each a pair of its type and its name.
pub fn parameters(data: &Value) -> Vec<Parameter> {
    let pairs = data.get("constructor-parameters").and_then(Value::as_array).cloned().unwrap_or_default();
    pairs
        .iter()
        .filter_map(|pair| Some((pair.get(0)?.as_str()?.to_string(), pair.get(1)?.as_str()?.to_string())))
        .enumerate()
        .map(|(at, (kind, name))| Parameter { position: at as i64 + 1, kind, name })
        .collect()
}

/// The name with its first letter in capitals (`upper`) or not.
fn first(name: &str, upper: bool) -> String {
    let mut letters = name.chars();
    match letters.next() {
        Some(c) if upper => c.to_uppercase().chain(letters).collect(),
        Some(c) => c.to_lowercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// A type of the model as C# code, its parameters in their order.
pub fn written(t: &ClassType, base: &str, mut parameters: Vec<Parameter>) -> String {
    parameters.sort_by_key(|p| p.position);
    let mut out = String::new();
    if !t.namespace.is_empty() {
        out.push_str(&format!("namespace {};\n\n", t.namespace));
    }
    if !t.summary.is_empty() {
        out.push_str(&format!("/// <summary>{}</summary>\n", t.summary));
    }
    let bases: Vec<&str> = std::iter::once(base).filter(|b| !b.is_empty()).chain(t.interfaces.iter().map(String::as_str)).collect();
    let inherits = if bases.is_empty() { String::new() } else { format!(" : {}", bases.join(", ")) };
    let word = match t.kind.as_str() {
        "Record" => "record",
        "Struct" => "struct",
        "Interface" => "interface",
        _ => "class",
    };
    out.push_str(&format!("public {word} {}{inherits}\n{{\n", t.name));
    if word == "interface" {
        let properties: Vec<String> = parameters.iter().map(|p| format!("    {} {} {{ get; }}\n", p.kind, first(&p.name, true))).collect();
        out.push_str(&properties.join("\n"));
        out.push_str("}\n");
        return out;
    }
    let mut blocks = Vec::new();
    if !parameters.is_empty() {
        let taken: Vec<String> = parameters.iter().map(|p| format!("        {} {}", p.kind, first(&p.name, false))).collect();
        let set: Vec<String> = parameters.iter().map(|p| format!("        {} = {};\n", first(&p.name, true), first(&p.name, false))).collect();
        blocks.push(format!("    public {}(\n{})\n    {{\n{}    }}\n", t.name, taken.join(",\n"), set.concat()));
    }
    for p in &parameters {
        blocks.push(format!("    public {} {} {{ get; init; }}\n", p.kind, first(&p.name, true)));
    }
    out.push_str(&blocks.join("\n"));
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of Formidable's export, as it gives it.
    const EXPORT: &str = r#"{"data":{"attributes":[["Bk","string",false,false,false,false,false,"","init"]],"base-entity":"Werknemer","constructor-parameters":[["string","bk"],["string","code"],["string?","functieklasse"]],"id":"ba59","interfaces":["IHasCode"],"kind":"class","name":"Functie","namespace":"Fontys.Fcdm.Hr","ontology":"entity"},"filename":"functie.meta.json","id":"ba59"}
not a line of the export
{"data":{"constructor-parameters":[],"kind":"record","name":"Leeg","namespace":"","ontology":"event"}}
"#;

    #[test]
    fn an_export_gives_its_types_with_their_bases_and_parameters() {
        let found = types(EXPORT);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], ClassType { name: "Functie".into(), namespace: "Fontys.Fcdm.Hr".into(), kind: "Class".into(), ontology: "Entity".into(), interfaces: vec!["IHasCode".into()], summary: String::new() });
        assert_eq!((found[1].kind.as_str(), found[1].ontology.as_str()), ("Record", "Event"));
        let functie = entry(EXPORT, "Functie").unwrap();
        assert_eq!(text(&functie, "base-entity"), "Werknemer");
        let p = parameters(&functie);
        assert_eq!(p.iter().map(|p| (p.position, p.kind.as_str(), p.name.as_str())).collect::<Vec<_>>(), vec![(1, "string", "bk"), (2, "string", "code"), (3, "string?", "functieklasse")]);
        assert!(entry(EXPORT, "Missing").is_none());
    }

    fn parameter(position: i64, kind: &str, name: &str) -> Parameter {
        Parameter { position, kind: kind.into(), name: name.into() }
    }

    #[test]
    fn a_class_gets_a_constructor_and_a_property_set_once_per_parameter() {
        let t = ClassType { name: "ToetsmomentResultaat".into(), namespace: "Fontys.Fcdm.Onderwijs".into(), kind: "Class".into(), ..Default::default() };
        let parameters = vec![
            parameter(2, "string", "ToetsmomentCode"),
            parameter(1, "string", "toetsmomentBk"),
            parameter(3, "DateTimeOffset?", "toetsdatum"),
        ];
        assert_eq!(
            written(&t, "", parameters),
            "namespace Fontys.Fcdm.Onderwijs;

public class ToetsmomentResultaat
{
    public ToetsmomentResultaat(
        string toetsmomentBk,
        string toetsmomentCode,
        DateTimeOffset? toetsdatum)
    {
        ToetsmomentBk = toetsmomentBk;
        ToetsmomentCode = toetsmomentCode;
        Toetsdatum = toetsdatum;
    }

    public string ToetsmomentBk { get; init; }

    public string ToetsmomentCode { get; init; }

    public DateTimeOffset? Toetsdatum { get; init; }
}
"
        );
    }

    #[test]
    fn a_record_has_its_base_and_interfaces_and_an_interface_only_properties() {
        let t = ClassType { name: "Order".into(), kind: "Record".into(), interfaces: vec!["IPriced".into()], summary: "An order.".into(), ..Default::default() };
        assert!(written(&t, "Document", vec![]).starts_with("/// <summary>An order.</summary>\npublic record Order : Document, IPriced\n{\n}\n"));
        let i = ClassType { name: "IPriced".into(), kind: "Interface".into(), ..Default::default() };
        assert_eq!(written(&i, "", vec![parameter(1, "decimal", "price")]), "public interface IPriced\n{\n    decimal Price { get; }\n}\n");
    }
}

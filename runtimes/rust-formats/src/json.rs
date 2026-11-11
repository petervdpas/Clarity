//! JSON, syntax only: Clarity's own tree (`tree.rs`, numbers as written,
//! names in their order) as nodes of the document model (doc.rs) and back,
//! written compact.

use crate::doc::{Document, Node};
use crate::format::Format;
use crate::tree::{self, Tree};
use crate::{FormatInfo, Nodes, OptionInfo, OptionKind, Problem, ProblemKind, Tables};

pub struct Json;

impl Format for Json {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            name: "json",
            title: "JSON",
            text: true,
            nests: true,
            nodes: Nodes::Values,
            attributes: false,
            tables: Tables::One,
            path_separator: Some('/'),
            schema_kinds: &[],
            options: vec![OptionInfo { name: "at", kind: OptionKind::Text, label: "formatOptionAt", reads: true, writes: true, required_writing: None }],
        }
    }

    fn parse(&self, input: &[u8], _options: &[(String, Tree)], _names: &[String]) -> Result<Document, Problem> {
        let value = std::str::from_utf8(input).ok().and_then(tree::parse).ok_or_else(|| Problem::new(ProblemKind::NotFitting, "the text is no JSON"))?;
        Ok(Document { top: Node::from_tree("", &value), columns: Vec::new(), kept: None })
    }

    fn write(&self, doc: &Document, _options: &[(String, Tree)]) -> Result<Vec<u8>, Problem> {
        Ok(tree::write(&doc.top.to_tree()).into_bytes())
    }
}

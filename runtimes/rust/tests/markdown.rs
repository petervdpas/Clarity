//! Markdown read into blocks and spans as every renderer reads it
//! (conformance/markdown.json).

use clarity_runtime::markdown::{self, Block, BlockKind, Span};
use serde_json::{Map, Value, json};

fn written(block: &Block) -> Value {
    let spans: Vec<Value> = block.spans.iter().map(written_span).collect();
    match block.kind {
        BlockKind::Heading => json!({ "kind": "heading", "level": block.level, "spans": spans }),
        BlockKind::Paragraph => json!({ "kind": "paragraph", "spans": spans }),
        BlockKind::Item => match block.number {
            Some(number) => json!({ "kind": "item", "depth": block.depth, "ordered": true, "number": number, "spans": spans }),
            None => json!({ "kind": "item", "depth": block.depth, "ordered": false, "spans": spans }),
        },
        BlockKind::Quote => json!({ "kind": "quote", "depth": block.depth, "spans": spans }),
        BlockKind::Rule => json!({ "kind": "rule" }),
        BlockKind::Code => json!({ "kind": "code", "text": block.text }),
    }
}

fn written_span(span: &Span) -> Value {
    let mut out = Map::new();
    out.insert("text".into(), json!(span.text));
    if span.bold {
        out.insert("bold".into(), json!(true));
    }
    if span.italic {
        out.insert("italic".into(), json!(true));
    }
    if span.code {
        out.insert("code".into(), json!(true));
    }
    if let Some(link) = &span.link {
        out.insert("link".into(), json!(link));
    }
    Value::Object(out)
}

#[test]
fn markdown_reads_as_every_renderer_reads_it() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/markdown.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let read: Vec<Value> = markdown::read(case["markdown"].as_str().unwrap()).iter().map(written).collect();
        assert_eq!(case["blocks"], Value::Array(read.clone()), "{}: {}", case["name"], Value::Array(read));
    }
}

#[test]
fn spans_read_bold_italic_code_and_links() {
    let spans = markdown::spans("a **b** `c`");
    assert!(spans.iter().any(|s| s.bold && s.text == "b"), "{spans:?}");
    assert!(spans.iter().any(|s| s.code && s.text == "c"), "{spans:?}");
}

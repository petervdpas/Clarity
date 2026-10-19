//! Engines (DESIGN 5.11): processing modules compiled to WebAssembly, which
//! code calls like functions; the project keeps their files in its
//! `engines` folder.

use clarity_spec::code::TypeRefKind;
use clarity_spec::engines::{self, EngineFile};
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, load_sources};

const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

const SHOP: &str = "module shop

enum Size
    Small
    Large

entity Line
    price: money
    count: integer

engine Pricing from \"test.wasm\"
    function add(a: integer, b: integer) -> integer
    function total(lines: list<Line>) -> money
    function size_of(n: integer) -> Size
    function discounted(line: Line) -> Line

function answer(a: integer) -> integer
    return Pricing.add(a, 40)

function sized(lines: list<Line>) -> Size
    return Pricing.size_of(count(lines))
";

fn project(source: &str, files: Vec<EngineFile>) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    engines::attach(&mut project, files);
    project
}

fn test_engine() -> Vec<EngineFile> {
    vec![EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]
}

fn errors(source: &str, files: Vec<EngineFile>) -> Vec<String> {
    project(source, files).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn an_engine_declares_its_file_and_functions() {
    let (file, diags) = parse(SHOP, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let pricing = &file.engines[0];
    assert_eq!((pricing.name.name.as_str(), pricing.file.as_str()), ("Pricing", "test.wasm"));
    let names: Vec<&str> = pricing.functions.iter().map(|f| f.name.name.as_str()).collect();
    assert_eq!(names, ["add", "total", "size_of", "discounted"]);
    assert_eq!(pricing.functions[1].returns.as_ref().map(|r| &r.kind), Some(&TypeRefKind::Money));
    assert!(errors(SHOP, test_engine()).is_empty(), "{:#?}", errors(SHOP, test_engine()));
}

#[test]
fn a_call_to_an_engine_is_checked_like_a_call_to_a_function() {
    let with = |body: &str| errors(&SHOP.replace("    return Pricing.add(a, 40)\n", &format!("    {body}\n")), test_engine());
    assert_has(&with("return Pricing.teleport(a)"), "engine Pricing has no function teleport");
    assert_has(&with("return Pricing.add(a)"), "Pricing.add takes 2 arguments, not 1");
    assert_has(&with("return Pricing.add(a, \"x\")"), "argument 'b' of Pricing.add is an integer, not text");
    assert_has(&with("return Nope.add(a, 1)"), "unknown engine 'Nope'");
    assert_has(&with("let s: text = Pricing.add(a, 1)\n    return a"), "cannot set text to integer");
}

#[test]
fn an_engine_function_takes_and_gives_values() {
    let bad = |from: &str, to: &str| errors(&SHOP.replace(from, to), test_engine());
    assert_has(&bad("function add(a: integer, b: integer) -> integer", "function add(a: integer, b: integer)"), "engine function add gives a value: write its type after '->'");
    assert_has(&bad("function add(a: integer, b: integer) -> integer", "function add(a: file, b: integer) -> integer"), "an engine takes values, records and lists of records, not file");
    assert_has(&bad("engine Pricing from \"test.wasm\"", "engine Pricing from \"test.txt\""), "an engine is a .wasm file");
}

#[test]
fn the_engine_file_is_checked_against_its_declaration() {
    assert_has(&errors(SHOP, Vec::new()), "engine Pricing: 'test.wasm' is not in the engines folder");
    let lacking = SHOP.replace("    function discounted(line: Line) -> Line\n", "    function discounted(line: Line) -> Line\n    function teleport(n: integer) -> integer\n");
    assert_has(&errors(&lacking, test_engine()), "engine Pricing: 'test.wasm' has no function teleport");
    // An engine that imports anything is refused: it would reach out of its sandbox.
    let importing = vec![EngineFile { name: "test.wasm".into(), bytes: vec![0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 2, 0x0b, 1, 3, b'e', b'n', b'v', 3, b'n', b'o', b'w', 0, 0] }];
    assert_has(&errors(SHOP, importing), "engine Pricing: 'test.wasm' imports env.now; an engine imports nothing");
    let junk = vec![EngineFile { name: "test.wasm".into(), bytes: vec![0, 1, 2, 3] }];
    assert_has(&errors(SHOP, junk), "engine Pricing: 'test.wasm' is no WebAssembly module");
}

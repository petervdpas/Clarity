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
    uses clock, random, log, language
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
    // An engine that imports anything but Clarity's services is refused: it would reach out of its sandbox.
    let importing = vec![EngineFile { name: "test.wasm".into(), bytes: vec![0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 2, 0x0b, 1, 3, b'e', b'n', b'v', 3, b'n', b'o', b'w', 0, 0] }];
    assert_has(&errors(SHOP, importing), "engine Pricing: 'test.wasm' imports env.now; an engine imports only the services it uses");
    let junk = vec![EngineFile { name: "test.wasm".into(), bytes: vec![0, 1, 2, 3] }];
    assert_has(&errors(SHOP, junk), "engine Pricing: 'test.wasm' is no WebAssembly module");
}

/// `limit 2 seconds, 64 MB`: what one call of the engine may take, each
/// part optional, the defaults for what is not written.
#[test]
fn an_engine_may_limit_its_calls() {
    let source = |limit: &str| format!("module shop\n\nengine Pricing from \"test.wasm\"\n    uses clock, random, log, language\n{limit}    function add(a: integer, b: integer) -> integer\n");
    let limits = |limit: &str| {
        let (file, diags) = parse(&source(limit), "shop.clarity");
        assert!(diags.is_empty(), "{diags:#?}");
        let e = &file.engines[0];
        (e.seconds(), e.megabytes())
    };
    assert_eq!(limits(""), (engines::DEFAULT_SECONDS, engines::DEFAULT_MEGABYTES));
    assert_eq!(limits("    limit 2 seconds, 64 MB\n"), (2, 64));
    assert_eq!(limits("    limit 1 second\n"), (1, engines::DEFAULT_MEGABYTES));
    assert_eq!(limits("    limit 32 MB\n"), (engines::DEFAULT_SECONDS, 32));

    let wrong = |limit: &str| errors(&source(limit), test_engine());
    assert_has(&wrong("    limit 0 seconds\n"), "between 1 and 3600 seconds");
    assert_has(&wrong("    limit 5000 MB\n"), "between 1 and 4096 MB");
    assert_has(&wrong("    limit 2 minutes\n"), "seconds");
    assert_has(&wrong("    limit 2 seconds\n    limit 3 seconds\n"), "one limit");
}

/// `uses clock, random, log`: the services of Clarity an engine may import;
/// the file may import those and nothing else.
#[test]
fn an_engine_declares_the_services_it_uses() {
    let source = |uses: &str| format!("module shop\n\nengine Pricing from \"test.wasm\"\n{uses}    function add(a: integer, b: integer) -> integer\n");
    let (file, diags) = parse(&source("    uses clock, random, log, language\n"), "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let uses: Vec<&str> = file.engines[0].uses.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(uses, ["clock", "random", "log", "language"]);
    assert!(errors(&source("    uses clock, random, log, language\n"), test_engine()).is_empty());

    // The test engine imports all four, so leaving one out is an error.
    assert_has(&errors(&source("    uses clock, log, language\n"), test_engine()), "imports clarity.random; an engine imports only the services it uses");
    assert_has(&errors(&source("    uses clock, teleport\n"), test_engine()), "'teleport' is no service; an engine uses clock, random, log or language");
    assert_has(&errors(&source("    uses clock, clock, random, log, language\n"), test_engine()), "uses clock twice");
}

/// An engine lists the texts it gives, so they are translated like the
/// project's own: `says "You win", "A draw"`.
#[test]
fn an_engine_says_the_texts_it_gives() {
    let source = "module shop\n\nengine Pricing from \"test.wasm\"\n    uses clock, random, log, language\n    says \"You win\", \"A draw\"\n    function add(a: integer, b: integer) -> integer\n";
    let (file, diags) = parse(source, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.engines[0].says, ["You win", "A draw"]);
    let texts = clarity_spec::translations::texts(&project(source, test_engine()));
    let said = texts.iter().find(|u| u.text == "You win").expect("collected");
    assert_eq!(said.places, ["engine Pricing"]);
    assert_has(&parse("module shop\n\nengine Pricing from \"test.wasm\"\n    says You\n", "shop.clarity").1.iter().map(|d| d.message.clone()).collect::<Vec<_>>(), "says gives texts");
}

/// `state per app` keeps one instance for every call; `on open` runs the
/// engine's `clarity_open` on every new instance, which its file must export.
#[test]
fn an_engine_may_keep_state_and_open() {
    use clarity_spec::code::EngineState;
    let source = |lines: &str| format!("module shop\n\nengine Pricing from \"test.wasm\"\n    uses clock, random, log, language\n{lines}    function add(a: integer, b: integer) -> integer\n");
    let declared = |lines: &str| {
        let (file, diags) = parse(&source(lines), "shop.clarity");
        assert!(diags.is_empty(), "{diags:#?}");
        let e = &file.engines[0];
        (e.state, e.opens)
    };
    assert_eq!(declared(""), (EngineState::None, false));
    assert_eq!(declared("    state per app\n    on open\n"), (EngineState::App, true));
    assert!(errors(&source("    state per app\n    on open\n"), test_engine()).is_empty());

    let wrong = |lines: &str| errors(&source(lines), test_engine());
    assert_has(&wrong("    state per day\n"), "expected 'per app' or 'per window'");
    assert_has(&wrong("    state per app\n    state per app\n"), "one state line");
    assert_has(&wrong("    on open\n    on open\n"), "one on open line");
    assert_has(&wrong("    on close\n"), "an engine has on open only");
    // A file without clarity_open cannot open.
    let plain = vec![EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE_WITHOUT_OPEN.to_vec() }];
    assert_has(&errors(&source("    on open\n").replace("    uses clock, random, log, language\n", ""), plain), "lacks clarity_open, which its on open needs");
}

/// The smallest engine: memory, clarity_alloc and clarity_free, no clarity_open
/// (the module of the refused case in conformance/engines.json).
const TEST_ENGINE_WITHOUT_OPEN: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0b, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x02, 0x7f, 0x7f, 0x00, 0x03, 0x03,
    0x02, 0x00, 0x01, 0x05, 0x03, 0x01, 0x00, 0x01, 0x07, 0x29, 0x03, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x0d, 0x63, 0x6c,
    0x61, 0x72, 0x69, 0x74, 0x79, 0x5f, 0x61, 0x6c, 0x6c, 0x6f, 0x63, 0x00, 0x00, 0x0c, 0x63, 0x6c, 0x61, 0x72, 0x69, 0x74, 0x79, 0x5f, 0x66,
    0x72, 0x65, 0x65, 0x00, 0x01, 0x0a, 0x09, 0x02, 0x04, 0x00, 0x41, 0x00, 0x0b, 0x02, 0x00, 0x0b,
];

/// `state per window`: each open window has an instance of its own, so only
/// a window's handlers call the engine; a function, an action or the app has
/// no window to call it in.
#[test]
fn an_engine_with_state_per_window_is_called_by_a_window() {
    use clarity_spec::code::EngineState;
    let source = |callers: &str| {
        format!(
            "module shop\n\nengine Board from \"test.wasm\"\n    uses clock, random, log, language\n    state per window\n    on open\n    function count() -> integer\n\n{callers}"
        )
    };
    let (file, diags) = parse(&source(""), "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.engines[0].state, EngineState::Window);

    let window = "page Game: window\n    data moves: integer\n\ncode Game\n    handler moved\n        moves = Board.count()\n";
    assert!(errors(&source(window), test_engine()).is_empty(), "{:#?}", errors(&source(window), test_engine()));
    let function = "function moves() -> integer\n    return Board.count()\n";
    assert_has(&errors(&source(function), test_engine()), "engine Board keeps state per window, so only a window's handlers call it, not a function");
    let app = "app Shop\n    data moves: integer\n\ncode Shop\n    handler moved\n        moves = Board.count()\n";
    assert_has(&errors(&source(app), test_engine()), "engine Board keeps state per window, so only a window's handlers call it, not the app");
}

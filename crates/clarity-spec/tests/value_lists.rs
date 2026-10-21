//! Lists of plain values in code: `list<integer>`, `list<text>`,
//! `list<decimal>`, `list<bool>`. Written `[0, 0, 0]`, read and set by
//! position from 1 (`squares[3]`), counted with `count`, walked with `for`.

use clarity_spec::{Severity, load_sources};

const MODULE: &str = "module game

function winner(squares: list<integer>) -> integer
    let lines = [1, 2, 3]
    let total = 0
    for s in squares
        total = total + s
    if count(squares) == 9 and squares[1] == squares[2]
        return squares[1]
    return total

function marked(squares: list<integer>, at: integer) -> list<integer>
    let changed = squares
    changed[at] = 1
    return changed

function names() -> list<text>
    return [\"a\", \"b\"]
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("game.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn code_works_with_lists_of_values() {
    assert!(errors(MODULE).is_empty(), "{:#?}", errors(MODULE));
}

#[test]
fn value_lists_are_checked() {
    assert_has(&errors(&MODULE.replace("let lines = [1, 2, 3]", "let lines = [1, \"two\"]")), "a list holds values of one type");
    assert_has(&errors(&MODULE.replace("squares[1] == squares[2]", "squares[\"a\"] == squares[2]")), "a position in a list is an integer");
    assert_has(&errors(&MODULE.replace("changed[at] = 1", "changed[at] = \"x\"")), "cannot set integer to text");
    assert_has(&errors(&MODULE.replace("return [\"a\", \"b\"]", "return [1, 2]")), "list<text>");
    assert_has(&errors(&MODULE.replace("let lines = [1, 2, 3]", "let lines = []")), "an empty list needs its type, as in let squares: list<integer> = []");
}


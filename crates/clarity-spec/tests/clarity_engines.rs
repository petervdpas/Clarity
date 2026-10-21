//! An engine written in Clarity: no file, its functions written in Clarity
//! with the values it keeps between calls (`keep`), per window or per app.
//! Every stack runs it as generated code of its own.

use clarity_spec::{Severity, load_sources};

const GAME: &str = "module game

engine Board
    state per window
    uses random
    says @yourMove, @thatSquareIsTaken
    keep squares: list<integer> = [0, 0, 0, 0, 0, 0, 0, 0, 0]
    keep moves: integer = 0

    function play(square: integer) -> text
        if squares[square] != 0
            refuse @thatSquareIsTaken
        squares[square] = 1
        moves = moves + 1
        answer()
        return @yourMove

    function answer() -> integer
        let pick = random(9)
        if squares[pick] == 0
            squares[pick] = 2
        return pick

    function mark(square: integer) -> integer
        return squares[square]

page Game: window
    data status: text
    button @playCaption name playButton at 1,1 span 2 on click: playClicked

code Game
    handler playClicked
        status = Board.play(5)
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("game.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn an_engine_is_written_in_clarity() {
    let project = load_sources([("game.clarity", GAME)]);
    let engine = &project.files[0].engines[0];
    assert!(engine.file.is_empty());
    let body = engine.body.as_ref().expect("written in Clarity");
    assert_eq!(body.keeps.iter().map(|k| k.name.name.as_str()).collect::<Vec<_>>(), ["squares", "moves"]);
    // Code calls its functions as any engine's.
    assert_eq!(engine.functions.iter().map(|f| f.name.name.as_str()).collect::<Vec<_>>(), ["play", "answer", "mark"]);
    assert!(errors(GAME).is_empty(), "{:#?}", errors(GAME));
}

#[test]
fn an_engine_in_clarity_is_checked() {
    assert_has(&errors(&GAME.replace("moves = moves + 1", "moves = \"x\"")), "cannot set integer to text");
    assert_has(&errors(&GAME.replace("    uses random\n", "")), "random needs 'uses random'");
    assert_has(&errors(&GAME.replace("keep moves: integer = 0", "keep moves: integer = \"none\"")), "expected integer");
    // An engine keeps to itself: no data, no windows, no database.
    assert_has(&errors(&GAME.replace("        moves = moves + 1\n", "        status = \"x\"\n")), "unknown name 'status'");
    assert_has(&errors(&GAME.replace("        answer()\n", "        message @yourMove\n")), "an engine shows nothing");
}

#[test]
fn the_keys_an_engine_in_clarity_names_are_used() {
    let project = load_sources([("game.clarity", GAME)]);
    let keys = clarity_spec::translations::keys(&project);
    let play = keys.iter().find(|k| k.key == "yourMove").expect("yourMove is used");
    assert!(play.places.iter().any(|p| p.contains("engine Board, function play")), "{:?}", play.places);
}

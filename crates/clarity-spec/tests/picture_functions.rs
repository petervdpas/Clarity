//! The picture functions of code (docs/IMAGE-PLAN.md, conformance/picture-functions.json):
//! typed by the checker, a project's own function of the same name taking its place.

use clarity_spec::{Severity, load_converted};

const DESK: &str = "module desk

entity Book
    title: text, label @bookTitle
    cover: image, label @bookCover

page Desk: window
    data book: Book
    data wide: integer
    data high: integer

code Desk
    handler coverMade
        let small = resize(book.cover, 200, 300)
        let part = crop(small, 10, 10, 100, 100)
        let shot = rotate(part, 90)
        shot = rotate(shot, 12.5)
        shot = flip(shot, true)
        shot = blend(book.cover, shot, 20, 30, 80)
        shot = adjust(shot, 30, -20, 10, 0)
        wide = pictureWidth(shot)
        high = pictureHeight(book.cover)
        book.cover = shot
";

fn errors(text: &str) -> Vec<String> {
    load_converted([("desk.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn picture_functions_are_typed() {
    assert!(errors(DESK).is_empty(), "{:#?}", errors(DESK));
    let e = errors(&DESK.replace("resize(book.cover, 200, 300)", "resize(book.title, 200, 300)"));
    assert!(e.iter().any(|m| m.contains("'resize' takes a picture, a whole number, a whole number")), "{e:?}");
    let e = errors(&DESK.replace("flip(shot, true)", "flip(shot, 1)"));
    assert!(e.iter().any(|m| m.contains("'flip' takes a picture, a yes or no")), "{e:?}");
    let e = errors(&DESK.replace("wide = pictureWidth(shot)", "wide = resize(shot, 1, 1)"));
    assert!(!e.is_empty(), "a picture is no integer: {e:?}");
}

#[test]
fn a_project_function_of_the_same_name_takes_its_place() {
    let own = DESK.replace("        book.cover = shot\n", "        book.cover = shot\n\nfunction flip(a: integer, b: integer) -> integer\n    return a + b\n").replace("shot = flip(shot, true)", "wide = flip(1, 2)");
    assert!(errors(&own).is_empty(), "{:#?}", errors(&own));
}

#[test]
fn an_engine_has_no_picture_functions() {
    let engine = "module eng\n\nengine Shrink\n    function wide(w: integer) -> integer\n        return pictureWidth(resize(read, w, w))\n";
    let e = errors(&engine.replace("read", "zip([\"a\"], [\"b\"])"));
    assert!(e.iter().any(|m| m.contains("'resize' is a picture function, which runs in the program, not in an engine")), "{e:?}");
}

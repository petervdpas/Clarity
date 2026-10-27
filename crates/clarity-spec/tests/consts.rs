//! Constants (DESIGN 5.12): `const lives = 3` in a module, read by name from
//! its code and from engines, never set; `const pillRed: picture =
//! "pill-red.svg"` names a picture of the resources folder, checked once.

use clarity_spec::resources::{self, ResourceFile};
use clarity_spec::{Project, Severity, load_sources};

const GAME: &str = "module play

const lives = 3
const speed: decimal = 1.5
const greeting = \"Hello\"
const levels = [1, 2, 3]
const pill: picture = \"pill.svg\"

engine Board
    const width = 160

    function room() -> integer
        return width - lives

    function art() -> text
        return pill

page Play: window
    data left: integer
    canvas name field at 1,1 span 6 rows 6 units 160 x 100 on draw: drawField
    button \"Go\" name go at 1,7 span 2 on click: goClicked

code Play
    handler drawField
        field.image(0, 0, 17, 5, pill)

    handler goClicked
        left = lives + count(levels)
";

fn with_pictures(source: &str, names: &[&str]) -> Project {
    let mut project = load_sources([("play.clarity", source)]);
    resources::attach(&mut project, names.iter().map(|n| ResourceFile { name: n.to_string(), bytes: b"<svg/>".to_vec() }).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn constants_are_read_by_name_everywhere() {
    assert_eq!(errors(&with_pictures(GAME, &["pill.svg"])), Vec::<String>::new());
    let file = &load_sources([("play.clarity", GAME)]).files[0];
    assert_eq!(file.consts.iter().map(|c| c.name.name.as_str()).collect::<Vec<_>>(), ["lives", "speed", "greeting", "levels", "pill"]);
    assert!(file.consts[4].picture);
    assert_eq!(file.engines[0].body.as_ref().unwrap().consts[0].name.name, "width");
}

#[test]
fn a_constant_is_never_set() {
    let set = GAME.replace("        left = lives + count(levels)\n", "        lives = 4\n");
    assert_has(&errors(&with_pictures(&set, &["pill.svg"])), "lives is a constant and cannot be set");
}

#[test]
fn a_constant_is_written_out() {
    let computed = GAME.replace("const lives = 3\n", "const lives = 3\nconst more = lives + 1\n");
    assert_has(&errors(&with_pictures(&computed, &["pill.svg"])), "const more is written out: a number, text, yes or no, a key, an enum value or a list of them");
    let wrong = GAME.replace("const speed: decimal = 1.5", "const speed: decimal = \"fast\"");
    assert_has(&errors(&with_pictures(&wrong, &["pill.svg"])), "const speed is decimal, not text");
    let twice = GAME.replace("const greeting = \"Hello\"\n", "const greeting = \"Hello\"\nconst greeting = \"Hi\"\n");
    assert_has(&errors(&with_pictures(&twice, &["pill.svg"])), "there is already a const greeting");
}

#[test]
fn a_picture_constant_names_a_picture_of_the_resources() {
    assert_eq!(errors(&with_pictures(GAME, &["paddle.svg"])), ["const pill is 'pill.svg', which is not in the resources folder; it has paddle.svg"]);
    let not = GAME.replace("\"pill.svg\"", "\"pill.txt\"");
    assert_eq!(errors(&with_pictures(&not, &["pill.txt"])), ["const pill is 'pill.txt', which is not a picture every stack can show; use a .png, .jpg, .gif, .bmp, .webp or .svg file"]);
}

use clarity_spec::edit::{Edit, apply};
use clarity_spec::load_sources;
use clarity_spec::source::{append_decl, decl_text, problems_in_decl, replace_decl};

const FILE: &str = "module m

// The card.
page Card: window
    title \"Card\"
    // A comment inside.
    button \"Save\" name save at 1,1 span 2

function double(n: integer) -> integer
    return n * 2

page Other: window
    title \"Other\"
";

fn offset(text: &str, needle: &str) -> u32 {
    text.find(needle).unwrap() as u32
}

#[test]
fn reads_one_declaration_with_its_comments() {
    assert_eq!(
        decl_text(FILE, offset(FILE, "page Card")).unwrap(),
        "page Card: window\n    title \"Card\"\n    // A comment inside.\n    button \"Save\" name save at 1,1 span 2\n"
    );
    assert_eq!(
        decl_text(FILE, offset(FILE, "function")).unwrap(),
        "function double(n: integer) -> integer\n    return n * 2\n"
    );
    assert!(decl_text(FILE, 3).is_err());
}

#[test]
fn replaces_one_declaration_and_keeps_the_rest() {
    let text = replace_decl(FILE, offset(FILE, "page Card"), "page Card: window\n    title \"Kaart\"\n").unwrap();
    assert_eq!(
        text,
        FILE.replace(
            "    title \"Card\"\n    // A comment inside.\n    button \"Save\" name save at 1,1 span 2\n",
            "    title \"Kaart\"\n"
        )
    );
}

#[test]
fn appends_a_declaration() {
    let text = append_decl(FILE, "action ping()\n    return\n").unwrap();
    assert!(text.ends_with("    title \"Other\"\n\naction ping()\n    return\n"), "{text}");
}

#[test]
fn refuses_text_that_is_not_one_declaration() {
    let at = offset(FILE, "function");
    assert!(replace_decl(FILE, at, "").unwrap_err().to_string().contains("one declaration"));
    assert!(
        replace_decl(FILE, at, "function a() -> integer\n    return 1\nfunction b() -> integer\n    return 2\n")
            .unwrap_err()
            .to_string()
            .contains("one declaration")
    );
    assert!(replace_decl(FILE, at, "    return 1\n").unwrap_err().to_string().contains("one declaration"));
}

#[test]
fn maps_problems_onto_the_declaration_lines() {
    let at = offset(FILE, "function");
    let text = replace_decl(FILE, at, "function double(n: integer) -> integer\n    return m * 2\n").unwrap();
    let project = load_sources([("m.clarity", text.as_str())]);
    let problems = problems_in_decl(&text, at, "m.clarity", &project.diagnostics);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].message, "unknown name 'm'");
    assert_eq!((problems[0].span.line, problems[0].span.col), (2, 12));
}

#[test]
fn are_edits() {
    let at = offset(FILE, "function");
    let replaced = apply(
        FILE,
        &Edit::ReplaceDecl {
            item: at,
            text: "function double(n: integer) -> integer\n    return n + n\n".into(),
        },
    )
    .unwrap();
    assert!(replaced.text.contains("return n + n"));
    assert_eq!(replaced.focus, Some(at));
    let added = apply(FILE, &Edit::AppendDecl { text: "function one() -> integer\n    return 1\n".into() }).unwrap();
    assert!(added.text.ends_with("function one() -> integer\n    return 1\n"));
    assert_eq!(added.focus, Some(offset(&added.text, "function one")));
}

#[test]
fn parts_and_libraries_are_declarations_too() {
    let file = "module m\n\npart Arena\n    native \"parts/Arena\"\n    hears restart\n\nlibrary Words\n    nuget \"Humanizer.Core\" 2.14.1\n    function spell(n: integer) -> text\n";
    assert_eq!(decl_text(file, offset(file, "part Arena")).unwrap(), "part Arena\n    native \"parts/Arena\"\n    hears restart\n");
    assert_eq!(decl_text(file, offset(file, "library Words")).unwrap(), "library Words\n    nuget \"Humanizer.Core\" 2.14.1\n    function spell(n: integer) -> text\n");
    let replaced = replace_decl(file, offset(file, "part Arena"), "part Arena\n    native \"parts/Arena\"\n").unwrap();
    assert!(replaced.contains("part Arena\n    native \"parts/Arena\"\n\nlibrary Words"), "{replaced}");
}

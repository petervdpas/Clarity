//! One line of a theme, a custom part or a library set by key, as the IDE's
//! forms write them (SetDeclLine): replaced in place, added after its kind,
//! taken away; every other line and comment as written.

use clarity_spec::edit::{Edit, apply};
use clarity_spec::parser::parse;

const THEME: &str = "module look\n\n// The shop's own look.\ntheme Shop\n    mode system\n    // Brand colors.\n    color accent \"#2f6fdf\" dark \"#5b8cff\"\n    color panel \"#ffffff\"\n    font \"Inter\"\n    font heading \"Georgia\"\n    radius 6\n    part button background raised\n    part button hover border accent\n    part input focus border focus\n\npage Main: window\n    title \"Main\"\n";

fn set(source: &str, decl: &str, key: &str, line: Option<&str>) -> Result<String, String> {
    let item = source.find(decl).unwrap() as u32;
    apply(source, &Edit::SetDeclLine { item, key: key.to_string(), line: line.map(str::to_string) })
        .map(|o| o.text)
        .map_err(|e| e.0)
}

#[test]
fn a_theme_line_is_replaced_in_place() {
    let text = set(THEME, "theme Shop", "color accent", Some("color accent \"#ff0000\"")).unwrap();
    assert_eq!(text, THEME.replace("color accent \"#2f6fdf\" dark \"#5b8cff\"", "color accent \"#ff0000\""));
    // A font without a role is the text font; with one it is that role's.
    let text = set(THEME, "theme Shop", "font text", Some("font \"Roboto\"")).unwrap();
    assert_eq!(text, THEME.replace("font \"Inter\"", "font \"Roboto\""));
    let text = set(THEME, "theme Shop", "font heading", Some("font heading \"Lora\"")).unwrap();
    assert_eq!(text, THEME.replace("font heading \"Georgia\"", "font heading \"Lora\""));
    // `radius 6` is the radius of all.
    let text = set(THEME, "theme Shop", "radius all", Some("radius 8")).unwrap();
    assert_eq!(text, THEME.replace("radius 6", "radius 8"));
    // A part's look by default and in a state are two lines.
    let text = set(THEME, "theme Shop", "part button", Some("part button background accent")).unwrap();
    assert_eq!(text, THEME.replace("part button background raised", "part button background accent"));
    let text = set(THEME, "theme Shop", "part button hover", Some("part button hover border none")).unwrap();
    assert_eq!(text, THEME.replace("part button hover border accent", "part button hover border none"));
}

#[test]
fn a_new_theme_line_goes_after_the_last_of_its_kind() {
    let text = set(THEME, "theme Shop", "color error", Some("color error \"#c00000\"")).unwrap();
    assert_eq!(text, THEME.replace("    color panel \"#ffffff\"\n", "    color panel \"#ffffff\"\n    color error \"#c00000\"\n"));
    // A state of a part goes after that part's lines, not after the last part.
    let text = set(THEME, "theme Shop", "part button pressed", Some("part button pressed background accent_soft")).unwrap();
    assert_eq!(
        text,
        THEME.replace("    part button hover border accent\n", "    part button hover border accent\n    part button pressed background accent_soft\n")
    );
    // A kind of line the theme has none of goes at its end.
    let text = set(THEME, "theme Shop", "padding", Some("padding 4")).unwrap();
    assert_eq!(text, THEME.replace("    part input focus border focus\n", "    part input focus border focus\n    padding 4\n"));
    assert!(parse(&text, "look.clarity").1.is_empty());
}

#[test]
fn a_theme_line_is_taken_away_and_its_comments_stay() {
    let text = set(THEME, "theme Shop", "color accent", None).unwrap();
    assert_eq!(text, THEME.replace("    color accent \"#2f6fdf\" dark \"#5b8cff\"\n", ""));
    assert!(text.contains("// Brand colors."));
    // Taking away a line that is not there changes nothing.
    assert_eq!(set(THEME, "theme Shop", "color error", None).unwrap(), THEME);
}

#[test]
fn a_line_that_does_not_fit_is_refused() {
    // A line of another key would be a second line of that key.
    let err = set(THEME, "theme Shop", "color accent", Some("color panel \"#000000\"")).unwrap_err();
    assert!(err.contains("panel"), "{err}");
    // A line the theme cannot read is refused by the parser.
    assert!(set(THEME, "theme Shop", "shadow", Some("shadow deep")).is_err());
    // Only a theme, a custom part or a library has lines set by key.
    assert!(set(THEME, "page Main", "title", Some("title \"X\"")).is_err());
}

const PART: &str = "module game\n\npart Arena\n    native \"parts/Arena\"\n    binds integer\n    speed: integer, min 1, max 10, default 3\n    title: text\n    tells scored(points: integer)\n    hears restart(level: integer)\n    size 8 x 6\n\nlibrary Words\n    nuget \"Humanizer.Core\" 2.14.1\n    function spell(n: integer) -> text\n";

#[test]
fn a_custom_part_line_is_set_by_key() {
    // The library line, whatever its kind.
    let text = set(PART, "part Arena", "library", Some("crate \"parts/Arena\"")).unwrap();
    assert_eq!(text, PART.replace("native \"parts/Arena\"", "crate \"parts/Arena\""));
    // A property by its name, renamed in place.
    let text = set(PART, "part Arena", "property speed", Some("pace: integer, default 2")).unwrap();
    assert_eq!(text, PART.replace("speed: integer, min 1, max 10, default 3", "pace: integer, default 2"));
    // A new property goes after the last property.
    let text = set(PART, "part Arena", "property label", Some("label: text")).unwrap();
    assert_eq!(text, PART.replace("    title: text\n", "    title: text\n    label: text\n"));
    // A new tell after the tells; a hear taken away.
    let text = set(PART, "part Arena", "tells missed", Some("tells missed()")).unwrap();
    assert_eq!(text, PART.replace("    tells scored(points: integer)\n", "    tells scored(points: integer)\n    tells missed()\n"));
    let text = set(PART, "part Arena", "hears restart", None).unwrap();
    assert_eq!(text, PART.replace("    hears restart(level: integer)\n", ""));
    // A rename onto a name it has is refused.
    assert!(set(PART, "part Arena", "property speed", Some("title: text")).is_err());
    let (file, diagnostics) = parse(&set(PART, "part Arena", "binds", Some("binds text")).unwrap(), "game.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(file.parts[0].name.name, "Arena");
}

#[test]
fn a_library_line_is_set_by_key() {
    let text = set(PART, "library Words", "source", Some("dll \"libs/Words.dll\"")).unwrap();
    assert_eq!(text, PART.replace("nuget \"Humanizer.Core\" 2.14.1", "dll \"libs/Words.dll\""));
    let text = set(PART, "library Words", "function count", Some("function count(t: text) -> integer")).unwrap();
    assert!(text.ends_with("    function spell(n: integer) -> text\n    function count(t: text) -> integer\n"), "{text}");
    let text = set(PART, "library Words", "function spell", None).unwrap();
    assert!(text.ends_with("library Words\n    nuget \"Humanizer.Core\" 2.14.1\n"), "{text}");
}

#[test]
fn a_block_without_lines_takes_the_usual_indentation() {
    let source = "module m\n\nlibrary Words\n";
    let text = set(source, "library Words", "source", Some("crate \"humanize\" \"0.1\"")).unwrap();
    assert_eq!(text, "module m\n\nlibrary Words\n    crate \"humanize\" \"0.1\"\n");
}

/// The IDE's theme form offers what a theme can set, from the spec.
#[test]
fn the_preview_carries_the_theme_catalog() {
    let catalog = clarity_spec::themes::catalog();
    assert_eq!(catalog.roles.len(), clarity_spec::themes::ROLES.len());
    assert!(catalog.part_kinds.contains(&"button"));
    assert!(catalog.selectable.contains(&"tab"));
    assert_eq!(catalog.fonts, ["text", "heading", "mono"]);
}

/// A toast's lines, as its form in the IDE and the agents' tools set them.
#[test]
fn a_toast_s_lines_are_set_one_by_one() {
    let toast = "module shop\n\ntoast saved\n    text @savedToast\n    lifespan 3000\n";
    let text = set(toast, "toast saved", "lifespan", Some("lifespan 8000")).unwrap();
    assert_eq!(text, toast.replace("lifespan 3000", "lifespan 8000"));
    let text = set(&text, "toast saved", "corner", Some("corner bottomLeft")).unwrap();
    assert!(text.ends_with("    lifespan 8000\n    corner bottomLeft\n"), "{text}");
    let text = set(&text, "toast saved", "level", Some("level success")).unwrap();
    let text = set(&text, "toast saved", "icon", Some("icon \"check\"")).unwrap();
    let (file, diagnostics) = parse(&text, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let saved = &file.toasts[0];
    assert_eq!((saved.lifespan, saved.icon.as_deref()), (Some(8000), Some("check")));
    let text = set(&text, "toast saved", "corner", None).unwrap();
    assert!(!text.contains("corner"), "{text}");
    assert!(set(&text, "toast saved", "lifespan", Some("colour red")).is_err(), "a line no toast has is refused");
}

/// A toast is declared at the end of its module and taken away whole, as the IDE's + New toast and its Delete do.
#[test]
fn a_toast_is_added_and_taken_away() {
    let source = "module shop\n\npage Main: window\n    title \"Main\"\n";
    let added = apply(source, &Edit::AppendDecl { text: "toast saved\n    text @savedToast\n    lifespan 3000\n".into() }).map_err(|e| e.0).unwrap().text;
    let (file, diagnostics) = parse(&added, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(file.toasts[0].name.name, "saved");
    let item = added.find("toast saved").unwrap() as u32;
    let gone = apply(&added, &Edit::Remove { item }).map_err(|e| e.0).unwrap().text;
    assert_eq!(gone.trim_end(), source.trim_end());
}

/// A command's code is a declaration of its own, read and written back whole, as its page in the IDE does.
#[test]
fn a_command_is_read_and_written_back_whole() {
    let source = "module shop\n\ncommand order list\n    help @orderListHelp\n    print \"none\"\n\npage Main: window\n    title \"Main\"\n";
    let item = source.find("command order list").unwrap() as u32;
    let text = clarity_spec::source::decl_text(source, item).map_err(|e| e.0).unwrap();
    assert_eq!(text, "command order list\n    help @orderListHelp\n    print \"none\"\n");
    let changed = apply(source, &Edit::ReplaceDecl { item, text: "command order list\n    help @orderListHelp\n    print \"all\"".into() }).map_err(|e| e.0).unwrap().text;
    assert!(changed.contains("print \"all\"") && changed.contains("page Main"), "{changed}");
}

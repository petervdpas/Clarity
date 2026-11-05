//! The colors of code in a memo (`code csharp`): each language's comments,
//! texts, numbers, keywords and type names, found the same on every stack
//! since Clarity.UI draws them all.

use clarity_ui::parts::code_colors::{self, Ink};

fn inks(language: &str, text: &str) -> Vec<(String, Ink)> {
    let chars: Vec<char> = text.chars().collect();
    let language = code_colors::language(language).expect("a language Clarity knows");
    code_colors::spans(language, &chars).into_iter().map(|(from, to, ink)| (chars[from..to].iter().collect(), ink)).collect()
}

fn has(found: &[(String, Ink)], text: &str, ink: Ink) {
    assert!(found.iter().any(|(t, i)| t == text && *i == ink), "{text} as {ink:?} in {found:?}");
}

#[test]
fn csharp_its_comments_texts_numbers_keywords_and_types() {
    let found = inks("csharp", "// A recipe\npublic class Recipe { int serves = 4; string Name = \"Soup \\\"hot\\\"\"; /* kept */ }");
    has(&found, "// A recipe", Ink::Comment);
    has(&found, "public", Ink::Keyword);
    has(&found, "class", Ink::Keyword);
    has(&found, "Recipe", Ink::Type);
    has(&found, "int", Ink::Type);
    has(&found, "4", Ink::Number);
    has(&found, "\"Soup \\\"hot\\\"\"", Ink::Text);
    has(&found, "/* kept */", Ink::Comment);
    assert!(!found.iter().any(|(t, _)| t == "serves"), "a plain name keeps the text's color");
    // A declared name, capital or not, is no type: the type before it is.
    let declared = inks("csharp", "public List<Recipe> Recipes { get; set; }\npublic int Serves = 4;\nRecipe soup = new Recipe();");
    has(&declared, "Recipe", Ink::Type);
    assert!(!declared.iter().any(|(t, _)| t == "Recipes" || t == "Serves" || t == "soup"), "{declared:?}");
}

#[test]
fn every_language_clarity_knows_colors_its_own_way() {
    has(&inks("rust", "fn main() { let x: u32 = 1; } // done"), "fn", Ink::Keyword);
    has(&inks("rust", "let x: u32 = 1;"), "u32", Ink::Type);
    has(&inks("go", "func main() { var s string = `raw` }"), "`raw`", Ink::Text);
    has(&inks("python", "def soup():\n    return 'hot'  # warm"), "# warm", Ink::Comment);
    has(&inks("python", "def soup():"), "def", Ink::Keyword);
    has(&inks("sql", "select name from Recipe where serves > 2 -- many"), "select", Ink::Keyword);
    has(&inks("sql", "SELECT name FROM Recipe"), "FROM", Ink::Keyword);
    has(&inks("sql", "select 1 -- many"), "-- many", Ink::Comment);
    has(&inks("json", "{\"name\": \"Soup\", \"serves\": 4, \"vegan\": true}"), "true", Ink::Keyword);
    has(&inks("json", "{\"serves\": 4}"), "\"serves\"", Ink::Type);
    has(&inks("xml", "<recipe serves=\"4\"><!-- note --></recipe>"), "recipe", Ink::Keyword);
    has(&inks("xml", "<recipe serves=\"4\">"), "serves", Ink::Type);
    has(&inks("xml", "<a><!-- note --></a>"), "<!-- note -->", Ink::Comment);
    has(&inks("yaml", "name: Soup # the dish"), "name", Ink::Type);
    has(&inks("shell", "if [ -f x ]; then echo \"yes\"; fi # check"), "then", Ink::Keyword);
    has(&inks("clarity", "entity Recipe\n    name: text(80) // the dish"), "entity", Ink::Keyword);
    has(&inks("markdown", "# Soup\nStir `slowly`."), "# Soup", Ink::Keyword);
    has(&inks("markdown", "Stir `slowly`."), "`slowly`", Ink::Text);
    for name in code_colors::LANGUAGES {
        assert!(code_colors::language(name).is_some(), "{name}");
    }
    assert!(code_colors::language("klingon").is_none());
}

#[test]
fn the_languages_are_the_ones_the_checker_takes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/code-languages.json");
    let file: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let listed: Vec<&str> = file["languages"].as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect();
    assert_eq!(listed, code_colors::LANGUAGES);
}

#[test]
fn a_diff_marks_what_was_added_and_what_was_taken_out() {
    let found = inks("diff", "--- Medewerker.cs\n+++ Medewerker.cs\n@@ -1,3 +1,3 @@\n namespace Hr\n-    string functieBK,\n+    string functieBk,\n");
    has(&found, "--- Medewerker.cs", Ink::Comment);
    has(&found, "+++ Medewerker.cs", Ink::Comment);
    has(&found, "@@ -1,3 +1,3 @@", Ink::Keyword);
    has(&found, "-    string functieBK,", Ink::Removed);
    has(&found, "+    string functieBk,", Ink::Added);
    assert!(!found.iter().any(|(t, _)| t.contains("namespace")), "a line both have keeps the text's color");
    assert_eq!(Ink::Added.role(), "code_added");
    assert_eq!(Ink::Removed.role(), "code_removed");
}

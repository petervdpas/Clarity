//! External libraries (DESIGN 5.21): a `library` declared in Clarity with
//! the functions code calls, written in the project's own language with
//! that language's package (NuGet, a crate, a Go module, a CMake package),
//! called as `Words.spell(42)`.

use clarity_spec::code::LibrarySource;
use clarity_spec::keyed::load_keyed;
use clarity_spec::native::{self, PartFile};
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Order
    total: money
    words: text

library Words
    nuget \"Humanizer.Core\" 2.14.1
    function spell(n: integer) -> text
    function rounded(amount: money, places: integer) -> money
    function today() -> date

action describe(order: Order)
    effects db.write
    order.words = Words.spell(42)
    order.total = Words.rounded(order.total, 2)
    save order
";

fn project(text: &str) -> Project {
    load_keyed([("shop.clarity", text)], &[], "en")
}

fn errors(text: &str) -> Vec<String> {
    project(text).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn has(errors: &[String], part: &str) -> bool {
    errors.iter().any(|e| e.contains(part))
}

#[test]
fn a_library_is_declared_with_its_package_and_functions() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let library = &file.libraries[0];
    assert_eq!(library.name.name, "Words");
    assert_eq!(library.source, Some(LibrarySource::Nuget { package: "Humanizer.Core".into(), version: "2.14.1".into() }));
    let names: Vec<&str> = library.functions.iter().map(|f| f.name.name.as_str()).collect();
    assert_eq!(names, ["spell", "rounded", "today"]);
}

#[test]
fn each_language_names_its_package() {
    let text = "module m\n\nlibrary A\n    crate \"humanize\" \"0.1\"\n\nlibrary B\n    go \"github.com/dustin/go-humanize\" v1.0.1\n\nlibrary C\n    cmake \"SFML\"\n\nlibrary D\n    dll \"libs/Plot.dll\"\n";
    let (file, diagnostics) = parse(text, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let sources: Vec<Option<LibrarySource>> = file.libraries.iter().map(|l| l.source.clone()).collect();
    assert_eq!(
        sources,
        [
            Some(LibrarySource::Crate { name: "humanize".into(), version: "0.1".into() }),
            Some(LibrarySource::Go { module: "github.com/dustin/go-humanize".into(), version: "v1.0.1".into() }),
            Some(LibrarySource::Cmake { package: "SFML".into() }),
            Some(LibrarySource::Dll { file: "libs/Plot.dll".into() }),
        ]
    );
}

#[test]
fn code_calls_its_functions_as_declared() {
    assert_eq!(errors(SHOP), Vec::<String>::new());
    let e = errors(&SHOP.replace("Words.spell(42)", "Words.spell(\"forty\")"));
    assert!(has(&e, "spell"), "{e:?}");
    let e = errors(&SHOP.replace("Words.spell(42)", "Words.shout(42)"));
    assert!(has(&e, "library Words has no function shout"), "{e:?}");
    let e = errors(&SHOP.replace("order.words = Words.spell(42)", "order.words = Words.today()"));
    assert!(has(&e, "cannot set text to date") || has(&e, "date"), "{e:?}");
}

#[test]
fn a_declaration_is_checked() {
    let e = errors(&SHOP.replace("    nuget \"Humanizer.Core\" 2.14.1\n", ""));
    assert!(has(&e, "library Words needs its package"), "{e:?}");
    let e = errors(&SHOP.replace("function today() -> date", "function today() -> Order"));
    assert!(has(&e, "a library's values are text, a number, a date, a duration or bool"), "{e:?}");
    let e = errors(&SHOP.replace("function today() -> date", "function spell(n: integer) -> text"));
    assert!(has(&e, "library Words has two functions named 'spell'"), "{e:?}");
}

#[test]
fn a_library_needs_its_adapter() {
    let checked = |files: Vec<PartFile>| {
        let mut p = project(SHOP);
        native::attach(&mut p, files);
        p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect::<Vec<_>>()
    };
    let file = |path: &str| PartFile { part: "Words".into(), path: path.into(), bytes: b"x".to_vec() };
    assert_eq!(checked(vec![file("Words.cs")]), Vec::<String>::new());
    assert!(has(&checked(vec![]), "library Words: 'libraries/Words/Words.cs' is not there"), "{:?}", checked(vec![]));
}

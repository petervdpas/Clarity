//! An app's guidance for AI agents (docs/ledger/app-mcp.md): `agent/guidance.md`,
//! Markdown in fixed sections, which the app's MCP service gives to agents.
//! The wizard and the agents' tools read and write it a section at a time,
//! and the checker keeps the names it writes in backticks true.

use clarity_spec::guidance::{self, SECTIONS};
use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

enum Kind
    Person
    Business

entity Customer
    name: text(80), required
    kind: Kind

cli Shop
    help \"The shop\"

command customer add(name: text)
    help \"Adds a customer\"
    agent
    let c = new Customer
    c.name = name
    save c
";

fn project() -> Project {
    load_keyed([("shop.clarity", SHOP)], &[], "en")
}

fn warnings(project: &Project, text: Option<&str>) -> Vec<String> {
    guidance::check(project, text, "agent/guidance.md").into_iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message).collect()
}

#[test]
fn a_new_guidance_has_its_sections_in_order() {
    let text = guidance::template("Shop");
    assert!(text.starts_with("# Shop for agents\n"), "{text}");
    let headings: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("## ")).collect();
    assert_eq!(headings, SECTIONS.iter().map(|s| s.heading).collect::<Vec<_>>());
    assert!(guidance::sections(&text).iter().all(|(_, body)| body.is_empty()));
    assert_eq!(SECTIONS.iter().map(|s| s.key).collect::<Vec<_>>(), ["purpose", "words", "flow", "rules"]);
    assert!(SECTIONS.iter().all(|s| !s.asks.is_empty()));
}

#[test]
fn a_section_is_read_and_written_by_its_heading_and_the_rest_stays() {
    let text = guidance::template("Shop");
    let text = guidance::with_section(&text, "flow", "A customer is added, then made a business.\n\nThen invoiced.").unwrap();
    let text = guidance::with_section(&text, "purpose", "Keeps the shop's customers.").unwrap();
    let read = guidance::sections(&text);
    assert_eq!(read[0], ("purpose", "Keeps the shop's customers.".to_string()));
    assert_eq!(read[2], ("flow", "A customer is added, then made a business.\n\nThen invoiced.".to_string()));
    // Written again: replaced, not added.
    let again = guidance::with_section(&text, "purpose", "Keeps customers.").unwrap();
    assert_eq!(again.matches("## What the app is for").count(), 1);
    assert_eq!(guidance::sections(&again)[0].1, "Keeps customers.");
    assert_eq!(guidance::sections(&again)[2], read[2]);
    // A heading of the developer's own stays where it is, with what is under it.
    let own = format!("{again}\n## Examples\n\nAsk `customer add` for each new name.\n");
    let written = guidance::with_section(&own, "rules", "Never remove a customer.").unwrap();
    assert!(written.contains("## Examples\n\nAsk `customer add` for each new name."), "{written}");
    assert_eq!(guidance::sections(&written)[3].1, "Never remove a customer.");
    assert!(guidance::with_section(&text, "nothing", "x").unwrap_err().contains("purpose, words, flow"));
}

#[test]
fn a_missing_section_goes_in_its_place() {
    let text = "# Shop for agents\n\n## How work flows\n\nAdd, then invoice.\n";
    let written = guidance::with_section(text, "purpose", "Keeps customers.").unwrap();
    let at = |h: &str| written.find(h).unwrap();
    assert!(at("## What the app is for") < at("## How work flows"), "{written}");
    let written = guidance::with_section(text, "rules", "None.").unwrap();
    assert!(at_in(&written, "## How work flows") < at_in(&written, "## Rules the app does not check"), "{written}");
}

fn at_in(text: &str, heading: &str) -> usize {
    text.find(heading).unwrap()
}

#[test]
fn the_names_in_backticks_are_the_app_s() {
    let p = project();
    let good = guidance::with_section(&guidance::template("Shop"), "words", "A `Customer` has a `Customer.name` and a `Kind`, `Person` or `Business`; `customer add` adds one.").unwrap();
    assert!(warnings(&p, Some(&good)).is_empty(), "{:#?}", warnings(&p, Some(&good)));
    let stale = guidance::with_section(&good, "flow", "Then `customer remove`, and `Client` keeps `Customer.email`.").unwrap();
    let found = warnings(&p, Some(&stale));
    for name in ["customer remove", "Client", "Customer.email"] {
        assert!(found.iter().any(|w| w.contains(&format!("`{name}`"))), "{name}: {found:#?}");
    }
    assert_eq!(found.len(), 3, "{found:#?}");
}

#[test]
fn an_app_offering_agent_commands_has_guidance() {
    let p = project();
    let found = warnings(&p, None);
    assert!(found.iter().any(|w| w.contains("agent/guidance.md")), "{found:#?}");
    let none = load_keyed([("shop.clarity", SHOP.replace("    agent\n", "").as_str())], &[], "en");
    assert!(warnings(&none, None).is_empty(), "no agent commands, no guidance needed");
}

/// What the IDE's Agents part shows: every section, its heading, what to
/// write in it, and what it says; each one empty without a file.
#[test]
fn the_ide_shows_every_section_with_what_it_asks() {
    let shown = clarity_spec::guidance::shown(None);
    assert_eq!(shown.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(), ["purpose", "words", "flow", "rules"]);
    assert!(shown.iter().all(|s| s.body.is_empty() && !s.asks.is_empty() && !s.heading.is_empty()));
    let text = clarity_spec::guidance::with_section(&clarity_spec::guidance::template("Shop"), "flow", "First `customer add`.").unwrap();
    let shown = clarity_spec::guidance::shown(Some(&text));
    assert_eq!(shown[2].heading, "How work flows");
    assert_eq!(shown[2].body, "First `customer add`.");
}

/// Saving a section of an app without guidance starts the file from the template.
#[test]
fn a_section_saved_without_a_file_starts_one() {
    let text = clarity_spec::guidance::saved(None, "Shop", "purpose", "Sells things.").unwrap();
    assert!(text.starts_with("# Shop for agents\n"), "{text}");
    assert_eq!(clarity_spec::guidance::sections(&text)[0].1, "Sells things.");
    assert!(text.contains("## Rules the app does not check"), "{text}");
    let again = clarity_spec::guidance::saved(Some(&text), "Other", "rules", "Never remove a customer.").unwrap();
    assert!(again.starts_with("# Shop for agents\n"), "{again}");
    assert_eq!(clarity_spec::guidance::sections(&again)[3].1, "Never remove a customer.");
}

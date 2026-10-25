//! Parameters with a default, named arguments, and `when` on a query or
//! header pair (DESIGN 5.24): an action or function may give a parameter a
//! default, `limit: integer = 0`; a call may leave such parameters out or
//! name them, `list_templates("published", limit: 10)`; and an http pair
//! is left out of the call when its condition does not hold.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::{Project, Severity, load_sources};

const API: &str = "module api

function greeting(name: text, polite: bool = true, times: integer = 1) -> text
    if polite
        return \"Good day, {name} x{times}\"
    return \"Hi {name}\"

action fetch(status: text, limit: integer = 0, sort: text = \"\")
    effects net
    let reply = http get \"https://api.example.com/items\" query \"status\": status query \"limit\": \"{limit}\" when limit != 0 query \"sort\": sort when sort != \"\"
    if not reply.ok
        refuse \"not fetched: {reply.status}\"

page Status: window
    data shown: text
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Status
    handler goClicked
        shown = greeting(\"Ann\")
        shown = greeting(\"Ann\", false)
        shown = greeting(\"Ann\", times: 3)
        shown = greeting(times: 2, name: \"Bo\")
        fetch(\"open\")
        fetch(\"open\", sort: \"name\")
";

fn load(source: &str) -> Project {
    load_sources([("api.clarity", source)])
}

fn errors(source: &str) -> Vec<String> {
    load(source).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(API.contains(old), "{old:?}");
    API.replacen(old, new, 1)
}

#[test]
fn defaults_named_arguments_and_when_check() {
    assert_eq!(errors(API), Vec::<String>::new());
}

#[test]
fn the_parse_keeps_defaults_names_and_conditions() {
    let project = load(API);
    let file = &project.files[0];
    let greeting = file.functions.iter().find(|f| f.name.name == "greeting").unwrap();
    assert!(greeting.params[0].default.is_none());
    assert!(matches!(greeting.params[1].default.as_ref().map(|d| &d.kind), Some(ExprKind::Bool { value: true })));
    let fetch = file.functions.iter().find(|f| f.name.name == "fetch").unwrap();
    let StmtKind::Let { value, .. } = &fetch.body[0].kind else { panic!("{:?}", fetch.body[0].kind) };
    let ExprKind::Http { query, .. } = &value.kind else { panic!() };
    assert!(query[0].when.is_none());
    assert!(query[1].when.is_some() && query[2].when.is_some());
    // greeting(times: 2, name: "Bo") holds no positional values and two named ones.
    let handler = &file.code[0].handlers[0];
    let StmtKind::Assign { value, .. } = &handler.body[3].kind else { panic!() };
    let ExprKind::Call { args, named, .. } = &value.kind else { panic!() };
    assert!(args.is_empty());
    assert_eq!(named.iter().map(|n| n.name.name.as_str()).collect::<Vec<_>>(), ["times", "name"]);
}

#[test]
fn a_call_gives_each_parameter_once_and_every_one_without_a_default() {
    let missing = with("shown = greeting(\"Ann\")\n", "shown = greeting()\n");
    assert_eq!(errors(&missing), ["greeting needs a value for 'name', which has no default"]);
    let twice = with("shown = greeting(\"Ann\", times: 3)", "shown = greeting(\"Ann\", name: \"Bo\")");
    assert_eq!(errors(&twice), ["greeting gets 'name' twice"]);
    let unknown = with("shown = greeting(\"Ann\", times: 3)", "shown = greeting(\"Ann\", loud: true)");
    assert_eq!(errors(&unknown), ["greeting has no parameter 'loud'"]);
    let many = with("shown = greeting(\"Ann\", false)", "shown = greeting(\"Ann\", false, 2, 3)");
    assert_eq!(errors(&many), ["greeting takes at most 3 arguments, not 4"]);
    let wrong = with("shown = greeting(\"Ann\", times: 3)", "shown = greeting(\"Ann\", times: \"three\")");
    assert_eq!(errors(&wrong), ["argument 'times' of greeting is an integer, not text"]);
    let after = with("shown = greeting(\"Ann\", times: 3)", "shown = greeting(times: 3, \"Ann\")");
    assert_eq!(errors(&after), ["a value without a name cannot follow a named one: write name: value"]);
}

#[test]
fn a_default_is_a_fixed_value_of_its_type_and_comes_last() {
    let typed = with("times: integer = 1", "times: integer = \"one\"");
    assert_eq!(errors(&typed), ["the default of 'times' is text, not an integer"]);
    let fixed = with("times: integer = 1", "times: integer = count(name)");
    assert_eq!(errors(&fixed), ["the default of 'times' is a fixed value, such as 0, \"\", true or an enum's value"]);
    let order = with("function greeting(name: text, polite: bool = true, times: integer = 1)", "function greeting(polite: bool = true, name: text, times: integer = 1)");
    assert_eq!(errors(&order).first().map(String::as_str), Some("parameter 'name' needs a default, as 'polite' before it has one"));
}

#[test]
fn a_condition_on_a_pair_is_a_bool() {
    let when = with("when limit != 0", "when limit");
    assert_eq!(errors(&when), ["the condition after 'when' is a bool, not an integer"]);
}

#[test]
fn a_handler_takes_its_values_in_order() {
    let named = with("        fetch(\"open\")\n", "        goClicked(x: 1)\n");
    assert_eq!(errors(&named), ["handler 'goClicked' takes its values in order, without names"]);
}

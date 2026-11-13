//! Commands an AI agent may call (docs/ledger/app-mcp.md): a command with
//! an `agent` line is offered to agents as a tool, so it says what it does
//! (`help`), and Clarity works out whether it changes anything, which the
//! person is asked to allow.

use clarity_spec::agent::{Changes, changes};
use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required
    active: bool

settings
    exportFolder: folder, default \"exports\"

action addCustomer(name: text) -> Customer
    effects db.write
    let c = new Customer
    c.name = name
    save c
    return c

action announce(name: text)
    effects net
    let reply = http post \"https://example.org/news\" with name

function shout(name: text) -> text
    return name

cli Shop
    help \"The shop from the command line\"
";

fn project(commands: &str) -> Project {
    let text = format!("{SHOP}\n{commands}");
    let mut p = load_keyed([("shop.clarity", text.as_str())], &[], "en");
    clarity_spec::attach_stores(&mut p, vec!["exports".into()]);
    p
}

fn errors(commands: &str) -> Vec<String> {
    project(commands).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

/// What the one command of `commands` changes.
fn changed(commands: &str) -> Changes {
    let p = project(commands);
    let found: Vec<String> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(found.is_empty(), "{found:#?}");
    let decl = p.files.iter().flat_map(|f| &f.commands).next().expect("a command");
    changes(&p, decl)
}

#[test]
fn an_agent_line_offers_a_command_to_agents() {
    let p = project("command customer list\n    help \"Lists the customers\"\n    agent\n    print count(find c in Customer)\n");
    let decl = p.files.iter().flat_map(|f| &f.commands).next().unwrap();
    assert!(decl.agent);
    let p = project("command customer count\n    help \"Counts the customers\"\n    print count(find c in Customer)\n");
    assert!(!p.files.iter().flat_map(|f| &f.commands).next().unwrap().agent, "off by default");
}

#[test]
fn a_command_for_agents_says_what_it_does() {
    let found = errors("command customer list\n    agent\n    print count(find c in Customer)\n");
    assert!(found.iter().any(|e| e.contains("agent") && e.contains("help")), "{found:#?}");
    let found = errors("command customer list\n    help \"Lists\"\n    agent\n    agent\n    print 1\n");
    assert!(found.iter().any(|e| e.contains("'agent' is written once")), "{found:#?}");
    let found = errors("command customer list\n    help \"Lists\"\n    print 1\n    agent\n");
    assert!(found.iter().any(|e| e.contains("'agent' goes before the statements")), "{found:#?}");
}

#[test]
fn reading_changes_nothing() {
    let none = changed("command customer list\n    help \"x\"\n    let found = find c in Customer\n    print count(found)\n    print shout(\"x\")\n");
    assert_eq!(none, Changes::default());
    assert!(!none.any());
    let got = changed("command news\n    help \"x\"\n    let reply = http get \"https://example.org/news\"\n    print reply.status\n");
    assert!(!got.any(), "a get reads: {got:?}");
}

#[test]
fn saving_deleting_files_settings_and_calls_out_change_something() {
    assert!(changed("command a(name: text)\n    help \"x\"\n    let c = new Customer\n    c.name = name\n    save c\n").data);
    assert!(changed("command a(name: text)\n    help \"x\"\n    let found = find c in Customer where c.name == name\n    for c in found\n        delete c\n").data);
    assert!(changed("command a(name: text)\n    help \"x\"\n    let c = addCustomer(name)\n    print c.name\n").data, "through an action that writes");
    assert!(changed("command a(name: text)\n    help \"x\"\n    write name to exports \"a.txt\"\n").files);
    assert!(changed("command a(folder: text)\n    help \"x\"\n    settings.exportFolder = folder\n").settings);
    assert!(changed("command a(name: text)\n    help \"x\"\n    let reply = http post \"https://example.org/x\" with name\n").net);
    assert!(changed("command a(name: text)\n    help \"x\"\n    announce(name)\n").net, "through an action that calls out");
}

#[test]
fn an_app_for_agents_has_clarity_s_words_for_its_refusals() {
    let keys = |p: &Project| clarity_spec::translations::keys(p).into_iter().map(|k| k.key).collect::<Vec<_>>();
    let offered = project("command customer count\n    help \"Counts\"\n    agent\n    print count(find c in Customer)\n");
    for (key, _) in clarity_spec::translations::AGENT_KEYS.iter().chain(&clarity_spec::translations::AGENT_GRANT_KEYS) {
        assert!(keys(&offered).contains(&key.to_string()), "{key}");
    }
    let closed = project("command customer count\n    help \"Counts\"\n    print count(find c in Customer)\n");
    assert!(!keys(&closed).contains(&"agentRefused".to_string()), "only an app with commands for agents");
}

#[test]
fn the_words_of_refusals_are_the_shared_ones() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/app-mcp.json")).unwrap();
    let mut words: Vec<(String, String)> = cases["words"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
    let mut ours: Vec<(String, String)> = clarity_spec::translations::AGENT_KEYS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    words.sort();
    ours.sort();
    assert_eq!(ours, words);
}

#[test]
fn mcp_is_the_program_s_own_word() {
    let errors = errors("command mcp\n    help \"Mine\"\n    print \"x\"\n\ncommand mcp start\n    help \"Mine too\"\n    print \"x\"\n");
    assert_eq!(errors.iter().filter(|e| e.contains("`mcp` is the program's own word")).count(), 2, "{errors:#?}");
}

/// The person turns what agents may do while the app is closed on and off in
/// the app's own settings: Clarity's own settings `agentReadsWhileClosed` and
/// `agentChangesWhileClosed`, bools labeled with Clarity's words, kept under
/// the keys the app's MCP service reads (docs/ledger/app-mcp.md).
#[test]
fn the_app_s_settings_turn_agents_on_and_off_while_it_is_closed() {
    const OFFERED: &str = "command customer count\n    help \"Counts\"\n    agent\n    print count(find c in Customer)\n";
    let with = |commands: &str| {
        let text = format!("{}\n{commands}", SHOP.replace("    exportFolder: folder, default \"exports\"\n", "    exportFolder: folder, default \"exports\"\n    agentReadsWhileClosed\n    agentChangesWhileClosed\n"));
        let mut p = load_keyed([("shop.clarity", text.as_str())], &[], "en");
        clarity_spec::attach_stores(&mut p, vec!["exports".into()]);
        p
    };
    let p = with(OFFERED);
    let found: Vec<&String> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| &d.message).collect();
    assert!(found.is_empty(), "{found:#?}");
    let settings = p.entities().find(|e| e.settings).unwrap();
    for (name, label) in [("agentReadsWhileClosed", "@agentReadsWhileClosedSetting"), ("agentChangesWhileClosed", "@agentChangesWhileClosedSetting")] {
        let field = settings.fields.iter().find(|f| f.name.name == name).unwrap();
        assert_eq!(field.ty, clarity_spec::model::FieldType::Bool, "{name}");
        assert_eq!(field.label, label);
    }
    let keys: Vec<String> = clarity_spec::translations::keys(&p).into_iter().map(|k| k.key).collect();
    for (key, _) in &clarity_spec::translations::AGENT_SETTING_KEYS {
        assert!(keys.contains(&key.to_string()), "{key}");
    }
    // Without a command for agents they turn nothing on.
    let closed = with("command customer count\n    help \"Counts\"\n    print count(find c in Customer)\n");
    let found: Vec<String> = closed.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert_eq!(found, ["agentReadsWhileClosed is a setting of an app whose commands agents call; no command says agent", "agentChangesWhileClosed is a setting of an app whose commands agents call; no command says agent"]);
}

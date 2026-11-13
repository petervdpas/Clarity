//! The tool list of an app's MCP service (docs/ledger/app-mcp.md): one writer
//! for every stack makes it from the commands with an `agent` line, so each
//! runtime serves the same tools: a name as the command is typed, what it
//! does, its inputs as a JSON schema, whether it changes anything, and who may
//! run it. The instructions are the app's guidance, with its data summed up.

use clarity_gen::agent_tools::tool_list;
use clarity_project::starter;
use serde_json::{Value, json};

const SHOP: &str = "module shop

enum Kind
    Person
    Business

entity Customer
    name: text(80), required
    kind: Kind
    since: date

cli Shop
    help \"The shop\"

command customer add(name: text, kind: Kind)
    help \"Adds a customer\"
    agent
    allow when me != none
    option limit: money, default 1000, min 0, max 5000, help \"The credit limit\"
    option vip: bool, help \"A good customer\"
    let c = new Customer
    c.name = name
    c.kind = kind
    save c

command customer count
    help \"Counts the customers\"
    agent
    print count(find c in Customer)

command backup
    help \"Not for agents\"
    print \"done\"
";

fn list(guidance: Option<&str>) -> Value {
    // `allow when` needs a sign-in in the project file; `a_tool_says_what_it_changes_and_who_may_run_it` reads it.
    let source = SHOP.replace("    allow when me != none\n", "");
    let mut project = clarity_spec::keyed::load_keyed([("shop.clarity", source.as_str())], &[], "en");
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project.guidance = guidance.map(str::to_string);
    tool_list(&project, &starter("Shop", "shop"))
}

fn tool<'a>(list: &'a Value, name: &str) -> &'a Value {
    list["tools"].as_array().unwrap().iter().find(|t| t["name"] == json!(name)).unwrap_or_else(|| panic!("no {name} in {list:#}"))
}

#[test]
fn only_commands_for_agents_are_tools_named_as_typed() {
    let list = list(None);
    let names: Vec<&str> = list["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["customer-add", "customer-count"]);
    assert_eq!(list["name"], json!("shop"));
    assert_eq!((tool(&list, "customer-add")["line"].clone(), tool(&list, "customer-add")["arguments"].clone()), (json!(["customer", "add"]), json!(["name", "kind"])), "a call is read as this command line");
}

#[test]
fn a_tool_says_what_it_does_in_the_default_language() {
    let list = list(None);
    assert_eq!(tool(&list, "customer-add")["description"], json!("Adds a customer"));
    assert_eq!(tool(&list, "customer-count")["description"], json!("Counts the customers"));
}

#[test]
fn the_inputs_are_a_json_schema_of_the_arguments_and_options() {
    let list = list(None);
    let schema = &tool(&list, "customer-add")["inputSchema"];
    assert_eq!(schema["type"], json!("object"));
    assert_eq!(schema["properties"]["name"]["type"], json!("string"));
    assert_eq!(schema["properties"]["kind"]["enum"], json!(["Person", "Business"]));
    let limit = &schema["properties"]["limit"];
    assert_eq!((limit["type"].clone(), limit["minimum"].clone(), limit["maximum"].clone(), limit["default"].clone()), (json!("number"), json!(0), json!(5000), json!(1000)));
    assert_eq!(limit["description"], json!("The credit limit"));
    assert_eq!(schema["properties"]["vip"]["type"], json!("boolean"));
    assert_eq!(schema["required"], json!(["name", "kind"]), "arguments are required, options are not");
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(tool(&list, "customer-count")["inputSchema"]["properties"], json!({}));
}

#[test]
fn a_tool_says_what_it_changes_and_who_may_run_it() {
    // The project file's sign-in is left out here: the list reads only that a command limits who may run it.
    let limited = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
    assert_eq!(tool(&tool_list(&limited, &starter("Shop", "shop")), "customer-add")["allow"], json!(true), "who may run it is limited; the runtime refuses others with the reason");
    let list = list(None);
    let add = tool(&list, "customer-add");
    assert_eq!(add["changes"], json!({ "data": true, "files": false, "settings": false, "net": false }));
    assert_eq!(add["allow"], json!(false));
    let count = tool(&list, "customer-count");
    assert_eq!(count["changes"], json!({ "data": false, "files": false, "settings": false, "net": false }));
    assert_eq!(count["allow"], json!(false));
}

#[test]
fn the_instructions_are_the_guidance_with_the_app_s_data() {
    let with = list(Some("# Shop for agents\n\n## What the app is for\n\nKeeps customers.\n"));
    let text = with["instructions"].as_str().unwrap();
    assert!(text.starts_with("# Shop for agents\n\n## What the app is for\n\nKeeps customers.\n"), "{text}");
    assert!(text.contains("## The app's data\n"), "{text}");
    assert!(text.contains("- Customer: name (text), kind (Kind: Person, Business), since (date)"), "{text}");
    let without = list(None);
    assert!(without["instructions"].as_str().unwrap().starts_with("## The app's data\n"), "{without:#}");
}

#[test]
fn the_list_has_the_shape_the_mcp_core_serves() {
    // conformance/app-mcp.json is served by Clarity.AppMcp on every stack; its tool list is one this writer makes.
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap()).unwrap();
    let keys = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    let list = list(None);
    assert_eq!(keys(&list), keys(&file["tools"]));
    assert_eq!(keys(tool(&list, "customer-add")), keys(&file["tools"]["tools"][0]));
    assert_eq!(keys(&tool(&list, "customer-add")["changes"]), keys(&file["tools"]["tools"][0]["changes"]));
}

#[test]
fn a_rust_desktop_program_runs_the_commands_for_agents_itself() {
    // The maintenance example: Rust, desktop and command line, with commands for agents.
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/maintenance.clarity");
    let settings = clarity_project::load(&file).settings.expect("its project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let programs = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let desktop = programs.iter().find(|p| p.target == clarity_project::Target::Desktop).expect("a desktop program");
    let file = |path: &str| desktop.files.iter().find(|f| f.path == path).map(|f| f.text.clone()).unwrap_or_else(|| panic!("no {path}"));
    let commands = file("src/commands.rs");
    assert!(commands.contains("pub fn offered() -> AgentsOffered") && commands.contains("clarity_runtime::cli::run_line(&program(), line, ask, "), "{commands}");
    assert!(file("src/tools.json").contains("\"review-set\""));
    let main = file("src/main.rs");
    assert!(main.contains("mod commands;") && main.contains("fn agent_call(&self, line: &[String]"), "{main}");
    assert!(main.contains("if std::env::args().skip(1).eq([\"mcp\"]) {\n        return commands::mcp(&db);"), "the desktop program answers mcp itself: {main}");
    assert!(file("Cargo.toml").contains("\"mcp\""), "the runtime's MCP service");
    assert!(file("src/modules/maintenance.rs").contains("pub fn command_review_set("), "the commands are in the desktop program too");
}

/// The maintenance example on another stack: its desktop program's files.
fn maintenance_desktop_on(backend: clarity_project::Backend) -> Vec<clarity_gen::OutFile> {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/maintenance.clarity");
    let mut settings = clarity_project::load(&file).settings.expect("its project file reads");
    settings.stack.backend = backend;
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let programs = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    programs.into_iter().find(|p| p.target == clarity_project::Target::Desktop).expect("a desktop program").files
}

fn text_of<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

#[test]
fn a_go_desktop_program_runs_the_commands_for_agents_itself() {
    let files = maintenance_desktop_on(clarity_project::Backend::Go);
    let commands = text_of(&files, "commands.go");
    assert!(commands.contains("func agentsOf(db *clarity.Db) *shell.Agents") && commands.contains("func serveMcp(db *clarity.Db) int"), "{commands}");
    assert!(text_of(&files, "tools.json").contains("\"review-set\""));
    assert!(files.iter().any(|f| f.path == "agents_library.go") && files.iter().any(|f| f.path == "agents_host.go"));
    let main = text_of(&files, "main.go");
    assert!(main.contains("Agents: agentsOf(db)") && main.contains("code := serveMcp(db)"), "{main}");
    assert!(text_of(&files, "maintenance_module.go").contains("func commandReviewSet("), "the commands are in the desktop program too");
}

#[test]
fn the_list_says_what_the_project_allows_while_the_app_is_closed() {
    let source = SHOP.replace("    allow when me != none\n", "");
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", source.as_str())], &[], "en");
    let none = tool_list(&project, &starter("Shop", "shop"));
    assert_eq!(none["whileClosed"], serde_json::json!({ "read": false, "change": false }));
    let settings = clarity_project::Settings { agents_read_closed: true, ..starter("Shop", "shop") };
    assert_eq!(tool_list(&project, &settings)["whileClosed"], serde_json::json!({ "read": true, "change": false }));
}

/// The app's own settings for agents start as the project file says
/// (`agents while closed read change`), until the person sets them.
#[test]
fn the_agent_settings_start_as_the_project_allows() {
    let source = SHOP.replace("    allow when me != none\n", "") + "\nsettings\n    agentReadsWhileClosed\n    agentChangesWhileClosed\n";
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", source.as_str())], &[], "en");
    let default_of = |p: &clarity_spec::Project, name: &str| p.entities().find(|e| e.settings).unwrap().fields.iter().find(|f| f.name.name == name).unwrap().modifiers.default.clone();
    let settings = clarity_project::Settings { agents_read_closed: true, ..starter("Shop", "shop") };
    let started = clarity_gen::agent_tools::with_agent_defaults(&project, &settings);
    assert_eq!(default_of(&started, "agentReadsWhileClosed"), Some(clarity_spec::model::Literal::Bool(true)));
    assert_eq!(default_of(&started, "agentChangesWhileClosed"), Some(clarity_spec::model::Literal::Bool(false)));
}

#[test]
fn the_list_says_how_many_agents_the_app_serves_at_once() {
    let source = SHOP.replace("    allow when me != none\n", "");
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert_eq!(tool_list(&project, &starter("Shop", "shop"))["atMost"], json!(3));
    let settings = clarity_project::Settings { agents_at_most: Some(1), ..starter("Shop", "shop") };
    assert_eq!(tool_list(&project, &settings)["atMost"], json!(1));
}

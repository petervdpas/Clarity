//! Command-line applications (DESIGN 5.20): a `cli` block and `command`
//! declarations with arguments, `option` lines and `help`, written inline
//! as a function is. In a command `print` writes a line or a table, `ask`
//! waits and gives the answer, and `exit` ends the program with a code.

use clarity_spec::code::{AskForm, ExprKind, StmtKind, TypeRefKind};
use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

enum Kind
    Person
    Business

entity Customer
    name: text(80), required
    email: email
    kind: Kind
    active: bool
    credit_limit: money

action add_customer(name: text) -> Customer
    effects db.write
    let c = new Customer
    c.name = name
    save c
    return c

cli Shop
    help \"The shop from the command line\"
";

fn project(commands: &str) -> Project {
    let text = format!("{SHOP}\n{commands}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(commands: &str) -> Vec<String> {
    let p = project(commands);
    p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn warnings(commands: &str) -> Vec<String> {
    let p = project(commands);
    p.diagnostics.into_iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message).collect()
}

const GOOD: &str = "command customer add(name: text, email: text)
    help \"Adds a customer\"
    option vip: bool, help \"A business customer\"
    option limit: money, default 1000, min 0, max 5000, help \"The credit limit\"
    let c = add_customer(name)
    c.credit_limit = limit
    if vip
        c.kind = Business
    save c
    print \"Added {c.name}\"

command customer list
    help \"Lists the customers\"
    option active: bool, help \"Only the active ones\"
    let found = find c in Customer where c.active == active order by c.name
    print found columns name, email

command customer remove(name: text)
    help \"Removes a customer\"
    let found = find c in Customer where c.name == name
    if count(found) == 0
        refuse \"No customer {name}\"
    if ask \"Remove {name}?\" yes \"Remove\" no \"Keep\"
        delete found[1]

command backup
    help \"Copies the customers\"
    option copies: integer, default 1, help \"How many\"
    let n = ask \"How many copies?\" for integer default copies
    let k = ask \"Which kind?\" for Kind
    message warning \"Backing up {n}\"
    if n > 9
        exit 3
";

#[test]
fn reads_the_cli_block_and_commands() {
    let p = project(GOOD);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let file = &p.files[0];
    assert_eq!(file.clis.len(), 1);
    assert_eq!(file.clis[0].name.name, "Shop");
    let names: Vec<String> = file.commands.iter().map(|c| c.line()).collect();
    assert_eq!(names, ["customer add", "customer list", "customer remove", "backup"]);

    let add = &file.commands[0];
    assert_eq!(add.group.as_ref().map(|g| g.name.as_str()), Some("customer"));
    assert_eq!(add.name.name, "add");
    assert_eq!(add.params.len(), 2);
    assert!(add.help.is_some());
    let limit = &add.options[1];
    assert_eq!(limit.name.name, "limit");
    assert_eq!(limit.ty.kind, TypeRefKind::Money);
    assert!(limit.default.is_some() && limit.min.is_some() && limit.max.is_some());
    assert!(!limit.required);
    assert!(matches!(add.body.last().map(|s| &s.kind), Some(StmtKind::Print { columns, .. }) if columns.is_empty()));

    let list = &file.commands[1];
    let Some(StmtKind::Print { columns, .. }) = list.body.last().map(|s| &s.kind) else { panic!("a print") };
    let columns: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(columns, ["name", "email"]);

    // `if ask ... yes ... no ...`: an ask that gives its answer.
    let remove = &file.commands[2];
    let Some(StmtKind::If { branches, .. }) = remove.body.last().map(|s| &s.kind) else { panic!("an if") };
    let ExprKind::Ask { form, .. } = &branches[0].condition.kind else { panic!("an ask") };
    assert!(matches!(**form, AskForm::Confirm { .. }));

    let backup = &file.commands[3];
    assert!(matches!(backup.body.last().map(|s| &s.kind), Some(StmtKind::If { .. })));
}

#[test]
fn a_command_line_is_written_with_hyphens() {
    let p = project("command addAll\n    help \"x\"\n    option dryRun: bool, help \"y\"\n    print \"done\"\n");
    let command = &p.files[0].commands[0];
    assert_eq!(command.line(), "add-all");
    assert_eq!(command.options[0].flag(), "--dry-run");
}

#[test]
fn print_and_exit_belong_in_a_command() {
    let errors = errors(
        "page Customers: window
    button \"Go\" name goButton at 1,1 span 2 on click: goClicked

code Customers
    handler goClicked
        print \"hello\"
        exit 1

function twice(n: integer) -> integer
    exit 2
    return n * 2
",
    );
    assert_eq!(
        errors,
        [
            "'exit' belongs in a command, not in a function",
            "'print' belongs in a command, not in a handler",
            "'exit' belongs in a command, not in a handler",
        ]
    );
}

#[test]
fn a_command_has_no_window() {
    let errors = errors(
        "page Customers: window
    data customer: Customer

command show
    help \"x\"
    open Customers
    close
    reload customer
    print save.enabled
",
    );
    assert_eq!(
        errors,
        [
            "a command has no window: 'open' belongs in a handler",
            "a command has no window: 'close' belongs in a handler",
            "a command has no window: 'reload' belongs in a handler",
            "unknown name 'save'",
        ]
    );
}

#[test]
fn ask_in_a_command_gives_its_answer() {
    let errors = errors(
        "command a
    help \"x\"
    ask \"Sure?\" yes \"Go\" no \"Stop\" then b
    let n: text = ask \"How many?\" for integer
    let f = ask \"Which?\" for Customer
",
    );
    assert_eq!(
        errors,
        [
            "in a command, ask gives its answer, as in 'let answer = ask ...' or 'if ask ...'",
            "cannot set text to integer",
            "a command asks for a record from a list: 'ask ... for Customer from customers'",
        ]
    );
    let errors = self::errors(
        "page Customers: window
    button \"Go\" name goButton at 1,1 span 2 on click: goClicked

code Customers
    handler goClicked
        let n = ask \"How many?\" for integer
",
    );
    assert_eq!(errors, ["a handler asks with 'then' and the handler the answer runs, as in 'ask ... then answered'"]);
}

#[test]
fn print_shows_a_value_or_a_table_of_records() {
    let errors = errors(
        "command a
    help \"x\"
    let found = find c in Customer
    print found
    print found columns name, phone
    print 3 columns name
    exit \"no\"
    exit 300
    return 1
",
    );
    assert_eq!(
        errors,
        [
            "print a list of records as a table, with its columns: 'print found columns name'",
            "Customer has no field 'phone'",
            "'columns' prints a list of records as a table, not integer",
            "the exit code is an integer, not text",
            "the exit code is 0 to 255",
            "a command does not return a value; it ends with 'exit' and a code",
        ]
    );
}

#[test]
fn arguments_and_options_are_plain_values() {
    let errors = errors(
        "command a(c: Customer, flag: bool, names: list<Customer>)
    help \"x\"
    option when: date, help \"y\"
    option kind: Kind, default Person, help \"z\"
    option limit: money, default \"lots\", help \"w\"
    option size: text, min 1, help \"v\"
    option must: integer, required, default 2, help \"u\"
    option c: text, help \"t\"
    print \"x\"
",
    );
    assert_eq!(
        errors,
        [
            "an argument is text, a number, a date, a duration or an enum, not Customer; a command looks records up with find",
            "a yes or no is an option, not an argument: option flag: bool",
            "an argument is text, a number, a date, a duration or an enum, not list<Customer>; a command looks records up with find",
            "command a has two arguments or options named 'c'",
            "the default of limit is text, but limit is money",
            "'min' and 'max' bound a number; size is text",
            "must is required, so it has no default",
        ]
    );
}

#[test]
fn every_command_line_is_one_command() {
    let errors = errors(
        "command customer add
    help \"x\"
    print \"a\"

command customer add
    help \"x\"
    print \"b\"

command customer
    help \"x\"
    print \"c\"

cli Other
    help \"y\"
",
    );
    assert_eq!(
        errors,
        [
            "there are two commands 'customer add'",
            "'customer' is a group of commands, so it is no command of its own",
            "a project has one 'cli' block",
        ]
    );
}

#[test]
fn commands_and_options_need_help() {
    let warnings = warnings("command a\n    option b: bool\n    print \"x\"\n");
    assert!(warnings.contains(&"command a has no help; write what it does: help @key".to_string()), "{warnings:#?}");
    assert!(warnings.contains(&"option b of command a has no help; write what it does: help @key".to_string()), "{warnings:#?}");
}

#[test]
fn a_command_sees_the_settings() {
    let errors = errors(
        "settings
    pageSize: integer, default 20

command a
    help \"x\"
    settings.pageSize = 30
    print settings.pageSize
",
    );
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn the_texts_of_commands_are_keys() {
    let p = project(GOOD);
    let keys: Vec<String> = clarity_spec::translations::keys(&p).into_iter().map(|k| k.key).collect();
    for key in ["theShopFromTheCommand", "addsACustomer", "aBusinessCustomer", "addedName", "removeName", "remove", "keep", "howManyCopies", "backingUpN"] {
        assert!(keys.contains(&key.to_string()), "{key} in {keys:#?}");
    }
    // Clarity's own words of the command line, as a program with commands needs them.
    for (key, _) in clarity_spec::translations::CLI_KEYS {
        assert!(keys.contains(&key.to_string()), "{key}");
    }
    let windowed = load_keyed([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n")], &[], "en");
    let keys: Vec<String> = clarity_spec::translations::keys(&windowed).into_iter().map(|k| k.key).collect();
    assert!(!keys.contains(&"cliUsage".to_string()));
}

#[test]
fn clarity_s_words_of_the_command_line_are_the_shared_ones() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/command-line.json")).unwrap();
    let words: Vec<(String, String)> = cases["words"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
    let ours: Vec<(String, String)> = clarity_spec::translations::CLI_KEYS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    // Sorted: JSON objects keep no order.
    let (mut ours, mut words) = (ours, words);
    ours.sort();
    words.sort();
    assert_eq!(ours, words);
}

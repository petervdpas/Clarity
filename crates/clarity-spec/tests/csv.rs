//! CSV in code (docs/FORMATS-PLAN.md, step 2): `read list<Login> from csv
//! text separator ";"` and `csv of logins`. CSV holds one flat table, so the
//! checker asks for records (or a list of them) without records inside.

use clarity_spec::{Severity, load_converted};

const LOGINS: &str = "module logins

entity Login
    site: text(200)
    user: text(100)
    tags: list<text>

entity Account
    name: text(100)
    logins: list<Login>

function loginsIn(text: text) -> list<Login>
    return read list<Login> from csv text separator \";\" noHeader true encoding \"windows-1252\"

function written(logins: list<Login>) -> text
    return csv of logins quote \"'\"

page Logins: window
    data logins: list<Login>
    button \"Load\" name load at 1,1 span 2 on click: loadClicked

code Logins
    handler loadClicked
        logins = loginsIn(\"a;b\")
        message written(logins)
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("logins.clarity", source)]).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(LOGINS.contains(old), "{old:?}");
    LOGINS.replacen(old, new, 1)
}

#[test]
fn logins_read_and_write_csv_with_options() {
    assert_eq!(errors(LOGINS), Vec::<String>::new());
}

#[test]
fn an_option_is_of_its_kind() {
    assert_eq!(errors(&with("noHeader true", "noHeader \"yes\"")), vec!["the option 'noHeader' of csv is true or false, not text"]);
}

#[test]
fn csv_reads_and_writes_no_records_inside_records() {
    assert_eq!(errors(&with("read list<Login> from csv", "read list<Account> from csv")).first().map(String::as_str), Some("CSV holds no records inside records: field 'logins' of Account does"));
    let account = with("function written(logins: list<Login>) -> text\n    return csv of logins", "function written(logins: list<Login>) -> text\n    return \"\"\n\nfunction account(a: Account) -> text\n    return csv of a");
    assert_eq!(errors(&account), vec!["CSV holds no records inside records: field 'logins' of Account does"]);
}

#[test]
fn csv_writes_records_not_a_value() {
    assert_eq!(errors(&with("return csv of logins", "return csv of 3")), vec!["'csv of' writes a record or a list of records: CSV is one table, not integer"]);
}

//! `clarity users`: the people of a project that signs people in
//! (docs/APP-CAPABILITIES-PLAN.md step 4), changed from the command line, for
//! a computer without a browser at hand.

use std::process::{Command, Stdio};

const PROJECT: &str = "project Club\n    stack rust cli\n    signin Member on cli\n        password email passwordHash\n            minimum 8\n        token apiToken\n        recovery email recoveryCodes\n    datasource main sqlite \"data/club.db\"\n";
const PEOPLE: &str = "module people\n\nentity Member\n    email: email, unique, required, label @memberEmail\n    passwordHash: text, hashed, label @memberPasswordHash\n    apiToken: text, hashed, unique, label @memberApiToken\n    recoveryCodes: text, hashed, label @memberRecoveryCodes\n\ncli Club\n    help @clubHelp\n\ncommand members\n    help @membersHelp\n    print find m in Member columns email\n";
const ENGLISH: &str = "translation en\n    memberEmail \"Email\"\n    memberPasswordHash \"Password\"\n    memberApiToken \"Token\"\n    memberRecoveryCodes \"Recovery codes\"\n    clubHelp \"The club\"\n    membersHelp \"The members\"\n    signinRefused \"Signing in did not work. Check what you typed and try again.\"\n    signinTooManyTries \"Too many tries. Wait a few minutes and try again.\"\n    notAllowed \"You are not allowed to do this.\"\n    notYours \"This record is not yours.\"\n    secretTooShort \"This needs at least {minimum} characters.\"\n    cliPassword \"Password\"\n";
const STEP: &str = "migration \"People\"\n    add entity Member\n        email: email, unique, required\n        passwordHash: text, hashed\n        apiToken: text, hashed, unique\n        recoveryCodes: text, hashed\n";

fn clarity(args: &[&str], input: Option<&str>) -> (bool, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clarity"));
    command.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(text) = input {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn people_are_added_given_a_password_a_token_and_codes_and_removed() {
    let root = std::env::temp_dir().join(format!("clarity-users-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("club/migrations/main")).unwrap();
    std::fs::create_dir_all(root.join("club/translations")).unwrap();
    std::fs::write(root.join("club/translations/en.clarity"), ENGLISH).unwrap();
    std::fs::write(root.join("club.clarity"), PROJECT).unwrap();
    std::fs::write(root.join("club/people.clarity"), PEOPLE).unwrap();
    std::fs::write(root.join("club/migrations/main/0001.clarity"), STEP).unwrap();
    let file = root.join("club.clarity");
    let file = file.to_str().unwrap();
    let (ok, _, said) = clarity(&["data", "migrate", file], None);
    assert!(ok, "{said}");

    let (ok, _, said) = clarity(&["users", file, "add", "ann@example.com", "--stdin"], Some("correct horse\n"));
    assert!(ok, "{said}");
    let (_, listed, _) = clarity(&["users", file, "list"], None);
    assert!(listed.contains("ann@example.com"), "{listed}");
    let db = rusqlite_hash(&root, "password_hash");
    assert!(db.starts_with("$argon2id$"), "{db}");

    // The project's minimum holds here too.
    let (ok, _, said) = clarity(&["users", file, "password", "ann@example.com", "--stdin"], Some("short\n"));
    assert!(!ok && said.contains("at least 8 characters"), "{said}");
    let (ok, _, said) = clarity(&["users", file, "password", "ann@example.com", "--stdin"], Some("a new horse\n"));
    assert!(ok, "{said}");
    assert_ne!(rusqlite_hash(&root, "password_hash"), db);

    // A token and codes are shown once and kept as their digests.
    let (ok, token, said) = clarity(&["users", file, "token", "ann@example.com"], None);
    assert!(ok && token.trim().len() >= 40, "{token} {said}");
    assert!(rusqlite_hash(&root, "api_token").starts_with("sha256$"));
    let (ok, codes, _) = clarity(&["users", file, "codes", "ann@example.com"], None);
    assert!(ok && codes.lines().count() == 10, "{codes}");
    assert!(rusqlite_hash(&root, "recovery_codes").starts_with("sha256-lines$"));

    let (ok, _, said) = clarity(&["users", file, "password", "bob@example.com", "--stdin"], Some("whatever horse\n"));
    assert!(!ok && said.contains("no Member with email bob@example.com"), "{said}");
    let (ok, _, said) = clarity(&["users", file, "remove", "ann@example.com"], None);
    assert!(ok, "{said}");
    let (_, listed, _) = clarity(&["users", file, "list"], None);
    assert!(!listed.contains("ann@example.com"), "{listed}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_project_without_signin_says_so() {
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/shop.clarity");
    let (ok, _, said) = clarity(&["users", examples.to_str().unwrap(), "list"], None);
    assert!(!ok && said.contains("signs nobody in"), "{said}");
}

/// The one person's value of `column`, as kept in the database.
fn rusqlite_hash(root: &std::path::Path, column: &str) -> String {
    let db = rusqlite::Connection::open(root.join("club/data/club.db")).unwrap();
    db.query_row(&format!("SELECT {column} FROM member"), [], |r| r.get::<_, Option<String>>(0)).unwrap().unwrap_or_default()
}

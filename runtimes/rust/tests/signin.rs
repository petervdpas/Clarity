//! Who is using the program on the Rust runtime (conformance/signin.json):
//! signing in with each strategy, refusals in Clarity's own words, the first
//! account, failed tries, signing out, and served sessions by their cookie.

#![cfg(feature = "auth")]

use clarity_runtime::signin::{self, SigninSource, SigninStrategy};
use clarity_runtime::{Db, Error, Id, Record, SqlValue, sql};
use serde_json::{Map, Value, json};

fn file() -> Value {
    serde_json::from_str(include_str!("../../../conformance/signin.json")).unwrap()
}

/// A row's value as SQL: text quoted, a {hash, text} hashed as clarity-auth does.
fn literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".into(),
        Value::Object(o) => literal(&Value::from(hash_of(o))),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => other.to_string(),
    }
}

/// A {hash, text} value as it is stored.
fn hash_of(o: &Map<String, Value>) -> String {
    signin::hashed(o["hash"].as_str().unwrap(), o["text"].as_str().unwrap(), None).unwrap()
}

fn source_of(file: &Value) -> SigninSource {
    let (people, block) = (&file["people"], &file["signin"]);
    let texts = |v: &Value| v.as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect::<Vec<_>>();
    SigninSource {
        source: String::new(),
        table: people["table"].as_str().unwrap().into(),
        key: people["key"].as_str().unwrap().into(),
        page: block["page"].as_str().unwrap().into(),
        targets: texts(&block["targets"]),
        strategies: block["strategies"].as_array().unwrap().iter().map(|s| SigninStrategy { name: s["name"].as_str().unwrap().into(), columns: texts(&s["columns"]), options: Vec::new() }).collect(),
        session_seconds: block["sessionMinutes"].as_u64().unwrap() * 60,
        tries: block["tries"].as_u64().unwrap() as u32,
        tries_seconds: block["triesMinutes"].as_u64().unwrap() * 60,
        program: block["program"].as_str().unwrap().into(),
        remember_seconds: None,
        remember_asks: false,
    }
}

fn people_db(file: &Value, rows: &[Value]) -> Db {
    let mut db = Db::memory().unwrap();
    let table = file["people"]["table"].as_str().unwrap();
    db.batch("", file["people"]["create"].as_str().unwrap()).unwrap();
    for row in rows {
        let row = row.as_object().unwrap();
        let columns: Vec<&str> = row.keys().map(String::as_str).collect();
        let values: Vec<String> = row.values().map(literal).collect();
        db.batch("", &format!("INSERT INTO {table} ({}) VALUES ({})", columns.join(", "), values.join(", "))).unwrap();
    }
    db.add_signin(source_of(file));
    db
}

/// A person as a generated program reads one: `members`, its name and password hash here.
#[derive(Debug, Clone, Default, PartialEq)]
struct Member {
    id: Id,
    name: String,
    password_hash: String,
}

impl Record for Member {
    const TABLE: &'static str = "members";
    const COLUMNS: &'static [&'static str] = &["name", "password_hash"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name), sql::text(&self.password_hash)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> clarity_runtime::Result<Self> {
        let text = |at: usize, column: &str| match &values[at] {
            SqlValue::Null => Ok(String::new()),
            value => sql::get_text(column, value),
        };
        Ok(Member { id, name: text(0, "name")?, password_hash: text(1, "password_hash")? })
    }
}

fn count(db: &Db) -> i64 {
    db.all::<Member>().unwrap().len() as i64
}

/// What a step answers, as the wire would carry it: `{"value": id}` or `{"error": Text}`.
fn signed(answer: Result<Id, Error>) -> Value {
    match answer {
        Ok(id) => json!({ "value": id }),
        Err(Error::Refused(text)) => json!({ "error": text }),
        Err(other) => panic!("not a refusal: {other}"),
    }
}

#[test]
fn people_sign_in_and_out_as_every_runtime_does() {
    let file = file();
    let words = file["refusals"].as_object().unwrap();
    for case in file["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        // `remember N days` keeps its entry in the secret store, here in memory, empty per case.
        let mut source = source_of(&file);
        source.remember_seconds = case["rememberDays"].as_u64().map(|days| days * 24 * 3600);
        // `remember N days when asked` keeps it only when the input remember is "true".
        source.remember_asks = case["rememberAsks"].as_bool().unwrap_or(false);
        if source.remember_seconds.is_some() && cfg!(not(feature = "secrets")) {
            continue;
        }
        #[cfg(feature = "secrets")]
        clarity_runtime::secrets::use_memory();
        let mut db = people_db(&file, case["rows"].as_array().unwrap());
        db.add_signin(source.clone());
        let program = source.program.clone();
        let served = case["served"].as_bool().unwrap_or(false);
        let address = case["address"].as_str().unwrap_or("");
        // The session cookie each browser carries, as the program last set it.
        let mut cookies: std::collections::BTreeMap<String, String> = Default::default();
        for (at, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let what = format!("{name}, step {}", at + 1);
            let browser = step["browser"].as_str().unwrap_or("a").to_string();
            let mut call = |command: &str, args: Value, run: &dyn Fn() -> Value| -> Value {
                if !served {
                    return run();
                }
                let header = cookies.get(&browser).map(|token| format!("theme=dark; {}={token}", signin::COOKIE));
                let (answer, set) = signin::served(header.as_deref(), address, command, &args, run);
                if let Some(set) = set {
                    let token = set.split(';').next().unwrap().split_once('=').unwrap().1.to_string();
                    if token.is_empty() {
                        cookies.remove(&browser);
                    } else {
                        cookies.insert(browser.clone(), token);
                    }
                }
                answer
            };
            if let Some(strategy) = step["signin"].as_str() {
                let inputs: Map<String, Value> = step["inputs"].as_object().cloned().unwrap_or_default();
                let answer = call("signin", json!({ "strategy": strategy, "inputs": inputs }), &|| signed(db.signin(strategy, &inputs)));
                match step["refused"].as_str() {
                    Some(key) => {
                        let error = &answer["error"];
                        assert_eq!(error["key"], key, "{what}: {answer}");
                        assert_eq!(error["text"], words[key], "{what}");
                    }
                    None => assert_eq!(answer["value"], step["me"], "{what}: {answer}"),
                }
            } else if step.get("signout").is_some() {
                call("signout", json!({}), &|| {
                    db.signout();
                    json!({ "value": null })
                });
            } else if let Some(me) = step.get("me") {
                let answer = call("started", json!({}), &|| json!({ "value": db.me_id() }));
                assert_eq!(&answer["value"], me, "{what}");
            } else if let Some(h) = step["hashed"].as_object() {
                let minimum = h.get("minimum").and_then(Value::as_u64).map(|m| m as u32);
                match (signin::hashed(h["method"].as_str().unwrap(), h["text"].as_str().unwrap(), minimum), step["refused"].as_str()) {
                    (Err(Error::Refused(text)), Some(key)) => {
                        assert_eq!(text.key.as_deref(), Some(key), "{what}");
                        assert_eq!(text.text, words[key], "{what}");
                        assert_eq!(text.values.get("minimum").map(|s| s.value.clone()), Some(step["minimum"].clone()), "{what}");
                    }
                    (Ok(kept), None) => assert!(kept.starts_with("$argon2id$"), "{what}"),
                    (got, want) => panic!("{what}: got {got:?}, want {want:?}"),
                }
            } else if let Some(save) = step["save"].as_object() {
                call("dispatch_event", json!({ "state": { "window": "Main" }, "handler": "save" }), &|| {
                    let mut member = db.get::<Member>(save["id"].as_i64().unwrap()).unwrap();
                    if let Some(name) = save.get("name").and_then(Value::as_str) {
                        member.name = name.to_string();
                    }
                    if let Some(secret) = save.get("password_hash").and_then(Value::as_object) {
                        member.password_hash = hash_of(secret);
                    }
                    db.save(&mut member).unwrap();
                    json!({ "value": null })
                });
            } else if step.get("restart").is_some() {
                db.add_signin(source.clone());
                assert_eq!(db.signin_first().map(Value::from).unwrap_or(Value::Null), step["page"], "{what}");
            } else if let Some(want) = step["remembered"].as_bool() {
                #[cfg(feature = "secrets")]
                let kept = clarity_runtime::secrets::get(&program, signin::REMEMBERED).is_some_and(|e| !e.is_empty());
                #[cfg(not(feature = "secrets"))]
                let kept = { let _ = &program; false };
                assert_eq!(kept, want, "{what}");
            } else if let Some(n) = step["count"].as_i64() {
                assert_eq!(count(&db), n, "{what}");
            } else {
                panic!("{what}: no step {step}");
            }
        }
    }
}

#[test]
fn allow_readable_and_writable_refuse_in_clarity_s_own_words() {
    let file = file();
    let words = &file["refusals"];
    let refused = |r: clarity_runtime::Result<()>| match r {
        Err(Error::Refused(text)) => (text.key.unwrap(), text.text),
        other => panic!("{other:?}"),
    };
    assert!(signin::allow(true).is_ok() && signin::yours(true).is_ok());
    assert_eq!(refused(signin::allow(false)), ("notAllowed".to_string(), words["notAllowed"].as_str().unwrap().to_string()));
    assert_eq!(refused(signin::yours(false)), ("notYours".to_string(), words["notYours"].as_str().unwrap().to_string()));
}

#[test]
fn a_hashed_field_keeps_the_hash_of_what_code_assigns() {
    let kept = signin::hashed("argon2id", "typed", None).unwrap();
    assert!(kept.starts_with("$argon2id$") && !kept.contains("typed"));
    assert!(signin::hashed("sha256", "t", None).unwrap().starts_with("sha256$"));
    assert!(matches!(signin::hashed("md5", "t", None), Err(Error::Value(_))));
    let codes = signin::new_recovery_codes(3).unwrap();
    assert_eq!(codes.lines().count(), 3);
    assert!(signin::hashed("sha256-lines", &codes, None).unwrap().starts_with("sha256-lines$"));
}

#[test]
fn a_served_call_without_a_session_reaches_only_the_sign_in_window() {
    let file = file();
    let _db = people_db(&file, &[]);
    let page = file["signin"]["page"].as_str().unwrap();
    let ran = |command: &str, args: Value| signin::served(None, "192.0.2.50", command, &args, || json!({ "value": "ran" })).0;
    assert_eq!(ran("started", json!({}))["value"], "ran");
    assert_eq!(ran("open_window", json!({ "window": page }))["value"], "ran");
    assert_eq!(ran("dispatch_event", json!({ "state": { "window": page }, "handler": "typed" }))["value"], "ran");
    for (command, args) in [("open_window", json!({ "window": "Main" })), ("start_app", json!({})), ("dispatch_event", json!({ "state": { "window": "Main" }, "handler": "delete" })), ("keep_settings", json!({}))] {
        assert_eq!(ran(command, args)["error"]["key"], "notAllowed", "{command}");
    }
    // A made-up session is no session.
    let made_up = format!("{}=made-up", signin::COOKIE);
    assert_eq!(signin::served(Some(&made_up), "", "start_app", &json!({}), || json!({ "value": "ran" })).0["error"]["key"], "notAllowed");
}

#[test]
fn the_program_starts_with_the_sign_in_window_until_someone_signed_in() {
    let file = file();
    let db = people_db(&file, &[json!({ "name": "Gus", "email": "gus@signin.test", "password_hash": { "hash": "argon2id", "text": "pw" } })]);
    let page = file["signin"]["page"].as_str().unwrap();
    // The desktop: the computer's user has no record, so the window shows (and no try counts).
    assert_eq!(db.signin_first().as_deref(), Some(page));
    assert!(matches!(db.me::<Member>(), Err(Error::Refused(t)) if t.key.as_deref() == Some("notAllowed")));
    db.signin("password", json!({ "name": "gus@signin.test", "password": "pw" }).as_object().unwrap()).unwrap();
    assert_eq!(db.signin_first(), None);
    assert_eq!(db.me::<Member>().unwrap().name, "Gus");
    // Served: per call, by its session.
    let (answer, _) = signin::served(None, "", "started", &json!({}), || json!({ "value": db.signin_first() }));
    assert_eq!(answer["value"], page);
    // A program without people starts as it did.
    assert_eq!(Db::memory().unwrap().signin_first(), None);
}

#[test]
fn the_computer_s_user_with_a_record_is_signed_in_at_the_start() {
    let file = file();
    let db = people_db(&file, &[]);
    // The first account of the computer's user, then a start of the program anew.
    db.signin("account", &Map::new()).unwrap();
    db.signout();
    assert_eq!(db.me_id(), None);
    assert_eq!(db.signin_first(), None);
    assert_eq!(db.me_id(), Some(1));
    // Signed out, the sign-in window shows: the computer's user is tried only at the start.
    db.signout();
    assert_eq!(db.signin_first().as_deref(), Some(file["signin"]["page"].as_str().unwrap()));
}


#[test]
fn a_command_line_signs_in_before_its_command_runs() {
    let file = file();
    let rows = [
        json!({ "name": "Hal", "email": "hal@signin.test", "password_hash": { "hash": "argon2id", "text": "open the doors" } }),
        json!({ "name": "Ivy", "api_token": { "hash": "sha256", "text": "cli-token-of-ivy" } }),
    ];
    let args = |list: &[&str]| list.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    let given = signin::CliSignin::from_args(&args(&["clean", "--user", "hal@signin.test"]));
    assert_eq!(given.user.as_deref(), Some("hal@signin.test"));
    // People who sign in only on the desktop and the web: nothing to do.
    assert_eq!(people_db(&file, &rows).signin_cli(&given, &mut || None).unwrap(), None);
    let with_cli = |db: Db| {
        let mut source = source_of(&file);
        source.targets.push("cli".into());
        let mut db = db;
        db.add_signin(source);
        db
    };
    let db = with_cli(people_db(&file, &rows));
    assert_eq!(db.signin_cli(&given, &mut || Some("open the doors".into())).unwrap(), Some(1));
    assert_eq!(db.me_id(), Some(1));
    let refused = db.signin_cli(&given, &mut || None).unwrap_err();
    assert!(matches!(refused, Error::Refused(t) if t.key.as_deref() == Some("signinRefused")));
    // A token works on the command line as well.
    let token = signin::CliSignin { user: None, token: Some("cli-token-of-ivy".into()) };
    assert_eq!(with_cli(people_db(&file, &rows)).signin_cli(&token, &mut || None).unwrap(), Some(2));
    // Without flags, the computer's user: here the first account, as there are no people yet.
    assert_eq!(with_cli(people_db(&file, &[])).signin_cli(&signin::CliSignin::default(), &mut || None).unwrap(), Some(1));
}

#[test]
fn the_web_server_sets_the_session_cookie_and_reads_it_back() {
    use clarity_runtime::serve::{Pages, QUIET, answer_from};
    let file = file();
    let db = std::sync::Mutex::new(people_db(&file, &[json!({ "name": "Joy", "email": "joy@signin.test", "password_hash": { "hash": "argon2id", "text": "kite" } })]));
    let pages = Pages::new(QUIET);
    let files = |_: &str| None;
    let mut invoke = |command: &str, args: Value| -> Value {
        let db = db.lock().unwrap();
        match command {
            "signin" => signed(db.signin(args["strategy"].as_str().unwrap(), args["inputs"].as_object().unwrap())),
            "signout" => {
                db.signout();
                json!({ "value": null })
            }
            _ => json!({ "value": db.me_id() }),
        }
    };
    let request = |command: &str, args: Value, cookie: Option<&str>| {
        let body = json!({ "command": command, "args": args, "browser": "tab" }).to_string();
        let cookie = cookie.map(|c| format!("Cookie: {c}\r\n")).unwrap_or_default();
        format!("POST /clarity/invoke HTTP/1.1\r\nContent-Length: {}\r\n{cookie}\r\n{body}", body.len()).into_bytes()
    };
    let text = |reply: Vec<u8>| String::from_utf8(reply).unwrap();
    let signed_in = text(answer_from(&pages, &request("signin", json!({ "strategy": "password", "inputs": { "name": "joy@signin.test", "password": "kite" } }), None), "192.0.2.60", &files, &mut invoke));
    let set = signed_in.lines().find_map(|l| l.strip_prefix("Set-Cookie: ")).expect("a session cookie");
    assert!(set.ends_with("; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800"), "{set}");
    let cookie = set.split(';').next().unwrap().to_string();
    let me = text(answer_from(&pages, &request("open_window", json!({ "window": "Main" }), Some(&cookie)), "192.0.2.60", &files, &mut invoke));
    assert!(me.ends_with("{\"value\":1}"), "{me}");
    // Without the cookie the main window is out of reach.
    let none = text(answer_from(&pages, &request("open_window", json!({ "window": "Main" }), None), "192.0.2.60", &files, &mut invoke));
    assert!(none.contains("notAllowed"), "{none}");
    let out = text(answer_from(&pages, &request("signout", json!({}), Some(&cookie)), "192.0.2.60", &files, &mut invoke));
    assert!(out.contains("Set-Cookie: clarity_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"), "{out}");
    let after = text(answer_from(&pages, &request("open_window", json!({ "window": "Main" }), Some(&cookie)), "192.0.2.60", &files, &mut invoke));
    assert!(after.contains("notAllowed"), "{after}");
}

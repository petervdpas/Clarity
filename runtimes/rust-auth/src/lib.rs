//! Clarity.Auth: the sign-in strategies of the programs Clarity generates,
//! their sessions and failed tries. Every operation is also reachable by
//! name with JSON arguments (`call`), which is what the C table offers.

mod account;
pub mod attempt;
mod catalog;
mod hash;
#[cfg(feature = "openid")]
pub mod openid;
#[cfg(feature = "openid")]
pub mod providers;
mod remember;
mod sessions;
mod tries;

pub use catalog::{Field, Strategy, catalog, catalog_json};

use serde_json::{Map, Value, json};

/// Why an operation gave no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub kind: ProblemKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    UnknownStrategy,
    BadInput,
    Failed,
}

impl Problem {
    pub(crate) fn new(kind: ProblemKind, text: impl Into<String>) -> Self {
        Problem { kind, text: text.into() }
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// Where to look the person up: the field of that role, equal to `equals`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Find {
    pub role: String,
    pub equals: String,
}

/// Which field to find the person by, for this strategy and these inputs.
pub fn find(strategy: &str, inputs: &Map<String, Value>) -> Result<Find, Problem> {
    let s = catalog::strategy(strategy)?;
    if s.provider {
        return Ok(Find { role: "match".into(), equals: provider_identity(strategy, inputs)? });
    }
    let input = |name: &str| catalog::input(s, inputs, name);
    // Every input the strategy takes is given, not only the one it finds by.
    for name in s.inputs {
        input(name)?;
    }
    let (role, equals) = match s.name {
        "password" => ("match", input("name")?.to_string()),
        "account" => ("match", account::system_account()?),
        "token" => ("secret", hash::hash("sha256", input("token")?)?),
        "recovery" => ("match", input("name")?.to_string()),
        _ => unreachable!("every strategy of the catalog finds"),
    };
    Ok(Find { role: role.into(), equals })
}

/// Who signed in at a provider, by the code its browser brought back.
fn provider_identity(strategy: &str, inputs: &Map<String, Value>) -> Result<String, Problem> {
    #[cfg(feature = "openid")]
    {
        providers::identity(strategy, inputs).map_err(|text| Problem::new(ProblemKind::Failed, text))
    }
    #[cfg(not(feature = "openid"))]
    {
        let _ = inputs;
        Err(Problem::new(ProblemKind::Failed, format!("this program carries no sign-in with a provider ({strategy}); its Clarity.Auth is built without the feature openid")))
    }
}

/// Whether the person found (their fields by role, as stored) is signed in.
pub fn check(strategy: &str, inputs: &Map<String, Value>, stored: &Map<String, Value>) -> Result<bool, Problem> {
    let s = catalog::strategy(strategy)?;
    let secret = stored.get("secret").and_then(Value::as_str).unwrap_or("");
    // The provider said who signed in when the person was found.
    if s.provider {
        return Ok(true);
    }
    Ok(match s.name {
        "password" => hash::verify(catalog::input(s, inputs, "password")?, secret),
        "account" => true,
        "token" => hash::verify(catalog::input(s, inputs, "token")?, secret),
        "recovery" => hash::verify(catalog::input(s, inputs, "code")?, secret),
        _ => unreachable!("every strategy of the catalog checks"),
    })
}

/// The fields of the first account (by role), made of the inputs when the
/// entity of people has no record yet: the match field equal to what `find`
/// says, a secret kept by its field's hash. Only a strategy whose catalog
/// entry says `first_account` makes one.
pub fn first_account(strategy: &str, inputs: &Map<String, Value>) -> Result<Map<String, Value>, Problem> {
    let s = catalog::strategy(strategy)?;
    if !s.first_account {
        return Err(Problem::new(ProblemKind::BadInput, format!("the {} strategy cannot make the first account", s.name)));
    }
    let found = find(strategy, inputs)?;
    let mut made = Map::new();
    made.insert(found.role, Value::from(found.equals));
    match s.name {
        "password" => {
            made.insert("secret".into(), Value::from(hash::hash("argon2id", catalog::input(s, inputs, "password")?)?));
        }
        "account" => {}
        _ if s.provider => {}
        _ => unreachable!("every strategy that makes the first account says its fields"),
    }
    Ok(made)
}

/// What to store after a successful sign-in (by role): a recovery code is
/// used up; nothing for a strategy that keeps what it stores as it is.
pub fn after_signin(strategy: &str, inputs: &Map<String, Value>, stored: &Map<String, Value>) -> Result<Map<String, Value>, Problem> {
    let s = catalog::strategy(strategy)?;
    let mut changed = Map::new();
    if s.name == "recovery" {
        let secret = stored.get("secret").and_then(Value::as_str).unwrap_or("");
        if let Some(left) = hash::without_code(catalog::input(s, inputs, "code")?, secret) {
            changed.insert("secret".into(), Value::from(left));
        }
    }
    Ok(changed)
}

pub use hash::{fits as secret_fits, hash, new_codes as new_recovery_codes, new_token, verify};
pub use remember::{remember_check, remember_make, remember_record};
pub use sessions::{session_close, session_close_all, session_close_others, session_find, session_open};
pub use tries::{tries_allowed, tries_cleared, tries_failed};

/// The time now in whole seconds, as sessions and tries count it.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// An operation by name with its arguments as a JSON object; the result as JSON.
pub fn call(op: &str, args: &str) -> Result<String, Problem> {
    let args: Map<String, Value> = if args.trim().is_empty() {
        Map::new()
    } else {
        serde_json::from_str(args).map_err(|e| Problem::new(ProblemKind::BadInput, format!("the arguments of {op} are no JSON object: {e}")))?
    };
    let text = |name: &str| args.get(name).and_then(Value::as_str).ok_or_else(|| Problem::new(ProblemKind::BadInput, format!("{op} needs the text '{name}'")));
    let number = |name: &str| args.get(name).and_then(Value::as_i64).ok_or_else(|| Problem::new(ProblemKind::BadInput, format!("{op} needs the number '{name}'")));
    let object = |name: &str| args.get(name).and_then(Value::as_object).cloned().unwrap_or_default();
    let at = args.get("now").and_then(Value::as_u64).unwrap_or_else(now);
    let result = match op {
        "catalog" => serde_json::from_str(&catalog_json()).expect("the catalog is JSON"),
        "hash" => json!(hash(text("method")?, text("text")?)?),
        "newToken" => json!(new_token()?),
        "newRecoveryCodes" => json!(new_recovery_codes(number("count")?.clamp(1, 100) as usize)?),
        "secretFits" => json!(secret_fits(text("text")?, number("minimum")?.max(0) as u32)),
        "afterSignin" => Value::Object(after_signin(text("strategy")?, &object("inputs"), &object("stored"))?),
        "find" => {
            let found = find(text("strategy")?, &object("inputs"))?;
            json!({"role": found.role, "equals": found.equals})
        }
        "check" => json!(check(text("strategy")?, &object("inputs"), &object("stored"))?),
        "firstAccount" => Value::Object(first_account(text("strategy")?, &object("inputs"))?),
        "triesAllowed" => json!(tries_allowed(text("key")?, number("tries")?.max(0) as u32, number("window")?.max(0) as u64, at)),
        "triesFailed" => {
            tries_failed(text("key")?, at);
            Value::Null
        }
        "triesCleared" => {
            tries_cleared(text("key")?);
            Value::Null
        }
        "sessionOpen" => json!(session_open(number("record")?, number("seconds")?.max(0) as u64, at)?),
        "sessionFind" => json!(session_find(text("token")?, at)),
        "sessionClose" => {
            session_close(text("token")?);
            Value::Null
        }
        "sessionCloseAll" => {
            session_close_all(number("record")?);
            Value::Null
        }
        "rememberMake" => json!(remember_make(number("record")?, number("seconds")?.max(0) as u64, text("stored")?, at)?),
        "rememberRecord" => json!(remember_record(text("entry")?)),
        "rememberCheck" => json!(remember_check(text("entry")?, text("stored")?, at)),
        "sessionCloseOthers" => {
            session_close_others(number("record")?, text("token")?);
            Value::Null
        }
        "signinBegin" => {
            let address = args.get("address").and_then(Value::as_str);
            match attempt::signin_begin(&attempt::block_of(&args["block"]), text("strategy")?, &object("inputs"), text("target")?, address, at) {
                attempt::Begun::Lookup { column, secret, equals, keys } => json!({"column": column, "secret": secret, "equals": equals, "keys": keys}),
                attempt::Begun::Refused(key) => json!({"refused": key}),
            }
        }
        "signinDecide" => {
            let keys: Vec<String> = args.get("keys").and_then(Value::as_array).map(|k| k.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let stored = args.get("found").and_then(|f| f.get("stored")).map(|s| s.as_str().unwrap_or_default().to_string());
            let no_people = args.get("noPeople").and_then(Value::as_bool).unwrap_or(false);
            match attempt::signin_decide(&attempt::block_of(&args["block"]), text("strategy")?, &object("inputs"), &keys, stored.as_deref(), no_people, at) {
                attempt::Decided::SignedIn { store } => json!({"signedIn": true, "store": store}),
                attempt::Decided::Make { columns } => json!({"make": columns}),
                attempt::Decided::Refused(key) => json!({"refused": key}),
            }
        }
        _ => return Err(Problem::new(ProblemKind::Failed, format!("Clarity.Auth has no operation '{op}'"))),
    };
    Ok(result.to_string())
}

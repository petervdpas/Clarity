//! One sign-in attempt, decided here for every stack: the runtime looks the
//! person up and stores what it is told, nothing more. `signin_begin` says
//! where to look (or refuses: a strategy the project does not name, one that
//! does not work on the target, inputs that find nobody, too many failed
//! tries); `signin_decide` takes what the lookup found and says what to
//! store, which first account to make, or the refusal, counting failed tries.
//! A refusal is Clarity's key: `signinRefused` or `signinTooManyTries`.

use serde_json::{Map, Value};

use crate::catalog::{self, Strategy};

/// Clarity's refusal of a sign-in that did not work.
pub const REFUSED: &str = "signinRefused";
/// Clarity's refusal while an account or address has failed too often.
pub const TOO_MANY_TRIES: &str = "signinTooManyTries";

/// A strategy of the project's `signin` block: its name, the columns of the
/// fields it names (in the catalog's order of roles) and its settings.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockStrategy {
    pub name: String,
    pub columns: Vec<String>,
    /// A provider's settings (`client`, `secret`, `tenant`, `issuer`, `register`), given to it with the inputs.
    pub options: Map<String, Value>,
}

/// The project's `signin` block as an attempt needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub strategies: Vec<BlockStrategy>,
    /// Failed tries allowed per account and per address within `tries_seconds`.
    pub tries: u32,
    pub tries_seconds: u64,
}

/// Where to look the person up, or why not.
#[derive(Debug, Clone, PartialEq)]
pub enum Begun {
    /// The column to find the person by, equal to `equals`; the column of
    /// the secret to read with it; the keys the tries are counted by.
    Lookup { column: String, secret: Option<String>, equals: String, keys: Vec<String> },
    Refused(&'static str),
}

/// What comes of the attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum Decided {
    /// Signed in; `store` holds what to write to the person's record, by column.
    SignedIn { store: Map<String, Value> },
    /// Nobody was found and a first account is made: its columns and values.
    Make { columns: Map<String, Value> },
    Refused(&'static str),
}

/// The strategy of the block and of the catalog by that name.
fn named<'b>(block: &'b Block, strategy: &str) -> Option<(&'b BlockStrategy, &'static Strategy)> {
    let used = block.strategies.iter().find(|s| s.name == strategy)?;
    let info = catalog::catalog().iter().find(|s| s.name == strategy)?;
    Some((used, info))
}

/// The inputs with the strategy's settings, as `find` and `check` take them.
fn with_options(used: &BlockStrategy, inputs: &Map<String, Value>) -> Map<String, Value> {
    let mut all = inputs.clone();
    for (key, value) in &used.options {
        all.entry(key.clone()).or_insert_with(|| value.clone());
    }
    all
}

/// The column of the field of `role`.
fn column(used: &BlockStrategy, info: &Strategy, role: &str) -> Option<String> {
    info.fields.iter().position(|f| f.role == role).and_then(|at| used.columns.get(at)).cloned()
}

/// Values by role as values by column.
fn by_column(used: &BlockStrategy, info: &Strategy, by_role: Map<String, Value>) -> Map<String, Value> {
    by_role.into_iter().filter_map(|(role, value)| column(used, info, &role).map(|c| (c, value))).collect()
}

/// Where to look the person up for `strategy` with `inputs` on `target`
/// (`desktop`, `web`, `cli`), from `address` when served; `now` in seconds.
pub fn signin_begin(block: &Block, strategy: &str, inputs: &Map<String, Value>, target: &str, address: Option<&str>, now: u64) -> Begun {
    let Some((used, info)) = named(block, strategy) else { return Begun::Refused(REFUSED) };
    if !info.targets.contains(&target) {
        return Begun::Refused(REFUSED);
    }
    let Ok(found) = crate::find(strategy, &with_options(used, inputs)) else { return Begun::Refused(REFUSED) };
    let Some(by) = column(used, info, &found.role) else { return Begun::Refused(REFUSED) };
    let mut keys = vec![format!("account:{}", found.equals)];
    if let Some(address) = address.filter(|a| !a.is_empty()) {
        keys.push(format!("address:{address}"));
    }
    if keys.iter().any(|k| !crate::tries_allowed(k, block.tries, block.tries_seconds, now)) {
        return Begun::Refused(TOO_MANY_TRIES);
    }
    Begun::Lookup { column: by, secret: column(used, info, "secret"), equals: found.equals, keys }
}

/// What comes of the attempt, from what the lookup found: `stored`, the
/// person's secret (empty without one), or none when nobody was found;
/// `no_people` whether the entity has no record at all.
pub fn signin_decide(block: &Block, strategy: &str, inputs: &Map<String, Value>, keys: &[String], stored: Option<&str>, no_people: bool, now: u64) -> Decided {
    let failed = || {
        for key in keys {
            crate::tries_failed(key, now);
        }
        Decided::Refused(REFUSED)
    };
    let Some((used, info)) = named(block, strategy) else { return failed() };
    let all = with_options(used, inputs);
    let decided = match stored {
        Some(secret) => {
            let mut kept = Map::new();
            kept.insert("secret".into(), Value::from(secret));
            if !crate::check(strategy, &all, &kept).unwrap_or(false) {
                return failed();
            }
            match crate::after_signin(strategy, &all, &kept) {
                Ok(changed) => Decided::SignedIn { store: by_column(used, info, changed) },
                Err(_) => return failed(),
            }
        }
        // A provider that lets people register makes their record at their first sign-in.
        None if (info.first_account && no_people) || registers(used) => match crate::first_account(strategy, &all) {
            Ok(made) => Decided::Make { columns: by_column(used, info, made) },
            Err(_) => return failed(),
        },
        None => return failed(),
    };
    for key in keys {
        crate::tries_cleared(key);
    }
    decided
}

/// Whether the strategy's settings let a first sign-in make the person's record (`register`).
fn registers(used: &BlockStrategy) -> bool {
    used.options.get("register").is_some_and(|v| v.as_bool() == Some(true) || v.as_str() == Some("true"))
}

/// The block as the JSON calls give it: `{strategies: [{name, columns, options}], tries, triesSeconds}`.
pub fn block_of(value: &Value) -> Block {
    let strategies = value["strategies"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|s| BlockStrategy {
                    name: s["name"].as_str().unwrap_or_default().to_string(),
                    columns: s["columns"].as_array().map(|c| c.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                    options: s["options"].as_object().cloned().unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Block { strategies, tries: value["tries"].as_u64().unwrap_or(5) as u32, tries_seconds: value["triesSeconds"].as_u64().unwrap_or(900) }
}

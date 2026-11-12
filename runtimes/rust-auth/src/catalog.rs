//! The strategies: what each one matches on the project's entity of people,
//! which inputs it takes and on which targets it works.

use serde_json::{Map, Value, json};

use crate::{Problem, ProblemKind};

/// A field of the entity of people a strategy names, by its role.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    /// `match`: equal to what identifies the person; `secret`: kept hashed.
    pub role: &'static str,
    /// The field types it may have.
    pub kinds: &'static [&'static str],
    /// How a secret field is kept, when it is one.
    pub hash: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct Strategy {
    pub name: &'static str,
    /// The fields named on its line in the project file, in this order.
    pub fields: &'static [Field],
    /// What the sign-in window (or caller) gives it.
    pub inputs: &'static [&'static str],
    pub targets: &'static [&'static str],
    /// Whether the first account can be made with it, when there is none yet.
    pub first_account: bool,
    /// A provider (GitHub, Microsoft, an OpenID provider): the browser signs
    /// in at the provider, who says who it is (`crate::providers`).
    pub provider: bool,
}

const MATCH_TEXT: Field = Field { role: "match", kinds: &["text", "email"], hash: None };

const STRATEGIES: &[Strategy] = &[
    Strategy { name: "password", fields: &[MATCH_TEXT, Field { role: "secret", kinds: &["text"], hash: Some("argon2id") }], inputs: &["name", "password"], targets: &["desktop", "web", "cli"], first_account: true, provider: false },
    Strategy { name: "account", fields: &[Field { role: "match", kinds: &["text"], hash: None }], inputs: &[], targets: &["desktop", "cli"], first_account: true, provider: false },
    Strategy { name: "token", fields: &[Field { role: "secret", kinds: &["text"], hash: Some("sha256") }], inputs: &["token"], targets: &["desktop", "web", "cli"], first_account: false, provider: false },
    // One-time codes shown once; a code that signs in is used up (`after_signin`).
    Strategy { name: "recovery", fields: &[MATCH_TEXT, Field { role: "secret", kinds: &["text"], hash: Some("sha256-lines") }], inputs: &["name", "code"], targets: &["desktop", "web", "cli"], first_account: false, provider: false },
    // Providers: they match the field of the e-mail address or the login name the provider gives, and take nothing typed.
    Strategy { name: "github", fields: &[MATCH_TEXT], inputs: &[], targets: &["desktop", "web"], first_account: true, provider: true },
    Strategy { name: "microsoft", fields: &[MATCH_TEXT], inputs: &[], targets: &["desktop", "web"], first_account: true, provider: true },
    Strategy { name: "openid", fields: &[MATCH_TEXT], inputs: &[], targets: &["desktop", "web"], first_account: true, provider: true },
];

pub fn catalog() -> &'static [Strategy] {
    STRATEGIES
}

pub fn catalog_json() -> String {
    let list: Vec<Value> = STRATEGIES
        .iter()
        .map(|s| {
            let fields: Vec<Value> = s
                .fields
                .iter()
                .map(|f| {
                    let mut field = json!({"role": f.role, "kinds": f.kinds});
                    if let Some(hash) = f.hash {
                        field["hash"] = json!(hash);
                    }
                    field
                })
                .collect();
            json!({"name": s.name, "fields": fields, "inputs": s.inputs, "targets": s.targets, "firstAccount": s.first_account, "provider": s.provider})
        })
        .collect();
    Value::Array(list).to_string()
}

pub(crate) fn strategy(name: &str) -> Result<&'static Strategy, Problem> {
    STRATEGIES.iter().find(|s| s.name == name).ok_or_else(|| {
        let names: Vec<&str> = STRATEGIES.iter().map(|s| s.name).collect();
        Problem::new(ProblemKind::UnknownStrategy, format!("there is no sign-in strategy '{name}'; there are {}", names.join(", ")))
    })
}

pub(crate) fn input<'a>(s: &Strategy, inputs: &'a Map<String, Value>, name: &str) -> Result<&'a str, Problem> {
    inputs.get(name).and_then(Value::as_str).ok_or_else(|| Problem::new(ProblemKind::BadInput, format!("the {} strategy needs the input '{name}'", s.name)))
}

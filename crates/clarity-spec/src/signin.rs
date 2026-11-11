//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4): the
//! project's `signin` block, as the project file says it, with the
//! strategies of Clarity.Auth's catalog.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `signin Member on desktop web` and its indented lines.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Signin {
    /// The entity of people.
    pub entity: String,
    /// The targets people sign in on (`desktop`, `web`, `cli`).
    pub targets: Vec<String>,
    /// The strategies, in the order written; in the model each with its
    /// inputs and targets from the catalog, which the sign-in part draws from.
    #[ts(as = "Vec<SigninStrategyModel>")]
    pub strategies: Vec<SigninStrategy>,
    /// How long a sign-in lasts, in minutes (`session 8 hours`).
    pub session_minutes: u32,
    /// Failed tries per account or address before it slows down (`tries 5 per 15 minutes`).
    pub tries: u32,
    pub tries_minutes: u32,
    /// How many days a desktop program remembers who signed in (`remember 30 days`); none without the line.
    #[serde(default)]
    #[ts(optional)]
    pub remember_days: Option<u32>,
    /// Whether the sign-in part asks whether to remember (`remember 30 days
    /// when asked`: a "keep me signed in" box), else it always remembers.
    #[serde(default)]
    pub remember_asks: bool,
    /// Which strategies the sign-in part offers, in this order, the first
    /// being the default (`shows password, recovery, account`); empty for
    /// every strategy of the block that works on the target, in block order.
    #[serde(default)]
    pub shows: Vec<String>,
    /// The line of the project file it is written on, for messages about it.
    #[serde(skip)]
    #[ts(skip)]
    pub line: u32,
}

/// Two blocks are the same when they say the same, wherever they are written.
impl PartialEq for Signin {
    fn eq(&self, other: &Self) -> bool {
        (&self.entity, &self.targets, &self.strategies, self.session_minutes, self.tries, self.tries_minutes, self.remember_days, self.remember_asks, &self.shows)
            == (&other.entity, &other.targets, &other.strategies, other.session_minutes, other.tries, other.tries_minutes, other.remember_days, other.remember_asks, &other.shows)
    }
}

impl Eq for Signin {}

/// One strategy line: `password email passwordHash`. Written out as a
/// `SigninStrategyModel`, with what the catalog says of it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct SigninStrategy {
    pub name: String,
    /// The entity's fields it names, in the catalog's order of roles.
    pub fields: Vec<String>,
    /// The fewest characters a secret people choose may have (`minimum 12`
    /// under the strategy's line); none without the line.
    #[serde(default)]
    #[ts(optional)]
    pub minimum: Option<u32>,
}

/// A strategy line as the model hands it to Clarity.UI and the runtimes: the
/// line, and from Clarity.Auth's catalog the inputs it takes (in the order a
/// sign-in part shows them) and the targets it works on, so nothing that
/// draws a sign-in part knows a strategy by its name. Read back, the catalog's
/// part is left: the line is what the project file says.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SigninStrategyModel {
    pub name: String,
    pub fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minimum: Option<u32>,
    /// The catalog's inputs; none for a strategy the catalog does not know.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub inputs: Option<Vec<String>>,
    /// The catalog's targets; none for a strategy the catalog does not know.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub targets: Option<Vec<String>>,
}

impl Serialize for SigninStrategy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SigninStrategyModel::from(self.clone()).serialize(serializer)
    }
}

impl From<SigninStrategy> for SigninStrategyModel {
    fn from(line: SigninStrategy) -> Self {
        let info = strategy_info(&line.name);
        let texts = |list: &[&str]| list.iter().map(|s| s.to_string()).collect();
        SigninStrategyModel { inputs: info.map(|i| texts(i.inputs)), targets: info.map(|i| texts(i.targets)), name: line.name, fields: line.fields, minimum: line.minimum }
    }
}

pub const DEFAULT_SESSION_MINUTES: u32 = 8 * 60;
pub const DEFAULT_TRIES: u32 = 5;
pub const DEFAULT_TRIES_MINUTES: u32 = 15;

impl Signin {
    pub fn new(entity: impl Into<String>, targets: Vec<String>) -> Self {
        Signin { entity: entity.into(), targets, strategies: Vec::new(), session_minutes: DEFAULT_SESSION_MINUTES, tries: DEFAULT_TRIES, tries_minutes: DEFAULT_TRIES_MINUTES, remember_days: None, remember_asks: false, shows: Vec::new(), line: 0 }
    }

    /// The strategy of that name, when the project uses it.
    pub fn strategy(&self, name: &str) -> Option<&SigninStrategy> {
        self.strategies.iter().find(|s| s.name == name)
    }

    /// The fewest characters text assigned to a hashed field may have: the
    /// `minimum` of the strategy that names it as its secret.
    pub fn minimum_of(&self, field: &str) -> Option<u32> {
        self.strategies
            .iter()
            .find(|line| strategy_info(&line.name).is_some_and(|info| info.fields.iter().zip(&line.fields).any(|(role, named)| named == field && role.role == "secret")))
            .and_then(|line| line.minimum)
    }

    /// The hash method of a field: the one of the strategy that names it as
    /// its secret, Argon2id for a hashed field no strategy names.
    pub fn hash_of(&self, field: &str) -> &'static str {
        for line in &self.strategies {
            if let Some(info) = strategy_info(&line.name) {
                for (role, named) in info.fields.iter().zip(&line.fields) {
                    if named == field && let Some(hash) = role.hash {
                        return hash;
                    }
                }
            }
        }
        "argon2id"
    }
}

/// The catalog's entry of a strategy.
pub fn strategy_info(name: &str) -> Option<&'static clarity_auth::Strategy> {
    clarity_auth::catalog().iter().find(|s| s.name == name)
}

/// The names of the catalog's strategies, for messages.
pub fn strategy_names() -> Vec<&'static str> {
    clarity_auth::catalog().iter().map(|s| s.name).collect()
}

/// Marks the entity the block names as the entity of people, so `me` is
/// typed while the code is checked. A missing entity is the checker's to report.
pub fn mark_people(files: &mut [crate::model::SpecFile], signin: &Signin) {
    for entity in files.iter_mut().flat_map(|f| f.entities.iter_mut()) {
        entity.people = entity.name.name == signin.entity;
    }
}

/// The word of a field's type the catalog's kinds use: `text`, `email`.
fn kind_word(ty: &crate::model::FieldType) -> String {
    match ty {
        crate::model::FieldType::Text { .. } => "text".to_string(),
        other => crate::dictionary::type_text(other),
    }
}

/// The `signin` block against the project: its entity is stored, each field
/// a strategy names is there with a type of its role, a secret is `hashed`,
/// a salted hash is not `unique`, a strategy that makes the first account
/// can (every other required field has a default), and people who sign in
/// on the desktop or the web have a sign-in window. What is about the block
/// itself is said at its line of the project file, with an empty path the
/// project loader fills in; what is about a field, at the field.
pub fn check(files: &[crate::model::SpecFile], signin: &Signin) -> Vec<crate::diagnostic::Diagnostic> {
    use crate::diagnostic::Diagnostic;
    let mut found = Vec::new();
    let at = crate::span::Span { start: 0, end: 0, line: signin.line, col: 1 };
    let Some((file, entity)) = files.iter().flat_map(|f| f.entities.iter().map(move |e| (f, e))).find(|(_, e)| e.name.name == signin.entity && !e.settings) else {
        found.push(Diagnostic::error(format!("there is no entity {} for 'signin {}': write entity {} with the fields its strategies name", signin.entity, signin.entity, signin.entity), at));
        return found;
    };
    if entity.record {
        found.push(Diagnostic::error(format!("{} is a record, which is never stored; signin names an entity", entity.name.name), at));
        return found;
    }
    let field_error = |message: String, span| Diagnostic::error(message, span).in_file(&file.path);
    for line in &signin.strategies {
        let Some(info) = strategy_info(&line.name) else { continue };
        for (role, named) in info.fields.iter().zip(&line.fields) {
            let Some(field) = entity.field(named) else {
                found.push(Diagnostic::error(format!("{} has no field {named} for the {} strategy", entity.name.name, line.name), at));
                continue;
            };
            let kind = kind_word(&field.ty);
            if !role.kinds.contains(&kind.as_str()) {
                let kinds = role.kinds.join(" or ");
                let verb = if role.hash.is_some() { "keeps" } else { "matches" };
                found.push(field_error(format!("{named} is {kind}, but the {} strategy {verb} {kinds}", line.name), field.span));
                continue;
            }
            match role.hash {
                Some(_) if !field.modifiers.hashed => {
                    found.push(field_error(format!("{named} keeps the {} strategy's secret, so it is hashed: {named}: {}, hashed", line.name, field.type_text), field.span));
                }
                // Only a token's plain digest is equal for equal tokens.
                Some(hash) if field.modifiers.unique && hash != "sha256" => {
                    let why = if hash == "argon2id" { "is hashed with a salt, so no two are equal" } else { "keeps a list of codes, which is never one value" };
                    found.push(field_error(format!("{named} {why}: take 'unique' away"), field.span));
                }
                None if field.modifiers.hashed => {
                    found.push(field_error(format!("the {} strategy looks people up by {named}, so it is kept as it is: take 'hashed' away", line.name), field.span));
                }
                _ => {}
            }
        }
        // The first account has the strategy's fields and the others at their defaults.
        if info.first_account {
            for field in entity.fields.iter().filter(|f| f.modifiers.required && f.modifiers.default.is_none() && !line.fields.contains(&f.name.name)) {
                let name = &field.name.name;
                found.push(field_error(
                    format!("{}'s {name} is required and has no default, so the {} strategy could not make the first account: give {name} a default, or take 'required' away", entity.name.name, line.name),
                    field.span,
                ));
            }
        }
    }
    // The program shows the sign-in window where people sign in on a screen.
    let shown: Vec<&str> = signin.targets.iter().map(String::as_str).filter(|t| matches!(*t, "desktop" | "web")).collect();
    let page = files.iter().flat_map(|f| &f.pages).any(|p| p.kind == crate::model::PageKind::Signin) || app_splash(files).is_some_and(crate::frame::page_signs_in);
    if !shown.is_empty() && !page {
        found.push(Diagnostic::error(format!("people sign in on {}, so the project needs a sign-in window: add a window of the kind signin, or sign-in parts on the app's splash", shown.join(" and ")), at));
    }
    found
}

/// The app's splash page, which may hold sign-in parts and so double as the
/// sign-in window (docs/APP-CAPABILITIES-PLAN.md step 4).
pub fn app_splash(files: &[crate::model::SpecFile]) -> Option<&crate::model::Page> {
    let name = files.iter().flat_map(|f| &f.apps).find_map(|a| a.splash.as_ref())?;
    files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == name.name && p.kind == crate::model::PageKind::Splash)
}

//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4), as every
//! stack's generator writes it: the people's table and the columns each
//! strategy names, the sign-in page, the session and tries, and how a
//! hashed field is kept. One description, so the five stacks pass the same.

use clarity_spec::Project;

/// The project's `signin` block as the runtimes' signin source takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SigninPlan {
    /// The data source the entity of people lives in.
    pub source: String,
    pub table: String,
    pub key: String,
    /// The window of kind `signin`.
    pub page: String,
    pub targets: Vec<String>,
    /// Each strategy with the columns it names, in the catalog's order of roles.
    pub strategies: Vec<(String, Vec<String>)>,
    pub session_seconds: u64,
    pub tries: u32,
    pub tries_seconds: u64,
    /// How long the desktop program remembers a sign-in (`remember N days`); none without the line.
    pub remember_seconds: Option<u64>,
    /// Whether the sign-in part asks whether to remember (`remember N days
    /// when asked`): the runtime then remembers only a sign-in whose input
    /// `remember` is "true".
    pub remember_asks: bool,
}

/// The project's signin source; none without a `signin` block or its entity.
pub(crate) fn signin_plan(project: &Project) -> Option<SigninPlan> {
    let signin = project.signin.as_ref()?;
    let entity = project.entity(&signin.entity)?;
    let page = signin_page(project).unwrap_or_default();
    Some(SigninPlan {
        source: project.source_of(&entity.name.name).unwrap_or(project.default_source()).to_string(),
        table: entity.table_name(),
        key: entity.key_column().to_string(),
        page,
        targets: signin.targets.clone(),
        strategies: signin.strategies.iter().map(|s| (s.name.clone(), s.fields.iter().map(|f| entity.column_of(f)).collect())).collect(),
        session_seconds: u64::from(signin.session_minutes) * 60,
        tries: signin.tries,
        tries_seconds: u64::from(signin.tries_minutes) * 60,
        remember_seconds: signin.remember_days.map(|d| u64::from(d) * 24 * 60 * 60),
        remember_asks: signin.remember_asks,
    })
}

/// The page people sign in on: the window of kind `signin`, else the app's
/// splash when it holds sign-in parts (a button that clicks to `signin ...`),
/// which then doubles as the sign-in window.
pub(crate) fn signin_page(project: &Project) -> Option<String> {
    if let Some(page) = project.pages().find(|p| p.kind == clarity_spec::model::PageKind::Signin) {
        return Some(page.name.name.clone());
    }
    let splash = project.apps().find_map(|a| a.splash.as_ref().map(|s| s.name.clone()))?;
    let page = project.pages().find(|p| p.name.name == splash)?;
    clarity_spec::frame::page_signs_in(page).then_some(splash)
}

/// Whether the program carries Clarity.Auth: its project signs people in.
pub fn carries_auth(project: &Project) -> bool {
    project.signin.is_some()
}

/// How text assigned to a `hashed` field is kept: the hash method of the
/// strategy that names it (Argon2id for one no strategy names) and its
/// `minimum`, checked first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hashing {
    pub method: &'static str,
    pub minimum: Option<u32>,
}

pub(crate) fn hashing(project: &Project, field: &str) -> Hashing {
    match &project.signin {
        Some(signin) => Hashing { method: signin.hash_of(field), minimum: signin.minimum_of(field) },
        None => Hashing { method: "argon2id", minimum: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEOPLE: &str = "module people\n\nentity Member\n    email: email, unique\n    passwordHash: text, hashed\n    recoveryCodes: text, hashed\n";

    fn project() -> Project {
        let mut signin = clarity_spec::signin::Signin::new("Member", vec!["desktop".into(), "web".into()]);
        signin.strategies = vec![
            clarity_spec::signin::SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: Some(12) },
            clarity_spec::signin::SigninStrategy { name: "recovery".into(), fields: vec!["email".into(), "recoveryCodes".into()], minimum: None },
        ];
        signin.session_minutes = 60;
        clarity_spec::load_in_signed([("family/people.clarity", PEOPLE)], [], &[], Some(&signin))
    }

    #[test]
    fn the_plan_names_the_table_and_the_columns_of_each_strategy() {
        let plan = signin_plan(&project()).unwrap();
        assert_eq!((plan.table.as_str(), plan.key.as_str(), plan.session_seconds, plan.tries, plan.tries_seconds), ("member", "id", 3600, 5, 900));
        assert_eq!(plan.strategies, vec![("password".to_string(), vec!["email".to_string(), "password_hash".to_string()]), ("recovery".to_string(), vec!["email".to_string(), "recovery_codes".to_string()])]);
        assert_eq!(plan.targets, vec!["desktop".to_string(), "web".to_string()]);
        assert_eq!((plan.remember_seconds, plan.remember_asks), (None, false));
    }

    #[test]
    fn remember_when_asked_is_handed_to_the_runtimes() {
        let mut project = project();
        let signin = project.signin.as_mut().unwrap();
        signin.remember_days = Some(30);
        signin.remember_asks = true;
        let plan = signin_plan(&project).unwrap();
        assert_eq!((plan.remember_seconds, plan.remember_asks), (Some(2_592_000), true));
    }

    #[test]
    fn a_hashed_field_is_kept_by_its_strategy_s_method_and_minimum() {
        let project = project();
        assert_eq!(hashing(&project, "passwordHash"), Hashing { method: "argon2id", minimum: Some(12) });
        assert_eq!(hashing(&project, "recoveryCodes"), Hashing { method: "sha256-lines", minimum: None });
        assert_eq!(hashing(&project, "other"), Hashing { method: "argon2id", minimum: None });
    }
}

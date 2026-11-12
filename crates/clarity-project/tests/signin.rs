//! `signin` in the project file (docs/APP-CAPABILITIES-PLAN.md step 4): the
//! entity of people, the targets, the strategies of Clarity.Auth's catalog
//! with their fields, and the session, tries and remember lines.

use clarity_spec::signin::{Signin, SigninStrategy};

const PROJECT: &str = "project Family\n    stack go desktop web cli\n    signin Member on desktop web\n        password email passwordHash\n            minimum 12\n        account login\n        session 2 days\n        tries 3 per 10 minutes\n        remember 30 days\n    datasource main sqlite \"data/family.db\"\n";

#[test]
fn the_signin_block_is_read_with_its_strategies_and_options() {
    let settings = clarity_project::parse(PROJECT).unwrap();
    let signin = settings.signin.clone().expect("a signin block");
    assert_eq!(
        signin,
        Signin {
            entity: "Member".into(),
            targets: vec!["desktop".into(), "web".into()],
            strategies: vec![SigninStrategy { name: "password".into(), fields: vec!["email".into(), "passwordHash".into()], minimum: Some(12), provider: None }, SigninStrategy { name: "account".into(), fields: vec!["login".into()], minimum: None, provider: None }],
            session_minutes: 2 * 24 * 60,
            tries: 3,
            tries_minutes: 10,
            remember_days: Some(30),
            remember_asks: false,
            shows: Vec::new(),
            language_picker: false,
            line: 3,
        }
    );
    assert_eq!(settings.signin.as_ref().unwrap().line, 3);
    // The data source after it is still a setting of its own.
    assert_eq!(settings.datasources.len(), 1);
}

#[test]
fn without_targets_people_sign_in_on_the_web_and_the_defaults_hold() {
    let settings = clarity_project::parse("project Club\n    stack go web\n    signin Member\n        password email passwordHash\n").unwrap();
    let signin = settings.signin.unwrap();
    assert_eq!(signin.targets, vec!["web".to_string()]);
    assert_eq!((signin.session_minutes, signin.tries, signin.tries_minutes, signin.remember_days), (480, 5, 15, None));
}

#[test]
fn the_block_is_written_back_as_it_was_read() {
    let settings = clarity_project::parse(PROJECT).unwrap();
    let written = clarity_project::write(None, &settings).unwrap();
    assert!(written.contains("    signin Member on desktop web\n        password email passwordHash\n            minimum 12\n        account login\n        session 2 days\n        tries 3 per 10 minutes\n        remember 30 days\n"), "{written}");
    assert_eq!(clarity_project::parse(&written).unwrap().signin, settings.signin);
    // Written over the file it came from, it stays as it is.
    assert_eq!(clarity_project::write(Some(PROJECT), &settings).unwrap(), PROJECT);
    // Defaults are not written.
    let plain = clarity_project::parse("project Club\n    stack go web\n    signin Member\n        password email passwordHash\n").unwrap();
    let written = clarity_project::write(None, &plain).unwrap();
    assert!(written.contains("    signin Member\n        password email passwordHash\n") && !written.contains("session") && !written.contains("tries"), "{written}");
}

#[test]
fn what_does_not_fit_the_catalog_or_the_stack_says_so() {
    let wrong = |block: &str| clarity_project::parse(&format!("project Club\n    stack go desktop web\n{block}")).unwrap_err();
    assert_eq!(wrong("    signin Member\n        pigeon email\n"), "line 4: there is no sign-in strategy 'pigeon'; there are password, account, token, recovery, github, microsoft, openid");
    assert_eq!(wrong("    signin Member\n        password email\n"), "line 4: the password strategy names 2 fields of Member (match, secret), like: password email passwordHash");
    assert_eq!(wrong("    signin Member\n"), "line 3: name at least one strategy under 'signin', indented, like: password email passwordHash");
    assert_eq!(wrong("    signin\n        password email passwordHash\n"), "line 3: write 'signin' and the entity of people, like: signin Member on desktop web");
    assert_eq!(wrong("    signin Member on cli\n        password email passwordHash\n"), "line 3: the stack has no cli target to sign in on");
    assert_eq!(wrong("    signin Member on web\n        account login\n"), "line 4: the account strategy works on desktop, cli, not on web");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n        session 0 hours\n"), "line 5: write how long a sign-in lasts, like: session 8 hours (minutes, hours or days)");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n        tries 5\n"), "line 5: write how many failed tries in how long, like: tries 5 per 15 minutes");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n        password name passwordHash\n"), "line 5: the password strategy is written twice");
    assert_eq!(wrong("    signin Member on web\n        password email passwordHash\n        remember 30 days\n"), "line 5: 'remember' keeps a sign-in on the desktop; add desktop to the signin line");
}

#[test]
fn a_minimum_belongs_under_a_strategy_people_choose_a_secret_for() {
    let wrong = |block: &str| clarity_project::parse(&format!("project Club\n    stack go desktop web\n{block}")).unwrap_err();
    assert_eq!(wrong("    signin Member\n        minimum 12\n"), "line 4: write 'minimum' indented under a strategy line, like: minimum 12");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n        minimum 12\n"), "line 5: write 'minimum' indented under the strategy it belongs to");
    assert_eq!(wrong("    signin Member on desktop\n        account login\n            minimum 12\n"), "line 5: the account strategy keeps no secret people choose, so it has no minimum");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n            minimum zero\n"), "line 5: write the fewest characters a secret may have, like: minimum 12");
    assert_eq!(wrong("    signin Member\n        password email passwordHash\n            session 2 hours\n"), "line 5: unexpected indented line under a strategy; only 'minimum' and a provider's settings go there");
    // The recovery strategy is in the catalog like the others.
    let settings = clarity_project::parse("project Club\n    stack go web\n    signin Member\n        password email passwordHash\n        recovery email recoveryCodes\n").unwrap();
    assert_eq!(settings.signin.unwrap().strategies[1].fields, vec!["email".to_string(), "recoveryCodes".to_string()]);
}

/// What the sign-in part offers is the project's to say, not the part's.
#[test]
fn the_block_says_what_the_sign_in_part_shows_and_whether_it_asks_to_remember() {
    let text = "project Family\n    stack go desktop\n    signin Member on desktop\n        password email passwordHash\n        account login\n        shows account, password\n        remember 30 days when asked\n";
    let settings = clarity_project::parse(text).unwrap();
    let signin = settings.signin.clone().unwrap();
    assert_eq!(signin.shows, vec!["account".to_string(), "password".to_string()]);
    assert_eq!((signin.remember_days, signin.remember_asks), (Some(30), true));
    let written = clarity_project::write(None, &settings).unwrap();
    assert!(written.contains("        shows account, password\n") && written.contains("        remember 30 days when asked\n"), "{written}");
    assert_eq!(clarity_project::write(Some(text), &settings).unwrap(), text);

    let wrong = |block: &str| clarity_project::parse(&format!("project Club\n    stack go desktop\n{block}")).unwrap_err();
    assert_eq!(wrong("    signin Member on desktop\n        password email passwordHash\n        shows pigeon\n"), "line 5: the sign-in part shows only strategies the block names above it; it names password");
    assert_eq!(wrong("    signin Member on desktop\n        password email passwordHash\n        shows password, password\n"), "line 5: password is shown twice");
    assert_eq!(wrong("    signin Member on desktop\n        password email passwordHash\n        shows\n"), "line 5: write the strategies the sign-in part shows, first the default, like: shows password, account");
    assert_eq!(wrong("    signin Member on desktop\n        password email passwordHash\n        remember 30 days if asked\n"), "line 5: write how many days a sign-in is remembered, like: remember 30 days (or remember 30 days when asked)");
}

/// `language picker` lets people pick their language on the sign-in part;
/// without the line the part has none, and it is written back only when on.
#[test]
fn the_block_may_give_the_sign_in_part_a_language_picker() {
    let text = "project Family\n    stack go desktop\n    signin Member on desktop\n        password email passwordHash\n        language picker\n";
    let settings = clarity_project::parse(text).unwrap();
    assert!(settings.signin.as_ref().unwrap().language_picker);
    assert_eq!(clarity_project::write(Some(text), &settings).unwrap(), text);
    let mut off = settings.clone();
    off.signin.as_mut().unwrap().language_picker = false;
    assert!(!clarity_project::write(Some(text), &off).unwrap().contains("language picker"));
    let plain = clarity_project::parse("project Family\n    stack go desktop\n    signin Member on desktop\n        password email passwordHash\n").unwrap();
    assert!(!plain.signin.unwrap().language_picker);
    let wrong = clarity_project::parse("project Club\n    stack go desktop\n    signin Member on desktop\n        password email passwordHash\n        language box\n").unwrap_err();
    assert_eq!(wrong, "line 5: write language picker to let people pick their language on the sign-in part");
}

/// A provider's settings are indented under its line: its client, its
/// secret as a secret setting (never the secret itself), Microsoft's
/// tenant, an OpenID provider's issuer, and whether a first sign-in makes
/// the person's record (`register`).
#[test]
fn a_provider_has_its_settings_indented_under_its_line() {
    let text = "project Club\n    stack rust desktop\n    signin Member on desktop\n        password email passwordHash\n        github email\n            client \"Ov23li\"\n            secret \"setting:githubSecret\"\n            register\n        microsoft email\n            client \"app\"\n            tenant \"contoso.onmicrosoft.com\"\n        openid email\n            client \"web\"\n            issuer \"https://accounts.google.com\"\n";
    let settings = clarity_project::parse(text).unwrap();
    let signin = settings.signin.as_ref().unwrap();
    let github = signin.strategy("github").unwrap().provider.as_ref().unwrap();
    assert_eq!((github.client.as_str(), github.secret.as_deref(), github.register), ("Ov23li", Some("setting:githubSecret"), true));
    assert_eq!(signin.strategy("microsoft").unwrap().provider.as_ref().unwrap().tenant.as_deref(), Some("contoso.onmicrosoft.com"));
    assert_eq!(signin.strategy("openid").unwrap().provider.as_ref().unwrap().issuer.as_deref(), Some("https://accounts.google.com"));
    assert!(signin.strategy("password").unwrap().provider.is_none());
    assert_eq!(clarity_project::write(Some(text), &settings).unwrap(), text);
    let fail = |lines: &str| clarity_project::parse(&format!("project Club\n    stack rust desktop\n    signin Member on desktop\n        github email\n{lines}")).unwrap_err();
    assert_eq!(fail("            secret \"hunter2\"\n"), "line 5: the secret is a secret setting, never the secret itself: secret \"setting:githubSecret\"");
    assert_eq!(fail("            tenant \"x\"\n"), "line 5: only the microsoft strategy has a tenant");
    assert_eq!(fail("            client \"a\"\n            client \"b\"\n"), "line 6: 'client' is written twice");
    let password = clarity_project::parse("project Club\n    stack rust desktop\n    signin Member on desktop\n        password email passwordHash\n            client \"a\"\n").unwrap_err();
    assert_eq!(password, "line 5: the password strategy is no provider, so it has no client");
}

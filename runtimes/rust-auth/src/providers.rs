//! Signing in with a provider (docs/APP-CAPABILITIES-PLAN.md, step 4,
//! providers): GitHub, Microsoft and any OpenID provider are strategies of
//! the catalog that match one field of the people (their e-mail address or
//! their login name, as the strategy's `claim` says) and take nothing typed.
//!
//! On the desktop the browser goes to the provider and back to a loopback
//! address with a code ([`begin`], [`finish`], [`through_browser`]), with the
//! `openid` module's PKCE and loopback. [`identity`] trades the code for a
//! token and asks the provider who signed in; an attempt asks twice (find,
//! then the first account), so what a code gave is kept for the run.
//!
//! A strategy's settings come from the project's signin block, given with
//! the inputs: `client`, `secret` (GitHub wants it to trade the code),
//! `tenant` (Microsoft, `common` without it), `issuer` (an OpenID provider,
//! found by discovery), `claim` (`email` or `login`). `authorizeEndpoint`,
//! `tokenEndpoint`, `userEndpoint` and `authority` give a provider's
//! addresses instead of the known ones: another OAuth provider, or a test.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::openid::{self, Pkce, Provider};

/// How long the browser has to come back.
const WAIT: Duration = Duration::from_secs(300);

/// A sign-in on its way through the browser: the provider's page to open,
/// and where the browser comes back.
pub struct Begun {
    pub url: String,
    pub redirect: String,
    pub verifier: String,
    state: String,
    provider: String,
    listener: TcpListener,
}

/// What a provider strategy needs to be known by.
struct Known {
    provider: Provider,
    scope: &'static str,
    user: String,
    /// GitHub keeps the e-mail addresses apart from the user.
    emails: Option<String>,
}

fn setting<'a>(options: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    options.get(name).and_then(Value::as_str).filter(|v| !v.is_empty())
}

/// The provider of `strategy` with its settings.
fn known(strategy: &str, options: &Map<String, Value>) -> Result<Known, String> {
    let client = setting(options, "client").ok_or_else(|| format!("the {strategy} sign-in needs its client: the app's id at the provider"))?;
    let given = |name: &str, otherwise: String| setting(options, name).map(str::to_string).unwrap_or(otherwise);
    let mut known = match strategy {
        "github" => Known {
            provider: Provider::new("GitHub", &given("authorizeEndpoint", "https://github.com/login/oauth/authorize".into()), &given("tokenEndpoint", "https://github.com/login/oauth/access_token".into()), client),
            scope: "read:user user:email",
            user: given("userEndpoint", "https://api.github.com/user".into()),
            emails: None,
        },
        "microsoft" => {
            let authority = given("authority", openid::ENTRA_AUTHORITY.into());
            let tenant = given("tenant", "common".into());
            let mut provider = Provider::entra(&authority, &tenant, client);
            if let Some(endpoint) = setting(options, "authorizeEndpoint") {
                provider.authorize_endpoint = endpoint.into();
            }
            if let Some(endpoint) = setting(options, "tokenEndpoint") {
                provider.token_endpoint = endpoint.into();
            }
            Known { provider, scope: "openid profile email", user: given("userEndpoint", "https://graph.microsoft.com/oidc/userinfo".into()), emails: None }
        }
        "openid" => {
            let issuer = setting(options, "issuer").ok_or("the openid sign-in needs its issuer: the provider's address")?;
            let found = discovery(issuer)?;
            let endpoint = |name: &str| found.get(name).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("{issuer} names no {name}"));
            Known {
                provider: Provider::new(issuer, &given("authorizeEndpoint", endpoint("authorization_endpoint")?), &given("tokenEndpoint", endpoint("token_endpoint")?), client),
                scope: "openid profile email",
                user: given("userEndpoint", endpoint("userinfo_endpoint")?),
                emails: None,
            }
        }
        other => return Err(format!("{other} is no provider")),
    };
    if strategy == "github" {
        // GitHub asks no refresh scope, and wants the client's secret to trade the code.
        known.provider.renews = false;
        known.emails = Some(format!("{}/emails", known.user.trim_end_matches('/')));
    }
    known.provider.secret = setting(options, "secret").map(str::to_string);
    Ok(known)
}

/// An OpenID provider's own description of its addresses.
fn discovery(issuer: &str) -> Result<Map<String, Value>, String> {
    let url = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));
    match get_json(&url, None)? {
        Value::Object(found) => Ok(found),
        _ => Err(format!("{issuer} does not describe itself")),
    }
}

/// A GET whose answer is JSON, with the access token when given.
fn get_json(url: &str, token: Option<&str>) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).http_status_as_error(false).build().into();
    let mut request = agent.get(url).header("Accept", "application/json").header("User-Agent", "Clarity");
    if let Some(token) = token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|err| format!("cannot reach {url}: {err}"))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().map_err(|err| format!("the answer of {url} broke off: {err}"))?;
    if !(200..300).contains(&status) {
        return Err(format!("{url} answered {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|_| format!("{url} answered no JSON: {text}"))
}

/// Starts a sign-in with `strategy`: a loopback address to come back to, and the provider's page.
pub fn begin(strategy: &str, options: &Map<String, Value>) -> Result<Begun, String> {
    let known = known(strategy, options)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|err| format!("cannot wait for the browser: {err}"))?;
    let port = listener.local_addr().map_err(|err| format!("cannot wait for the browser: {err}"))?.port();
    let redirect = format!("http://127.0.0.1:{port}/");
    let pkce = Pkce::new()?;
    let state = openid::random_text()?;
    let url = known.provider.authorize_url(&redirect, known.scope, &state, &pkce);
    Ok(Begun { url, redirect, verifier: pkce.verifier, state, provider: known.provider.name, listener })
}

/// Waits for the browser to come back with the code: the inputs the
/// attempt takes (`code`, `verifier`, `redirect`); `program` is named on the
/// page the browser shows; `stop` ends the wait.
pub fn finish(begun: Begun, program: &str, stop: &AtomicBool) -> Result<Map<String, Value>, String> {
    let code = openid::wait_for_code(&begun.listener, &begun.state, &begun.provider, program, stop, WAIT)?;
    let mut inputs = Map::new();
    inputs.insert("code".into(), Value::from(code));
    inputs.insert("verifier".into(), Value::from(begun.verifier));
    inputs.insert("redirect".into(), Value::from(begun.redirect));
    Ok(inputs)
}

/// A sign-in through the system's browser: it opens the provider's page and waits for it to come back.
pub fn through_browser(strategy: &str, options: &Map<String, Value>, program: &str, stop: &AtomicBool) -> Result<Map<String, Value>, String> {
    let begun = begin(strategy, options)?;
    open_browser(&begun.url)?;
    finish(begun, program, stop)
}

/// Opens `url` in the system's browser.
fn open_browser(url: &str) -> Result<(), String> {
    let opened = if cfg!(target_os = "windows") {
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    opened.map(|_| ()).map_err(|err| format!("cannot open the browser: {err}"))
}

/// What each code gave, for the run: an attempt asks more than once, a code trades once.
fn given() -> &'static Mutex<HashMap<String, String>> {
    static GIVEN: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    GIVEN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Who signed in with the code in `inputs`: the e-mail address or the login name, as `claim` says.
pub fn identity(strategy: &str, inputs: &Map<String, Value>) -> Result<String, String> {
    let code = setting(inputs, "code").ok_or_else(|| format!("the {strategy} sign-in needs the code the browser brought back"))?;
    let claim = setting(inputs, "claim").unwrap_or("email");
    let key = format!("{strategy}\n{claim}\n{code}");
    if let Some(found) = given().lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Ok(found.clone());
    }
    let known = known(strategy, inputs)?;
    let verifier = setting(inputs, "verifier").unwrap_or_default();
    let redirect = setting(inputs, "redirect").unwrap_or_default();
    let tokens = known.provider.redeem(code, redirect, verifier, known.scope)?;
    let user = get_json(&known.user, Some(&tokens.access_token))?;
    let found = match (claim, &known.emails) {
        // GitHub: the login, or the primary e-mail address it has verified.
        ("login", Some(_)) => user["login"].as_str().map(str::to_string),
        ("email", Some(emails)) => {
            let list = get_json(emails, Some(&tokens.access_token))?;
            list.as_array().and_then(|l| l.iter().find(|e| e["primary"].as_bool() == Some(true) && e["verified"].as_bool() == Some(true))).and_then(|e| e["email"].as_str()).map(str::to_string)
        }
        // OpenID: its claims.
        ("login", None) => user["preferred_username"].as_str().or(user["sub"].as_str()).map(str::to_string),
        _ => user["email"].as_str().map(str::to_string),
    };
    let found = found.ok_or_else(|| format!("{} did not say who signed in ({claim})", known.provider.name))?;
    given().lock().unwrap_or_else(|e| e.into_inner()).insert(key, found.clone());
    Ok(found)
}

//! Clarity's agent on the company's models in Microsoft Foundry
//! (docs/FOUNDRY-PLAN.md): a connection of kind `foundry`, signed in with an
//! API key or the person's Microsoft work account. The work account's
//! refresh token is kept in the vault, never in a plain file.

use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use clarity_agent::anthropic::{Anthropic, Auth, Cache};
use clarity_agent::foundry::openid::{Pkce, Provider, random_text, wait_for_code};
use clarity_agent::foundry::{FOUNDRY_SCOPE, Foundry, SignedIn};
use tauri::{AppHandle, Manager};

use crate::vault::VaultState;

/// The vault's category for the refresh tokens of Microsoft sign-ins; the key is the connection's name.
const CATEGORY: &str = "Microsoft";

/// How long Sign in waits for the browser.
const SIGN_IN_WAIT: Duration = Duration::from_secs(300);

/// Whether `connection` is of kind `foundry`.
pub fn is_foundry(app: &AppHandle, connection: &str) -> bool {
    crate::vault::resolve(&app.state::<VaultState>(), connection, "kind").is_ok_and(|kind| kind.trim().eq_ignore_ascii_case("foundry"))
}

fn foundry_of(app: &AppHandle, connection: &str) -> Result<Foundry, String> {
    let vault = app.state::<VaultState>();
    Foundry::of(connection, |key| crate::vault::resolve(&vault, connection, key))
}

/// The model of a Foundry connection, or None when its Microsoft sign-in is missing.
pub fn model(app: &AppHandle, connection: &str, max_tokens: u32, cache: Cache) -> Result<Option<Anthropic>, String> {
    let foundry = foundry_of(app, connection)?;
    let auth = match foundry.signed_in {
        SignedIn::Key(key) => Auth::Key(key),
        SignedIn::Microsoft(provider) => {
            let Some(refresh) = crate::vault::secret(&app.state::<VaultState>(), CATEGORY, connection)? else {
                return Ok(None);
            };
            Auth::Token(Box::new(tokens(app.clone(), connection.to_string(), provider, refresh)))
        }
    };
    Anthropic::new(&foundry.base_url, auth, &foundry.deployment, max_tokens, cache).map(Some)
}

/// Hands out an access token, renewed from the refresh token when it is
/// near its end; a new refresh token from Microsoft goes to the vault.
fn tokens(app: AppHandle, connection: String, provider: Provider, mut refresh: String) -> impl FnMut() -> Result<String, String> + Send {
    let mut access: Option<(String, std::time::Instant)> = None;
    move || {
        if let Some((token, until)) = &access
            && std::time::Instant::now() < *until
        {
            return Ok(token.clone());
        }
        let renewed = provider.renew(&refresh, FOUNDRY_SCOPE).map_err(|err| format!("{err} Click Sign in on the connection {connection} to sign in again."))?;
        if let Some(new) = renewed.refresh_token.filter(|new| *new != refresh) {
            keep(&app, &connection, &new)?;
            refresh = new;
        }
        // Renewed five minutes before Microsoft's end, so no request goes out with a token about to run out.
        let until = std::time::Instant::now() + Duration::from_secs(renewed.expires_in.saturating_sub(300));
        access = Some((renewed.access_token.clone(), until));
        Ok(renewed.access_token)
    }
}

fn keep(app: &AppHandle, connection: &str, refresh: &str) -> Result<(), String> {
    let about = format!("Microsoft sign-in of the connection {connection}, kept by Clarity");
    crate::vault::keep_secret(&app.state::<VaultState>(), CATEGORY, connection, refresh, &about)
}

/// Signs in with the person's Microsoft work account: the browser opens
/// Microsoft's sign-in page and comes back to localhost. Run only when the
/// person asks (the panel's Sign in).
pub fn sign_in(app: &AppHandle, connection: &str) -> Result<String, String> {
    let foundry = foundry_of(app, connection)?;
    let SignedIn::Microsoft(provider) = foundry.signed_in else {
        return Err(format!("the connection {connection} signs in with its apikey"));
    };
    // A locked vault asks for its password before the browser opens, not after.
    crate::vault::secret(&app.state::<VaultState>(), CATEGORY, connection)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|err| format!("cannot wait for the browser: {err}"))?;
    let port = listener.local_addr().map_err(|err| err.to_string())?.port();
    // Microsoft takes any port on http://localhost for an app on the person's computer.
    let redirect = format!("http://localhost:{port}");
    let pkce = Pkce::new()?;
    let state = random_text()?;
    crate::links::open_link(provider.authorize_url(&redirect, FOUNDRY_SCOPE, &state, &pkce))?;
    let code = wait_for_code(&listener, &state, &provider.name, "Clarity", &AtomicBool::new(false), SIGN_IN_WAIT)?;
    let tokens = provider.redeem(&code, &redirect, &pkce.verifier, FOUNDRY_SCOPE)?;
    let refresh = tokens.refresh_token.ok_or_else(|| "Microsoft signed you in but gave no refresh token, so Clarity cannot stay signed in".to_string())?;
    keep(app, connection, &refresh)?;
    Ok(format!("Signed in to Microsoft Foundry for the connection {connection}."))
}

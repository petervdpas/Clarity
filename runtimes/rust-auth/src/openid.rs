//! The `openid` strategy's desktop flow: OpenID Connect's authorization code
//! flow with PKCE (RFC 7636), the system browser and a loopback address on
//! 127.0.0.1 (RFC 8252), one for the programs Clarity makes and for the IDE.
//! A provider is data: its endpoints, its name in what is said, and any
//! fields it asks for on its sign-in page (Microsoft Entra ID's
//! `prompt=select_account`). The browser shows the provider's own sign-in
//! page and comes back to a page only the program listens on; the program
//! never sees a password.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Microsoft's sign-in service, for [`Provider::entra`].
pub const ENTRA_AUTHORITY: &str = "https://login.microsoftonline.com";

/// The Entra ID tenant when none is named: any work or school account.
pub const ANY_ORGANIZATION: &str = "organizations";

/// Who signs in where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// The provider's name in what is said: "Microsoft refused: ...".
    pub name: String,
    pub authorize_endpoint: String,
    pub token_endpoint: String,
    /// The app registration the sign-in goes through.
    pub client_id: String,
    /// Fields the provider's sign-in page is asked for besides the standard ones.
    pub extra: Vec<(String, String)>,
}

/// What a sign-in or a renewal gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    /// Sent with each request to what the sign-in is for.
    pub access_token: String,
    /// Seconds the access token holds.
    pub expires_in: u64,
    /// Renews the access token; a provider may hand a new one with a
    /// renewal, which then replaces the old.
    pub refresh_token: Option<String>,
}

/// Proof that the code comes back to the program that asked for it (RFC 7636).
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// A fresh verifier of 32 random bytes.
    pub fn new() -> Result<Pkce, String> {
        Ok(Pkce::from_verifier(&random_text()?))
    }

    pub fn from_verifier(verifier: &str) -> Pkce {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Pkce { verifier: verifier.to_string(), challenge }
    }
}

/// 32 random bytes as base64url: a PKCE verifier or a state.
pub fn random_text() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| format!("no random numbers from the system: {err}"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// The scope asked for: `scope` and `offline_access`, which brings the refresh token.
fn with_renewal(scope: &str) -> String {
    format!("{scope} offline_access")
}

impl Provider {
    /// A provider by its endpoints.
    pub fn new(name: &str, authorize_endpoint: &str, token_endpoint: &str, client_id: &str) -> Provider {
        Provider { name: name.to_string(), authorize_endpoint: authorize_endpoint.to_string(), token_endpoint: token_endpoint.to_string(), client_id: client_id.to_string(), extra: Vec::new() }
    }

    /// Microsoft Entra ID: `authority` is [`ENTRA_AUTHORITY`] (another
    /// address only in tests), `tenant` the company's id or domain, or
    /// [`ANY_ORGANIZATION`]. The person picks the work account, even when
    /// the browser holds another.
    pub fn entra(authority: &str, tenant: &str, client_id: &str) -> Provider {
        let mut provider = Provider::new("Microsoft", &format!("{authority}/{tenant}/oauth2/v2.0/authorize"), &format!("{authority}/{tenant}/oauth2/v2.0/token"), client_id);
        provider.extra.push(("prompt".into(), "select_account".into()));
        provider
    }

    /// The provider's sign-in page, which sends the browser back to `redirect` with a code.
    pub fn authorize_url(&self, redirect: &str, scope: &str, state: &str, pkce: &Pkce) -> String {
        let scope = with_renewal(scope);
        let mut fields: Vec<(&str, &str)> = vec![
            ("client_id", &self.client_id),
            ("response_type", "code"),
            ("redirect_uri", redirect),
            ("response_mode", "query"),
            ("scope", &scope),
            ("state", state),
            ("code_challenge", &pkce.challenge),
            ("code_challenge_method", "S256"),
        ];
        fields.extend(self.extra.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        format!("{}?{}", self.authorize_endpoint, form(&fields))
    }

    /// Trades the browser's code for tokens.
    pub fn redeem(&self, code: &str, redirect: &str, verifier: &str, scope: &str) -> Result<Tokens, String> {
        self.token(&[
            ("client_id", &self.client_id),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
            ("scope", &with_renewal(scope)),
        ])
    }

    /// A fresh access token from a refresh token.
    pub fn renew(&self, refresh_token: &str, scope: &str) -> Result<Tokens, String> {
        self.token(&[("client_id", &self.client_id), ("grant_type", "refresh_token"), ("refresh_token", refresh_token), ("scope", &with_renewal(scope))])
    }

    fn token(&self, fields: &[(&str, &str)]) -> Result<Tokens, String> {
        let name = &self.name;
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).http_status_as_error(false).build().into();
        let mut response = agent
            .post(&self.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(form(fields))
            .map_err(|err| format!("cannot reach {name}'s sign-in: {err}"))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(|err| format!("{name}'s answer broke off: {err}"))?;
        let answer: Value = serde_json::from_str(&text).map_err(|_| format!("{name}'s sign-in answered {status}: {text}"))?;
        if let Some(error) = answer["error"].as_str() {
            return Err(format!("{name} refused: {}", answer["error_description"].as_str().unwrap_or(error)));
        }
        let access_token = answer["access_token"].as_str().ok_or_else(|| format!("{name}'s answer holds no access token ({status})"))?;
        let expires_in = answer["expires_in"].as_u64().or_else(|| answer["expires_in"].as_str().and_then(|s| s.parse().ok())).unwrap_or(3600);
        Ok(Tokens { access_token: access_token.to_string(), expires_in, refresh_token: answer["refresh_token"].as_str().map(String::from) })
    }
}

/// The code in the browser's request line, when its state is the one sent;
/// `provider` names the provider in what is said.
pub fn code_of(request_line: &str, state: &str, provider: &str) -> Result<String, String> {
    let target = request_line.split_whitespace().nth(1).unwrap_or_default();
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();
    let fields: Vec<(String, String)> = query.split('&').filter_map(|pair| pair.split_once('=')).map(|(k, v)| (decode(k), decode(v))).collect();
    let field = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
    if let Some(error) = field("error") {
        return Err(format!("{provider} did not sign you in: {}", field("error_description").unwrap_or(error)));
    }
    let code = field("code").ok_or_else(|| "the browser's answer holds no code".to_string())?;
    if field("state") != Some(state) {
        return Err("the browser's answer does not match the sign-in that was started".into());
    }
    Ok(code.to_string())
}

/// Waits for the browser to come back to `listener` with the code, and
/// answers it with a page saying the person may go back to `program`.
/// Ends when `stop` is set or after `timeout`.
pub fn wait_for_code(listener: &TcpListener, state: &str, provider: &str, program: &str, stop: &AtomicBool, timeout: Duration) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(|err| format!("cannot wait for the browser: {err}"))?;
    let until = Instant::now() + timeout;
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err("the sign-in was stopped".into());
        }
        if Instant::now() > until {
            return Err("the sign-in took too long; sign in again to try once more".into());
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(answer) = answered(stream, state, provider, program) {
                    return answer;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
            Err(err) => return Err(format!("cannot wait for the browser: {err}")),
        }
    }
}

/// Reads one request of the browser. None when it is not the answer (the
/// browser asking for its icon, say).
fn answered(stream: TcpStream, state: &str, provider: &str, program: &str) -> Option<Result<String, String>> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    // The rest of the request is read too, so closing does not break the page.
    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(n) if n > 0 && !header.trim().is_empty() => {}
            _ => break,
        }
    }
    let is_answer = line.contains("code=") || line.contains("error=");
    let answer = code_of(&line, state, provider);
    let (status, page) = match (&answer, is_answer) {
        (_, false) => ("404 Not Found", String::new()),
        (Ok(_), true) => ("200 OK", page(program, "Signed in", &format!("You are signed in. Close this tab and go back to {program}."))),
        (Err(said), true) => ("200 OK", page(program, "Not signed in", said)),
    };
    let stream = reader.get_mut();
    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}", page.len());
    let _ = stream.flush();
    is_answer.then_some(answer)
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The page the browser shows after the sign-in.
fn page(program: &str, title: &str, said: &str) -> String {
    let (program, said) = (escaped(program), escaped(said));
    format!("<!doctype html><html><head><meta charset=\"utf-8\"><title>{program}: {title}</title></head><body style=\"font-family:sans-serif;margin:3em\"><h1>{program}</h1><p>{said}</p></body></html>")
}

/// Fields as a form or a query: `name=value&...`, encoded.
fn form(fields: &[(&str, &str)]) -> String {
    fields.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect::<Vec<_>>().join("&")
}

/// Percent-encoding: letters, digits and `-._~` stay, the rest is `%XX`.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// A query's value as written: `+` is a space, `%XX` a byte.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                Some(byte) => {
                    out.push(byte);
                    i += 2;
                }
                None => out.push(b'%'),
            },
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

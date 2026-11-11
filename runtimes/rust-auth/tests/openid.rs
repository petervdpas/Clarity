//! The `openid` strategy's desktop flow (OpenID Connect's authorization code
//! flow with PKCE, the system browser and a loopback address), one for the
//! programs Clarity makes and the IDE: the sign-in page's address, the
//! browser's answer on localhost, and the tokens, against a local server so
//! the tests need no network. Microsoft Entra ID is the provider here.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use clarity_auth::openid::{Pkce, Provider, Tokens, code_of, wait_for_code};

/// The scope of Microsoft Foundry's models.
const FOUNDRY_SCOPE: &str = "https://ai.azure.com/.default";

fn login(authority: &str) -> Provider {
    Provider::entra(authority, "contoso.onmicrosoft.com", "11111111-2222-3333-4444-555555555555")
}

#[test]
fn the_challenge_is_the_verifier_s_sha256_in_base64url() {
    // RFC 7636, appendix B.
    let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    assert_eq!(pkce.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    let fresh = Pkce::new().unwrap();
    assert!(fresh.verifier.len() >= 43, "RFC 7636 asks for 43 to 128 characters");
    assert_ne!(fresh.verifier, Pkce::new().unwrap().verifier);
}

#[test]
fn the_sign_in_page_asks_for_a_code_with_the_challenge() {
    let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    let url = login("https://login.microsoftonline.com").authorize_url("http://localhost:53100", FOUNDRY_SCOPE, "st4te", &pkce);
    assert_eq!(
        url,
        "https://login.microsoftonline.com/contoso.onmicrosoft.com/oauth2/v2.0/authorize\
         ?client_id=11111111-2222-3333-4444-555555555555\
         &response_type=code\
         &redirect_uri=http%3A%2F%2Flocalhost%3A53100\
         &response_mode=query\
         &scope=https%3A%2F%2Fai.azure.com%2F.default%20offline_access\
         &state=st4te\
         &code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM\
         &code_challenge_method=S256\
         &prompt=select_account"
    );
}

#[test]
fn the_browser_s_answer_gives_the_code_only_with_the_same_state() {
    assert_eq!(code_of("GET /?code=0.AAA-x%2By&state=st4te&session_state=9 HTTP/1.1", "st4te", "Microsoft"), Ok("0.AAA-x+y".to_string()));
    assert!(code_of("GET /?code=abc&state=other HTTP/1.1", "st4te", "Microsoft").unwrap_err().contains("does not match"));
    let refused = code_of("GET /?error=access_denied&error_description=The+user+cancelled. HTTP/1.1", "st4te", "Microsoft").unwrap_err();
    assert_eq!(refused, "Microsoft did not sign you in: The user cancelled.");
    assert!(code_of("GET /favicon.ico HTTP/1.1", "st4te", "Microsoft").is_err());
}

#[test]
fn the_code_comes_back_from_the_browser_on_localhost() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let browser = std::thread::spawn(move || {
        // A browser asks for its icon first; then comes the answer.
        for path in ["/favicon.ico", "/?code=the-code&state=st4te"] {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
            let mut page = String::new();
            stream.read_to_string(&mut page).unwrap();
            if path != "/favicon.ico" {
                assert!(page.starts_with("HTTP/1.1 200"), "{page}");
                assert!(page.contains("Clarity"), "{page}");
            }
        }
    });
    let stop = AtomicBool::new(false);
    assert_eq!(wait_for_code(&listener, "st4te", "Microsoft", "Clarity", &stop, Duration::from_secs(10)), Ok("the-code".to_string()));
    browser.join().unwrap();
}

#[test]
fn waiting_ends_when_stopped() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let stop = AtomicBool::new(true);
    assert!(wait_for_code(&listener, "st4te", "Microsoft", "Clarity", &stop, Duration::from_secs(10)).unwrap_err().contains("stopped"));
}

/// A token endpoint that answers one request with `reply` and gives back the form it got.
fn token_server(reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let served = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("POST /contoso.onmicrosoft.com/oauth2/v2.0/token "), "{line}");
        let mut length = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header.trim().is_empty() {
                break;
            }
            if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut form = vec![0; length];
        reader.read_exact(&mut form).unwrap();
        let status = if reply.contains("\"error\"") { "400 Bad Request" } else { "200 OK" };
        write!(reader.get_mut(), "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
        String::from_utf8(form).unwrap()
    });
    (authority, served)
}

#[test]
fn the_code_is_traded_for_tokens_with_the_verifier() {
    let (authority, served) = token_server(r#"{"token_type":"Bearer","expires_in":3599,"access_token":"at-1","refresh_token":"rt-1"}"#);
    let tokens = login(&authority).redeem("the-code", "http://localhost:53100", "the-verifier", FOUNDRY_SCOPE).unwrap();
    assert_eq!(tokens, Tokens { access_token: "at-1".into(), expires_in: 3599, refresh_token: Some("rt-1".into()) });
    assert_eq!(
        served.join().unwrap(),
        "client_id=11111111-2222-3333-4444-555555555555&grant_type=authorization_code&code=the-code\
         &redirect_uri=http%3A%2F%2Flocalhost%3A53100&code_verifier=the-verifier\
         &scope=https%3A%2F%2Fai.azure.com%2F.default%20offline_access"
    );
}

#[test]
fn a_refresh_token_renews_the_access_token() {
    let (authority, served) = token_server(r#"{"token_type":"Bearer","expires_in":4000,"access_token":"at-2"}"#);
    let tokens = login(&authority).renew("rt-1", FOUNDRY_SCOPE).unwrap();
    // Microsoft may leave out a new refresh token: the old one stays.
    assert_eq!(tokens, Tokens { access_token: "at-2".into(), expires_in: 4000, refresh_token: None });
    assert_eq!(
        served.join().unwrap(),
        "client_id=11111111-2222-3333-4444-555555555555&grant_type=refresh_token&refresh_token=rt-1\
         &scope=https%3A%2F%2Fai.azure.com%2F.default%20offline_access"
    );
}

#[test]
fn microsoft_s_refusal_is_told_in_its_own_words() {
    let (authority, served) = token_server(r#"{"error":"invalid_grant","error_description":"AADSTS700082: The refresh token has expired."}"#);
    let said = login(&authority).renew("rt-old", FOUNDRY_SCOPE).unwrap_err();
    assert_eq!(said, "Microsoft refused: AADSTS700082: The refresh token has expired.");
    served.join().unwrap();
}

/// Any provider by its endpoints: the provider's name in what is said, and
/// no extra fields unless it asks for them.
#[test]
fn another_provider_is_the_same_flow_with_its_own_endpoints() {
    let provider = Provider::new("Keycloak", "https://id.example.org/realms/club/protocol/openid-connect/auth", "https://id.example.org/realms/club/protocol/openid-connect/token", "club-app");
    let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    assert_eq!(
        provider.authorize_url("http://127.0.0.1:41000", "openid email", "s", &pkce),
        "https://id.example.org/realms/club/protocol/openid-connect/auth?client_id=club-app&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A41000&response_mode=query&scope=openid%20email%20offline_access&state=s&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256"
    );
    assert_eq!(code_of("GET /?error=x HTTP/1.1", "s", "Keycloak").unwrap_err(), "Keycloak did not sign you in: x");
}

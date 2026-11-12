//! Signing in with a provider (docs/APP-CAPABILITIES-PLAN.md, step 4,
//! providers): GitHub, Microsoft and any OpenID provider are strategies of
//! the catalog that match a field of the people and take nothing typed. On
//! the desktop the browser goes to the provider and back to a loopback
//! address with a code (`begin`, `finish`); `find` trades the code for a
//! token and asks the provider who signed in: their e-mail address or their
//! login name, as the strategy's `claim` says. The provider's addresses can
//! be given, which these tests do with a server of their own.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

/// A provider's server for the tests: it answers each path with its JSON and keeps the requests.
struct Server {
    address: String,
    requests: Arc<Mutex<Vec<String>>>,
}

fn server(routes: Vec<(&'static str, String)>) -> Server {
    server_at(|_| routes)
}

/// A server whose answers may name its own address (OpenID discovery does).
fn server_at(routes: impl FnOnce(&str) -> Vec<(&'static str, String)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let routes = routes(&address);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let kept = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut headers = String::new();
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
                headers.push_str(&header);
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let path = line.split_whitespace().nth(1).unwrap_or_default().to_string();
            kept.lock().unwrap().push(format!("{}{}\n{}", line.trim(), headers.to_ascii_lowercase(), String::from_utf8_lossy(&body)));
            let reply = routes.iter().find(|(p, _)| path.starts_with(p)).map(|(_, r)| r.clone()).unwrap_or_else(|| "{}".into());
            let _ = write!(reader.get_mut(), "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
        }
    });
    Server { address, requests }
}

fn options(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), Value::from(*v))).collect()
}

/// The browser coming back to the loopback address with `code` and the state of `url`.
fn browser_returns(url: &str, redirect: &str, code: &str) {
    let state = url.split('&').chain(url.split('?')).find_map(|p| p.strip_prefix("state=")).unwrap().split('&').next().unwrap().to_string();
    let host = redirect.trim_start_matches("http://").trim_end_matches('/');
    let mut stream = TcpStream::connect(host).unwrap();
    write!(stream, "GET /?code={code}&state={state} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut answer = String::new();
    let _ = stream.read_to_string(&mut answer);
    assert!(answer.contains("You are signed in"), "{answer}");
}

#[test]
fn providers_are_strategies_that_take_nothing_typed() {
    for name in ["github", "microsoft", "openid"] {
        let s = clarity_auth::catalog().iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name}"));
        assert!(s.provider && s.inputs.is_empty(), "{name}");
        assert_eq!(s.fields.len(), 1, "{name} matches one field");
        assert_eq!(s.targets, &["desktop", "web"], "{name}");
    }
    assert!(!clarity_auth::catalog().iter().find(|s| s.name == "password").unwrap().provider);
}

#[test]
fn github_signs_in_with_its_login_name_through_the_browser() {
    let server = server(vec![
        ("/login/oauth/access_token", json!({"access_token": "gho_x", "token_type": "bearer"}).to_string()),
        ("/user/emails", json!([{"email": "old@example.com", "primary": false, "verified": true}, {"email": "ann@example.com", "primary": true, "verified": true}]).to_string()),
        ("/user", json!({"login": "ann", "id": 7}).to_string()),
    ]);
    let set = options(&[
        ("client", "Ov23li"),
        ("secret", "shh"),
        ("claim", "login"),
        ("authorizeEndpoint", &format!("{}/login/oauth/authorize", server.address)),
        ("tokenEndpoint", &format!("{}/login/oauth/access_token", server.address)),
        ("userEndpoint", &format!("{}/user", server.address)),
    ]);
    let begun = clarity_auth::providers::begin("github", &set).unwrap();
    assert!(begun.url.starts_with(&format!("{}/login/oauth/authorize?client_id=Ov23li", server.address)), "{}", begun.url);
    assert!(begun.url.contains("code_challenge_method=S256"), "{}", begun.url);
    assert!(!begun.url.contains("offline_access"), "GitHub has no refresh scope: {}", begun.url);
    let url = begun.url.clone();
    let redirect = begun.redirect.clone();
    let browser = std::thread::spawn(move || browser_returns(&url, &redirect, "abc"));
    let inputs = clarity_auth::providers::finish(begun, "Shop", &std::sync::atomic::AtomicBool::new(false)).unwrap();
    browser.join().unwrap();
    assert_eq!(inputs["code"], "abc");
    // The runtime adds the strategy's settings to what the browser gave, and finds the person.
    let mut all = inputs.clone();
    all.extend(set.clone());
    let found = clarity_auth::find("github", &all).unwrap();
    assert_eq!((found.role.as_str(), found.equals.as_str()), ("match", "ann"));
    assert!(clarity_auth::check("github", &all, &Map::new()).unwrap());
    let requests = server.requests.lock().unwrap().join("\n---\n");
    assert!(requests.contains("client_secret=shh") && requests.contains("code_verifier="), "{requests}");
    assert!(requests.contains("accept: application/json"), "{requests}");
    assert!(requests.contains("authorization: bearer gho_x"), "{requests}");
    // With the e-mail as the claim: the primary verified address.
    all.insert("claim".into(), json!("email"));
    assert_eq!(clarity_auth::find("github", &all).unwrap().equals, "ann@example.com");
}

#[test]
fn openid_finds_its_endpoints_by_discovery_and_the_person_by_userinfo() {
    let server = server_at(|address| {
        vec![
            ("/.well-known/openid-configuration", json!({"authorization_endpoint": format!("{address}/authorize"), "token_endpoint": format!("{address}/token"), "userinfo_endpoint": format!("{address}/userinfo")}).to_string()),
            ("/token", json!({"access_token": "at", "expires_in": 3600}).to_string()),
            ("/userinfo", json!({"sub": "123", "email": "bo@example.com", "preferred_username": "bo"}).to_string()),
        ]
    });
    let set = options(&[("client", "app"), ("issuer", &server.address), ("claim", "email")]);
    let begun = clarity_auth::providers::begin("openid", &set).unwrap();
    assert!(begun.url.starts_with(&format!("{}/authorize?", server.address)), "{}", begun.url);
    assert!(begun.url.contains("scope=openid"), "{}", begun.url);
    let mut all = Map::new();
    all.insert("code".into(), json!("c"));
    all.insert("verifier".into(), json!(begun.verifier.clone()));
    all.insert("redirect".into(), json!(begun.redirect.clone()));
    all.extend(set);
    assert_eq!(clarity_auth::find("openid", &all).unwrap().equals, "bo@example.com");
    all.insert("claim".into(), json!("login"));
    assert_eq!(clarity_auth::find("openid", &all).unwrap().equals, "bo");
}

#[test]
fn microsoft_signs_in_with_its_tenant_and_the_userinfo_e_mail() {
    let server = server(vec![
        ("/common/oauth2/v2.0/token", json!({"access_token": "at", "expires_in": 3600}).to_string()),
        ("/oidc/userinfo", json!({"sub": "x", "email": "cy@contoso.com"}).to_string()),
    ]);
    let set = options(&[("client", "app"), ("claim", "email"), ("authority", &server.address), ("userEndpoint", &format!("{}/oidc/userinfo", server.address))]);
    let begun = clarity_auth::providers::begin("microsoft", &set).unwrap();
    assert!(begun.url.starts_with(&format!("{}/common/oauth2/v2.0/authorize?", server.address)), "{}", begun.url);
    let mut all = Map::new();
    all.insert("code".into(), json!("c"));
    all.insert("verifier".into(), json!(begun.verifier.clone()));
    all.insert("redirect".into(), json!(begun.redirect.clone()));
    all.extend(set);
    assert_eq!(clarity_auth::find("microsoft", &all).unwrap().equals, "cy@contoso.com");
    let made = clarity_auth::first_account("microsoft", &all).unwrap();
    assert_eq!(made.get("match"), Some(&json!("cy@contoso.com")));
}

#[test]
fn a_provider_without_its_client_or_code_says_what_is_missing() {
    let missing = clarity_auth::providers::begin("github", &Map::new()).err().unwrap();
    assert!(missing.contains("client"), "{missing}");
    let no_code = clarity_auth::find("microsoft", &options(&[("client", "app")])).err().unwrap();
    assert!(no_code.text.contains("code"), "{}", no_code.text);
}

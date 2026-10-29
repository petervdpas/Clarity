//! HTTP calls (DESIGN 5.23) against the cases every runtime shares
//! (conformance/http.json), on a small server of the test's own.

#![cfg(feature = "http")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use clarity_runtime::http::{self, Auth, Method, Paging, PagingKind};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/http.json");

/// The routes of the conformance file, on a free port; gives its address.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = address.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut words = line.split_whitespace();
            let (method, target) = (words.next().unwrap_or_default().to_string(), words.next().unwrap_or_default().to_string());
            let (path, query) = target.split_once('?').map_or((target.clone(), String::new()), |(p, q)| (p.to_string(), q.to_string()));
            let (mut length, mut test, mut authorization, mut api_key) = (0, String::new(), String::new(), String::new());
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (name, value) = header.split_once(':').unwrap_or((header, ""));
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.trim().parse().unwrap_or(0),
                    "x-test" => test = value.trim().to_string(),
                    "authorization" => authorization = value.trim().to_string(),
                    "x-api-key" => api_key = value.trim().to_string(),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let number = |name: &str, otherwise: i64| -> i64 {
                query.split('&').find_map(|pair| pair.strip_prefix(&format!("{name}="))).and_then(|v| v.parse().ok()).unwrap_or(otherwise)
            };
            // The numbers from `from` up to `to`, and at most 5, as a JSON list.
            let numbers = |from: i64, to: i64| format!("[{}]", (from..=to.min(5)).map(|n| n.to_string()).collect::<Vec<_>>().join(","));
            let mut link = String::new();
            let (status, text) = match path.as_str() {
                "/paged" => {
                    let (page, size) = (number("page", 1), number("size", 5));
                    (200, numbers((page - 1) * size + 1, page * size))
                }
                "/offset" => {
                    let (offset, limit) = (number("offset", 0), number("limit", 5));
                    (200, format!("{{\"data\":{}}}", numbers(offset + 1, offset + limit)))
                }
                "/cursor" => {
                    let after = number("after", 0);
                    let next = if after + 2 >= 5 { "null".to_string() } else { format!("\"{}\"", after + 2) };
                    (200, format!("{{\"data\":{},\"meta\":{{\"next\":{next}}}}}", numbers(after + 1, after + 2)))
                }
                "/linked" => {
                    let page = number("page", 1);
                    if page * 2 < 5 {
                        link = format!("Link: <{server}/linked?page={}>; rel=\"next\"\r\n", page + 1);
                    }
                    (200, numbers((page - 1) * 2 + 1, page * 2))
                }
                "/broken" if number("page", 1) == 1 => (200, "[1,2]".to_string()),
                "/broken" => (500, "down".to_string()),
                "/endless" => (200, format!("[{}]", number("page", 1))),
                "/hello" => (200, "hello".to_string()),
                "/echo" => (200, format!("{method} {}", String::from_utf8_lossy(&body))),
                "/header" => (200, test),
                "/query" => (200, query.clone()),
                "/auth" => (200, if authorization.is_empty() { api_key.clone() } else { authorization.clone() }),
                _ => (404, "no such page".to_string()),
            };
            let reason = match status {
                200 => "OK",
                500 => "Internal Server Error",
                _ => "Not Found",
            };
            let mut stream = reader.into_inner();
            let reply = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\n{link}Content-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    address
}

fn method(word: &str) -> Method {
    match word {
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "PATCH" => Method::Patch,
        "DELETE" => Method::Delete,
        other => panic!("no method {other}"),
    }
}

#[test]
fn every_call_gives_the_reply_the_cases_name() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for (name, value) in cases["env"].as_object().unwrap() {
        // SAFETY: this test alone sets the variable, before it reads it.
        unsafe { std::env::set_var(name, value.as_str().unwrap()) };
    }
    let server = serve();
    for case in cases["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let url = case["url"].as_str().map(str::to_string).unwrap_or_else(|| format!("{server}{}", case["path"].as_str().unwrap()));
        let headers: Vec<(String, String)> =
            case["headers"].as_object().map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect()).unwrap_or_default();
        let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let reply = match case["base"].as_str() {
            Some(base) => {
                let query: Vec<(&str, &str)> = case["query"].as_array().map(|q| q.iter().map(|p| (p[0].as_str().unwrap(), p[1].as_str().unwrap())).collect()).unwrap_or_default();
                let secret = case["auth"]["secret"].as_str().unwrap_or_default();
                let auth = match case["auth"]["kind"].as_str() {
                    Some("bearer") => Auth::Bearer(secret),
                    Some("api-key") => Auth::ApiKey(secret),
                    Some("basic") => Auth::Basic(secret),
                    _ => Auth::None,
                };
                let base = base.replace("{server}", &server);
                let p = &case["paging"];
                if p.is_object() {
                    let kind = match p["kind"].as_str().unwrap() {
                        "page" => PagingKind::Page,
                        "offset" => PagingKind::Offset,
                        "cursor" => PagingKind::Cursor,
                        _ => PagingKind::Link,
                    };
                    let text = |key: &str| p[key].as_str().unwrap();
                    let paging = Paging { kind, param: text("param"), size_param: text("size_param"), size: p["size"].as_u64().unwrap() as u32, next: text("next"), items: text("items") };
                    http::pages_at(&base, case["path"].as_str().unwrap(), &query, &headers, auth, &paging)
                } else {
                http::call_at(method(case["method"].as_str().unwrap()), &base, case["path"].as_str().unwrap(), &query, case["body"].as_str(), &headers, auth)
                }
            }
            None => http::call(method(case["method"].as_str().unwrap()), &url, case["body"].as_str(), &headers),
        };
        assert_eq!(reply.status, case["status"].as_i64().unwrap(), "{name}: {reply:?}");
        assert_eq!(reply.ok, case["ok"].as_bool().unwrap(), "{name}: {reply:?}");
        if let Some(text) = case["text"].as_str() {
            assert_eq!(reply.text, text, "{name}");
        }
        if let Some(start) = case["text_starts"].as_str() {
            assert!(reply.text.starts_with(start), "{name}: {:?}", reply.text);
        }
    }
}

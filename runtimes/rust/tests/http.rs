//! HTTP calls (DESIGN 5.23) against the cases every runtime shares
//! (conformance/http.json), on a small server of the test's own.

#![cfg(feature = "http")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use clarity_runtime::http::{self, Method};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/http.json");

/// The routes of the conformance file, on a free port; gives its address.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut words = line.split_whitespace();
            let (method, path) = (words.next().unwrap_or_default().to_string(), words.next().unwrap_or_default().to_string());
            let (mut length, mut test) = (0, String::new());
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
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let (status, text) = match path.as_str() {
                "/hello" => (200, "hello".to_string()),
                "/echo" => (200, format!("{method} {}", String::from_utf8_lossy(&body))),
                "/header" => (200, test),
                _ => (404, "no such page".to_string()),
            };
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let mut stream = reader.into_inner();
            let reply = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
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
        "DELETE" => Method::Delete,
        other => panic!("no method {other}"),
    }
}

#[test]
fn every_call_gives_the_reply_the_cases_name() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let server = serve();
    for case in cases["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let url = case["url"].as_str().map(str::to_string).unwrap_or_else(|| format!("{server}{}", case["path"].as_str().unwrap()));
        let headers: Vec<(String, String)> =
            case["headers"].as_object().map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect()).unwrap_or_default();
        let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let reply = http::call(method(case["method"].as_str().unwrap()), &url, case["body"].as_str(), &headers);
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

//! HTTP calls from code (DESIGN 5.23): `http get url` and the others, giving
//! a reply that code reads. A call never fails: when no answer comes, the
//! reply has status 0 and says why, so the handler decides what that means.
//! Every runtime gives the same replies (conformance/http.json).

use std::time::Duration;

/// How long a call waits for its whole answer.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The kind of call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// What a web API answered: its status (0 when no answer came), whether it
/// is ok (200 to 299), and its text, or why no answer came.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
    pub status: i64,
    pub ok: bool,
    pub text: String,
}

impl Reply {
    fn none(why: String) -> Reply {
        Reply { status: 0, ok: false, text: why }
    }
}

/// The host a url names, for saying it cannot be reached: without the
/// scheme, a login, the port and the path.
fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    if host.starts_with('[') {
        return host.split_once(']').map_or(host, |(inner, _)| inner.trim_start_matches('['));
    }
    host.split(':').next().unwrap_or(host)
}

/// Calls `url` with `method`, sending `body` with post, put and patch and each of
/// `headers`, and gives the reply.
pub fn call(method: Method, url: &str, body: Option<&str>, headers: &[(&str, &str)]) -> Reply {
    fetch(method, url, body, headers).0
}

/// The address a `Link` header names `rel="next"`: `<url>; rel="next"`.
fn next_link(header: &str) -> Option<String> {
    header.split(',').find_map(|part| {
        let (address, params) = part.split_once(';')?;
        let next = params.split(';').any(|p| p.trim().replace(' ', "").eq_ignore_ascii_case("rel=\"next\"") || p.trim().eq_ignore_ascii_case("rel=next"));
        next.then(|| address.trim().trim_start_matches('<').trim_end_matches('>').to_string())
    })
}

/// A call as `call` makes it, with the address its `Link` header names next.
fn fetch(method: Method, url: &str, body: Option<&str>, headers: &[(&str, &str)]) -> (Reply, Option<String>) {
    let scheme = url.split_once("://").map(|(scheme, _)| scheme.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return (Reply::none(format!("only http and https addresses are called, not {url}")), None);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).http_status_as_error(false).build().into();
    let answer = match method {
        Method::Get | Method::Delete => {
            let mut request = if method == Method::Get { agent.get(url) } else { agent.delete(url) };
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.call()
        }
        Method::Post | Method::Put | Method::Patch => {
            let mut request = match method {
                Method::Post => agent.post(url),
                Method::Put => agent.put(url),
                _ => agent.patch(url),
            };
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.send(body.unwrap_or_default())
        }
    };
    let mut response = match answer {
        Ok(response) => response,
        Err(err) => return (Reply::none(format!("cannot reach {}: {err}", host_of(url))), None),
    };
    let status = i64::from(response.status().as_u16());
    let next = response.headers().get_all("link").iter().filter_map(|v| v.to_str().ok()).find_map(next_link);
    let reply = match response.body_mut().read_to_vec() {
        Ok(bytes) => Reply { status, ok: (200..300).contains(&status), text: String::from_utf8_lossy(&bytes).into_owned() },
        Err(err) => Reply::none(format!("cannot read the answer of {}: {err}", host_of(url))),
    };
    (reply, next)
}

/// How a web API data source signs its calls: the project file's `auth`
/// line, its secret read from where it says (`env:NAME`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth<'a> {
    None,
    Bearer(&'a str),
    ApiKey(&'a str),
    Basic(&'a str),
}

/// A secret where the project file says it is: `env:NAME` reads that
/// environment variable; empty when it is not set.
fn secret(location: &str) -> String {
    if let Some(name) = location.strip_prefix("env:") {
        return std::env::var(name).unwrap_or_default();
    }
    // A secret setting (DESIGN 5.27): `setting:<program>/<name>`, from the system's protected store.
    if let Some(at) = location.strip_prefix("setting:") {
        #[cfg(feature = "secrets")]
        if let Some((program, name)) = at.split_once('/') {
            return crate::secrets::get(program, name).unwrap_or_default();
        }
        let _ = at;
        return String::new();
    }
    location.to_string()
}

/// Text as a part of a url: letters, digits and `- . _ ~` as they are, every
/// other byte of its UTF-8 as `%XX`.
pub fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

/// `path` at `base`, one slash between them; a whole url stays as it is.
fn joined(base: &str, path: &str) -> String {
    if base.is_empty() || path.contains("://") {
        return path.to_string();
    }
    format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'))
}

/// Calls `path` of the web API at `base` (a data source's address) with the
/// query parameters, encoded and in order, and the source's auth, as
/// `http get "/orders" from shop query "status": s` does.
pub fn call_at(method: Method, base: &str, path: &str, query: &[(&str, &str)], body: Option<&str>, headers: &[(&str, &str)], auth: Auth) -> Reply {
    fetch_at(method, base, path, query, body, headers, auth).0
}

/// How a web API hands out a long list (DESIGN 5.23, paging): the project
/// file's `paging` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingKind {
    Page,
    Offset,
    Cursor,
    Link,
}

/// A `paging` line: the query parameters it sends, the size of a page,
/// where the next cursor is and where a page's items are (empty when a page
/// is a list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paging<'a> {
    pub kind: PagingKind,
    pub param: &'a str,
    pub size_param: &'a str,
    pub size: u32,
    pub next: &'a str,
    pub items: &'a str,
}

/// At most this many pages are read.
pub const MOST_PAGES: usize = 1000;

/// The value at a dotted path of `value`: `meta.next`; the value itself for "".
fn at_path<'v>(value: &'v serde_json::Value, path: &str) -> Option<&'v serde_json::Value> {
    if path.is_empty() {
        return Some(value);
    }
    path.split('.').try_fold(value, |v, step| v.get(step))
}

/// Gets `path` of the web API at `base` page after page, as `paging` says,
/// as `http get "/orders" from shop every page` does: one reply whose text
/// is one JSON list of the items of every page (conformance/http.json).
pub fn pages_at(base: &str, path: &str, query: &[(&str, &str)], headers: &[(&str, &str)], auth: Auth, paging: &Paging) -> Reply {
    let mut items: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut link: Option<String> = None;
    for n in 0..MOST_PAGES {
        let mut sent: Vec<(String, String)> = query.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let size = paging.size as usize;
        match paging.kind {
            PagingKind::Page => sent.extend([(paging.param.to_string(), (n + 1).to_string()), (paging.size_param.to_string(), size.to_string())]),
            PagingKind::Offset => sent.extend([(paging.param.to_string(), (n * size).to_string()), (paging.size_param.to_string(), size.to_string())]),
            PagingKind::Cursor => sent.extend(cursor.take().map(|c| (paging.param.to_string(), c))),
            PagingKind::Link => {}
        }
        let sent: Vec<(&str, &str)> = sent.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        // A next link is a whole address, with its own query.
        let (reply, next_link) = match link.take() {
            Some(address) => fetch_at(Method::Get, "", &address, &[], None, headers, auth),
            None => fetch_at(Method::Get, base, path, &sent, None, headers, auth),
        };
        if !reply.ok {
            return reply;
        }
        let page: Option<serde_json::Value> = serde_json::from_str(&reply.text).ok();
        let Some(list) = page.as_ref().and_then(|p| at_path(p, paging.items)).and_then(|l| l.as_array()) else {
            let place = if paging.items.is_empty() { "the top" } else { paging.items };
            return Reply::none(format!("page {} of {path} has no list at {place}", n + 1));
        };
        items.extend(list.iter().map(|item| item.to_string()));
        let done = match paging.kind {
            PagingKind::Page | PagingKind::Offset => list.len() < size,
            PagingKind::Cursor => {
                cursor = match page.as_ref().and_then(|p| at_path(p, paging.next)) {
                    Some(serde_json::Value::String(text)) if !text.is_empty() => Some(text.clone()),
                    Some(serde_json::Value::Number(number)) => Some(number.to_string()),
                    _ => None,
                };
                cursor.is_none()
            }
            PagingKind::Link => {
                link = next_link;
                link.is_none()
            }
        };
        if done {
            return Reply { status: reply.status, ok: true, text: format!("[{}]", items.join(",")) };
        }
    }
    Reply::none(format!("stopped after {MOST_PAGES} pages of {path}"))
}

/// A call as `call_at` makes it, with the address its `Link` header names next.
fn fetch_at(method: Method, base: &str, path: &str, query: &[(&str, &str)], body: Option<&str>, headers: &[(&str, &str)], auth: Auth) -> (Reply, Option<String>) {
    let mut url = joined(base, path);
    for (i, (name, value)) in query.iter().enumerate() {
        let joint = if i == 0 && !url.contains('?') { '?' } else { '&' };
        url.push_str(&format!("{joint}{}={}", encode(name), encode(value)));
    }
    let signed = match auth {
        Auth::None => None,
        Auth::Bearer(at) => Some(("Authorization", format!("Bearer {}", secret(at)))),
        Auth::ApiKey(at) => Some(("X-Api-Key", secret(at))),
        Auth::Basic(at) => {
            use base64::Engine as _;
            Some(("Authorization", format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(secret(at)))))
        }
    };
    let mut all: Vec<(&str, &str)> = headers.to_vec();
    if let Some((name, value)) = &signed {
        all.push((name, value.as_str()));
    }
    fetch(method, &url, body, &all)
}

#[cfg(test)]
mod tests {
    use super::host_of;

    #[test]
    fn the_host_is_named_without_the_rest_of_the_url() {
        assert_eq!(host_of("https://api.example.com/orders?x=1"), "api.example.com");
        assert_eq!(host_of("http://127.0.0.1:1/"), "127.0.0.1");
        assert_eq!(host_of("https://me:secret@example.com:8443"), "example.com");
        assert_eq!(host_of("http://[::1]:8080/x"), "::1");
    }
}

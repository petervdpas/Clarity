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

/// Calls `url` with `method`, sending `body` with post and put and each of
/// `headers`, and gives the reply.
pub fn call(method: Method, url: &str, body: Option<&str>, headers: &[(&str, &str)]) -> Reply {
    let scheme = url.split_once("://").map(|(scheme, _)| scheme.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return Reply::none(format!("only http and https addresses are called, not {url}"));
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
        Method::Post | Method::Put => {
            let mut request = if method == Method::Post { agent.post(url) } else { agent.put(url) };
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.send(body.unwrap_or_default())
        }
    };
    let mut response = match answer {
        Ok(response) => response,
        Err(err) => return Reply::none(format!("cannot reach {}: {err}", host_of(url))),
    };
    let status = i64::from(response.status().as_u16());
    match response.body_mut().read_to_vec() {
        Ok(bytes) => Reply { status, ok: (200..300).contains(&status), text: String::from_utf8_lossy(&bytes).into_owned() },
        Err(err) => Reply::none(format!("cannot read the answer of {}: {err}", host_of(url))),
    }
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

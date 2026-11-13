//! The running app and `<app> mcp` (docs/ledger/app-mcp.md): the app listens
//! on loopback TCP on a free port and writes running.json beside its
//! settings (`discovery`); every connection first gives the file's token,
//! then speaks the service's lines. `<app> mcp` reads the file, connects and
//! forwards standard input and output to the app; with no app answering it
//! serves as the closed app does. Two copies of one app: the first one
//! listens, the second finds it answering and does not.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use crate::discovery::{self, Running};
use crate::server::Lines;

/// How long `<app> mcp` waits for the app to take the connection.
const CONNECT_WAIT: Duration = Duration::from_secs(2);

/// One connection's lines.
pub struct SocketLines {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl SocketLines {
    fn of(stream: TcpStream) -> std::io::Result<SocketLines> {
        Ok(SocketLines { reader: BufReader::new(stream.try_clone()?), writer: stream })
    }
}

impl Lines for SocketLines {
    fn read(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_string()),
        }
    }

    fn write(&mut self, line: &str) {
        let _ = writeln!(self.writer, "{line}");
        let _ = self.writer.flush();
    }
}

/// Closes one connection from the app's side: its agent's session ends, and its grants with it.
pub type Closer = Arc<dyn Fn() + Send + Sync>;

/// The app listening; it stops and removes its running.json when dropped.
pub struct Listening {
    file: PathBuf,
    address: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl Listening {
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the waiting accept, which then sees the stop.
        let _ = TcpStream::connect_timeout(&self.address, CONNECT_WAIT);
        discovery::remove(&self.file);
    }
}

/// Whether two tokens are the same, looking at every byte.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// Listens for `app` and writes its running.json at `file`; each connection
/// that gives the token is served by `serve` on a thread of its own, with
/// what closes it. Fails when another copy of the app answers at `file` already.
pub fn listen(app: &str, file: &Path, serve: impl Fn(&mut dyn Lines, Closer) + Send + Sync + 'static) -> Result<Listening, String> {
    listen_limited(app, file, usize::MAX, "", |_| {}, serve)
}

/// Answers every request on `lines` with the error `full`, until it ends:
/// the agent reads why it is not served. `turned_away` hears of it once, at
/// its first message, with the agent's name when it gives one.
fn refuse_all(lines: &mut dyn Lines, full: &str, turned_away: &dyn Fn(Option<String>)) {
    let mut told = false;
    while let Some(line) = lines.read() {
        let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
        if !told {
            told = true;
            turned_away(message["params"]["clientInfo"]["name"].as_str().map(str::to_string));
        }
        // A notification has no id and gets no answer.
        if let Some(id) = message.get("id").filter(|id| !id.is_null()) {
            lines.write(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": full } }).to_string());
        }
    }
}

/// As `listen`, serving at most `at_most` agents at once (the project file's
/// `agents at most`); one more is refused with `full` on every request, and
/// `turned_away` hears of it, so the open app can say so.
pub fn listen_limited(
    app: &str,
    file: &Path,
    at_most: usize,
    full: &str,
    turned_away: impl Fn(Option<String>) + Send + Sync + 'static,
    serve: impl Fn(&mut dyn Lines, Closer) + Send + Sync + 'static,
) -> Result<Listening, String> {
    if connect(file).is_some() {
        return Err(format!("{app} is running already and answers agents"));
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("cannot listen: {e}"))?;
    let address = listener.local_addr().map_err(|e| format!("cannot listen: {e}"))?;
    let running = Running::new(app, &address.to_string())?;
    discovery::write(file, &running)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let serve = Arc::new(serve);
    let token = running.token;
    let serving = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let full = full.to_string();
    let turned_away = Arc::new(turned_away);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if stopped.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = stream else { continue };
            let (serve, token, serving, full, turned_away) = (serve.clone(), token.clone(), serving.clone(), full.clone(), turned_away.clone());
            std::thread::spawn(move || {
                let Ok(mut lines) = SocketLines::of(stream) else { return };
                let given = lines.read().and_then(|l| serde_json::from_str::<Value>(&l).ok()).and_then(|v| v["token"].as_str().map(str::to_string));
                if given.is_some_and(|g| same(&g, &token)) {
                    // A place among the agents served, or the refusal.
                    if serving.try_update(Ordering::SeqCst, Ordering::SeqCst, |n| (n < at_most).then_some(n + 1)).is_err() {
                        refuse_all(&mut lines, &full, turned_away.as_ref());
                        let _ = lines.writer.shutdown(Shutdown::Both);
                        return;
                    }
                    let closer: Closer = match lines.writer.try_clone() {
                        Ok(stream) => Arc::new(move || {
                            let _ = stream.shutdown(Shutdown::Both);
                        }),
                        Err(_) => Arc::new(|| {}),
                    };
                    serve(&mut lines, closer);
                    serving.fetch_sub(1, Ordering::SeqCst);
                }
                let _ = lines.writer.shutdown(Shutdown::Both);
            });
        }
    });
    Ok(Listening { file: file.to_path_buf(), address, stop })
}

/// A connection to the app that running.json at `file` names, the token
/// given; none when there is no file or nothing answers there.
pub fn connect(file: &Path) -> Option<SocketLines> {
    let running = discovery::read(file)?;
    let address: SocketAddr = running.address.parse().ok()?;
    let stream = TcpStream::connect_timeout(&address, CONNECT_WAIT).ok()?;
    let mut lines = SocketLines::of(stream).ok()?;
    lines.write(&json!({ "token": running.token }).to_string());
    Some(lines)
}

/// Forwards standard input and output to the running app at `file` until
/// either ends: true when an app answered, false when none does (serve as
/// the closed app then).
pub fn forward(file: &Path) -> bool {
    let Some(lines) = connect(file) else { return false };
    let SocketLines { mut reader, writer } = lines;
    let mut to_app = writer;
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        let mut buffer = [0u8; 8192];
        loop {
            match input.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if to_app.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = to_app.shutdown(Shutdown::Write);
    });
    let mut out = std::io::stdout();
    let mut line = String::new();
    while matches!(reader.read_line(&mut line), Ok(n) if n > 0) {
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
        line.clear();
    }
    true
}

//! The desktop portal's file dialog (org.freedesktop.portal.FileChooser)
//! over the session's D-Bus, on Linux: the dialog of the desktop the person
//! runs, in a sandbox too. D-Bus is spoken here with no crate of its own:
//! the session bus's socket, the EXTERNAL login, and the few messages the
//! dialog needs (Hello, AddMatch, OpenFile or SaveFile, and the Response
//! signal that brings what the person chose).

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;

use super::{Ask, Kind};

/// A value of a D-Bus message, as far as the dialog needs them.
#[derive(Debug, Clone, PartialEq)]
pub enum Dbus {
    Byte(u8),
    Bool(bool),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F64(f64),
    U16(u16),
    I16(i16),
    Str(String),
    Path(String),
    Sig(String),
    Array(Vec<Dbus>),
    Struct(Vec<Dbus>),
    Entry(Box<Dbus>, Box<Dbus>),
    Variant(String, Box<Dbus>),
}

/// Writes a message's values with D-Bus's alignment, little-endian.
#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    fn pad(&mut self, to: usize) {
        while !self.bytes.len().is_multiple_of(to) {
            self.bytes.push(0);
        }
    }

    fn u32(&mut self, v: u32) {
        self.pad(4);
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    fn text(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes.extend_from_slice(s.as_bytes());
        self.bytes.push(0);
    }

    fn sig(&mut self, s: &str) {
        self.bytes.push(s.len() as u8);
        self.bytes.extend_from_slice(s.as_bytes());
        self.bytes.push(0);
    }

    /// Writes `value` of the type `sig` names.
    pub fn value(&mut self, sig: &str, value: &Dbus) {
        let first = sig.as_bytes()[0];
        match (first, value) {
            (b'y', Dbus::Byte(v)) => self.bytes.push(*v),
            (b'b', Dbus::Bool(v)) => self.u32(*v as u32),
            (b'u', Dbus::U32(v)) => self.u32(*v),
            (b'i', Dbus::I32(v)) => self.u32(*v as u32),
            (b's', Dbus::Str(v)) | (b'o', Dbus::Path(v)) => self.text(v),
            (b'g', Dbus::Sig(v)) => self.sig(v),
            (b'v', Dbus::Variant(inner, v)) => {
                self.sig(inner);
                self.value(inner, v);
            }
            (b'a', Dbus::Array(items)) => {
                let element = &sig[1..sig.len().min(1 + element_len(&sig[1..]))];
                self.u32(0);
                let length_at = self.bytes.len() - 4;
                self.pad(alignment(element.as_bytes()[0]));
                let start = self.bytes.len();
                for item in items {
                    self.value(element, item);
                }
                let length = (self.bytes.len() - start) as u32;
                self.bytes[length_at..length_at + 4].copy_from_slice(&length.to_le_bytes());
            }
            (b'(', Dbus::Struct(fields)) => {
                self.pad(8);
                let mut at = 1;
                for field in fields {
                    let len = element_len(&sig[at..]);
                    self.value(&sig[at..at + len], field);
                    at += len;
                }
            }
            (b'{', Dbus::Entry(key, value)) => {
                self.pad(8);
                let key_len = element_len(&sig[1..]);
                self.value(&sig[1..1 + key_len], key);
                let value_len = element_len(&sig[1 + key_len..]);
                self.value(&sig[1 + key_len..1 + key_len + value_len], value);
            }
            _ => {}
        }
    }
}

/// How a type is aligned in a message.
fn alignment(sig: u8) -> usize {
    match sig {
        b'y' | b'g' | b'v' => 1,
        b'n' | b'q' => 2,
        b'b' | b'i' | b'u' | b's' | b'o' | b'a' => 4,
        _ => 8,
    }
}

/// How long the first complete type of a signature is: `a{sv}` in `a{sv}s` is 5.
pub fn element_len(sig: &str) -> usize {
    let bytes = sig.as_bytes();
    match bytes.first() {
        Some(b'a') => 1 + element_len(&sig[1..]),
        Some(b'(') | Some(b'{') => {
            let mut depth = 0;
            for (i, b) in bytes.iter().enumerate() {
                match b {
                    b'(' | b'{' => depth += 1,
                    b')' | b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
            }
            bytes.len()
        }
        Some(_) => 1,
        None => 0,
    }
}

/// Reads a message's values by their signature.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8], at: usize) -> Reader<'a> {
        Reader { bytes, at }
    }

    fn pad(&mut self, to: usize) {
        while !self.at.is_multiple_of(to) {
            self.at += 1;
        }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let taken = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(taken)
    }

    fn u32(&mut self) -> Option<u32> {
        self.pad(4);
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn text(&mut self) -> Option<String> {
        let len = self.u32()? as usize;
        let text = String::from_utf8_lossy(self.take(len)?).into_owned();
        self.at += 1;
        Some(text)
    }

    fn sig(&mut self) -> Option<String> {
        let len = *self.take(1)?.first()? as usize;
        let text = String::from_utf8_lossy(self.take(len)?).into_owned();
        self.at += 1;
        Some(text)
    }

    /// Reads one value of the type `sig` names.
    pub fn value(&mut self, sig: &str) -> Option<Dbus> {
        let first = *sig.as_bytes().first()?;
        Some(match first {
            b'y' => Dbus::Byte(*self.take(1)?.first()?),
            b'b' => Dbus::Bool(self.u32()? != 0),
            b'u' => Dbus::U32(self.u32()?),
            b'i' => Dbus::I32(self.u32()? as i32),
            b'n' | b'q' => {
                self.pad(2);
                let v = u16::from_le_bytes(self.take(2)?.try_into().ok()?);
                if first == b'n' { Dbus::I16(v as i16) } else { Dbus::U16(v) }
            }
            b'x' | b't' | b'd' => {
                self.pad(8);
                let raw: [u8; 8] = self.take(8)?.try_into().ok()?;
                match first {
                    b'x' => Dbus::I64(i64::from_le_bytes(raw)),
                    b't' => Dbus::U64(u64::from_le_bytes(raw)),
                    _ => Dbus::F64(f64::from_le_bytes(raw)),
                }
            }
            b'h' => Dbus::U32(self.u32()?),
            b's' => Dbus::Str(self.text()?),
            b'o' => Dbus::Path(self.text()?),
            b'g' => Dbus::Sig(self.sig()?),
            b'v' => {
                let inner = self.sig()?;
                let value = self.value(&inner)?;
                Dbus::Variant(inner, Box::new(value))
            }
            b'a' => {
                let element = sig[1..1 + element_len(&sig[1..])].to_string();
                let len = self.u32()? as usize;
                self.pad(alignment(*element.as_bytes().first()?));
                let end = self.at + len;
                let mut items = Vec::new();
                while self.at < end {
                    items.push(self.value(&element)?);
                }
                Dbus::Array(items)
            }
            b'(' => {
                self.pad(8);
                let inner = &sig[1..element_len(sig) - 1];
                let mut fields = Vec::new();
                let mut at = 0;
                while at < inner.len() {
                    let len = element_len(&inner[at..]);
                    fields.push(self.value(&inner[at..at + len])?);
                    at += len;
                }
                Dbus::Struct(fields)
            }
            b'{' => {
                self.pad(8);
                let key_len = element_len(&sig[1..]);
                let key = self.value(&sig[1..1 + key_len])?;
                let value_len = element_len(&sig[1 + key_len..]);
                let value = self.value(&sig[1 + key_len..1 + key_len + value_len])?;
                Dbus::Entry(Box::new(key), Box::new(value))
            }
            _ => return None,
        })
    }
}

/// A message as it came: its type, serial, the serial it answers, its path, member and body.
#[derive(Debug, Default)]
pub struct Message {
    pub kind: u8,
    pub reply_to: Option<u32>,
    pub path: Option<String>,
    pub member: Option<String>,
    pub signature: String,
    pub body: Vec<u8>,
}

/// A whole method call: its header and body, serial `serial`.
pub fn method_call(serial: u32, destination: &str, path: &str, interface: &str, member: &str, signature: &str, args: &[Dbus]) -> Vec<u8> {
    let mut body = Writer::default();
    let mut at = 0;
    for arg in args {
        let len = element_len(&signature[at..]);
        body.value(&signature[at..at + len], arg);
        at += len;
    }
    let mut fields = vec![
        Dbus::Struct(vec![Dbus::Byte(1), Dbus::Variant("o".into(), Box::new(Dbus::Path(path.into())))]),
        Dbus::Struct(vec![Dbus::Byte(2), Dbus::Variant("s".into(), Box::new(Dbus::Str(interface.into())))]),
        Dbus::Struct(vec![Dbus::Byte(3), Dbus::Variant("s".into(), Box::new(Dbus::Str(member.into())))]),
        Dbus::Struct(vec![Dbus::Byte(6), Dbus::Variant("s".into(), Box::new(Dbus::Str(destination.into())))]),
    ];
    if !signature.is_empty() {
        fields.push(Dbus::Struct(vec![Dbus::Byte(8), Dbus::Variant("g".into(), Box::new(Dbus::Sig(signature.into())))]));
    }
    let mut head = Writer::default();
    head.bytes.extend_from_slice(&[b'l', 1, 0, 1]);
    head.bytes.extend_from_slice(&(body.bytes.len() as u32).to_le_bytes());
    head.bytes.extend_from_slice(&serial.to_le_bytes());
    head.value("a(yv)", &Dbus::Array(fields));
    head.pad(8);
    head.bytes.extend_from_slice(&body.bytes);
    head.bytes
}

/// Reads the next whole message from the bus.
fn read_message(stream: &mut impl Read) -> Option<Message> {
    let mut fixed = [0u8; 16];
    stream.read_exact(&mut fixed).ok()?;
    if fixed[0] != b'l' {
        return None;
    }
    let body_len = u32::from_le_bytes(fixed[4..8].try_into().ok()?) as usize;
    let fields_len = u32::from_le_bytes(fixed[12..16].try_into().ok()?) as usize;
    let header_end = (16 + fields_len).div_ceil(8) * 8;
    let mut rest = vec![0u8; header_end - 16 + body_len];
    stream.read_exact(&mut rest).ok()?;
    let mut all = fixed.to_vec();
    all.extend_from_slice(&rest);
    let mut message = Message { kind: fixed[1], ..Message::default() };
    let mut reader = Reader::new(&all, 12);
    if let Some(Dbus::Array(fields)) = reader.value("a(yv)") {
        for field in fields {
            let Dbus::Struct(parts) = field else { continue };
            let (Some(Dbus::Byte(code)), Some(Dbus::Variant(_, value))) = (parts.first(), parts.get(1)) else { continue };
            match (code, value.as_ref()) {
                (1, Dbus::Path(p)) => message.path = Some(p.clone()),
                (3, Dbus::Str(m)) => message.member = Some(m.clone()),
                (5, Dbus::U32(s)) => message.reply_to = Some(*s),
                (8, Dbus::Sig(s)) => message.signature = s.clone(),
                _ => {}
            }
        }
    }
    message.body = all[header_end..].to_vec();
    Some(message)
}

/// The session bus's socket, by the address the session gives.
fn connect() -> Result<UnixStream, String> {
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok().filter(|a| !a.is_empty()).or_else(|| std::env::var("XDG_RUNTIME_DIR").ok().map(|d| format!("unix:path={d}/bus"))).ok_or("there is no D-Bus session")?;
    for one in address.split(';') {
        let Some(rest) = one.strip_prefix("unix:") else { continue };
        for pair in rest.split(',') {
            if let Some(path) = pair.strip_prefix("path=") {
                return UnixStream::connect(path).map_err(|e| e.to_string());
            }
            #[cfg(target_os = "linux")]
            if let Some(name) = pair.strip_prefix("abstract=") {
                use std::os::linux::net::SocketAddrExt;
                let at = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes()).map_err(|e| e.to_string())?;
                return UnixStream::connect_addr(&at).map_err(|e| e.to_string());
            }
        }
    }
    Err(format!("no D-Bus address to reach: {address}"))
}

/// Logs in on the bus as the person running the program (EXTERNAL, by their user id).
fn login(stream: &mut UnixStream) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).map_err(|e| e.to_string())?;
    let hex: String = uid.to_string().bytes().map(|b| format!("{b:02x}")).collect();
    stream.write_all(format!("\0AUTH EXTERNAL {hex}\r\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(&mut *stream).read_line(&mut line).map_err(|e| e.to_string())?;
    if !line.starts_with("OK") {
        return Err(format!("the bus refused the login: {}", line.trim()));
    }
    stream.write_all(b"BEGIN\r\n").map_err(|e| e.to_string())
}

/// The options of the dialog: its token, modal, a folder, the name to save, the kinds of file.
fn options(ask: &Ask, token: &str) -> Dbus {
    let entry = |key: &str, sig: &str, value: Dbus| Dbus::Entry(Box::new(Dbus::Str(key.into())), Box::new(Dbus::Variant(sig.into(), Box::new(value))));
    let mut entries = vec![entry("handle_token", "s", Dbus::Str(token.into())), entry("modal", "b", Dbus::Bool(true))];
    match ask.kind {
        Kind::Folder => entries.push(entry("directory", "b", Dbus::Bool(true))),
        Kind::Save => entries.push(entry("current_name", "s", Dbus::Str(ask.name.clone().unwrap_or_default()))),
        Kind::Open => {
            if let Some(filter) = &ask.filter {
                // filters: a(sa(us)), each a name and its patterns (0 is a glob).
                let patterns = filter.patterns.iter().map(|p| Dbus::Struct(vec![Dbus::U32(0), Dbus::Str(p.clone())])).collect();
                entries.push(entry("filters", "a(sa(us))", Dbus::Array(vec![Dbus::Struct(vec![Dbus::Str(filter.name.clone()), Dbus::Array(patterns)])])));
            }
        }
    }
    Dbus::Array(entries)
}

/// The first uri of a Response's results, as a local path; none for none.
fn chosen_path(results: &Dbus) -> Option<String> {
    let Dbus::Array(entries) = results else { return None };
    for entry in entries {
        let Dbus::Entry(key, value) = entry else { continue };
        if **key != Dbus::Str("uris".into()) {
            continue;
        }
        let Dbus::Variant(_, uris) = value.as_ref() else { continue };
        let Dbus::Array(uris) = uris.as_ref() else { continue };
        if let Some(Dbus::Str(uri)) = uris.first() {
            return uri.strip_prefix("file://").map(unescape);
        }
    }
    None
}

/// A uri's path with its %xx escapes read back.
pub fn unescape(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&path[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The path the person chose in the portal's dialog, or none when they chose
/// none; an error when there is no bus or no portal on it (then the next
/// way is tried).
pub fn choose(ask: &Ask) -> Result<Option<String>, String> {
    let mut stream = connect()?;
    login(&mut stream)?;
    let bus = "org.freedesktop.DBus";
    stream.write_all(&method_call(1, bus, "/org/freedesktop/DBus", bus, "Hello", "", &[])).map_err(|e| e.to_string())?;
    let rule = "type='signal',interface='org.freedesktop.portal.Request',member='Response'";
    stream.write_all(&method_call(2, bus, "/org/freedesktop/DBus", bus, "AddMatch", "s", &[Dbus::Str(rule.into())])).map_err(|e| e.to_string())?;
    let token = format!("clarity{}", std::process::id());
    let member = if ask.kind == Kind::Save { "SaveFile" } else { "OpenFile" };
    let args = [Dbus::Str(String::new()), Dbus::Str(ask.title.clone()), options(ask, &token)];
    stream
        .write_all(&method_call(3, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.FileChooser", member, "ssa{sv}", &args))
        .map_err(|e| e.to_string())?;
    // Every Response is heard and kept by its request until the call says which is this dialog's.
    let mut handle: Option<String> = None;
    let mut early: Vec<(String, u32, Option<String>)> = Vec::new();
    loop {
        let message = read_message(&mut stream).ok_or("the bus closed")?;
        match message.kind {
            2 | 3 if message.reply_to == Some(3) => {
                if message.kind == 3 {
                    return Err("the desktop portal has no file chooser".to_string());
                }
                let path = match Reader::new(&message.body, 0).value("o") {
                    Some(Dbus::Path(p)) => p,
                    _ => return Err("the portal answered no request".to_string()),
                };
                if let Some((_, code, chosen)) = early.iter().find(|(p, _, _)| *p == path) {
                    return Ok(if *code == 0 { chosen.clone() } else { None });
                }
                handle = Some(path);
            }
            4 if message.member.as_deref() == Some("Response") => {
                let mut reader = Reader::new(&message.body, 0);
                let code = match reader.value("u") {
                    Some(Dbus::U32(c)) => c,
                    _ => continue,
                };
                let chosen = reader.value("a{sv}").as_ref().and_then(chosen_path);
                let path = message.path.unwrap_or_default();
                // 0 is a choice, 1 is Cancel; 2 is a dialog that ended another way, also no choice.
                if handle.as_deref() == Some(path.as_str()) {
                    return Ok(if code == 0 { chosen } else { None });
                }
                early.push((path, code, chosen));
            }
            _ => {}
        }
    }
}

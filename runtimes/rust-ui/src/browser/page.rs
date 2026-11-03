//! What only the page can do, asked by telling it: the worker the library
//! runs in has no clipboard, no speakers and no downloads, so each of these
//! sends the page a message (`Tell`, JSON) and the page does it. What the
//! page tells back (pasted text, a sound that ended) comes in through the
//! host.
//!
//! The messages, by their `tell`:
//! - `title` {title}: the tab's title.
//! - `cursor` {cursor}: the pointer's CSS shape over the canvas.
//! - `caret` {at: [x, y, width, height] | null}: where an input method writes, in CSS pixels of the canvas.
//! - `copy` {text}: the text goes on the system's clipboard.
//! - `open` {address}: a link opens in a new tab.
//! - `download` {name, data}: a file to save (base64), downloaded under its name.
//! - `show` {name, data, type}: a file opened in a tab of its own, as its media type; only
//!   kinds that run nothing in the page's origin (`shown_type`), others are a `download`.
//! - `play` {id, data, volume, repeat}: a sound (WAV, base64) plays; `stop` {id} stops it.
//! - `choices` {choices}: the person's choices (language, theme, columns), all of them, for the page to keep.
//! - `ended` {quit}: the program's windows closed.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use serde_json::{Value, json};

use crate::files::{ChosenFile, FileFilter, FilePicker};
use crate::overlay::Clipboard;
use crate::parts::sound::{Playing, SoundOutput};

/// What sends the page a message.
pub type Tell = Arc<dyn Fn(Value) + Send + Sync>;

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The clipboard as the page has it: what was copied goes to the page, and
/// what the page last pasted (or this screen copied) is what a paste reads.
pub struct PageClipboard {
    tell: Tell,
    known: Rc<RefCell<Option<String>>>,
}

impl PageClipboard {
    /// The clipboard over the page; `known` is what the page told it holds.
    pub fn new(tell: Tell, known: Rc<RefCell<Option<String>>>) -> PageClipboard {
        PageClipboard { tell, known }
    }
}

impl Clipboard for PageClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.known.borrow().clone()
    }

    fn set_text(&mut self, text: &str) {
        *self.known.borrow_mut() = Some(text.to_string());
        (self.tell)(json!({ "tell": "copy", "text": text }));
    }
}

/// The sounds the page plays, by number, and those it told ended.
#[derive(Default)]
pub struct Sounds {
    next: u64,
    ended: BTreeSet<u64>,
}

/// Sound through the page's Web Audio.
pub struct PageSound {
    tell: Tell,
    sounds: Rc<RefCell<Sounds>>,
}

impl PageSound {
    pub fn new(tell: Tell, sounds: Rc<RefCell<Sounds>>) -> PageSound {
        PageSound { tell, sounds }
    }
}

struct PagePlaying {
    id: u64,
    tell: Tell,
    sounds: Rc<RefCell<Sounds>>,
    stopped: bool,
}

impl Playing for PagePlaying {
    fn stop(&mut self) {
        if !self.stopped {
            self.stopped = true;
            (self.tell)(json!({ "tell": "stop", "id": self.id }));
        }
    }

    fn ended(&self) -> bool {
        self.stopped || self.sounds.borrow().ended.contains(&self.id)
    }
}

impl SoundOutput for PageSound {
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing> {
        let id = {
            let mut sounds = self.sounds.borrow_mut();
            sounds.next += 1;
            sounds.next
        };
        (self.tell)(json!({ "tell": "play", "id": id, "data": base64(&file), "volume": volume, "repeat": repeat }));
        Box::new(PagePlaying { id, tell: self.tell.clone(), sounds: self.sounds.clone(), stopped: false })
    }
}

impl Sounds {
    /// The page told sound `id` played to its end.
    pub fn ended(&mut self, id: u64) {
        self.ended.insert(id);
    }
}

/// The media type a file named `name` shows in a tab as: only kinds that
/// run nothing (pictures, PDF, plain text, sound, video). HTML, SVG, XML
/// and anything unknown could run script in the page's origin: none, and
/// such a file is downloaded instead.
pub fn shown_type(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    Some(match extension.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "txt" | "log" | "md" | "csv" | "json" => "text/plain; charset=utf-8",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => return None,
    })
}

/// Files through the page: a saved file is a download, a shown one a tab
/// of its own. A browser chooses no folder (one is typed there). Choosing a
/// file to open needs the page's file input, which answers later than a
/// picker may wait: it chooses none here, and the file the page gives is
/// taken by the next open (`given`).
pub struct PageFiles {
    tell: Tell,
    given: Mutex<Option<ChosenFile>>,
}

impl PageFiles {
    pub fn new(tell: Tell) -> PageFiles {
        PageFiles { tell, given: Mutex::new(None) }
    }

    /// The file the person chose in the page, for the next open.
    pub fn give(&self, file: ChosenFile) {
        if let Ok(mut given) = self.given.lock() {
            *given = Some(file);
        }
    }
}

impl FilePicker for PageFiles {
    fn picks_folders(&self) -> bool {
        false
    }

    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile> {
        if let Some(file) = self.given.lock().ok().and_then(|mut g| g.take()) {
            return Some(file);
        }
        let accept: Vec<String> = filter.map(|f| f.patterns.iter().map(|p| p.trim_start_matches('*').to_string()).collect()).unwrap_or_default();
        (self.tell)(json!({ "tell": "pick", "title": title, "accept": accept }));
        None
    }

    fn save_file(&self, _title: &str, name: &str, bytes: &[u8]) -> Option<String> {
        let name = crate::files::safe_name(name);
        (self.tell)(json!({ "tell": "download", "name": name, "data": base64(bytes) }));
        Some(name)
    }

    fn choose_folder(&self, _title: &str) -> Option<String> {
        None
    }

    fn show(&self, name: &str, bytes: &[u8]) {
        let name = crate::files::safe_name(name);
        match shown_type(&name) {
            Some(kind) => (self.tell)(json!({ "tell": "show", "name": name, "data": base64(bytes), "type": kind })),
            None => (self.tell)(json!({ "tell": "download", "name": name, "data": base64(bytes) })),
        }
    }
}

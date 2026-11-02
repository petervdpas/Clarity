//! The every-part project used as a person uses it on `stack rust desktop`
//! (Clarity.UI), with no display: the generated program runs with
//! CLARITY_HEADLESS set (clarity_ui::headless), draws its screen on Skia's
//! raster surface, and takes each step on its standard input. The steps are
//! those of tests/live/drive.mjs, the same records inserted first and the
//! same checks on the screen and the database: clicks on parts and on the
//! buttons of dialogs, wizards, tabs and menus by what they say, keys
//! (Tab, Enter, Ctrl+Z, Ctrl+Y, Alt+arrows), typed text and pointer drags.
//! A part's value is read from the window's data at the path its markup
//! binds (the generated windows/*.cxaml), as the part shows it; typed text
//! is taken on Tab or Enter, as on the web.
//!
//! The rows of lists, tables and trees, a tree row's toggle, a table's
//! headers and page buttons are found where the screen shows them (the
//! read's `rows`). A shape on a canvas is found from the record it is drawn
//! for (the board's objects), or from where it was just drawn.
//!
//! Not driven here: the web's checks of how a page was built (drawn from
//! markup, which a Clarity.UI window always is) and the browser's console;
//! what the program writes on its standard error shows in the test's output
//! instead. Each step says `ok` or `FAIL` with what differed and a shot of
//! the screen, and the next step runs.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long one step may take before the program counts as stuck.
const STEP_TIME: Duration = Duration::from_secs(60);

/// A step's outcome: what differed when it failed.
type Done = Result<(), String>;

/// The program driven, and what the run found.
pub struct Driver {
    child: Child,
    input: ChildStdin,
    answers: Receiver<String>,
    folder: PathBuf,
    database: PathBuf,
    /// The bound path of each part, by window and part name.
    binds: BTreeMap<String, BTreeMap<String, String>>,
    failed: Vec<String>,
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The value of a markup attribute in one element's text (`name="x"`).
fn attribute(element: &str, name: &str) -> Option<String> {
    let start = element.find(&format!(" {name}=\""))? + name.len() + 3;
    let end = element[start..].find('"')?;
    Some(element[start..start + end].to_string())
}

/// Every window's parts and their bound paths, from the generated markup in `folder`.
fn binds_in(folder: &Path) -> BTreeMap<String, BTreeMap<String, String>> {
    fn walk(dir: &Path, out: &mut BTreeMap<String, BTreeMap<String, String>>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "target") {
                    walk(&path, out);
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "cxaml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let Some(window) = text.split("<Window ").nth(1).and_then(|w| attribute(&format!(" {w}"), "name")) else { continue };
            let parts = out.entry(window).or_default();
            for element in text.split('<').skip(1) {
                let element = format!(" {}", element.split('>').next().unwrap_or_default());
                if let (Some(name), Some(bind)) = (attribute(&element, "name"), attribute(&element, "bind")) {
                    parts.entry(name).or_insert(bind);
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(folder, &mut out);
    out
}

/// The value at a bound path (`album.title`, `tasks[1].done`) in a window's data.
fn at_path(data: &Value, path: &str) -> Value {
    let mut here = data;
    for segment in path.split('.') {
        let (name, index) = match segment.split_once('[') {
            Some((name, rest)) => (name, rest.trim_end_matches(']').parse::<usize>().ok()),
            None => (segment, None),
        };
        here = match here.get(name) {
            Some(v) => v,
            None => return Value::Null,
        };
        if let Some(i) = index {
            here = match here.get(i) {
                Some(v) => v,
                None => return Value::Null,
            };
        }
    }
    here.clone()
}

/// A value as a person reads it in its box: nothing for none, numbers without a trailing `.0`.
fn shown(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            _ => n.to_string(),
        },
        other => other.to_string(),
    }
}

fn same<T: PartialEq + std::fmt::Debug>(actual: T, expected: T, what: &str) -> Done {
    if actual == expected { Ok(()) } else { Err(format!("{what}: expected {expected:?}, got {actual:?}")) }
}

fn check(ok: bool, what: impl Into<String>) -> Done {
    if ok { Ok(()) } else { Err(what.into()) }
}

impl Driver {
    /// Starts `program` in `folder` with no display, its databases in `data`.
    pub fn start(program: &Path, folder: &Path, data: &Path, scratch: &Path) -> Driver {
        let home = scratch.join("home");
        let run = scratch.join("run");
        for dir in [&home, &run, &data.to_path_buf()] {
            std::fs::create_dir_all(dir).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700));
        }
        let mut child = Command::new(program)
            .current_dir(folder)
            .env("CLARITY_HEADLESS", "1")
            .env("CLARITY_DATA", data)
            // Nothing reaches the person's session: their bus, keyring, settings or display.
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("XDG_RUNTIME_DIR", &run)
            .env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", scratch.join("nobus").display()))
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", program.display()));
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (send, answers) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Driver { child, input, answers, folder: folder.to_path_buf(), database: data.join("everything.db"), binds: binds_in(folder), failed: Vec::new() }
    }

    /// Sends one step and gives its answer; an error when it could not be done.
    fn send(&mut self, step: Value) -> Result<Value, String> {
        writeln!(self.input, "{step}").and_then(|_| self.input.flush()).map_err(|e| format!("the program took no step: {e}"))?;
        let line = loop {
            match self.answers.recv_timeout(STEP_TIME) {
                // Lines that are not an answer are what the program printed itself.
                Ok(line) if line.starts_with('{') => break line,
                Ok(line) => println!("     (program) {line}"),
                Err(_) => return Err(format!("no answer to {step} (the program ended or hangs)")),
            }
        };
        let answer: Value = serde_json::from_str(&line).map_err(|e| format!("{e}: {line}"))?;
        if answer["ok"] != json!(true) {
            return Err(format!("{step}: {}", answer["error"].as_str().unwrap_or("not done")));
        }
        Ok(answer)
    }

    fn read(&mut self) -> Result<Value, String> {
        self.send(json!({ "do": "read" }))
    }

    fn click(&mut self, part: &str) -> Done {
        self.send(json!({ "do": "click", "part": part })).map(drop)
    }

    fn click_at(&mut self, x: f64, y: f64) -> Done {
        self.send(json!({ "do": "click", "x": x, "y": y })).map(drop)
    }

    /// Presses the button that says `text` (a dialog's, a wizard's, a tab, a menu item).
    fn press(&mut self, text: &str) -> Done {
        self.send(json!({ "do": "click", "text": text })).map(drop)
    }

    fn focus(&mut self, part: &str) -> Done {
        self.send(json!({ "do": "focus", "part": part })).map(drop)
    }

    fn key(&mut self, key: &str) -> Done {
        self.send(json!({ "do": "key", "key": key })).map(drop)
    }

    fn ctrl(&mut self, key: &str) -> Done {
        self.send(json!({ "do": "key", "key": key, "ctrl": true })).map(drop)
    }

    fn alt(&mut self, key: &str) -> Done {
        self.send(json!({ "do": "key", "key": key, "alt": true })).map(drop)
    }

    fn typed(&mut self, text: &str) -> Done {
        self.send(json!({ "do": "type", "text": text })).map(drop)
    }

    fn settle(&mut self, ms: u64) -> Done {
        self.send(json!({ "do": "settle", "ms": ms })).map(drop)
    }

    fn drag(&mut self, from: (f64, f64), to: (f64, f64)) -> Done {
        self.send(json!({ "do": "drag", "from": [from.0, from.1], "to": [to.0, to.1] })).map(drop)
    }

    fn undo(&mut self) -> Done {
        self.ctrl("Z")?;
        self.settle(400)
    }

    fn redo(&mut self) -> Done {
        self.ctrl("Y")?;
        self.settle(400)
    }

    /// Ctrl+Z with the focus in the part, put there as Tab would.
    fn undo_in(&mut self, part: &str) -> Done {
        self.focus(part)?;
        self.undo()
    }

    /// Clicks into a box, selects what it holds, types `text` and leaves with Tab.
    fn edit(&mut self, part: &str, text: &str) -> Done {
        self.click(part)?;
        self.ctrl("A")?;
        self.typed(text)?;
        self.key("Tab")
    }

    /// Clicks the part and lets the window it opens come.
    fn open_window(&mut self, button: &str) -> Done {
        self.click(button)?;
        self.settle(600)
    }

    /// The place of a part that shows, [x, y, w, h], the window scrolled to it first.
    fn place(&mut self, part: &str) -> Result<[f64; 4], String> {
        let answer = self.send(json!({ "do": "show", "part": part }))?;
        let at = answer["at"].as_array().ok_or_else(|| format!("no part {part} shows"))?;
        let n = |i: usize| at.get(i).and_then(Value::as_f64).unwrap_or(0.0);
        Ok([n(0), n(1), n(2), n(3)])
    }

    /// A point in a part, `fx` and `fy` of its width and height from its top left.
    fn point(&mut self, part: &str, fx: f64, fy: f64) -> Result<(f64, f64), String> {
        let [x, y, w, h] = self.place(part)?;
        Ok((x + w * fx, y + h * fy))
    }

    fn shows(&mut self, part: &str) -> Result<bool, String> {
        Ok(self.read()?["shown"].get(part).is_some())
    }

    /// The value of a part as bound: the dialog's when the part is in the dialog over the window.
    fn value(&mut self, part: &str) -> Result<Value, String> {
        let read = self.read()?;
        if let Some(dialog) = read.get("dialog").filter(|d| d.is_object()) {
            let window = dialog["window"].as_str().unwrap_or_default();
            if let Some(bind) = self.binds.get(window).and_then(|b| b.get(part)) {
                return Ok(at_path(&dialog["data"], bind));
            }
        }
        let window = read["window"].as_str().unwrap_or_default();
        let bind = self.binds.get(window).and_then(|b| b.get(part)).ok_or_else(|| format!("the window {window} binds no part {part}"))?;
        Ok(at_path(&read["data"], bind))
    }

    /// The value of a part as a person reads it.
    fn text(&mut self, part: &str) -> Result<String, String> {
        Ok(shown(&self.value(part)?))
    }

    /// The warnings and errors the app said, closed so the next step starts clean.
    fn error_said(&mut self) -> Result<String, String> {
        let read = self.read()?;
        let said: Vec<String> = read["said"].as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default();
        for _ in 0..said.len() {
            self.key("Escape")?;
            self.settle(200)?;
        }
        Ok(said.join(" / "))
    }

    // -- The database ---------------------------------------------------------

    /// Runs a query with sqlite3, as any client of the database does, waiting a moment for the app's write.
    fn sql(&self, query: &str) -> String {
        let ran = Command::new("sqlite3").args(["-cmd", ".timeout 5000", "-separator", "|"]).arg(&self.database).arg(query).output().expect("sqlite3 runs");
        assert!(ran.status.success(), "sqlite3 {query}: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8_lossy(&ran.stdout).trim().to_string()
    }

    fn rows(&self, query: &str) -> Vec<String> {
        let out = self.sql(query);
        if out.is_empty() { Vec::new() } else { out.lines().map(str::to_string).collect() }
    }

    fn count(&self, query: &str) -> i64 {
        self.sql(query).parse().unwrap_or(-1)
    }

    /// Waits up to 3 s for `read` to give `expected`: what follows a round trip to the runtime.
    fn same_soon<T: PartialEq + std::fmt::Debug>(&mut self, read: impl Fn(&mut Driver) -> Result<T, String>, expected: T, what: &str) -> Done {
        let end = Instant::now() + Duration::from_secs(3);
        let mut actual = read(self)?;
        while actual != expected && Instant::now() < end {
            self.settle(100)?;
            actual = read(self)?;
        }
        same(actual, expected, what)
    }

    // -- Steps ----------------------------------------------------------------

    /// Runs one step: `ok` or `FAIL` with what differed and a shot, and whatever it left open closed.
    fn step(&mut self, name: &str, run: impl FnOnce(&mut Driver) -> Done) {
        let done = run(self).and_then(|_| {
            let said = self.error_said()?;
            check(said.is_empty(), format!("the app said: {said}"))
        });
        match done {
            Ok(()) => println!("ok   {name}"),
            Err(what) => {
                self.failed.push(name.to_string());
                let shot = self.folder.join(format!("failed-{}.png", self.failed.len()));
                let _ = self.send(json!({ "do": "shot", "path": shot }));
                let _ = self.error_said();
                // Whatever the step left open (a question, a menu) is closed, as a person would.
                for _ in 0..3 {
                    match self.read() {
                        Ok(read) if read["overlays"].as_u64().unwrap_or(0) > 0 => {
                            let _ = self.key("Escape");
                            let _ = self.settle(200);
                        }
                        _ => break,
                    }
                }
                println!("FAIL {name}\n     {}\n     (shot {})", what.replace('\n', "\n     "), shot.display());
            }
        }
    }

    /// The steps, in drive.mjs's order; the names of those that failed.
    pub fn run(mut self) -> Vec<String> {
        if let Err(e) = self.read().and_then(|r| same(r["window"].as_str().unwrap_or_default().to_string(), "Albums".to_string(), "the first window")) {
            println!("FAIL start\n     {e}");
            return vec!["start".into()];
        }
        // Records to work with, as a person would have them.
        self.sql(
            "INSERT INTO album (title, position, copies) VALUES ('Alpha', 1, 1), ('Beta', 2, 2), ('Gamma', 3, 3);
             INSERT INTO folder (name, position, size, parent) VALUES ('Music', 1, 10, NULL), ('Docs', 2, 20, NULL);
             INSERT INTO folder (name, position, size, parent) VALUES ('Rock', 1, 5, (SELECT id FROM folder WHERE name = 'Music'));
             INSERT INTO friend (name, city, position) VALUES ('Ann', 'Oslo', 1), ('Bob', 'Rome', 2), ('Cid', 'Lima', 3);",
        );
        albums(&mut self);
        gallery(&mut self);
        folders(&mut self);
        let _ = self.send(json!({ "do": "quit" }));
        self.failed.clone()
    }
}

// ---- Inputs in a form, and every input of the albums window.
fn albums(d: &mut Driver) {
    d.step("a form's fields are taken back one by one, the last first", |d| {
        d.click("albumTitleBox")?;
        d.typed("Zed")?;
        d.key("Tab")?;
        d.typed("4")?;
        d.key("Tab")?;
        same(d.text("albumTitleBox")?, "Zed".into(), "title typed")?;
        same(d.text("albumCopiesBox")?, "4".into(), "copies typed")?;
        d.click("albumTitleBox")?;
        d.key("Tab")?;
        d.undo()?;
        same(d.text("albumCopiesBox")?, "0".into(), "copies taken back first")?;
        same(d.text("albumTitleBox")?, "Zed".into(), "title still there")?;
        d.undo()?;
        same(d.text("albumTitleBox")?, "".into(), "title taken back next")?;
        d.redo()?;
        same(d.text("albumTitleBox")?, "Zed".into(), "title done again")
    });

    d.step("a toggle's press and what its handler saved are taken back together", |d| {
        let before = d.count("SELECT count(*) FROM album");
        d.click("favouriteToggle")?;
        d.settle(500)?;
        same(d.count("SELECT count(*) FROM album"), before + 1, "favouriteChanged saved the album")?;
        d.undo()?;
        same(d.count("SELECT count(*) FROM album"), before, "the saved album is taken back")
    });

    d.step("a slider and a spin box are edits", |d| {
        d.click("copiesSpin")?;
        d.key("Up")?;
        d.key("Tab")?;
        let up = d.text("copiesSpin")?;
        d.click("copiesSpin")?;
        d.undo()?;
        let now = d.text("copiesSpin")?;
        check(now != up || up == "0", format!("spin box taken back (was {up}, now {now})"))
    });

    d.step("a switch is an edit", |d| {
        let was = d.value("toolbarSwitch")?.as_bool().unwrap_or(false);
        d.click("toolbarSwitch")?;
        same(d.value("toolbarSwitch")?.as_bool().unwrap_or(false), !was, "switched")?;
        d.undo_in("toolbarSwitch")?;
        same(d.value("toolbarSwitch")?.as_bool().unwrap_or(false), was, "taken back")
    });

    d.step("a dropdown of records is an edit", |d| {
        d.focus("favouriteFriendBox")?;
        d.key("Down")?;
        d.settle(300)?;
        // Down opens the list; the next Down marks the first friend and Enter takes it.
        if d.read()?["overlays"].as_u64().unwrap_or(0) > 0 {
            d.key("Down")?;
            d.key("Enter")?;
            d.settle(300)?;
        }
        let picked = d.value("favouriteFriendBox")?;
        check(!picked.is_null(), "a friend picked")?;
        d.undo_in("favouriteFriendBox")?;
        same(d.value("favouriteFriendBox")?, Value::Null, "taken back")
    });

    d.step("a color typed is an edit", |d| {
        d.edit("tintBox", "#ff0000")?;
        same(d.text("tintBox")?, "#ff0000".into(), "typed")?;
        d.undo_in("tintBox")?;
        same(d.text("tintBox")?, "#3b82f6".into(), "taken back")
    });

    d.step("a slider moved with the keys is an edit", |d| {
        d.focus("ratingSlider")?;
        d.key("Right")?;
        d.settle(300)?;
        same(d.text("ratingSlider")?, "0.5".into(), "moved")?;
        d.undo_in("ratingSlider")?;
        same(d.text("ratingSlider")?, "0".into(), "taken back")
    });

    d.step("a tag added is an edit", |d| {
        d.click("tagsBox")?;
        d.typed("live")?;
        d.key("Enter")?;
        d.settle(300)?;
        check(d.value("tagsBox")?.to_string().contains("live"), "the tag shows")?;
        d.undo_in("tagsBox")?;
        let now = d.value("tagsBox")?;
        check(!now.to_string().contains("live"), format!("taken back (now {now})"))
    });

    d.step("a typed dropdown is an edit", |d| {
        d.edit("cityBox", "Oslo")?;
        same(d.text("cityBox")?, "Oslo".into(), "typed")?;
        d.undo_in("cityBox")?;
        same(d.text("cityBox")?, "".into(), "taken back")
    });

    d.step("a file chosen is an edit", |d| {
        let file = d.folder.join("notes.pdf");
        std::fs::write(&file, "%PDF-1.4\n%%EOF\n").map_err(|e| e.to_string())?;
        d.send(json!({ "do": "file", "path": file }))?;
        d.place("bookletBox")?;
        d.press("Choose a file")?;
        d.settle(300)?;
        let chosen = d.value("bookletBox")?;
        check(chosen.to_string().contains("notes.pdf"), format!("the file shows (the value is {chosen})"))?;
        d.undo_in("bookletBox")?;
        let now = d.value("bookletBox")?;
        check(!now.to_string().contains("notes.pdf"), format!("taken back (now {now})"))
    });

    d.step("an accordion opens another section", |d| {
        d.press("Site")?;
        d.settle(200)?;
        check(d.shows("siteText")?, "the site section shows")
    });

    d.step("a question for a number sets what it answers", |d| {
        d.click("copiesButton")?;
        d.settle(200)?;
        check(d.read()?["overlays"].as_u64().unwrap_or(0) > 0, "the question shows")?;
        d.ctrl("A")?;
        d.typed("7")?;
        d.key("Enter")?;
        d.settle(500)?;
        same(d.text("copiesBadge")?, "7".into(), "the copies")
    });

    d.step("a wizard refuses an empty title, then saves the album", |d| {
        let before = d.count("SELECT count(*) FROM album");
        d.click("newButton")?;
        d.settle(300)?;
        check(d.read()?["top"].as_array().is_some_and(|t| t.contains(&json!("wizard"))), "the wizard shows")?;
        d.press("Next")?;
        d.settle(300)?;
        same(d.error_said()?, "A title first".into(), "the refusal said")?;
        check(d.read()?["top"].as_array().is_some_and(|t| t.contains(&json!("titleBox"))), "still on the first step")?;
        d.click("titleBox")?;
        d.typed("Delta")?;
        d.key("Tab")?;
        d.press("Next")?;
        d.settle(200)?;
        d.press("Next")?;
        d.settle(200)?;
        d.press("Finish")?;
        d.settle(600)?;
        same(d.count("SELECT count(*) FROM album"), before + 1, "saved")?;
        same(d.sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "1".into(), "the title")
    });

    d.step("a yes-or-no question deletes on yes", |d| {
        d.click("deleteButton")?;
        d.settle(200)?;
        check(d.read()?["overlays"].as_u64().unwrap_or(0) > 0, "the question shows")?;
        d.press("Delete")?;
        d.settle(500)?;
        same(d.sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "0".into(), "deleted")
    });
    // The steps after work on the albums they started with, whatever happened above.
    d.sql("DELETE FROM album WHERE title NOT IN ('Alpha', 'Beta', 'Gamma')");

    d.step("a split button opens its menu", |d| {
        // Its arrow is the right end of the button.
        let [x, y, w, h] = d.place("saveButton")?;
        d.click_at(x + w - 8.0, y + h / 2.0)?;
        d.settle(200)?;
        let read = d.read()?;
        check(read["buttons"].as_array().is_some_and(|b| b.iter().any(|b| b["text"] == json!("Save a copy"))), "the menu shows")?;
        d.key("Escape")
    });

    d.step("a menu item's key opens its window", |d| {
        d.ctrl("F")?;
        d.settle(600)?;
        check(d.shows("folderTree")?, "Ctrl+F opened the folders")?;
        d.open_window("albumsButton")
    });
}

/// The point on the screen of the unit `ux`, `uy` of a canvas, the window scrolled to it first.
fn on_canvas(d: &mut Driver, part: &str, ux: f64, uy: f64) -> Result<(f64, f64), String> {
    d.place(part)?;
    let read = d.read()?;
    let c = &read["canvases"][part];
    let n = |v: &Value| v.as_f64().ok_or_else(|| format!("no canvas {part}: {c}"));
    let scale = n(&c["scale"])?;
    Ok((n(&c["at"][0])? + (ux - n(&c["x"])?) * scale, n(&c["at"][1])? + (uy - n(&c["y"])?) * scale))
}

/// The rows a list, table or tree shows, top to bottom, each as it reads.
fn shown_rows(d: &mut Driver, part: &str) -> Result<Vec<String>, String> {
    let read = d.read()?;
    let rows = read["rows"][part].as_array().ok_or_else(|| format!("no rows of {part} show"))?;
    Ok(rows.iter().map(|r| r["text"].as_str().unwrap_or_default().to_string()).collect())
}

/// The middle of a box `[x, y, w, h]` as a read gives it.
fn middle_of(at: &Value) -> Result<(f64, f64), String> {
    let n = |i: usize| at.get(i).and_then(Value::as_f64).ok_or_else(|| format!("no box: {at}"));
    Ok((n(0)? + n(2)? / 2.0, n(1)? + n(3)? / 2.0))
}

/// The middle of the row shown `index` (from 0, top to bottom) of a list, table or tree.
fn row_at(d: &mut Driver, part: &str, index: usize) -> Result<(f64, f64), String> {
    d.place(part)?;
    let read = d.read()?;
    middle_of(&read["rows"][part][index]["at"]).map_err(|e| format!("row {index} of {part}: {e}"))
}

/// The index of the row shown that says `text`.
fn row_saying(d: &mut Driver, part: &str, text: &str) -> Result<usize, String> {
    shown_rows(d, part)?.iter().position(|r| r == text).ok_or_else(|| format!("no row {text} in {part}"))
}

// ---- The gallery: a list that reorders, a panel, a no-undo input, a picture, canvases.
fn gallery(d: &mut Driver) {
    let ordered = |d: &Driver| d.rows("SELECT title FROM album WHERE title IN ('Alpha','Beta','Gamma') ORDER BY position");
    d.step("the gallery opens with the albums in their order", |d| {
        d.open_window("galleryButton")?;
        same(shown_rows(d, "albumList")?, vec!["Alpha | 1".to_string(), "Beta | 2".into(), "Gamma | 3".into()], "albums")
    });

    d.step("Alt+Down moves a list row, saved; Ctrl+Z takes it back, Ctrl+Y does it again", |d| {
        let (x, y) = row_at(d, "albumList", 0)?;
        d.click_at(x, y)?;
        d.alt("Down")?;
        d.settle(500)?;
        d.same_soon(|d| shown_rows(d, "albumList"), vec!["Beta | 2".to_string(), "Alpha | 1".into(), "Gamma | 3".into()], "moved down")?;
        same(ordered(d), vec!["Beta".to_string(), "Alpha".into(), "Gamma".into()], "saved order")?;
        d.undo()?;
        same(shown_rows(d, "albumList")?, vec!["Alpha | 1".to_string(), "Beta | 2".into(), "Gamma | 3".into()], "taken back")?;
        same(ordered(d), vec!["Alpha".to_string(), "Beta".into(), "Gamma".into()], "saved order taken back")?;
        d.redo()?;
        same(ordered(d), vec!["Beta".to_string(), "Alpha".into(), "Gamma".into()], "done again")?;
        d.undo()
    });

    d.step("a dragged list row lands where it is dropped, and stays after reopening", |d| {
        let from = row_at(d, "albumList", 2)?;
        d.place("albumList")?;
        let read = d.read()?;
        let top = &read["rows"]["albumList"][0]["at"];
        let (x, y) = middle_of(top)?;
        // A little above the middle of the first row: before it.
        d.drag(from, (x, y - top[3].as_f64().unwrap_or(20.0) * 0.3))?;
        d.same_soon(|d| shown_rows(d, "albumList"), vec!["Gamma | 3".to_string(), "Alpha | 1".into(), "Beta | 2".into()], "dragged to the top")?;
        same(ordered(d), vec!["Gamma".to_string(), "Alpha".into(), "Beta".into()], "saved")?;
        d.open_window("foldersButton")?;
        d.open_window("galleryButton")?;
        let again = shown_rows(d, "albumList")?;
        same(again.into_iter().take(3).collect::<Vec<_>>(), vec!["Gamma | 3".to_string(), "Alpha | 1".into(), "Beta | 2".into()], "order after reopening")
    });

    d.step("a panel's fields are taken back one by one", |d| {
        let (x, y) = row_at(d, "albumList", 1)?;
        d.send(json!({ "do": "click", "x": x, "y": y, "clicks": 2 }))?;
        d.settle(300)?;
        d.edit("priceBox", "12")?;
        d.click("notesMemo")?;
        d.typed("note")?;
        d.click("releasedOn")?;
        same(d.text("notesMemo")?, "note".into(), "memo typed")?;
        d.undo()?;
        same(d.text("notesMemo")?, "".into(), "memo taken back first")?;
        let price = d.text("priceBox")?;
        check(price.starts_with("12"), format!("price still there (now {price})"))?;
        d.undo()?;
        let price = d.text("priceBox")?;
        check(!price.starts_with("12"), format!("price taken back (now {price})"))
    });

    d.step("a timer keeps ticking in a window under another, without an error", |d| d.settle(2500));

    d.step("a radio choice is an edit", |d| {
        // The second option of a vertical radio of two rows: its circle, in the lower half.
        let [x, y, _, h] = d.place("formatRadio")?;
        let (x, y) = (x + 10.0, y + h * 0.675);
        d.click_at(x, y)?;
        same(d.text("formatRadio")?, "Cd".into(), "CD chosen")?;
        d.undo_in("formatRadio")?;
        let now = d.text("formatRadio")?;
        check(now != "Cd", format!("taken back (now {now})"))
    });

    d.step("a date typed is an edit; Ctrl+Z takes back the typing, and the day once taken", |d| {
        // Typed in the person's format, month first in English, as the box shows a day.
        let (x, y) = d.point("releasedOn", 0.1, 0.5)?;
        d.click_at(x, y)?;
        d.typed("02/14/2024")?;
        d.undo()?;
        same(d.text("releasedOn")?, "".into(), "typing taken back in the input")?;
        d.click_at(x, y)?;
        d.ctrl("A")?;
        d.typed("02/14/2024")?;
        d.click("priceBox")?;
        same(d.text("releasedOn")?, "2024-02-14".into(), "taken when left")?;
        d.undo_in("releasedOn")?;
        same(d.text("releasedOn")?, "".into(), "the day taken back")
    });

    d.step("a password and a folder typed are edits", |d| {
        d.edit("tokenBox", "s3cret")?;
        d.undo_in("tokenBox")?;
        same(d.text("tokenBox")?, "".into(), "password taken back")?;
        d.edit("scansBox", "papers")?;
        same(d.text("scansBox")?, "papers".into(), "folder typed")?;
        d.undo_in("scansBox")?;
        same(d.text("scansBox")?, "scans".into(), "folder taken back")
    });

    d.step("a picture loaded into an image is an edit", |d| {
        d.press("Cover")?;
        d.settle(200)?;
        let banner = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/everything/resources/banner.png");
        d.send(json!({ "do": "file", "path": banner }))?;
        d.place("coverImage")?;
        d.press("Load a picture")?;
        d.settle(300)?;
        check(!d.value("coverImage")?.is_null(), "the picture")?;
        d.undo_in("coverImage")?;
        same(d.value("coverImage")?, Value::Null, "the picture taken back")?;
        d.press("Albums")
    });

    d.step("a drag on a canvas and its undo say nothing wrong", |d| {
        let from = d.point("dial", 0.3, 0.5)?;
        d.drag(from, (from.0 + 60.0, from.1 + 20.0))?;
        d.undo_in("dial")
    });

    // Canvas modes (DESIGN 5.31): each gesture is one edit the runtime makes.
    let places = |d: &Driver| d.rows("SELECT id || ':' || copies || ',' || position FROM album ORDER BY id");
    d.step("an object dragged on a board lands on the grid, saved; Ctrl+Z takes it back", |d| {
        let before = places(d);
        // The object drawn last is on top: the album with the highest id, 200 by 80 units at
        // (copies, position) on a board of 1000 by 600 units.
        let last = before.last().cloned().ok_or("the board has no albums")?;
        let (cx, py) = last.split_once(':').and_then(|(_, p)| p.split_once(',')).ok_or("no place")?;
        let (ux, uy) = (cx.parse::<f64>().unwrap_or(0.0) + 100.0, py.parse::<f64>().unwrap_or(0.0) + 40.0);
        let from = on_canvas(d, "board", ux, uy)?;
        d.drag(from, (from.0 + 60.0, from.1 + 30.0))?;
        d.settle(500)?;
        let after = places(d);
        let moved: Vec<&String> = after.iter().zip(&before).filter(|(a, b)| a != b).map(|(a, _)| a).collect();
        check(moved.len() == 1, format!("one object moved: {moved:?}"))?;
        let at: Vec<f64> = moved[0].split_once(':').map(|(_, p)| p.split(',').filter_map(|n| n.parse().ok()).collect()).unwrap_or_default();
        check(at.len() == 2 && at[0] % 20.0 == 0.0 && at[1] % 20.0 == 0.0, format!("snapped to the grid of 20: {at:?}"))?;
        d.undo_in("board")?;
        d.same_soon(|d| Ok(places(d)), before, "Ctrl+Z puts it back")
    });

    let covers = |d: &Driver| d.count("SELECT count(*) FROM album WHERE cover IS NOT NULL AND cover != ''");
    d.step("a stroke painted on a picture is saved; Ctrl+Z takes it back", |d| {
        let before = covers(d);
        let from = d.point("sketch", 0.3, 0.5)?;
        d.drag(from, (from.0 + 50.0, from.1 + 10.0))?;
        d.same_soon(|d| Ok(covers(d)), before + 1, "the picture saved")?;
        d.undo_in("sketch")?;
        d.same_soon(|d| Ok(covers(d)), before, "Ctrl+Z takes the stroke back")
    });

    let drawn = |d: &Driver| d.count("SELECT count(*) FROM album WHERE notes LIKE '%<rect %'");
    d.step("a rectangle drawn on a vector drawing is saved as SVG; Delete takes it away, Ctrl+Z brings it back", |d| {
        same(drawn(d), 0, "no rectangle yet")?;
        d.place("drawing")?;
        d.press("Rectangle")?;
        let from = d.point("drawing", 0.3, 0.4)?;
        d.drag(from, (from.0 + 40.0, from.1 + 30.0))?;
        d.same_soon(|d| Ok(drawn(d)), 1, "the rectangle saved")?;
        d.press("Select")?;
        d.click_at(from.0 + 20.0, from.1 + 15.0)?;
        d.key("Delete")?;
        d.same_soon(|d| Ok(drawn(d)), 0, "Delete takes it away")?;
        d.undo_in("drawing")?;
        d.same_soon(|d| Ok(drawn(d)), 1, "Ctrl+Z brings it back")
    });

    d.step("a sound plays and an engine call answers", |d| {
        d.click("playButton")?;
        d.click("planButton")?;
        d.settle(800)
    });

    d.step("a tab shows its markdown", |d| {
        d.press("About")?;
        d.settle(200)?;
        check(d.shows("aboutText")?, "the text shows")?;
        d.press("Albums")
    });

    d.step("an input with no undo keeps what was typed", |d| {
        d.click("contactBox")?;
        d.typed("ann@example.com")?;
        d.key("Tab")?;
        d.click("contactBox")?;
        d.undo()?;
        same(d.text("contactBox")?, "ann@example.com".into(), "kept")
    });
}

// ---- Folders: a tree that reorders, a tree table that reorders without undo,
// and what a table, a tree and a chart show; then a dialog's table.
fn folders(d: &mut Driver) {
    let top = |d: &Driver| d.rows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position");
    let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    d.step("a tree row moves down, and Ctrl+Z takes it back", |d| {
        d.open_window("foldersButton")?;
        same(shown_rows(d, "folderTree")?, names(&["Music", "Docs"]), "the top level")?;
        let (x, y) = row_at(d, "folderTree", 0)?;
        d.click_at(x, y)?;
        d.alt("Down")?;
        d.settle(500)?;
        same(top(d), names(&["Docs", "Music"]), "saved order")?;
        d.undo()?;
        same(top(d), names(&["Music", "Docs"]), "taken back")
    });

    d.step("Alt+Right puts a tree row under the row before it; Ctrl+Z takes it out again", |d| {
        let docs = row_saying(d, "folderTree", "Docs")?;
        let (x, y) = row_at(d, "folderTree", docs)?;
        d.click_at(x, y)?;
        d.alt("Right")?;
        d.settle(500)?;
        same(d.sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music".into(), "Docs under Music")?;
        d.undo()?;
        same(d.sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1".into(), "Docs back at the top")
    });

    d.step("a tree row dragged onto another goes under it", |d| {
        let docs = row_saying(d, "folderTree", "Docs")?;
        let music = row_saying(d, "folderTree", "Music")?;
        let from = row_at(d, "folderTree", docs)?;
        let to = row_at(d, "folderTree", music)?;
        d.drag(from, to)?;
        d.settle(300)?;
        same(d.sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music".into(), "dropped into Music")?;
        d.undo()?;
        same(d.sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1".into(), "taken back")
    });

    d.step("a tree table that reorders with no undo keeps its move", |d| {
        let (x, y) = row_at(d, "folderTable", 0)?;
        d.click_at(x, y)?;
        d.alt("Down")?;
        d.settle(500)?;
        same(top(d), names(&["Docs", "Music"]), "moved")?;
        d.undo()?;
        same(top(d), names(&["Docs", "Music"]), "kept: no undo")
    });

    d.step("a table shows its first order a page at a time, and turns pages", |d| {
        d.same_soon(|d| shown_rows(d, "albumGrid"), names(&["Alpha 1", "Beta 2"]), "the first page, by title")?;
        d.place("albumGrid")?;
        let read = d.read()?;
        let (x, y) = middle_of(&read["rows"]["albumGrid.pages"]["next"]).map_err(|e| format!("the next page button: {e}"))?;
        d.click_at(x, y)?;
        d.settle(400)?;
        d.same_soon(|d| shown_rows(d, "albumGrid"), names(&["Gamma 3"]), "the second page")
    });

    d.step("a header click sorts the other way, from the first page", |d| {
        d.place("albumGrid")?;
        let read = d.read()?;
        let (x, y) = middle_of(&read["rows"]["albumGrid.header"]["title"]).map_err(|e| format!("the title header: {e}"))?;
        d.click_at(x, y)?;
        d.settle(400)?;
        d.same_soon(|d| shown_rows(d, "albumGrid"), names(&["Gamma 3", "Beta 2"]), "sorted the other way")
    });

    d.step("a tree row's toggle opens and closes the rows under it", |d| {
        let shows_rock = |d: &mut Driver| -> Result<bool, String> { Ok(shown_rows(d, "folderTree")?.iter().any(|r| r == "Rock")) };
        let before = shows_rock(d)?;
        let music = row_saying(d, "folderTree", "Music")?;
        d.place("folderTree")?;
        let read = d.read()?;
        let toggle = middle_of(&read["rows"]["folderTree"][music]["toggle"]).map_err(|e| format!("Music's toggle: {e}"))?;
        d.click_at(toggle.0, toggle.1)?;
        d.settle(400)?;
        same(shows_rock(d)?, !before, "the first press turns it")?;
        d.click_at(toggle.0, toggle.1)?;
        d.settle(400)?;
        same(shows_rock(d)?, before, "the second press turns it back")
    });

    let friends = |d: &Driver| d.rows("SELECT name FROM friend ORDER BY position");
    d.step("a table in a dialog reorders, saved, and Ctrl+Z takes it back", |d| {
        d.open_window("albumsButton")?;
        d.click("pickButton")?;
        d.settle(400)?;
        same(shown_rows(d, "grid")?, names(&["Ann", "Bob", "Cid"]), "the friends")?;
        let (x, y) = row_at(d, "grid", 2)?;
        d.click_at(x, y)?;
        d.alt("Up")?;
        d.settle(500)?;
        same(friends(d), names(&["Ann", "Cid", "Bob"]), "saved order")?;
        d.undo()?;
        same(friends(d), names(&["Ann", "Bob", "Cid"]), "taken back")
    });

    d.step("a popover's checkbox runs its handler, which closes it", |d| {
        d.click("filterButton")?;
        d.settle(300)?;
        check(d.shows("namedBox")?, "the popover shows")?;
        d.click("namedBox")?;
        d.settle(400)?;
        check(!d.shows("namedBox")?, "the popover closed")
    });

    d.step("a table row's context menu picks it, and the dialog gives it back", |d| {
        let (x, y) = row_at(d, "grid", 1)?;
        d.send(json!({ "do": "click", "x": x, "y": y, "button": "right" }))?;
        d.settle(300)?;
        d.press("Pick")?;
        d.settle(600)?;
        check(!d.shows("grid")?, "the dialog closed")
    });
}

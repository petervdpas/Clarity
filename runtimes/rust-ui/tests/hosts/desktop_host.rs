//! The desktop host (DESIGN 7.9) without a window: what it gives the parts
//! (sound through rodio, the system's file dialogs), a sound device that
//! does not open leaving the program silent, and the dialog each system
//! shows. No test plays a sound or opens a dialog (C# DesktopHostTests).

#![cfg(feature = "desktop")]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use clarity_ui::desktop::files::{answer, commands, extensions, show_command, ways, Ask, Kind, Os, SystemFilePicker, Way};
use clarity_ui::desktop::sound::Speakers;
use clarity_ui::files::{safe_name, FileFilter, FilePicker};
use clarity_ui::melodies::{self, Note};
use clarity_ui::parts::sound::SoundOutput;

fn open() -> Ask {
    Ask { kind: Kind::Open, title: "Choose a file".into(), name: None, filter: Some(FileFilter { name: "Pictures".into(), patterns: vec!["*.png".into(), "*.jpg".into()] }) }
}

fn save() -> Ask {
    Ask { kind: Kind::Save, title: "Save a copy".into(), name: Some("notes.txt".into()), filter: None }
}

fn folder() -> Ask {
    Ask { kind: Kind::Folder, title: "Choose a folder".into(), name: None, filter: None }
}

fn args(program: &clarity_ui::desktop::files::Program) -> Vec<&str> {
    program.args.iter().map(String::as_str).collect()
}

#[test]
fn the_desktop_plays_through_rodio_and_gives_the_system_s_file_dialogs() {
    let files = clarity_ui::desktop::prepare();
    // The parts' picker is the host's: one that picks folders, and no dialog is open.
    assert!(clarity_ui::files::system().picks_folders());
    assert!(!files.showing());
    // Made, not opened: no device is asked for until a sound plays.
    assert!(!Speakers::default().failed());
}

#[test]
fn without_a_sound_device_it_plays_nothing_and_runs_on() {
    let asked = Rc::new(Cell::new(0));
    let counted = asked.clone();
    let speakers = Speakers::with(Box::new(move || {
        counted.set(counted.get() + 1);
        Err("no playback device".to_string())
    }));
    let mut playing = speakers.play(melodies::wav(&[Note { hertz: Some(440.0), seconds: 0.1 }]), 1.0, false);
    playing.stop();
    speakers.play(melodies::wav(&[Note { hertz: Some(220.0), seconds: 0.1 }]), 0.5, true).stop();
    speakers.play(vec![0; 10], 0.5, true).stop();
    assert!(speakers.failed());
    // Asked once: a device that did not open is not asked for again.
    assert_eq!(1, asked.get());
}

#[test]
fn a_sound_that_ended_is_let_go_and_one_that_repeats_goes_on() {
    use std::num::NonZero;
    let (mixer, mut speakers) = clarity_ui::desktop::sound::mixer(NonZero::new(1).unwrap(), NonZero::new(melodies::SAMPLE_RATE).unwrap());
    let tone = melodies::wav(&[Note { hertz: Some(440.0), seconds: 0.05 }]);
    let once = clarity_ui::desktop::sound::play_on(&mixer, &tone, 1.0, false);
    let again = clarity_ui::desktop::sound::play_on(&mixer, &tone, 1.0, true);
    assert!(!once.ended() && !again.ended());
    // The speakers take a second of samples: the tone of 50 ms is over, the repeated one plays on.
    for _ in 0..melodies::SAMPLE_RATE {
        speakers.next();
    }
    assert!(once.ended());
    assert!(!again.ended());
    // Data that is no WAV plays nothing: it has ended at once.
    assert!(clarity_ui::desktop::sound::play_on(&mixer, &[0; 10], 1.0, false).ended());
}

#[test]
fn each_system_shows_its_own_dialogs() {
    assert_eq!(&[Way::Portal, Way::Commands], ways(Os::Linux));
    assert_eq!(&[Way::Commands], ways(Os::MacOs));
    assert_eq!(&[Way::CommonItemDialog], ways(Os::Windows));
    assert!(commands(Os::Windows, &open()).is_empty());
}

#[test]
fn linux_asks_zenity_then_kdialog() {
    let home = std::env::var("HOME").unwrap_or_default();
    let open = commands(Os::Linux, &open());
    assert_eq!(vec!["zenity", "kdialog"], open.iter().map(|c| c.program.as_str()).collect::<Vec<_>>());
    assert_eq!(vec!["--file-selection", "--title=Choose a file", "--file-filter=Pictures | *.png *.jpg"], args(&open[0]));
    assert_eq!(vec!["--title", "Choose a file", "--getopenfilename", home.as_str(), "Pictures (*.png *.jpg)"], args(&open[1]));
    let save = commands(Os::Linux, &save());
    assert_eq!(vec!["--file-selection", "--title=Save a copy", "--save", "--confirm-overwrite", "--filename=notes.txt"], args(&save[0]));
    let saved_at = std::path::Path::new(&home).join("notes.txt").to_string_lossy().into_owned();
    assert_eq!(vec!["--title", "Save a copy", "--getsavefilename", saved_at.as_str()], args(&save[1]));
    let folder = commands(Os::Linux, &folder());
    assert_eq!(vec!["--file-selection", "--title=Choose a folder", "--directory"], args(&folder[0]));
    assert_eq!(vec!["--title", "Choose a folder", "--getexistingdirectory", home.as_str()], args(&folder[1]));
}

#[test]
fn macos_asks_applescript() {
    assert_eq!(vec!["-e", "POSIX path of (choose file with prompt \"Choose a file\" of type {\"png\", \"jpg\"})"], args(&commands(Os::MacOs, &open())[0]));
    assert_eq!(vec!["-e", "POSIX path of (choose file name with prompt \"Save a copy\" default name \"notes.txt\")"], args(&commands(Os::MacOs, &save())[0]));
    assert_eq!(vec!["-e", "POSIX path of (choose folder with prompt \"Choose a folder\")"], args(&commands(Os::MacOs, &folder())[0]));
    // A title is quoted as an AppleScript string: it cannot end the script's own.
    let sly = Ask { title: "a \"b\" \\c".into(), ..folder() };
    assert_eq!("POSIX path of (choose folder with prompt \"a \\\"b\\\" \\\\c\")", commands(Os::MacOs, &sly)[0].args[1]);
    assert_eq!("osascript", commands(Os::MacOs, &sly)[0].program);
}

#[test]
fn a_file_is_shown_in_the_system_s_program_for_its_kind() {
    let linux = show_command(Os::Linux, "/tmp/a.pdf");
    assert_eq!(("xdg-open", vec!["/tmp/a.pdf"]), (linux.program.as_str(), args(&linux)));
    assert_eq!("open", show_command(Os::MacOs, "/tmp/a.pdf").program);
    // Windows opens it through its shell: start, by the file's kind.
    let windows = show_command(Os::Windows, r"C:\a.pdf");
    assert_eq!(("cmd", vec!["/C", "start", "", r"C:\a.pdf"]), (windows.program.as_str(), args(&windows)));
    assert_eq!("passwd", safe_name("../../etc/passwd"));
    assert_eq!("file", safe_name(".."));
}

#[test]
fn a_program_s_answer_is_its_first_line_and_nothing_is_no_choice() {
    assert_eq!(Some("/home/ada/notes.txt".to_string()), answer("/home/ada/notes.txt\n"));
    assert_eq!(Some("/Users/ada/Documents".to_string()), answer("/Users/ada/Documents/\n"));
    assert_eq!(None, answer("\n"));
    assert!(extensions(Some(&FileFilter { name: "All".into(), patterns: vec!["*".into()] })).is_empty());
}

/// A file of its own in the scratch folder, removed when the test ends.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let folder = std::env::temp_dir().join(format!("clarity-desktop-host-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        Scratch(folder.join(name))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn what_the_person_chose_comes_back_to_the_part() {
    // The part asks through Cx::later, so the dialog waits off the screen's thread and its answer comes back on it.
    let file = Scratch::new("chosen.bin");
    std::fs::write(&file.0, [1, 2, 3]).unwrap();
    let path = file.0.to_string_lossy().into_owned();
    let picker = SystemFilePicker::with(Box::new(move |_, _| Some(path.clone())), Box::new(|_| {}));
    let got = picker.open_file("Choose a file", None).unwrap();
    assert_eq!("chosen.bin", got.name);
    assert_eq!(vec![1, 2, 3], got.bytes);
    let none = SystemFilePicker::with(Box::new(|_, _| None), Box::new(|_| {}));
    assert_eq!(None, none.choose_folder("Choose a folder"));
    assert_eq!(None, none.open_file("Choose a file", None));
}

#[test]
fn saving_writes_the_copy_where_the_person_chose() {
    let place = Scratch::new("saved.txt");
    let path = place.0.to_string_lossy().into_owned();
    let asked: Arc<Mutex<Option<Ask>>> = Arc::default();
    let kept = asked.clone();
    let chosen = path.clone();
    let picker = SystemFilePicker::with(
        Box::new(move |ask, _| {
            *kept.lock().unwrap() = Some(ask.clone());
            Some(chosen.clone())
        }),
        Box::new(|_| {}),
    );
    assert_eq!(Some(path), picker.save_file("Save a copy", "notes.txt", &[7, 8]));
    assert_eq!(Some(save()), asked.lock().unwrap().clone());
    assert_eq!(vec![7, 8], std::fs::read(&place.0).unwrap());
}

#[test]
fn a_shown_file_goes_to_the_system_s_program_as_a_copy() {
    let started: Arc<Mutex<Vec<String>>> = Arc::default();
    let kept = started.clone();
    let picker = SystemFilePicker::with(Box::new(|_, _| None), Box::new(move |program| kept.lock().unwrap().push(program.args.last().cloned().unwrap_or_default())));
    picker.show("../report.pdf", b"%PDF");
    let shown = started.lock().unwrap().clone();
    let path = std::path::Path::new(&shown[0]);
    assert_eq!(Some("report.pdf"), path.file_name().and_then(|n| n.to_str()));
    assert_eq!(b"%PDF".to_vec(), std::fs::read(path).unwrap());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

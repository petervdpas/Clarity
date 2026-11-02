//! The system's dialogs as the file, folder and picture field tests give
//! them: what they choose next, and what they were asked. Installed as the
//! host's picker on the test's thread (`clarity_ui::files::set_system`).

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use clarity_ui::files::{ChosenFile, FileFilter, FilePicker};

#[derive(Default)]
struct Kept {
    next_file: Option<ChosenFile>,
    next_folder: Option<String>,
    no_folders: bool,
    asked: Vec<String>,
    shown: Vec<(String, Vec<u8>)>,
    saved: Vec<(String, Vec<u8>)>,
    filter: Option<FileFilter>,
    /// Where a saved copy goes; none: the default, a Downloads folder.
    saved_to: Option<String>,
}

#[derive(Default)]
pub struct TestPicker {
    kept: Mutex<Kept>,
}

impl TestPicker {
    /// A picker that picks folders and chooses nothing yet, installed as the host's.
    pub fn installed() -> Arc<TestPicker> {
        let picker = Arc::new(TestPicker::default());
        clarity_ui::files::set_system(picker.clone());
        picker
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.kept.lock().unwrap()
    }

    /// The file the next open dialog chooses, once.
    pub fn next_file(&self, name: &str, bytes: Vec<u8>) {
        self.kept().next_file = Some(ChosenFile { name: name.to_string(), bytes });
    }

    /// The folder the next folder dialog chooses, once.
    pub fn next_folder(&self, folder: &str) {
        self.kept().next_folder = Some(folder.to_string());
    }

    /// It cannot choose a folder, as in a browser.
    pub fn no_folders(&self) {
        self.kept().no_folders = true;
    }

    /// Where the next saved copies go.
    pub fn save_to(&self, place: &str) {
        self.kept().saved_to = Some(place.to_string());
    }

    pub fn asked(&self) -> Vec<String> {
        self.kept().asked.clone()
    }

    pub fn shown(&self) -> Vec<(String, Vec<u8>)> {
        self.kept().shown.clone()
    }

    pub fn saved(&self) -> Vec<(String, Vec<u8>)> {
        self.kept().saved.clone()
    }

    pub fn filter(&self) -> Option<FileFilter> {
        self.kept().filter.clone()
    }
}

impl FilePicker for TestPicker {
    fn picks_folders(&self) -> bool {
        !self.kept().no_folders
    }

    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile> {
        let mut kept = self.kept();
        kept.asked.push(format!("open {title}"));
        kept.filter = filter.cloned();
        kept.next_file.take()
    }

    fn save_file(&self, title: &str, name: &str, bytes: &[u8]) -> Option<String> {
        let mut kept = self.kept();
        kept.asked.push(format!("save {title} {name}"));
        kept.saved.push((name.to_string(), bytes.to_vec()));
        Some(kept.saved_to.clone().unwrap_or_else(|| "/home/ada/Downloads/copy".to_string()))
    }

    fn choose_folder(&self, title: &str) -> Option<String> {
        let mut kept = self.kept();
        kept.asked.push(format!("folder {title}"));
        kept.next_folder.take()
    }

    fn show(&self, name: &str, bytes: &[u8]) {
        self.kept().shown.push((name.to_string(), bytes.to_vec()));
    }
}

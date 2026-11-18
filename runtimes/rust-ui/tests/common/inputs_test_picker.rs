//! The system's dialogs as the file, folder and picture field tests give
//! them: what they choose next, and what they were asked. Installed as the
//! host's picker on the test's thread (`clarity_ui::files::set_system`).

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use clarity_ui::files::{FileFilter, PickerOptions, StorageFile, StorageFolder, StorageProvider};

#[derive(Default)]
struct Kept {
    next_file: Option<StorageFile>,
    next_folder: Option<String>,
    no_folders: bool,
    asked: Vec<String>,
    shown: Vec<(String, Vec<u8>)>,
    saved: Vec<(String, Vec<u8>)>,
    filter: Option<FileFilter>,
    /// Where a saved copy goes; none: the default, a Downloads folder.
    saved_to: Option<String>,
    /// The folder each dialog was asked to open in, in order.
    starts: Vec<Option<String>>,
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
        self.kept().next_file = Some(StorageFile::given(name, bytes));
    }

    /// The file on the disk the next open dialog chooses, once, by its path.
    pub fn next_file_at(&self, path: &str) {
        self.kept().next_file = Some(StorageFile::at(path));
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

    /// The folder each dialog was asked to open in, in order (none: where the system opens it).
    pub fn starts(&self) -> Vec<Option<String>> {
        self.kept().starts.clone()
    }
}

impl StorageProvider for TestPicker {
    fn picks_folders(&self) -> bool {
        !self.kept().no_folders
    }

    fn open_file(&self, options: &PickerOptions) -> Option<StorageFile> {
        let mut kept = self.kept();
        kept.asked.push(format!("open {}", options.title));
        kept.filter = options.file_type.clone();
        kept.starts.push(options.start.clone());
        kept.next_file.take()
    }

    fn save_file(&self, options: &PickerOptions) -> Option<StorageFile> {
        let mut kept = self.kept();
        let name = options.suggested_name.clone().unwrap_or_default();
        kept.asked.push(format!("save {} {name}", options.title));
        kept.starts.push(options.start.clone());
        let path = kept.saved_to.clone().unwrap_or_else(|| "/home/ada/Downloads/copy".to_string());
        Some(StorageFile { name, path: Some(path), bytes: None })
    }

    fn write(&self, file: &StorageFile, bytes: &[u8]) -> Option<String> {
        self.kept().saved.push((file.name.clone(), bytes.to_vec()));
        file.path.clone()
    }

    fn open_folder(&self, options: &PickerOptions) -> Option<StorageFolder> {
        let mut kept = self.kept();
        kept.asked.push(format!("folder {}", options.title));
        kept.filter = options.file_type.clone();
        kept.starts.push(options.start.clone());
        kept.next_folder.take().map(|f| StorageFolder::at(&f))
    }

    fn show(&self, name: &str, bytes: &[u8]) {
        self.kept().shown.push((name.to_string(), bytes.to_vec()));
    }
}

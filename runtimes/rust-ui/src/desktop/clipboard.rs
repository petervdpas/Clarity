//! The system's clipboard through arboard, on Linux, Windows and macOS. On
//! Linux the copied text stays offered while this clipboard lives, so the
//! host keeps it for as long as its window is open. Where the system has no
//! clipboard the screen keeps its own.

use crate::overlay::Clipboard;

/// The system's clipboard.
pub(crate) struct SystemClipboard {
    system: arboard::Clipboard,
}

impl SystemClipboard {
    /// The system's clipboard, or none where it cannot be reached.
    pub(crate) fn open() -> Option<SystemClipboard> {
        arboard::Clipboard::new().ok().map(|system| SystemClipboard { system })
    }
}

impl Clipboard for SystemClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.system.get_text().ok()
    }

    fn set_text(&mut self, text: &str) {
        // A failed copy leaves the clipboard as it was, as the system's own copy does.
        let _ = self.system.set_text(text.to_string());
    }
}

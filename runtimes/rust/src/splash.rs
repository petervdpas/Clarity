//! What the splash screen says is loading (`splash.status = @loadingData`):
//! code sets it while the app's `on start` runs, and the screen asks for it
//! a few times a second until the main window shows. It is a text as a
//! message is one, so the screen shows a key in the person's language.

use std::sync::Mutex;

use crate::ui::Text;

static STATUS: Mutex<Option<Text>> = Mutex::new(None);

/// Code says what loads now.
pub fn set_status(text: Text) {
    *STATUS.lock().unwrap_or_else(|p| p.into_inner()) = Some(text);
}

/// What code last said loads; none while it said nothing.
pub fn status() -> Option<Text> {
    STATUS.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_status_said_is_the_one_shown() {
        set_status(Text::plain("Loading"));
        set_status(Text::key("openingDatabase"));
        assert_eq!(status(), Some(Text::key("openingDatabase")));
    }
}

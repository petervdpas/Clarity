//! What a generated program carries for Clarity.UI to show it: its windows'
//! and apps' markup (`.cxaml`), its resolved themes and its texts, each by
//! name, built into the program.

/// A program's files, by name.
#[derive(Debug, Clone, Copy, Default)]
pub struct AppFiles {
    /// Each window's markup, by the window's name: `("Main", "<Window ...>")`.
    pub windows: &'static [(&'static str, &'static str)],
    /// Each app's frame, by the app's name.
    pub apps: &'static [(&'static str, &'static str)],
    /// The project's resolved themes, by their name and mode: `("shop.light", "{...}")`;
    /// Clarity's own (`clarity.light`, `clarity.dark`) when it has none of its own.
    pub themes: &'static [(&'static str, &'static str)],
    /// The model the program is made with, as JSON: its translations and its language.
    pub model: &'static str,
    /// The title of the desktop window.
    pub title: &'static str,
    /// The size the main window opens at, in the screen's units; 0 for the host's own.
    pub width: f64,
    pub height: f64,
}

impl AppFiles {
    /// The markup of a window by its name; none for a window the program does not carry.
    pub fn window(&self, name: &str) -> Option<&'static str> {
        self.windows.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
    }

    /// The frame of an app by its name.
    pub fn app(&self, name: &str) -> Option<&'static str> {
        self.apps.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
    }

    /// The first app's frame, which every window but a dialog shows; none for a program without an app.
    pub fn first_app(&self) -> Option<&'static str> {
        self.apps.first().map(|(_, m)| *m)
    }

    /// The project's theme for light or dark mode: the first written for that mode.
    pub fn theme(&self, dark: bool) -> Option<&'static str> {
        let mode = if dark { ".dark" } else { ".light" };
        self.themes.iter().find(|(n, _)| n.ends_with(mode) && !n.starts_with("clarity.")).map(|(_, t)| *t)
    }
}

impl AppFiles {
    /// The look the program shows in: its own theme for the mode over Clarity's, else Clarity's.
    pub fn look(&self, dark: bool) -> std::sync::Arc<crate::theme::ThemeLook> {
        let clarity = crate::theme::ThemeLook::clarity(dark);
        match self.theme(dark) {
            Some(own) => crate::theme::ThemeLook::over(own, clarity),
            None => clarity,
        }
    }
}

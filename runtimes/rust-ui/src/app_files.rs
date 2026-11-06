//! What a generated program carries for Clarity.UI to show it: its windows'
//! and apps' markup (`.cxaml`), its resolved themes, its texts and its
//! resources, each by name, built into the program.

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
    /// The files of the project's resources folder by their names: `("logo.png", <bytes>)`.
    pub resources: &'static [(&'static str, &'static [u8])],
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

    /// A resource by its name (`logo.png`); none for a file the program does not carry.
    pub fn resource(&self, name: &str) -> Option<&'static [u8]> {
        self.resources.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
    }

    /// The application's icon: the picture among its resources the project
    /// names (`icon "app.png"`, carried in the model); none for Clarity's own.
    pub fn icon(&self) -> Option<&'static [u8]> {
        let model: serde_json::Value = serde_json::from_str(self.model).ok()?;
        self.resource(model.get("project")?.get("icon")?.as_str()?)
    }

    /// The splash screen the app shows while it starts, as its markup names
    /// it (`<App splash="Welcome" splashFor="2">`, as an Avalonia app starts
    /// its splash window), when the program carries that window.
    pub fn splash(&self) -> Option<Splash> {
        let root = crate::xml::parse(self.first_app()?).ok()?;
        let page = root.attribute("splash")?.trim().to_string();
        self.window(&page)?;
        let seconds = root.attribute("splashFor").and_then(|s| s.trim().parse::<f64>().ok()).filter(|s| s.is_finite() && *s > 0.0).unwrap_or(0.0);
        Some(Splash { page, at_least: std::time::Duration::from_secs_f64(seconds) })
    }

    /// The app's title, as its markup says it (`<App title="@todoTitle">`): a
    /// key or a text; none when it says none.
    pub fn app_title(&self) -> Option<String> {
        let root = crate::xml::parse(self.first_app()?).ok()?;
        root.attribute("title").map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
    }

    /// What window `name`'s markup says of its desktop window; the system's own window for one that says nothing.
    pub fn window_look(&self, name: &str) -> crate::markup::WindowLook {
        self.window(name).and_then(|m| crate::markup::read(m).ok()).map(|s| s.look).unwrap_or_default()
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

/// The splash screen of an application: its page, and how long it shows at least.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splash {
    pub page: String,
    pub at_least: std::time::Duration,
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

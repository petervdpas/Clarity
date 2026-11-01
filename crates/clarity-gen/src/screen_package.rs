//! The screen package (`@clarity/ui`, runtimes/web) as a generated web
//! project carries it: in `ui/clarity-ui/`, which the project's `ui` depends
//! on (`file:./clarity-ui`), so it builds without fetching Clarity's own
//! package (DESIGN 7.8), as the C runtime travels in a C project.

#[path = "screen_files.rs"]
mod screen_files;

/// Each file of the package by its path in the package, and its text.
pub fn files() -> &'static [(&'static str, &'static str)] {
    screen_files::SCREEN_FILES
}

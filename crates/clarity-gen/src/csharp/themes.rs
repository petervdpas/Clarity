//! The themes a C# application offers (DESIGN 1.5, 7.9): each as Clarity
//! resolves it for light and dark mode, the JSON Clarity.UI reads and lays
//! over Clarity's own theme, which Clarity.UI carries. The look is defined in
//! Clarity; the application only carries it.

use clarity_spec::Project;
use clarity_spec::model::ThemeDecl;

use super::HEADER;
use super::names::string;
use crate::OutFile;

/// The themes the project offers, the one it starts with first.
fn offered(project: &Project) -> Vec<&ThemeDecl> {
    project.themes.iter().filter_map(|n| project.files.iter().flat_map(|f| &f.themes).find(|t| t.name.name == *n)).collect()
}

/// A theme's files: `Themes/night-owl.light.json` and `.dark.json` for the class `theme-night-owl`.
pub fn theme_file(theme: &ThemeDecl, dark: bool) -> String {
    format!("Themes/{}.{}.json", theme.class.trim_start_matches("theme-"), if dark { "dark" } else { "light" })
}

/// Each offered theme for light and dark mode, and `AppThemes`, which tells
/// the runtime the themes offered. Without any, Clarity's own is offered.
pub fn theme_files(project: &Project, namespace: &str) -> Vec<OutFile> {
    let themes = offered(project);
    let mut files: Vec<OutFile> = themes
        .iter()
        .flat_map(|theme| {
            [false, true].map(|dark| OutFile {
                path: theme_file(theme, dark),
                text: serde_json::to_string_pretty(&clarity_spec::themes::resolve(theme, dark)).expect("a theme writes as JSON") + "\n",
                bytes: None,
            })
        })
        .collect();
    let built_in = clarity_spec::themes::built_in();
    let names: Vec<String> = if themes.is_empty() { vec![&built_in] } else { themes }
        .iter()
        .map(|t| format!("new({}, {})", string(&t.name.name), string(&t.label)))
        .collect();
    files.push(OutFile {
        path: "AppThemes.cs".to_string(),
        text: format!(
            "{HEADER}
using Clarity.Runtime;

namespace {namespace};

/// <summary>The themes the application offers, the one it starts with first; each in Themes/ as Clarity resolves it.</summary>
public static class AppThemes
{{
    public static void Start() => Themes.Start([{}]);
}}
",
            names.join(", ")
        ),
        bytes: None,
    });
    files
}

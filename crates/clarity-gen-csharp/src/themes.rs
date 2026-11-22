//! `AppThemes` of a C# application (manual/themes.md): the themes it offers,
//! each resolved as every stack resolves it (clarity_gen_core::themes).

use clarity_spec::Project;

use super::HEADER;
use super::names::string;
use clarity_gen_core::themes::{offered, resolved_files};
use clarity_gen_core::OutFile;

/// Each offered theme for light and dark mode, and `AppThemes`, which tells
/// the runtime the themes offered. Without any, Clarity's own is offered.
pub fn theme_files(project: &Project, namespace: &str) -> Vec<OutFile> {
    let themes = offered(project);
    let mut files = resolved_files(project, "Themes");
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

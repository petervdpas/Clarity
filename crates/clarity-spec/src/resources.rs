//! The resources of a project: the files of its `resources` folder, which
//! every generated application carries. Pictures, which
//! `image source "logo.png"` shows, and Markdown files, which
//! `markdown source "rules.md"` shows: `rules.md` in the default language
//! and one file per other language beside it, `rules.nl.md`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::model::{Project, Widget};
use crate::widgets::WidgetKind;

/// The folder of resources in a project's folder.
pub const RESOURCES_DIR: &str = "resources";

/// The extension of a Markdown document.
pub const MARKDOWN_EXTENSION: &str = "md";

/// The kinds of picture every stack shows without extra libraries: the
/// web renderer, and Avalonia through Skia.
pub const PICTURE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// A file of the resources folder that the application carries as it is,
/// such as a picture. The renderer knows it by name; the bytes go into
/// generated applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ResourceFile {
    pub name: String,
    #[serde(skip)]
    #[ts(skip)]
    pub bytes: Vec<u8>,
}

/// What a project's resources folder holds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[ts(export)]
pub struct Resources {
    /// The files the application carries as they are, such as pictures, by name.
    pub files: Vec<ResourceFile>,
    /// The Markdown files, by name (`rules.md`), each by language: the
    /// default language's from `rules.md`, Dutch from `rules.nl.md`.
    pub markdown: BTreeMap<String, BTreeMap<String, String>>,
}

impl Resources {
    /// The pictures among the files, by name.
    pub fn pictures(&self) -> impl Iterator<Item = &ResourceFile> {
        self.files.iter().filter(|f| is_picture(&f.name))
    }
}

fn has_extension(name: &str, wanted: &[&str]) -> bool {
    Path::new(name).extension().is_some_and(|e| wanted.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

/// Whether a file name is a picture every stack can show.
pub fn is_picture(name: &str) -> bool {
    has_extension(name, &PICTURE_EXTENSIONS)
}

/// Whether a file name is a Markdown document.
pub fn is_markdown(name: &str) -> bool {
    has_extension(name, &[MARKDOWN_EXTENSION])
}

/// The files of the resources folder in `dir`, by name; none when there is
/// no folder. Hidden files and folders are left out.
pub fn resource_files(dir: &Path) -> std::io::Result<Option<Vec<ResourceFile>>> {
    let folder = dir.join(RESOURCES_DIR);
    if !folder.is_dir() {
        return Ok(None);
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_file() && !name.starts_with('.') {
            found.push(ResourceFile { bytes: std::fs::read(&path)?, name });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(found))
}

/// The document a Markdown file is a version of, and its language:
/// `rules.nl.md` is `rules.md` in `nl` when the project has that language,
/// and any other file is its own document in the default language.
fn document_of<'a>(name: &'a str, languages: &[&str], default: &'a str) -> (String, &'a str) {
    let stem = &name[..name.len() - MARKDOWN_EXTENSION.len() - 1];
    if let Some((base, language)) = stem.rsplit_once('.')
        && language != default
        && languages.contains(&language)
    {
        return (format!("{base}.{MARKDOWN_EXTENSION}"), language);
    }
    (name.to_string(), default)
}

/// Gives the project its resources: its Markdown files by name and language,
/// and the files it carries. Checks that every picture and document a
/// window shows is there, each document in every language.
pub fn attach(project: &mut Project, files: Vec<ResourceFile>) {
    let default = crate::translations::default_language(project);
    let languages: Vec<String> = project.translations.keys().cloned().collect();
    let language_refs: Vec<&str> = languages.iter().map(String::as_str).collect();
    let mut markdown: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut carried = Vec::new();
    for file in files {
        if is_markdown(&file.name) {
            let (document, language) = document_of(&file.name, &language_refs, &default);
            markdown.entry(document).or_default().insert(language.to_string(), String::from_utf8_lossy(&file.bytes).into_owned());
        } else {
            carried.push(file);
        }
    }
    let pictures: Vec<&str> = carried.iter().map(|f| f.name.as_str()).filter(|n| is_picture(n)).collect();
    let markdown_names: Vec<&str> = markdown.keys().map(String::as_str).collect();
    let mut found = Vec::new();
    for file in &project.files {
        for page in &file.pages {
            each_widget(&page.widgets, &mut |widget| {
                let Some(source) = widget.props.source.as_deref() else { return };
                let message = match widget.props.kind {
                    WidgetKind::Image if !is_picture(source) => {
                        format!("'{source}' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp or .webp file")
                    }
                    WidgetKind::Image if pictures.contains(&source) => return,
                    WidgetKind::Image if pictures.is_empty() => format!("image '{source}' is not in the resources folder, which has no pictures yet"),
                    WidgetKind::Image => format!("image '{source}' is not in the resources folder; it has {}", pictures.join(", ")),
                    WidgetKind::Markdown if !is_markdown(source) => format!("'{source}' is not a Markdown file; use a .md file"),
                    WidgetKind::Markdown if markdown_names.is_empty() => {
                        format!("markdown '{source}' is not in the resources folder, which has no Markdown files yet")
                    }
                    WidgetKind::Markdown if !markdown_names.contains(&source) => {
                        format!("markdown '{source}' is not in the resources folder; it has {}", markdown_names.join(", "))
                    }
                    WidgetKind::Markdown => {
                        let have = &markdown[source];
                        let Some(language) = languages.iter().find(|l| !have.contains_key(*l)) else { return };
                        let stem = &source[..source.len() - MARKDOWN_EXTENSION.len() - 1];
                        format!(
                            "markdown '{source}' has no {language} file; add {RESOURCES_DIR}/{stem}.{language}.{MARKDOWN_EXTENSION}, as every language has each Markdown file"
                        )
                    }
                    _ => return,
                };
                found.push(Diagnostic::error(message, widget.span).in_file(&file.path));
            });
        }
    }
    project.diagnostics.extend(found);
    project.resources = Some(Resources { files: carried, markdown });
}

fn each_widget(widgets: &[Widget], found: &mut impl FnMut(&Widget)) {
    for widget in widgets {
        found(widget);
        each_widget(&widget.children, found);
        for page in &widget.pages {
            each_widget(&page.children, found);
        }
    }
}

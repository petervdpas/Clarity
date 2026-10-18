//! The fixed pictures of a project: the files of its `images` folder, which
//! `image source "logo.png"` shows and every generated application carries.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::model::{Project, Widget};
use crate::widgets::WidgetKind;

/// The folder of pictures in a project's folder.
pub const IMAGES_DIR: &str = "images";

/// The kinds of picture every stack shows without extra libraries: the
/// web renderer, and Avalonia through Skia.
pub const PICTURE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// A file of the images folder. The renderer knows it by name; the bytes
/// go into generated applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProjectImage {
    pub name: String,
    #[serde(skip)]
    #[ts(skip)]
    pub bytes: Vec<u8>,
}

/// Whether a file name is a picture every stack can show.
pub fn is_picture(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| PICTURE_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

/// The files of the images folder in `dir`, by name; none when there is no
/// folder. Hidden files and folders are left out.
pub fn image_files(dir: &Path) -> std::io::Result<Option<Vec<ProjectImage>>> {
    let folder = dir.join(IMAGES_DIR);
    if !folder.is_dir() {
        return Ok(None);
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_file() && !name.starts_with('.') {
            found.push(ProjectImage { bytes: std::fs::read(&path)?, name });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(found))
}

/// Gives the project its pictures, and checks that every fixed picture a
/// window shows is one of them.
pub fn attach(project: &mut Project, images: Vec<ProjectImage>) {
    let names: Vec<&str> = images.iter().map(|i| i.name.as_str()).collect();
    let mut found = Vec::new();
    for file in &project.files {
        for page in &file.pages {
            each_widget(&page.widgets, &mut |widget| {
                let Some(source) = widget.props.source.as_deref().filter(|_| widget.props.kind == WidgetKind::Image) else { return };
                let message = if !is_picture(source) {
                    format!("'{source}' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp or .webp file")
                } else if names.contains(&source) {
                    return;
                } else if names.is_empty() {
                    format!("image '{source}' is not in the images folder, which has no pictures yet")
                } else {
                    format!("image '{source}' is not in the images folder; it has {}", names.join(", "))
                };
                found.push(Diagnostic::error(message, widget.span).in_file(&file.path));
            });
        }
    }
    project.diagnostics.extend(found);
    project.images = Some(images);
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

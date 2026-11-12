//! The resources of a project: the files of its `resources` folder, which
//! every generated application carries, each by its own name. A part shows
//! one by its `source` (`image source "logo.png"`, `markdown source
//! "rules.md"`), or by a key whose value in each language points at a
//! resource (`gameRules resource "rules-nl.md"`), as `markdown @gameRules`.

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
/// web renderer, and Clarity.UI through Skia.
pub const PICTURE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg"];

/// The sound files every stack plays (DESIGN 5.22): the browser engines of
/// the web stacks and C# read these alike.
pub const SOUND_EXTENSIONS: [&str; 4] = ["wav", "mp3", "flac", "ogg"];

/// Videos every stack plays (docs/VIDEO-PLAN.md).
pub const VIDEO_EXTENSIONS: [&str; 1] = ["mp4"];

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
    /// Every file, by name, as the application carries it.
    pub files: Vec<ResourceFile>,
    /// The text of each Markdown file, by name, for the renderers.
    pub markdown: BTreeMap<String, String>,
}

impl Resources {
    /// The pictures among the files, by name.
    pub fn pictures(&self) -> impl Iterator<Item = &ResourceFile> {
        self.files.iter().filter(|f| is_picture(&f.name))
    }
}

/// The type of a resource, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ResourceKind {
    /// Text in Markdown, which a markdown part shows and Clarity edits.
    Markdown,
    /// A picture every stack shows, which an image shows.
    Picture,
    Pdf,
    /// A vector drawing in DXF, which no part shows yet; an SVG is a picture.
    Drawing,
    /// A sound every stack plays, which a sound part plays.
    Sound,
    /// A video every stack plays, which a video part plays.
    Video,
    /// Any other file, carried as it is.
    File,
}

/// A type a resource can have: its name, and its extensions, the first the
/// one a new file of the type gets. Other files take any extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ResourceKindInfo {
    pub kind: ResourceKind,
    pub name: String,
    pub extensions: Vec<String>,
}

/// The types of resource, in the order they are offered.
pub fn kinds() -> Vec<ResourceKindInfo> {
    let kind = |kind, name: &str, extensions: &[&str]| ResourceKindInfo { kind, name: name.into(), extensions: extensions.iter().map(|e| e.to_string()).collect() };
    vec![
        kind(ResourceKind::Markdown, "Markdown", &[MARKDOWN_EXTENSION]),
        kind(ResourceKind::Picture, "Picture", &PICTURE_EXTENSIONS),
        kind(ResourceKind::Pdf, "PDF", &["pdf"]),
        kind(ResourceKind::Drawing, "Drawing", &["dxf"]),
        kind(ResourceKind::Sound, "Sound", &SOUND_EXTENSIONS),
        kind(ResourceKind::Video, "Video", &VIDEO_EXTENSIONS),
        kind(ResourceKind::File, "Other file", &[]),
    ]
}

/// The type of a resource by its file name.
pub fn kind_of(name: &str) -> ResourceKind {
    let extension = Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    kinds().into_iter().find(|k| k.extensions.contains(&extension)).map_or(ResourceKind::File, |k| k.kind)
}

fn has_extension(name: &str, wanted: &[&str]) -> bool {
    Path::new(name).extension().is_some_and(|e| wanted.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

/// Whether a file name is a picture every stack can show.
pub fn is_picture(name: &str) -> bool {
    has_extension(name, &PICTURE_EXTENSIONS)
}

/// Whether a file name is a sound every stack can play.
pub fn is_sound(name: &str) -> bool {
    has_extension(name, &SOUND_EXTENSIONS)
}

/// Whether a file name is a video every stack can play.
pub fn is_video(name: &str) -> bool {
    has_extension(name, &VIDEO_EXTENSIONS)
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

/// Gives the project its resources: every file, and the text of its
/// Markdown files by name. Checks that every picture and Markdown file a
/// window shows is there, by its `source` or by a key that points each
/// language at a resource.
pub fn attach(project: &mut Project, mut files: Vec<ResourceFile>) {
    files.sort_by(|a, b| a.name.cmp(&b.name));
    let markdown: BTreeMap<String, String> =
        files.iter().filter(|f| is_markdown(&f.name)).map(|f| (f.name.clone(), String::from_utf8_lossy(&f.bytes).into_owned())).collect();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    let pictures: Vec<&str> = names.iter().copied().filter(|n| is_picture(n)).collect();
    let documents: Vec<&str> = markdown.keys().map(String::as_str).collect();
    let sounds: Vec<&str> = names.iter().copied().filter(|n| is_sound(n)).collect();
    let videos: Vec<&str> = names.iter().copied().filter(|n| is_video(n)).collect();
    let mut found = Vec::new();
    for file in &project.files {
        for page in &file.pages {
            each_widget(&page.widgets, &mut |widget| {
                let kind = widget.props.kind;
                let mut problems = Vec::new();
                if let Some(source) = widget.props.source.as_deref() {
                    problems.extend(source_problem(kind, source, &pictures, &documents, &sounds, &videos));
                }
                if let Some(key) = widget.props.caption.as_deref().and_then(crate::translations::key_of) {
                    problems.extend(key_problems(project, kind, key, &names));
                }
                for message in problems {
                    found.push(Diagnostic::error(message, widget.span).in_file(&file.path));
                }
            });
        }
    }
    // The pictures canvases draw by name, written out in their draw handlers.
    for file in &project.files {
        for block in &file.code {
            for handler in &block.handlers {
                crate::code::check::each_stmt(&handler.body, &mut |stmt| {
                    if let crate::code::StmtKind::Draw { canvas, shape, args } = &stmt.kind
                        && shape.name == "image"
                        && let Some(crate::code::Expr { kind: crate::code::ExprKind::Str { value }, span }) = args.get(4)
                        && let Some(message) = named_problem(&format!("{}.image draws", canvas.name), value, &pictures)
                    {
                        found.push(Diagnostic::error(message, *span).in_file(&file.path));
                    }
                });
            }
        }
    }
    // Picture constants, of the module and of its engines.
    for file in &project.files {
        let consts = file.consts.iter().chain(file.engines.iter().filter_map(|e| e.body.as_ref()).flat_map(|b| &b.consts));
        for decl in consts.filter(|c| c.picture) {
            if let crate::code::ExprKind::Str { value } = &decl.value.kind
                && let Some(message) = named_problem(&format!("const {} is", decl.name.name), value, &pictures)
            {
                found.push(Diagnostic::error(message, decl.value.span).in_file(&file.path));
            }
        }
    }
    project.diagnostics.extend(found);
    project.resources = Some(Resources { files, markdown });
}

/// Why a file is no picture, after its name.
const NOT_A_PICTURE: &str = "is not a picture every stack can show; use a .png, .jpg, .gif, .bmp, .webp or .svg file";

/// What is wrong with a picture named in code: one a canvas draws
/// (`field.image(..., "pill.svg")`, `said` "field.image draws") or a
/// picture constant's (`const pill: picture = "pill.svg"`).
fn named_problem(said: &str, picture: &str, pictures: &[&str]) -> Option<String> {
    Some(if !is_picture(picture) {
        format!("{said} '{picture}', which {NOT_A_PICTURE}")
    } else if pictures.contains(&picture) {
        return None;
    } else if pictures.is_empty() {
        format!("{said} '{picture}', which is not in the resources folder, which has no pictures yet")
    } else {
        format!("{said} '{picture}', which is not in the resources folder; it has {}", pictures.join(", "))
    })
}

/// What is wrong with the file a part names as its `source`.
fn source_problem(kind: WidgetKind, source: &str, pictures: &[&str], documents: &[&str], sounds: &[&str], videos: &[&str]) -> Option<String> {
    Some(match kind {
        WidgetKind::Sound if !is_sound(source) => format!("'{source}' is not a sound every stack can play; use a .wav, .mp3, .flac or .ogg file"),
        WidgetKind::Sound if sounds.contains(&source) => return None,
        WidgetKind::Sound if sounds.is_empty() => format!("sound '{source}' is not in the resources folder, which has no sounds yet"),
        WidgetKind::Sound => format!("sound '{source}' is not in the resources folder; it has {}", sounds.join(", ")),
        WidgetKind::Video if !is_video(source) => format!("'{source}' is not a video every stack can play; use an .mp4 file"),
        WidgetKind::Video if names_contain(videos, source) => return None,
        WidgetKind::Video => format!("video '{source}' is not in the resources folder"),
        WidgetKind::Image if !is_picture(source) => format!("'{source}' {NOT_A_PICTURE}"),
        WidgetKind::Image if pictures.contains(&source) => return None,
        WidgetKind::Image if pictures.is_empty() => format!("image '{source}' is not in the resources folder, which has no pictures yet"),
        WidgetKind::Image => format!("image '{source}' is not in the resources folder; it has {}", pictures.join(", ")),
        WidgetKind::Markdown if !is_markdown(source) => format!("'{source}' is not a Markdown file; use a .md file"),
        WidgetKind::Markdown if documents.contains(&source) => return None,
        WidgetKind::Markdown if documents.is_empty() => format!("markdown '{source}' is not in the resources folder, which has no Markdown files yet"),
        WidgetKind::Markdown => format!("markdown '{source}' is not in the resources folder; it has {}", documents.join(", ")),
        _ => return None,
    })
}

/// What is wrong with what a part's key gives in each language: a markdown
/// shows a Markdown file, an image a picture, and every other part text.
fn key_problems(project: &Project, kind: WidgetKind, key: &str, names: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    for language in project.translations.keys() {
        let resource = project.resource_keys.get(language).and_then(|r| r.get(key));
        let has_text = project.translations[language].contains_key(key);
        match (kind, resource) {
            (WidgetKind::Image, None) if has_text => problems.push(format!(
                "@{key} is a text in {language}; an image shows a picture: point the key at one, as {key} resource \"logo.png\""
            )),
            (WidgetKind::Markdown, None) if has_text => problems.push(format!(
                "@{key} is a text in {language}; a markdown shows a Markdown file: point the key at one, as {key} resource \"rules.md\""
            )),
            (_, None) => {}
            (_, Some(file)) if !names.contains(&file.as_str()) => {
                problems.push(format!("@{key} points at {file} in {language}, which is not in the resources folder"))
            }
            (WidgetKind::Markdown, Some(file)) if !is_markdown(file) => {
                problems.push(format!("@{key} points at {file} in {language}; a markdown shows a Markdown file"))
            }
            (WidgetKind::Image, Some(file)) if !is_picture(file) => problems.push(format!("@{key} points at {file} in {language}; an image shows a picture")),
            (WidgetKind::Markdown | WidgetKind::Image, Some(_)) => {}
            (_, Some(file)) => problems.push(format!("@{key} points at {file} in {language}; a {} shows text", crate::widgets::keyword(kind))),
        }
    }
    problems
}

/// Every widget of `widgets`, with what they hold, in order.
/// Whether the project plays songs (docs/AUDIO-PLAN.md), which the desktop
/// plays through Clarity.Audio, so the generator adds it: a sound part whose
/// source is no WAV (mp3, flac, ogg), one wired `on end` or `on fail`, or
/// code that plays a sound a file, pauses, resumes or seeks it.
pub fn plays_songs(project: &Project) -> bool {
    let mut songs = false;
    let mut sounds = Vec::new();
    for file in &project.files {
        for page in &file.pages {
            each_widget(&page.widgets, &mut |widget| {
                if widget.props.kind != WidgetKind::Sound {
                    return;
                }
                songs |= widget.props.source.as_deref().is_some_and(|source| is_sound(source) && !has_extension(source, &["wav"]));
                songs |= !widget.props.handlers.is_empty();
                sounds.extend(widget.props.name.clone());
            });
        }
    }
    if songs || sounds.is_empty() {
        return songs;
    }
    for block in project.files.iter().flat_map(|f| &f.code) {
        for handler in &block.handlers {
            crate::code::check::each_stmt(&handler.body, &mut |stmt| {
                if let crate::code::StmtKind::Hear { part, hear, args } = &stmt.kind {
                    songs |= sounds.contains(&part.name) && (matches!(hear.name.as_str(), "pause" | "resume" | "seek") || (hear.name == "play" && !args.is_empty()));
                }
            });
        }
    }
    songs
}

/// Whether the project plays videos (docs/VIDEO-PLAN.md): a video part on
/// a window, which the desktop plays through Clarity.Video, so the generator
/// adds it.
pub fn plays_videos(project: &Project) -> bool {
    let mut videos = false;
    for file in &project.files {
        for page in &file.pages {
            each_widget(&page.widgets, &mut |widget| videos |= widget.props.kind == WidgetKind::Video);
        }
    }
    videos
}

/// Whether the program carries Clarity.Audio: it plays songs, or videos, whose sound plays through it.
pub fn carries_audio(project: &Project) -> bool {
    plays_songs(project) || plays_videos(project)
}

fn names_contain(names: &[&str], name: &str) -> bool {
    names.contains(&name)
}

/// Every part of `widgets`, at any depth, in the order `each_widget` visits them.
pub(crate) fn all_widgets<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
    for widget in widgets {
        out.push(widget);
        all_widgets(&widget.children, out);
        for page in &widget.pages {
            all_widgets(&page.children, out);
        }
    }
}

pub(crate) fn each_widget(widgets: &[Widget], found: &mut impl FnMut(&Widget)) {
    for widget in widgets {
        found(widget);
        each_widget(&widget.children, found);
        for page in &widget.pages {
            each_widget(&page.children, found);
        }
    }
}

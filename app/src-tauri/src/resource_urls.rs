//! The pictures and sounds of the open project's `resources` folder, for the
//! designer to show and play: each as a data URL, since the page cannot read
//! the project folder.

use std::path::Path;

use base64::Engine;
use clarity_spec::resources::{RESOURCES_DIR, is_picture, is_sound};
use tauri::State;

use super::{WatchState, open_project_of};

/// The media type of a picture or a sound, by its extension.
fn media_type(name: &str) -> &'static str {
    let extension = Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

/// The picture or sound `name` of the resources folder in `dir` as a data
/// URL. Only a picture or a sound directly in that folder is read.
pub fn resource_url(dir: &Path, name: &str) -> Result<String, String> {
    if name.contains(['/', '\\']) || name.starts_with('.') || !(is_picture(name) || is_sound(name)) {
        return Err(format!("'{name}' is not a picture or a sound of the resources folder"));
    }
    let path = dir.join(RESOURCES_DIR).join(name);
    let bytes = std::fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    Ok(format!("data:{};base64,{}", media_type(name), base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn project_resource(name: String, state: State<WatchState>) -> Result<String, String> {
    let (_, dir) = open_project_of(&state)?;
    resource_url(&dir, &name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_or_a_sound_is_read_as_a_data_url_from_the_resources_folder_only() {
        let dir = std::env::temp_dir().join(format!("clarity-pictures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(RESOURCES_DIR)).unwrap();
        std::fs::write(dir.join("resources/logo.png"), [1, 2, 3]).unwrap();
        std::fs::write(dir.join("secret.png"), [9]).unwrap();
        assert_eq!(resource_url(&dir, "logo.png").unwrap(), "data:image/png;base64,AQID");
        assert!(resource_url(&dir, "../secret.png").is_err());
        assert!(resource_url(&dir, "logo.txt").is_err());
        assert!(resource_url(&dir, "missing.png").is_err());
        std::fs::write(dir.join("resources/roll.wav"), [4, 5, 6]).unwrap();
        assert_eq!(resource_url(&dir, "roll.wav").unwrap(), "data:audio/wav;base64,BAUG");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

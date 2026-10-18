//! The pictures of the open project's `images` folder, for the designer to
//! show: each as a data URL, since the page cannot read the project folder.

use std::path::Path;

use base64::Engine;
use clarity_spec::images::{IMAGES_DIR, is_picture};
use tauri::State;

use super::{WatchState, open_project_of};

/// The media type of a picture, by its extension.
fn media_type(name: &str) -> &'static str {
    let extension = Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// The picture `name` of the images folder in `dir` as a data URL. Only a
/// picture directly in that folder is read.
pub fn picture_url(dir: &Path, name: &str) -> Result<String, String> {
    if name.contains(['/', '\\']) || name.starts_with('.') || !is_picture(name) {
        return Err(format!("'{name}' is not a picture of the images folder"));
    }
    let path = dir.join(IMAGES_DIR).join(name);
    let bytes = std::fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    Ok(format!("data:{};base64,{}", media_type(name), base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn project_picture(name: String, state: State<WatchState>) -> Result<String, String> {
    let (_, dir) = open_project_of(&state)?;
    picture_url(&dir, &name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_read_as_a_data_url_from_the_images_folder_only() {
        let dir = std::env::temp_dir().join(format!("clarity-pictures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(IMAGES_DIR)).unwrap();
        std::fs::write(dir.join("images/logo.png"), [1, 2, 3]).unwrap();
        std::fs::write(dir.join("secret.png"), [9]).unwrap();
        assert_eq!(picture_url(&dir, "logo.png").unwrap(), "data:image/png;base64,AQID");
        assert!(picture_url(&dir, "../secret.png").is_err());
        assert!(picture_url(&dir, "logo.txt").is_err());
        assert!(picture_url(&dir, "missing.png").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

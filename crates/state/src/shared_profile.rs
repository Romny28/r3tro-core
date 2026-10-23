//! The local profile any R3TRO COMPANY app can read: name, avatar and background.
//! It lives at `%LOCALAPPDATA%\R3TRO COMPANY\Shared\`, as `profile.json` plus whichever
//! picture files are actually set, so every app opens the same files rather than its own copy.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SharedProfile {
    #[serde(default)]
    pub name: String,
    /// The avatar's own file name inside the shared folder, empty when there is none.
    #[serde(default)]
    pub avatar: String,
    /// The background's own file name inside the shared folder, empty when there is none.
    #[serde(default)]
    pub background: String,
}

fn folder() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("R3TRO COMPANY").join("Shared"))
}

fn file() -> Option<PathBuf> {
    folder().map(|dir| dir.join("profile.json"))
}

/// Reads the shared profile, or an empty one if the folder, the file, or the computer's local
/// data directory itself is not there yet.
pub fn read() -> SharedProfile {
    file()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn write(profile: &SharedProfile) {
    let Some(dir) = folder() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(encoded) = serde_json::to_vec_pretty(profile) {
        fs::write(dir.join("profile.json"), encoded).ok();
    }
}

/// Copies a picture the user picked into the shared folder under a fixed name (`avatar.png`,
/// `background.jpg`…), replacing whatever was there before, so every app reading the shared
/// profile can open the picture without needing access to wherever it first came from. Returns
/// the picture's own file name inside the folder, which is what `SharedProfile` stores.
pub fn adopt_picture(source: &Path, stem: &str) -> Option<String> {
    let dir = folder()?;
    fs::create_dir_all(&dir).ok()?;
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("png");
    let name = format!("{stem}.{extension}");
    for existing in ["png", "jpg", "jpeg", "gif", "webp", "bmp"] {
        if existing != extension {
            fs::remove_file(dir.join(format!("{stem}.{existing}"))).ok();
        }
    }
    fs::copy(source, dir.join(&name)).ok()?;
    Some(name)
}

/// The absolute path to a picture the shared profile names, for anything that needs to open it
/// (a stored file name is relative to the shared folder, not a path on its own).
pub fn picture_path(name: &str) -> Option<PathBuf> {
    (!name.is_empty()).then(|| folder()).flatten().map(|dir| dir.join(name))
}

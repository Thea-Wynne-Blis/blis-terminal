use std::{
    fs::{self, Metadata},
    io,
    path::Path,
};

use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct PathCandidate {
    pub path: String,
    pub directory: bool,
}

pub(crate) fn complete(
    parent: &Path,
    path_prefix: &str,
    name_prefix: &str,
    separator: char,
    directories_only: bool,
) -> io::Result<Vec<PathCandidate>> {
    let mut candidates = Vec::new();
    let key = if cfg!(windows) {
        name_prefix.to_lowercase()
    } else {
        name_prefix.to_owned()
    };
    for entry in fs::read_dir(parent)? {
        let Ok(entry) = entry else { continue };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.chars().any(char::is_control) {
            continue;
        }
        let matches = if cfg!(windows) {
            name.to_lowercase().starts_with(&key)
        } else {
            name.starts_with(&key)
        };
        if !matches || (name.starts_with('.') && !name_prefix.starts_with('.')) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if name_prefix.is_empty() && is_hidden(&metadata) {
            continue;
        }
        let directory =
            metadata.is_dir() || (metadata.file_type().is_symlink() && entry.path().is_dir());
        if directories_only && !directory {
            continue;
        }
        let mut path = format!("{path_prefix}{name}");
        if directory {
            path.push(separator);
        }
        candidates.push(PathCandidate { path, directory });
    }
    candidates.sort_by_cached_key(|entry| entry.path.to_lowercase());
    Ok(candidates)
}

#[cfg(windows)]
pub(crate) fn is_hidden(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
pub(crate) fn is_hidden(_: &Metadata) -> bool {
    false
}

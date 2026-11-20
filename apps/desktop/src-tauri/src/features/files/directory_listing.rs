use std::{
    fs::{self, Metadata},
    io,
    path::Path,
    time::UNIX_EPOCH,
};

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum EntryKind {
    Directory,
    File,
    Link,
    Other,
}

#[derive(Serialize)]
pub(crate) struct ListingEntry {
    name: String,
    kind: EntryKind,
    bytes: Option<u64>,
    modified_ms: Option<i64>,
}

pub(crate) fn list(
    path: &Path,
    all: bool,
    detailed: bool,
) -> io::Result<(String, Option<Vec<ListingEntry>>)> {
    let mut rows = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !all && name.starts_with('.') {
            continue;
        }
        let metadata = entry.metadata()?;
        if !all && is_hidden(&metadata) {
            continue;
        }
        let kind = if metadata.is_dir() {
            EntryKind::Directory
        } else if metadata.is_file() {
            EntryKind::File
        } else if metadata.file_type().is_symlink() {
            EntryKind::Link
        } else {
            EntryKind::Other
        };
        let suffix = if matches!(kind, EntryKind::Directory) {
            "/"
        } else {
            ""
        };
        let name = name.replace('\r', "\\r").replace('\n', "\\n");
        rows.push(ListingEntry {
            name: format!("{name}{suffix}"),
            bytes: (detailed && matches!(kind, EntryKind::File)).then_some(metadata.len()),
            modified_ms: if detailed {
                modified_ms(&metadata)
            } else {
                None
            },
            kind,
        });
    }
    rows.sort_by_cached_key(|entry| entry.name.to_lowercase());
    if rows.is_empty() {
        return Ok(("目录中没有可显示的项目。".to_owned(), None));
    }
    if detailed {
        return Ok((String::new(), Some(rows)));
    }
    Ok((
        rows.into_iter()
            .map(|row| row.name)
            .collect::<Vec<_>>()
            .join("\n"),
        None,
    ))
}

fn modified_ms(metadata: &Metadata) -> Option<i64> {
    Some(match metadata.modified().ok()?.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).ok()?,
        Err(before) => -i64::try_from(before.duration().as_millis()).ok()?,
    })
}

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
    all: bool,
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
        if !matches || (!all && name.starts_with('.') && !name_prefix.starts_with('.')) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !all && name_prefix.is_empty() && is_hidden(&metadata) {
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
fn is_hidden(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
fn is_hidden(_: &Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_hidden_entries_and_file_details_only_when_requested() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        fs::create_dir(path.join("项目 文件")).unwrap();
        fs::write(path.join("项目笔记.txt"), b"12345").unwrap();
        fs::write(path.join(".隐藏.txt"), b"x").unwrap();
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN,
            };
            let hidden = path.join("Windows 隐藏属性.txt");
            fs::write(&hidden, b"x").unwrap();
            let name: Vec<_> = hidden.as_os_str().encode_wide().chain([0]).collect();
            assert_ne!(
                unsafe { SetFileAttributesW(name.as_ptr(), FILE_ATTRIBUTE_HIDDEN) },
                0
            );
        }
        let (plain, _) = list(path, false, false).unwrap();
        assert!(plain.contains("项目 文件/"));
        assert!(!plain.contains("隐藏"));
        let (_, rows) = list(path, true, true).unwrap();
        let rows = rows.unwrap();
        assert!(rows.iter().any(|row| row.name == ".隐藏.txt"));
        #[cfg(windows)]
        assert!(rows.iter().any(|row| row.name == "Windows 隐藏属性.txt"));
        let file = rows.iter().find(|row| row.name == "项目笔记.txt").unwrap();
        assert_eq!(file.bytes, Some(5));
        assert!(file.modified_ms.is_some());

        let both = complete(path, "../", "项目", '/', false, false).unwrap();
        assert_eq!(both.len(), 2);
        let only_dirs = complete(path, "../", "项目", '/', true, false).unwrap();
        assert_eq!(only_dirs.len(), 1);
        assert_eq!(only_dirs[0].path, "../项目 文件/");
        assert!(only_dirs[0].directory);
        assert_eq!(
            complete(path, "", ".隐", '/', false, false).unwrap()[0].path,
            ".隐藏.txt"
        );
    }
}

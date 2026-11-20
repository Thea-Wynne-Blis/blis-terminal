use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;
use tempfile::NamedTempFile;

const PREVIEW_LIMIT: usize = 32 * 1024;
const EDIT_LIMIT: usize = 512 * 1024;
const BOM: &[u8] = b"\xef\xbb\xbf";

#[derive(Serialize)]
pub(super) struct Draft {
    id: u32,
    path: String,
    content: String,
}

pub(super) struct Editor {
    pub(super) id: u32,
    path: PathBuf,
    original: Option<Vec<u8>>,
}

pub(super) fn preview(path: &Path) -> Result<String, String> {
    let target = dunce::canonicalize(path).map_err(|error| file_error(path, error))?;
    let mut bytes = read_prefix(&target, PREVIEW_LIMIT)?;
    let truncated = bytes.len() > PREVIEW_LIMIT;
    bytes.truncate(PREVIEW_LIMIT);
    let text = decode_text(&bytes, truncated)?;
    Ok(if truncated {
        format!("{text}\n…（仅显示文件开头）")
    } else if text.is_empty() {
        "（空文件）".to_owned()
    } else {
        text.to_owned()
    })
}

impl Editor {
    pub(super) fn open(id: u32, path: &Path) -> Result<(Self, Draft), String> {
        let (path, original) = match fs::symlink_metadata(path) {
            Ok(_) => {
                let target = dunce::canonicalize(path).map_err(|error| file_error(path, error))?;
                let bytes = read_prefix(&target, EDIT_LIMIT)?;
                check_edit_size(bytes.len())?;
                (target, Some(bytes))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = path.file_name().ok_or("请输入新文件的名称。")?;
                let parent = path.parent().ok_or("找不到新文件的上级目录。")?;
                let parent =
                    dunce::canonicalize(parent).map_err(|error| file_error(parent, error))?;
                if !parent.is_dir() {
                    return Err("新文件的上级路径必须是目录。".to_owned());
                }
                (parent.join(name), None)
            }
            Err(error) => return Err(file_error(path, error)),
        };
        let content =
            normalize_newlines(decode_text(original.as_deref().unwrap_or_default(), false)?);
        let draft = Draft {
            id,
            path: path.display().to_string(),
            content,
        };
        Ok((Self { id, path, original }, draft))
    }

    pub(super) fn save(&mut self, content: &str) -> Result<String, String> {
        check_edit_size(content.len())?;
        if content.contains('\0') {
            return Err("文本不能包含空字符。".to_owned());
        }
        let path = &self.path;
        let parent = path.parent().ok_or("找不到文件的上级目录。")?;
        let content = normalize_newlines(content);
        let mut permissions = None;
        let bytes = if let Some(original) = &self.original {
            let metadata = fs::symlink_metadata(path).map_err(|error| file_error(path, error))?;
            if !metadata.is_file() || read_prefix(path, EDIT_LIMIT)? != *original {
                return Err("文件已在外部修改。请保留编辑内容，重新打开文件后合并。".to_owned());
            }
            let original_text = decode_text(original, false)?;
            if normalize_newlines(original_text) == content {
                return Ok(format!("已保存「{}」。", path.display()));
            }
            if metadata.permissions().readonly() {
                return Err("该文件为只读。".to_owned());
            }
            permissions = Some(metadata.permissions());
            encode_text(&content, original, original_text)
        } else {
            content.into_bytes()
        };
        check_edit_size(bytes.len())?;

        let write = || -> io::Result<()> {
            let mut temporary = NamedTempFile::new_in(parent)?;
            temporary.write_all(&bytes)?;
            if let Some(permissions) = permissions {
                temporary.as_file().set_permissions(permissions)?;
            }
            temporary.as_file().sync_all()?;
            if self.original.is_some() {
                temporary.persist(path)?;
            } else {
                temporary.persist_noclobber(path)?;
            }
            Ok(())
        };
        write().map_err(|error| file_error(path, error))?;
        self.original = Some(bytes);
        Ok(format!("已保存「{}」。", path.display()))
    }
}

fn read_prefix(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let result = (|| -> io::Result<Vec<u8>> {
        if !fs::metadata(path)?.is_file() {
            return Err(io::Error::other("路径不是普通文件"));
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    })();
    result.map_err(|error| file_error(path, error))
}

fn decode_text(bytes: &[u8], truncated: bool) -> Result<&str, String> {
    let invalid = "只支持 UTF-8 文本，无法读取该文件。";
    let bytes = bytes.strip_prefix(BOM).unwrap_or(bytes);
    if bytes.contains(&0) {
        return Err(invalid.to_owned());
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) if truncated && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()]).map_err(|_| invalid.to_owned())
        }
        Err(_) => Err(invalid.to_owned()),
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn encode_text(content: &str, original: &[u8], original_text: &str) -> Vec<u8> {
    let ending = original_text.find(['\r', '\n']).map_or("\n", |index| {
        if original_text[index..].starts_with("\r\n") {
            "\r\n"
        } else if original_text[index..].starts_with('\r') {
            "\r"
        } else {
            "\n"
        }
    });
    let mut bytes = Vec::new();
    if original.starts_with(BOM) {
        bytes.extend_from_slice(BOM);
    }
    bytes.extend(content.replace('\n', ending).into_bytes());
    bytes
}

fn check_edit_size(bytes: usize) -> Result<(), String> {
    if bytes > EDIT_LIMIT {
        return Err("内置编辑器支持最多 512 KiB 的 UTF-8 文本。".to_owned());
    }
    Ok(())
}

fn file_error(path: &Path, error: io::Error) -> String {
    format!("无法访问文件「{}」（{error}）。", path.display())
}

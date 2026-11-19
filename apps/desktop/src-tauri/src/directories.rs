use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::desktop_open::open_with_system;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DirectoryOperation {
    Pwd,
    Cd { path: Option<String> },
    Ls { path: Option<String> },
    OpenCurrent,
}

#[derive(Serialize)]
pub struct DirectoryOutcome {
    directory: String,
    text: String,
}

pub struct DirectoryStore {
    home: PathBuf,
    directory: Mutex<PathBuf>,
}

impl DirectoryStore {
    pub fn new(home: PathBuf) -> Self {
        let directory = dunce::canonicalize(&home).unwrap_or_else(|_| home.clone());
        Self {
            home,
            directory: Mutex::new(directory),
        }
    }

    fn execute(&self, operation: DirectoryOperation) -> Result<DirectoryOutcome, String> {
        let mut directory = self.directory.lock().map_err(|_| "当前目录暂时不可用。")?;
        let text = match operation {
            DirectoryOperation::Pwd => directory.display().to_string(),
            DirectoryOperation::Cd { path } => {
                let destination = self.resolve(&directory, path.as_deref().unwrap_or("~"))?;
                *directory = require_directory(&destination)?;
                String::new()
            }
            DirectoryOperation::Ls { path } => {
                let target = self.resolve(&directory, path.as_deref().unwrap_or("."))?;
                let target = require_directory(&target)?;
                list_directory(&target).map_err(|error| directory_error(&target, error))?
            }
            DirectoryOperation::OpenCurrent => {
                let target = require_directory(&directory)?;
                open_with_system(target.as_os_str())
                    .map_err(|error| format!("无法打开当前目录（{error}）。"))?;
                "已交给系统打开当前目录。".to_owned()
            }
        };
        Ok(DirectoryOutcome {
            directory: directory.display().to_string(),
            text,
        })
    }

    fn resolve(&self, current: &Path, input: &str) -> Result<PathBuf, String> {
        if input.is_empty() || input.chars().any(char::is_control) {
            return Err("请输入有效的目录路径。".to_owned());
        }
        let path = Path::new(input);
        if let Ok(relative) = path.strip_prefix("~") {
            return Ok(self.home.join(relative));
        }

        #[cfg(windows)]
        if !path.has_root()
            && matches!(
                path.components().next(),
                Some(std::path::Component::Prefix(_))
            )
        {
            if input.len() == 2 {
                return Ok(PathBuf::from(format!("{input}\\")));
            }
            return Err("跨盘路径请从盘符根目录写起，例如 D:\\项目。".to_owned());
        }

        Ok(current.join(path))
    }
}

fn list_directory(path: &Path) -> io::Result<String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let metadata = entry.metadata()?;
        if is_hidden(&metadata) {
            continue;
        }
        let mut name = name.replace('\r', "\\r").replace('\n', "\\n");
        if metadata.is_dir() {
            name.push('/');
        }
        names.push(name);
    }
    names.sort_by_cached_key(|name| name.to_lowercase());
    if names.is_empty() {
        return Ok("目录中没有可显示的项目。".to_owned());
    }
    Ok(names.join("\n"))
}

#[cfg(windows)]
fn is_hidden(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
fn is_hidden(_: &fs::Metadata) -> bool {
    false
}

fn require_directory(path: &Path) -> Result<PathBuf, String> {
    let resolved = dunce::canonicalize(path).map_err(|error| directory_error(path, error))?;
    let metadata = fs::metadata(&resolved).map_err(|error| directory_error(&resolved, error))?;
    if !metadata.is_dir() {
        return Err(format!("「{}」不是目录。", resolved.display()));
    }
    Ok(resolved)
}

fn directory_error(path: &Path, error: io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        return format!("找不到目录「{}」。", path.display());
    }
    format!("无法访问目录「{}」（{error}）。", path.display())
}

#[tauri::command]
pub async fn execute_directory(
    app: AppHandle,
    operation: DirectoryOperation,
) -> Result<DirectoryOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<DirectoryStore>().execute(operation))
        .await
        .map_err(|_| "无法处理目录命令。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_directory_and_lists_relative_paths_without_moving_on_failure() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path();
        let projects = root.join("项目 文件");
        fs::create_dir_all(projects.join("子目录")).unwrap();
        fs::write(projects.join("说明.txt"), "test").unwrap();
        fs::write(projects.join(".hidden"), "test").unwrap();
        let store = DirectoryStore::new(root.to_path_buf());
        let cd = |path: &str| {
            store.execute(DirectoryOperation::Cd {
                path: Some(path.into()),
            })
        };
        let pwd = || store.execute(DirectoryOperation::Pwd).unwrap();

        let listed = store
            .execute(DirectoryOperation::Ls {
                path: Some("项目 文件".into()),
            })
            .unwrap();
        assert!(listed.text.contains("子目录/"));
        assert!(listed.text.contains("说明.txt"));
        assert!(!listed.text.contains(".hidden"));
        assert_eq!(listed.directory, pwd().directory);

        cd("项目 文件").unwrap();
        let projects = dunce::canonicalize(projects).unwrap();
        assert_eq!(pwd().text, projects.display().to_string());
        assert!(cd("说明.txt").is_err());
        assert!(cd("此处不存在").is_err());
        assert_eq!(pwd().text, projects.display().to_string());

        cd("子目录").unwrap();
        cd("..").unwrap();
        assert_eq!(pwd().text, projects.display().to_string());
        cd("~/项目 文件").unwrap();
        assert_eq!(pwd().text, projects.display().to_string());
        store
            .execute(DirectoryOperation::Cd { path: None })
            .unwrap();
        assert_eq!(
            pwd().text,
            dunce::canonicalize(root).unwrap().display().to_string()
        );
    }
}

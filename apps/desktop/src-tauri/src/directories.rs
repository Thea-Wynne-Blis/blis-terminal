use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{desktop_open::open_with_system, file_operations as files};

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DirectoryOperation {
    Pwd,
    Cd {
        path: Option<String>,
    },
    Ls {
        path: Option<String>,
    },
    OpenCurrent,
    Mkdir {
        path: String,
    },
    Copy {
        source: String,
        destination: String,
        recursive: bool,
    },
    Move {
        source: String,
        destination: String,
    },
    Remove {
        path: String,
        recursive: bool,
    },
    FinishRemove {
        id: u32,
        confirmed: bool,
    },
}

#[derive(Serialize)]
struct RemovalRequest {
    id: u32,
    path: String,
}

#[derive(Serialize)]
pub struct DirectoryOutcome {
    directory: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    removal: Option<RemovalRequest>,
}

struct DirectorySession {
    current: PathBuf,
    pending_removal: Option<(u32, files::Item)>,
    next_removal: u32,
}

pub struct DirectoryStore {
    home: PathBuf,
    session: Mutex<DirectorySession>,
}

impl DirectoryStore {
    pub fn new(home: PathBuf) -> Self {
        let directory = dunce::canonicalize(&home).unwrap_or_else(|_| home.clone());
        Self {
            home,
            session: Mutex::new(DirectorySession {
                current: directory,
                pending_removal: None,
                next_removal: 0,
            }),
        }
    }

    fn execute(&self, operation: DirectoryOperation) -> Result<DirectoryOutcome, String> {
        let mut session = self.session.lock().map_err(|_| "当前目录暂时不可用。")?;
        let mut removal = None;
        let text = match operation {
            DirectoryOperation::Pwd => session.current.display().to_string(),
            DirectoryOperation::Cd { path } => {
                let destination = self.resolve(&session.current, path.as_deref().unwrap_or("~"))?;
                session.current = require_directory(&destination)?;
                String::new()
            }
            DirectoryOperation::Ls { path } => {
                let target = self.resolve(&session.current, path.as_deref().unwrap_or("."))?;
                let target = require_directory(&target)?;
                list_directory(&target).map_err(|error| directory_error(&target, error))?
            }
            DirectoryOperation::OpenCurrent => {
                let target = require_directory(&session.current)?;
                open_with_system(target.as_os_str())
                    .map_err(|error| format!("无法打开当前目录（{error}）。"))?;
                "已交给系统打开当前目录。".to_owned()
            }
            DirectoryOperation::Mkdir { path } => {
                files::make_directory(&self.resolve(&session.current, &path)?)?
            }
            DirectoryOperation::Copy {
                source,
                destination,
                recursive,
            } => {
                let source = self.resolve(&session.current, &source)?;
                let destination = self.resolve(&session.current, &destination)?;
                files::copy(&source, &destination, recursive)?
            }
            DirectoryOperation::Move {
                source,
                destination,
            } => {
                let source = self.resolve(&session.current, &source)?;
                let destination = self.resolve(&session.current, &destination)?;
                files::move_item(&source, &destination, &session.current)?
            }
            DirectoryOperation::Remove { path, recursive } => {
                let target = files::Item::read(&self.resolve(&session.current, &path)?)?;
                target.check_movable(&session.current)?;
                if target.is_dir() && !recursive {
                    return Err("回收目录及其内容请使用 rm -r。".to_owned());
                }
                session.next_removal = session.next_removal.wrapping_add(1);
                let id = session.next_removal;
                let path = target.path.display().to_string();
                session.pending_removal = Some((id, target));
                let text = format!("等待确认将「{path}」送入回收站。");
                removal = Some(RemovalRequest { id, path });
                text
            }
            DirectoryOperation::FinishRemove { id, confirmed } => {
                let target = session
                    .pending_removal
                    .take_if(|(pending_id, _)| *pending_id == id)
                    .ok_or("该删除确认已失效，请重新执行 rm。")?
                    .1;
                if confirmed {
                    target.check_movable(&session.current)?;
                    target.ensure_unchanged()?;
                    files::recycle(&target)?
                } else {
                    format!("已取消删除「{}」。", target.path.display())
                }
            }
        };
        Ok(DirectoryOutcome {
            directory: session.current.display().to_string(),
            text,
            removal,
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
    #[cfg(windows)]
    fn creates_copies_and_moves_items_without_overwriting() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path();
        let store = DirectoryStore::new(root.into());
        let mkdir = || DirectoryOperation::Mkdir {
            path: "项目 备份".into(),
        };
        store.execute(mkdir()).unwrap();
        assert!(store.execute(mkdir()).err().unwrap().contains("已存在"));
        fs::write(root.join("计划 草稿.md"), "original").unwrap();
        let copy = || DirectoryOperation::Copy {
            source: "计划 草稿.md".into(),
            destination: "项目 备份".into(),
            recursive: false,
        };
        store.execute(copy()).unwrap();
        let copied = root.join("项目 备份/计划 草稿.md");
        assert_eq!(fs::read_to_string(&copied).unwrap(), "original");
        fs::write(&copied, "existing").unwrap();
        assert!(store.execute(copy()).err().unwrap().contains("已存在"));

        let mv = |destination: &str| DirectoryOperation::Move {
            source: "计划 草稿.md".into(),
            destination: destination.into(),
        };
        assert!(store
            .execute(mv("项目 备份"))
            .err()
            .unwrap()
            .contains("已存在"));
        assert_eq!(fs::read_to_string(&copied).unwrap(), "existing");
        store.execute(mv("完整 计划.md")).unwrap();
        assert!(!root.join("计划 草稿.md").exists());
        assert_eq!(
            fs::read_to_string(root.join("完整 计划.md")).unwrap(),
            "original"
        );

        let copy_dir = |destination: &str, recursive| DirectoryOperation::Copy {
            source: "项目 备份".into(),
            destination: destination.into(),
            recursive,
        };
        assert!(store.execute(copy_dir("另一份", false)).is_err());
        assert!(store.execute(copy_dir("项目 备份/自身", true)).is_err());
        store.execute(copy_dir("另一份", true)).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("另一份/计划 草稿.md")).unwrap(),
            "existing"
        );
    }

    #[test]
    fn removal_requires_confirmation_and_rejects_changed_targets() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path();
        let path = root.join("待回收 文件.txt");
        fs::write(&path, "original").unwrap();
        let store = DirectoryStore::new(root.into());
        let request = || DirectoryOperation::Remove {
            path: "待回收 文件.txt".into(),
            recursive: false,
        };
        let first = store.execute(request()).unwrap().removal.unwrap();
        assert!(path.exists());
        store
            .execute(DirectoryOperation::FinishRemove {
                id: first.id,
                confirmed: false,
            })
            .unwrap();
        assert!(path.exists());
        assert!(store
            .execute(DirectoryOperation::FinishRemove {
                id: first.id,
                confirmed: true
            })
            .is_err());
        let second = store.execute(request()).unwrap().removal.unwrap();
        fs::write(&path, "modified after confirmation request").unwrap();
        assert!(store
            .execute(DirectoryOperation::FinishRemove {
                id: second.id,
                confirmed: true
            })
            .err()
            .unwrap()
            .contains("目标已发生变化"));
        assert!(path.exists());
        assert!(store
            .execute(DirectoryOperation::Remove {
                path: ".".into(),
                recursive: true
            })
            .is_err());
        fs::create_dir(root.join("child")).unwrap();
        assert!(store
            .execute(DirectoryOperation::Remove {
                path: "child".into(),
                recursive: false
            })
            .err()
            .unwrap()
            .contains("rm -r"));
    }

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

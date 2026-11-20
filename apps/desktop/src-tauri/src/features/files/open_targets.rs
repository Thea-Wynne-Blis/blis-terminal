use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::platform::{
    app_start::{observe_launch, LaunchStatus},
    config_file::write_json,
    desktop_open::open_with_system,
    web_url::{open_in_browser, parse_web_url},
};

const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    App,
    Folder,
    File,
    Url,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    kind: TargetKind,
    target: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum OpenOperation {
    Add {
        alias: String,
        kind: TargetKind,
        target: String,
    },
    List,
    Remove {
        alias: String,
    },
    Launch {
        alias: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenOutcome {
    Added { alias: String },
    Listed { targets: BTreeMap<String, Target> },
    Removed { alias: String },
    Launched { alias: String, status: LaunchStatus },
}

pub struct OpenTargets {
    path: PathBuf,
    access: Mutex<()>,
}

impl OpenTargets {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: OpenOperation) -> Result<OpenOutcome, String> {
        let access = self.access.lock().map_err(|_| "打开清单暂时不可用。")?;
        let mut config = self.read_config()?;

        match operation {
            OpenOperation::Add {
                alias,
                kind,
                target,
            } => {
                let alias = normalize_alias(&alias)?;
                let Entry::Vacant(entry) = config.targets.entry(alias.clone()) else {
                    return Err(format!(
                        "别名「{alias}」已存在。请换一个别名，或先移除旧记录。"
                    ));
                };
                let target = if kind == TargetKind::Url {
                    parse_web_url(&target)?.to_string()
                } else {
                    validate_path(kind, &target)?;
                    target
                };
                entry.insert(Target { kind, target });
                self.save_config(&config)?;
                Ok(OpenOutcome::Added { alias })
            }
            OpenOperation::List => Ok(OpenOutcome::Listed {
                targets: config.targets,
            }),
            OpenOperation::Remove { alias } => {
                let alias = normalize_alias(&alias)?;
                config
                    .targets
                    .remove(&alias)
                    .ok_or_else(|| unknown_alias(&alias))?;
                self.save_config(&config)?;
                Ok(OpenOutcome::Removed { alias })
            }
            OpenOperation::Launch { alias } => {
                let alias = normalize_alias(&alias)?;
                let target = config
                    .targets
                    .get(&alias)
                    .ok_or_else(|| unknown_alias(&alias))?;
                drop(access);
                let status = launch_target(target)?;
                Ok(OpenOutcome::Launched { alias, status })
            }
        }
    }

    fn read_config(&self) -> Result<Config, String> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Config {
                    version: CONFIG_VERSION,
                    targets: BTreeMap::new(),
                });
            }
            Err(error) => return Err(format!("无法读取打开清单（{error}）。")),
        };
        let config: Config = serde_json::from_slice(&data)
            .map_err(|error| format!("打开清单的格式有误（{error}）。"))?;
        if config.version != CONFIG_VERSION {
            return Err(format!("无法读取版本为 {} 的打开清单。", config.version));
        }
        for alias in config.targets.keys() {
            if normalize_alias(alias).as_ref() != Ok(alias) {
                return Err(format!("打开清单中的别名「{alias}」格式有误。"));
            }
        }
        Ok(config)
    }

    fn save_config(&self, config: &Config) -> Result<(), String> {
        write_json(&self.path, config).map_err(|error| format!("无法保存打开清单（{error}）。"))
    }
}

fn normalize_alias(alias: &str) -> Result<String, &'static str> {
    if alias.chars().count() > 64
        || !alias.chars().next().is_some_and(char::is_alphanumeric)
        || !alias
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err("别名须以文字或数字开头，可包含短横线、下划线和点，最多 64 个字符。");
    }
    let alias = alias.to_ascii_lowercase();
    if matches!(alias.as_str(), "add" | "list" | "remove") {
        return Err("add、list 和 remove 是保留名称，请换一个别名。");
    }
    Ok(alias)
}

fn unknown_alias(alias: &str) -> String {
    format!("未找到别名「{alias}」。输入 open list 查看清单。")
}

pub(crate) fn validate_path(kind: TargetKind, value: &str) -> Result<&Path, String> {
    let path = Path::new(value);
    if !path.is_absolute() || value.chars().any(char::is_control) {
        return Err("请输入有效的绝对路径。".to_owned());
    }
    if kind == TargetKind::App
        && !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err("应用目标须是 .exe 文件。".to_owned());
    }

    let metadata = fs::metadata(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => "目标路径不存在。".to_owned(),
        _ => format!("无法访问目标路径（{error}）。"),
    })?;
    if kind == TargetKind::Folder && !metadata.is_dir() {
        return Err("目标路径须是文件夹。".to_owned());
    }
    if kind != TargetKind::Folder && !metadata.is_file() {
        return Err("目标路径须是文件。".to_owned());
    }
    Ok(path)
}

fn launch_target(target: &Target) -> Result<LaunchStatus, String> {
    if target.kind == TargetKind::Url {
        open_in_browser(&target.target)?;
        return Ok(LaunchStatus::SystemHandoff);
    }
    let path = validate_path(target.kind, &target.target)?;
    if target.kind == TargetKind::App {
        return observe_launch(spawn_app(path, &[])?);
    }
    open_with_system(path.as_os_str()).map_err(|error| format!("无法打开目标（{error}）。"))?;
    Ok(LaunchStatus::SystemHandoff)
}

pub(crate) fn spawn_app(path: &Path, args: &[&str]) -> Result<Child, String> {
    let mut app = Command::new(path);
    app.args(args);
    if let Some(directory) = path.parent() {
        app.current_dir(directory);
    }
    app.spawn()
        .map_err(|error| format!("无法启动应用（{error}）。"))
}

#[tauri::command]
pub async fn execute_open_target(
    app: AppHandle,
    operation: OpenOperation,
) -> Result<OpenOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<crate::features::storage::Storage>()
            .stores()?
            .open_targets
            .execute(operation)
    })
    .await
    .map_err(|_| "无法处理打开清单。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(alias: &str, kind: TargetKind, target: &str) -> OpenOperation {
        OpenOperation::Add {
            alias: alias.to_owned(),
            kind,
            target: target.to_owned(),
        }
    }

    #[test]
    fn restores_aliases_and_removes_only_the_saved_entry() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("config").join("open-targets.json");
        let file = directory.path().join("文档 1.txt");
        fs::write(&file, "sample").unwrap();

        let registry = OpenTargets::new(config_path.clone());
        assert!(matches!(registry.execute(OpenOperation::List).unwrap(),
            OpenOutcome::Listed { targets } if targets.is_empty()));
        assert!(!config_path.exists());
        registry
            .execute(add("Report", TargetKind::File, file.to_str().unwrap()))
            .unwrap();
        registry
            .execute(add("页面", TargetKind::Url, "https://example.com"))
            .unwrap();
        assert!(registry
            .execute(add("REPORT", TargetKind::Url, "https://other.example"))
            .is_err());
        drop(registry);

        let restored = OpenTargets::new(config_path.clone());
        let OpenOutcome::Listed { targets } = restored.execute(OpenOperation::List).unwrap() else {
            panic!("list did not return targets");
        };
        assert_eq!(targets["report"].target, file.to_str().unwrap());
        assert_eq!(targets["页面"].target, "https://example.com/");
        restored
            .execute(OpenOperation::Remove {
                alias: "REPORT".to_owned(),
            })
            .unwrap();
        assert!(file.is_file());
        assert!(!OpenTargets::new(config_path)
            .read_config()
            .unwrap()
            .targets
            .contains_key("report"));
    }

    #[test]
    fn rejects_wrong_types_relative_paths_and_reserved_aliases() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("open-targets.json");
        let store = OpenTargets::new(config_path.clone());
        let folder = directory.path().to_str().unwrap();
        let file = directory.path().join("text.txt");
        fs::write(&file, "sample").unwrap();

        for operation in [
            add("list", TargetKind::Url, "https://example.com"),
            add("test", TargetKind::Url, "file:///example.txt"),
            add("test", TargetKind::File, "relative.txt"),
            add("test", TargetKind::File, folder),
            add("test", TargetKind::Folder, file.to_str().unwrap()),
            add("test", TargetKind::App, file.to_str().unwrap()),
        ] {
            assert!(store.execute(operation).is_err());
        }
        assert!(!config_path.exists());
    }

    #[test]
    fn preserves_a_corrupt_or_newer_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("open-targets.json");
        let store = OpenTargets::new(path.clone());
        for original in ["invalid JSON", r#"{"version":2,"targets":{}}"#] {
            fs::write(&path, original).unwrap();
            assert!(store
                .execute(add("test", TargetKind::Url, "https://example.com"))
                .is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }
}

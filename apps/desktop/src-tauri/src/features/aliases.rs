use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs, io,
    path::PathBuf,
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::platform::config_file::write_json;

const VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    aliases: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AliasOperation {
    List,
    Add { name: String, target: String },
    Remove { name: String },
}

pub struct AliasStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl AliasStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: AliasOperation) -> Result<BTreeMap<String, String>, String> {
        let _access = self.access.lock().map_err(|_| "别名配置暂时不可用。")?;
        let mut config = self.read()?;
        match operation {
            AliasOperation::List => return Ok(config.aliases),
            AliasOperation::Add { name, target } => {
                let name = normalize_name(&name)?;
                validate_target(&target)?;
                let Entry::Vacant(entry) = config.aliases.entry(name.clone()) else {
                    return Err(format!(
                        "别名「{name}」已存在。请先用 alias remove 移除旧记录。"
                    ));
                };
                entry.insert(target);
            }
            AliasOperation::Remove { name } => {
                let name = normalize_name(&name)?;
                config
                    .aliases
                    .remove(&name)
                    .ok_or_else(|| format!("未找到别名「{name}」。输入 alias list 查看。"))?;
            }
        }
        write_json(&self.path, &config).map_err(|error| format!("无法保存别名（{error}）。"))?;
        Ok(config.aliases)
    }

    fn read(&self) -> Result<Config, String> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Config {
                    version: VERSION,
                    aliases: BTreeMap::new(),
                });
            }
            Err(error) => return Err(format!("无法读取别名（{error}）。")),
        };
        let config: Config = serde_json::from_slice(&data)
            .map_err(|error| format!("别名配置格式有误（{error}）。"))?;
        if config.version != VERSION {
            return Err(format!("无法读取版本为 {} 的别名配置。", config.version));
        }
        for (name, target) in &config.aliases {
            if normalize_name(name).as_ref() != Ok(name) || validate_target(target).is_err() {
                return Err(format!("别名「{name}」的配置格式有误。"));
            }
        }
        Ok(config)
    }
}

fn normalize_name(name: &str) -> Result<String, &'static str> {
    if name.chars().count() > 64
        || !name.chars().next().is_some_and(char::is_alphanumeric)
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("别名须以文字或数字开头，可包含短横线、下划线和点，最多 64 个字符。");
    }
    Ok(name.to_ascii_lowercase())
}

fn validate_target(target: &str) -> Result<(), &'static str> {
    if target.trim().is_empty()
        || target.chars().count() > 4096
        || target.chars().any(|c| c.is_control() && c != '\t')
    {
        return Err("请输入有效的别名目标命令。");
    }
    Ok(())
}

#[tauri::command]
pub async fn execute_alias(
    app: AppHandle,
    operation: AliasOperation,
) -> Result<BTreeMap<String, String>, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AliasStore>().execute(operation))
        .await
        .map_err(|_| "无法处理别名请求。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_removes_and_refuses_duplicate_aliases() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("aliases.json");
        let target = r#"search bilibili "Counter-Strike 2""#;
        let add = || AliasOperation::Add {
            name: "Bilibili".into(),
            target: target.into(),
        };
        let store = AliasStore::new(path.clone());
        assert!(store.execute(AliasOperation::List).unwrap().is_empty());
        assert_eq!(store.execute(add()).unwrap()["bilibili"], target);
        assert!(store.execute(add()).is_err());
        let reopened = AliasStore::new(path.clone());
        assert_eq!(
            reopened.execute(AliasOperation::List).unwrap()["bilibili"],
            target
        );
        assert!(reopened
            .execute(AliasOperation::Remove {
                name: "BILIBILI".into()
            })
            .unwrap()
            .is_empty());
        assert!(AliasStore::new(path)
            .execute(AliasOperation::List)
            .unwrap()
            .is_empty());
    }
}

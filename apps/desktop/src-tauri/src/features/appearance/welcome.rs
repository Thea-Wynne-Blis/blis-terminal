use std::{fs, io, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::platform::config_file::write_json;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WelcomeOperation {
    Get,
    Set { message: String },
    Reset,
}

pub struct WelcomeStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl WelcomeStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: WelcomeOperation) -> Result<Option<String>, String> {
        let _access = self.access.lock().map_err(|_| "欢迎语设置暂时不可用。")?;
        let message = match operation {
            WelcomeOperation::Get => return self.read_message(),
            WelcomeOperation::Set { message } => Some(validate_message(&message)?.to_owned()),
            WelcomeOperation::Reset => None,
        };
        let config = Config { message };
        write_json(&self.path, &config).map_err(|error| format!("无法保存欢迎语（{error}）。"))?;
        Ok(config.message)
    }

    fn read_message(&self) -> Result<Option<String>, String> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("无法读取欢迎语（{error}）。")),
        };
        let config: Config = serde_json::from_slice(&data)
            .map_err(|error| format!("欢迎语文件格式有误（{error}）。"))?;
        config
            .message
            .as_deref()
            .map(|message| validate_message(message).map(str::to_owned))
            .transpose()
            .map_err(str::to_owned)
    }
}

fn validate_message(message: &str) -> Result<&str, &'static str> {
    let message = message.trim();
    if message.is_empty() || message.chars().count() > 200 || message.chars().any(char::is_control)
    {
        Err("请输入一行欢迎语，最多 200 个字符。")
    } else {
        Ok(message)
    }
}

#[tauri::command]
pub async fn execute_welcome(
    app: AppHandle,
    operation: WelcomeOperation,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<crate::features::storage::Storage>()
            .stores()?
            .welcome
            .execute(operation)
    })
    .await
    .map_err(|_| "无法处理欢迎语设置。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_resets_the_welcome_message_across_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("welcome.json");
        let store = WelcomeStore::new(path.clone());
        assert_eq!(store.execute(WelcomeOperation::Get).unwrap(), None);
        let message = "欢迎回来，Blis 🚀";
        assert_eq!(
            store
                .execute(WelcomeOperation::Set {
                    message: message.into()
                })
                .unwrap()
                .as_deref(),
            Some(message)
        );
        assert!(store
            .execute(WelcomeOperation::Set {
                message: "多行\n消息".into()
            })
            .is_err());
        let reopened = WelcomeStore::new(path.clone());
        assert_eq!(
            reopened.execute(WelcomeOperation::Get).unwrap().as_deref(),
            Some(message)
        );
        reopened.execute(WelcomeOperation::Reset).unwrap();
        assert_eq!(
            WelcomeStore::new(path)
                .execute(WelcomeOperation::Get)
                .unwrap(),
            None
        );
    }
}

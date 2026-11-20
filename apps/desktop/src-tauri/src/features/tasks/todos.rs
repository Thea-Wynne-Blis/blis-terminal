use std::{collections::BTreeMap, fs, io, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::platform::config_file::write_json;

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Todo {
    title: String,
    done: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    tasks: BTreeMap<u32, Todo>,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TodoOperation {
    Add { title: String },
    List,
    Done { id: u32 },
}

pub struct TodoStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl TodoStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: TodoOperation) -> Result<String, String> {
        let _access = self.access.lock().map_err(|_| "待办列表暂时不可用。")?;
        let mut config = self.read()?;
        let output = match operation {
            TodoOperation::List => return Ok(list_pending(&config)),
            TodoOperation::Add { title } => {
                let title = valid_title(&title)?;
                let id = config
                    .tasks
                    .last_key_value()
                    .map_or(0, |(id, _)| *id)
                    .checked_add(1)
                    .ok_or("待办已达到编号上限。")?;
                let output = format!("已添加 [{id}] {title}");
                config.tasks.insert(
                    id,
                    Todo {
                        title: title.to_owned(),
                        done: false,
                    },
                );
                output
            }
            TodoOperation::Done { id } => {
                let task = config
                    .tasks
                    .get_mut(&id)
                    .ok_or_else(|| format!("找不到编号 {id}。输入 todo list 查看待办。"))?;
                if task.done {
                    return Ok(format!("[{id}] {} 已经完成。", task.title));
                }
                task.done = true;
                format!("已完成 [{id}] {}", task.title)
            }
        };
        write_json(&self.path, &config).map_err(|error| format!("无法保存待办（{error}）。"))?;
        Ok(output)
    }

    fn read(&self) -> Result<Config, String> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Config {
                    version: VERSION,
                    tasks: BTreeMap::new(),
                });
            }
            Err(error) => return Err(format!("无法读取待办（{error}）。")),
        };
        let config: Config = serde_json::from_slice(&data)
            .map_err(|error| format!("待办文件格式有误（{error}）。"))?;
        if config.version != VERSION {
            return Err(format!("无法读取版本为 {} 的待办文件。", config.version));
        }
        if config
            .tasks
            .iter()
            .any(|(id, task)| *id == 0 || valid_title(&task.title).is_err())
        {
            return Err("待办文件包含无效编号或内容。".to_owned());
        }
        Ok(config)
    }
}

fn list_pending(config: &Config) -> String {
    let pending = config
        .tasks
        .iter()
        .filter(|(_, task)| !task.done)
        .map(|(id, task)| format!("[{id}] {}", task.title))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        "没有未完成的待办。".to_owned()
    } else {
        pending.join("\n")
    }
}

fn valid_title(title: &str) -> Result<&str, &'static str> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 512 || title.chars().any(char::is_control) {
        Err("请输入一行待办内容，最多 512 个字符。")
    } else {
        Ok(title)
    }
}

#[tauri::command]
pub async fn execute_todo(app: AppHandle, operation: TodoOperation) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<TodoStore>().execute(operation))
        .await
        .map_err(|_| "无法处理待办请求。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_pending_tasks_and_stable_ids_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("todos.json");
        let add = |title: &str| TodoOperation::Add {
            title: title.into(),
        };
        let store = TodoStore::new(path.clone());
        assert_eq!(
            store.execute(add("补发版说明")).unwrap(),
            "已添加 [1] 补发版说明"
        );
        store.execute(add("检查安装包")).unwrap();
        store.execute(TodoOperation::Done { id: 1 }).unwrap();
        drop(store);

        let reopened = TodoStore::new(path);
        assert_eq!(
            reopened.execute(TodoOperation::List).unwrap(),
            "[2] 检查安装包"
        );
        assert!(reopened
            .execute(TodoOperation::Done { id: 1 })
            .unwrap()
            .contains("已经完成"));
        assert!(reopened.execute(TodoOperation::Done { id: 3 }).is_err());
        assert_eq!(
            reopened.execute(add("记录测试结果")).unwrap(),
            "已添加 [3] 记录测试结果"
        );
    }

    #[test]
    fn does_not_overwrite_unreadable_todo_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("todos.json");
        fs::write(&path, b"invalid").unwrap();
        let store = TodoStore::new(path.clone());
        assert!(store
            .execute(TodoOperation::Add {
                title: "待办".into()
            })
            .is_err());
        assert_eq!(fs::read(path).unwrap(), b"invalid");
    }
}

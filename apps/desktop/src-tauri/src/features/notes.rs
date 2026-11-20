use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tempfile::NamedTempFile;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteFormat {
    Md,
    Txt,
}

impl NoteFormat {
    fn extension(&self) -> &'static str {
        match self {
            Self::Md => "md",
            Self::Txt => "txt",
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NoteOperation {
    List,
    New {
        title: String,
    },
    Edit {
        title: String,
    },
    Remove {
        title: String,
        #[serde(default)]
        confirmed: bool,
    },
    Save {
        title: String,
        content: String,
        create: bool,
    },
    Export {
        title: String,
        format: NoteFormat,
        output: Option<PathBuf>,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NoteOutcome {
    Editor {
        title: String,
        content: String,
        create: bool,
    },
    Message {
        text: String,
    },
    ConfirmRemove {
        title: String,
    },
}

pub struct NoteStore {
    directory: PathBuf,
    access: Mutex<()>,
}

impl NoteStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            access: Mutex::new(()),
        }
    }

    fn execute(
        &self,
        operation: NoteOperation,
        default_export_dir: Option<&Path>,
    ) -> Result<NoteOutcome, String> {
        let _access = self.access.lock().map_err(|_| "笔记暂时不可用。")?;
        match operation {
            NoteOperation::List => self.list(),
            NoteOperation::New { title } => {
                let title = validate_title(&title)?;
                if self.path(title).try_exists().map_err(file_error)? {
                    return Err(already_exists(title));
                }
                Ok(NoteOutcome::Editor {
                    title: title.into(),
                    content: String::new(),
                    create: true,
                })
            }
            NoteOperation::Edit { title } => {
                let title = validate_title(&title)?;
                let content = self.read(title)?;
                Ok(NoteOutcome::Editor {
                    title: title.into(),
                    content,
                    create: false,
                })
            }
            NoteOperation::Remove { title, confirmed } => {
                let title = validate_title(&title)?;
                let path = self.path(title);
                let map_error = |error: io::Error| {
                    if error.kind() == io::ErrorKind::NotFound {
                        missing_note(title)
                    } else {
                        file_error(error)
                    }
                };
                if !fs::symlink_metadata(&path).map_err(map_error)?.is_file() {
                    return Err(missing_note(title));
                }
                if !confirmed {
                    return Ok(NoteOutcome::ConfirmRemove {
                        title: title.into(),
                    });
                }
                fs::remove_file(path).map_err(map_error)?;
                Ok(NoteOutcome::Message {
                    text: format!("已删除笔记「{title}」。"),
                })
            }
            NoteOperation::Save {
                title,
                content,
                create,
            } => {
                let title = validate_title(&title)?;
                let path = self.path(title);
                if create {
                    fs::create_dir_all(&self.directory).map_err(file_error)?;
                } else if !path.try_exists().map_err(file_error)? {
                    return Err(missing_note(title));
                }
                write_text(&path, &content, !create).map_err(|error| {
                    if create && error.kind() == io::ErrorKind::AlreadyExists {
                        already_exists(title)
                    } else {
                        file_error(error)
                    }
                })?;
                Ok(NoteOutcome::Message {
                    text: format!("已保存笔记「{title}」。"),
                })
            }
            NoteOperation::Export {
                title,
                format,
                output,
            } => {
                let title = validate_title(&title)?;
                let markdown = self.read(title)?;
                let extension = format.extension();
                let path = match output {
                    Some(path) => {
                        if !path.is_absolute()
                            || !path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
                        {
                            return Err(format!(
                                "-o 或 --out 请指定以 .{extension} 结尾的绝对文件路径。"
                            ));
                        }
                        path
                    }
                    None => {
                        let directory = default_export_dir
                            .ok_or("无法找到文档目录，请用 -o 或 --out 指定导出路径。")?;
                        fs::create_dir_all(directory).map_err(file_error)?;
                        directory.join(format!("{title}.{extension}"))
                    }
                };
                let content = match format {
                    NoteFormat::Md => markdown,
                    NoteFormat::Txt => plain_text(&markdown),
                };
                write_text(&path, &content, false).map_err(|error| {
                    if error.kind() == io::ErrorKind::AlreadyExists {
                        format!(
                            "导出文件已存在（{}）。请用 -o 或 --out 指定其他路径。",
                            path.display()
                        )
                    } else {
                        file_error(error)
                    }
                })?;
                Ok(NoteOutcome::Message {
                    text: format!("已导出到 {}", path.display()),
                })
            }
        }
    }

    fn path(&self, title: &str) -> PathBuf {
        self.directory.join(format!("{title}.md"))
    }

    fn read(&self, title: &str) -> Result<String, String> {
        fs::read_to_string(self.path(title)).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                missing_note(title)
            } else {
                file_error(error)
            }
        })
    }

    fn list(&self) -> Result<NoteOutcome, String> {
        let mut titles = Vec::new();
        match fs::read_dir(&self.directory) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(file_error)?;
                    if !entry.file_type().map_err(file_error)?.is_file() {
                        continue;
                    }
                    let name = entry.file_name();
                    if let Some(title) = name.to_str().and_then(|name| name.strip_suffix(".md")) {
                        titles.push(title.to_owned());
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(file_error(error)),
        }
        titles.sort();
        let text = if titles.is_empty() {
            "暂无笔记。输入 help note 查看用法。".into()
        } else {
            titles.join("\n")
        };
        Ok(NoteOutcome::Message { text })
    }
}

fn validate_title(title: &str) -> Result<&str, &'static str> {
    let title = title.trim();
    let stem = title
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    let reserved_port = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|number| {
            matches!(
                number,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    if title.is_empty()
        || title.chars().count() > 80
        || title.ends_with('.')
        || title
            .chars()
            .any(|c| c.is_control() || r#"<>:"/\|?*"#.contains(c))
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || reserved_port
    {
        return Err("请输入有效的笔记标题，最多 80 个字符。标题会用于文件名。");
    }
    Ok(title)
}

fn already_exists(title: &str) -> String {
    format!("笔记「{title}」已存在。请用 note edit 编辑。")
}

fn missing_note(title: &str) -> String {
    format!("找不到笔记「{title}」。输入 note list 查看。")
}

fn file_error(error: io::Error) -> String {
    format!("笔记文件操作失败（{error}）。")
}

fn write_text(path: &Path, content: &str, replace: bool) -> io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("missing note directory"))?;
    let mut file = NamedTempFile::new_in(directory)?;
    file.write_all(content.as_bytes())?;
    file.as_file().sync_all()?;
    if replace {
        file.persist(path)?;
    } else {
        file.persist_noclobber(path)?;
    }
    Ok(())
}

fn plain_text(markdown: &str) -> String {
    let mut output = String::new();
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(text) | Event::Code(text) => output.push_str(&text),
            Event::Start(Tag::Item) => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("- ");
            }
            Event::TaskListMarker(done) => output.push_str(if done { "[x] " } else { "[ ] " }),
            Event::SoftBreak | Event::HardBreak | Event::Rule => output.push('\n'),
            Event::End(
                TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::CodeBlock | TagEnd::Item,
            ) if !output.ends_with('\n') => output.push('\n'),
            _ => {}
        }
    }
    output
}

#[tauri::command]
pub async fn execute_note(app: AppHandle, operation: NoteOperation) -> Result<NoteOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let default_export_dir = match &operation {
            NoteOperation::Export { output: None, .. } => app
                .path()
                .document_dir()
                .ok()
                .map(|dir| dir.join("Blis Notes")),
            _ => None,
        };
        app.state::<NoteStore>()
            .execute(operation, default_export_dir.as_deref())
    })
    .await
    .map_err(|_| "无法处理笔记请求。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use NoteOperation::*;

    #[test]
    fn removes_only_after_confirmation() {
        let root = tempfile::tempdir().unwrap();
        let storage = root.path().join("notes");
        let store = NoteStore::new(storage.clone());
        let title = "项目 计划".to_owned();
        store
            .execute(
                Save {
                    title: title.clone(),
                    content: "计划正文".into(),
                    create: true,
                },
                None,
            )
            .unwrap();
        let copied = root.path().join("项目 计划.md");
        fs::copy(storage.join("项目 计划.md"), &copied).unwrap();

        let request = serde_json::json!({ "action": "remove", "title": title });
        assert!(matches!(
            store.execute(serde_json::from_value(request).unwrap(), None).unwrap(),
            NoteOutcome::ConfirmRemove { title: pending } if pending == title
        ));
        assert_eq!(
            fs::read_to_string(storage.join("项目 计划.md")).unwrap(),
            "计划正文"
        );

        let remove = || Remove {
            title: title.clone(),
            confirmed: true,
        };
        assert!(matches!(
            store.execute(remove(), None).unwrap(),
            NoteOutcome::Message { .. }
        ));
        assert!(!storage.join("项目 计划.md").exists());
        assert_eq!(fs::read_to_string(copied).unwrap(), "计划正文");
        assert!(store.execute(remove(), None).is_err());
    }

    #[test]
    fn saves_edits_lists_and_exports_markdown_and_plain_text() {
        let root = tempfile::tempdir().unwrap();
        let storage = root.path().join("notes");
        let exports = root.path().join("exports");
        let store = NoteStore::new(storage.clone());
        let title = || "项目 计划".to_owned();
        let save = |content: &str, create| Save {
            title: title(),
            content: content.into(),
            create,
        };
        let original = "# 计划\n\n- **任务** [链接](https://example.com)\n- 完成 `demo`\n";

        assert!(matches!(
            store.execute(New { title: title() }, None).unwrap(),
            NoteOutcome::Editor { create: true, .. }
        ));
        assert!(!storage.exists());
        store.execute(save("草稿", true), None).unwrap();
        assert!(store.execute(save("意外覆盖", true), None).is_err());
        store.execute(save(original, false), None).unwrap();

        let reopened = NoteStore::new(storage);
        assert!(
            matches!(reopened.execute(List, None).unwrap(), NoteOutcome::Message { text } if text == title())
        );
        assert!(
            matches!(reopened.execute(Edit { title: title() }, None).unwrap(), NoteOutcome::Editor { content, create: false, .. } if content == original)
        );

        reopened
            .execute(
                Export {
                    title: title(),
                    format: NoteFormat::Md,
                    output: None,
                },
                Some(&exports),
            )
            .unwrap();
        assert_eq!(
            fs::read_to_string(exports.join("项目 计划.md")).unwrap(),
            original
        );
        let txt = exports.join("copy.txt");
        let export_txt = || Export {
            title: title(),
            format: NoteFormat::Txt,
            output: Some(txt.clone()),
        };
        reopened.execute(export_txt(), None).unwrap();
        assert_eq!(
            fs::read_to_string(&txt).unwrap(),
            "计划\n- 任务 链接\n- 完成 demo\n"
        );
        assert!(reopened.execute(export_txt(), None).is_err());
        assert!(reopened
            .execute(
                Edit {
                    title: "../escape".into()
                },
                None
            )
            .is_err());
        assert!(reopened
            .execute(
                Export {
                    title: title(),
                    format: NoteFormat::Md,
                    output: Some(PathBuf::from("relative.md"))
                },
                None
            )
            .is_err());
    }
}

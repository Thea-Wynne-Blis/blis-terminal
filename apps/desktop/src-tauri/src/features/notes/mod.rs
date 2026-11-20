use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

mod markdown;

use markdown::{normalize_headings, plain_text, render_markdown};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tempfile::NamedTempFile;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteFormat {
    #[default]
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
        #[serde(default)]
        format: NoteFormat,
    },
    Edit {
        title: String,
    },
    Remove {
        title: String,
        #[serde(default)]
        confirmed: bool,
        format: Option<NoteFormat>,
    },
    Save {
        title: String,
        content: String,
        create: bool,
        #[serde(default)]
        format: NoteFormat,
    },
    Export {
        title: String,
        format: NoteFormat,
        output: Option<PathBuf>,
    },
    Preview {
        content: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NoteOutcome {
    Editor {
        title: String,
        content: String,
        create: bool,
        format: NoteFormat,
    },
    Message {
        text: String,
    },
    ConfirmRemove {
        title: String,
        format: NoteFormat,
    },
    Saved {
        text: String,
        content: String,
    },
    Preview {
        text: String,
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
        let operation = match operation {
            NoteOperation::Preview { content } => {
                return Ok(NoteOutcome::Preview {
                    text: render_markdown(&content),
                });
            }
            other => other,
        };
        let _access = self.access.lock().map_err(|_| "笔记暂时不可用。")?;
        match operation {
            NoteOperation::List => self.list(),
            NoteOperation::New { title, format } => {
                let title = validate_title(&title)?;
                self.require_available_title(title)?;
                Ok(NoteOutcome::Editor {
                    title: title.into(),
                    content: String::new(),
                    create: true,
                    format,
                })
            }
            NoteOperation::Edit { title } => {
                let title = validate_title(&title)?;
                let (content, format) = self.read(title)?;
                Ok(NoteOutcome::Editor {
                    title: title.into(),
                    content,
                    create: false,
                    format,
                })
            }
            NoteOperation::Remove {
                title,
                confirmed,
                format,
            } => {
                let title = validate_title(&title)?;
                if !confirmed {
                    return Ok(NoteOutcome::ConfirmRemove {
                        title: title.into(),
                        format: self.find_format(title)?,
                    });
                }
                let format = match format {
                    Some(format) => format,
                    None => self.find_format(title)?,
                };
                let path = self.require_file(title, format)?;
                fs::remove_file(path).map_err(file_error)?;
                Ok(NoteOutcome::Message {
                    text: format!("已删除笔记「{title}」。"),
                })
            }
            NoteOperation::Save {
                title,
                content,
                create,
                format,
            } => {
                let title = validate_title(&title)?;
                let path = if create {
                    self.require_available_title(title)?;
                    fs::create_dir_all(&self.directory).map_err(file_error)?;
                    self.path(title, format)
                } else {
                    self.require_file(title, format)?
                };
                let content = match format {
                    NoteFormat::Md => normalize_headings(&content).into_owned(),
                    NoteFormat::Txt => content,
                };
                write_text(&path, &content, !create).map_err(|error| {
                    if create && error.kind() == io::ErrorKind::AlreadyExists {
                        already_exists(title)
                    } else {
                        file_error(error)
                    }
                })?;
                Ok(NoteOutcome::Saved {
                    text: format!("已保存笔记「{title}」。"),
                    content,
                })
            }
            NoteOperation::Export {
                title,
                format,
                output,
            } => {
                let title = validate_title(&title)?;
                let (content, source_format) = self.read(title)?;
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
                let content = match (source_format, format) {
                    (NoteFormat::Md, NoteFormat::Txt) => plain_text(&content),
                    _ => content,
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
            NoteOperation::Preview { .. } => unreachable!("handled before locking"),
        }
    }

    fn path(&self, title: &str, format: NoteFormat) -> PathBuf {
        self.directory
            .join(format!("{title}.{}", format.extension()))
    }

    fn has_entry(&self, title: &str, format: NoteFormat) -> Result<bool, String> {
        match fs::symlink_metadata(self.path(title, format)) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(file_error(error)),
        }
    }

    fn require_available_title(&self, title: &str) -> Result<(), String> {
        if self.has_entry(title, NoteFormat::Md)? || self.has_entry(title, NoteFormat::Txt)? {
            return Err(already_exists(title));
        }
        Ok(())
    }

    fn find_format(&self, title: &str) -> Result<NoteFormat, String> {
        match (
            self.has_entry(title, NoteFormat::Md)?,
            self.has_entry(title, NoteFormat::Txt)?,
        ) {
            (true, false) => Ok(NoteFormat::Md),
            (false, true) => Ok(NoteFormat::Txt),
            (false, false) => Err(missing_note(title)),
            (true, true) => Err(format!(
                "「{title}」同时有 MD 和 TXT 文件，请先在笔记目录中调整其中一个文件的名称。"
            )),
        }
    }

    fn require_file(&self, title: &str, format: NoteFormat) -> Result<PathBuf, String> {
        let path = self.path(title, format);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => Ok(path),
            Ok(_) => Err(format!("「{}」不是普通笔记文件。", path.display())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Err(missing_note(title)),
            Err(error) => Err(file_error(error)),
        }
    }

    fn read(&self, title: &str) -> Result<(String, NoteFormat), String> {
        let format = self.find_format(title)?;
        let content = fs::read_to_string(self.require_file(title, format)?).map_err(file_error)?;
        Ok((content, format))
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
                    if let Some((title, format @ ("md" | "txt"))) =
                        name.to_str().and_then(|name| name.rsplit_once('.'))
                    {
                        titles.push(format!("{title}  [{format}]"));
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

#[tauri::command]
pub async fn execute_note(app: AppHandle, operation: NoteOperation) -> Result<NoteOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let NoteOperation::Preview { content } = operation {
            return Ok(NoteOutcome::Preview {
                text: render_markdown(&content),
            });
        }
        let default_export_dir = match &operation {
            NoteOperation::Export { output: None, .. } => app
                .path()
                .document_dir()
                .ok()
                .map(|dir| dir.join("Blis Notes")),
            _ => None,
        };
        app.state::<super::storage::Storage>()
            .stores()?
            .notes
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
    fn preview_normalizes_headings_and_keeps_code_and_raw_html_inert() {
        let source = "##这是二级标题\n\n~~~\n##原样代码\n~~~\n\n<img src=x onerror=alert(1)>\n";
        let markdown = normalize_headings(source);
        assert!(markdown.starts_with("## 这是二级标题\n"));
        assert!(markdown.contains("~~~\n##原样代码\n~~~"));
        let preview = render_markdown(source);
        assert!(preview.contains("<h2>这是二级标题</h2>"));
        assert!(preview.contains("##原样代码"));
        assert!(!preview.contains("<img"));
        assert!(!render_markdown("[点这里](javascript:alert%281%29)").contains("href="));
    }

    #[test]
    fn reopens_and_exports_plain_text_without_markdown_conversion() {
        let root = tempfile::tempdir().unwrap();
        let store = NoteStore::new(root.path().join("notes"));
        let title = "纯文本".to_owned();
        let content = "##原样记录\n**保留符号**\n".to_owned();
        store
            .execute(
                Save {
                    title: title.clone(),
                    content: content.clone(),
                    create: true,
                    format: NoteFormat::Txt,
                },
                None,
            )
            .unwrap();
        assert!(matches!(
            store.execute(Edit { title: title.clone() }, None).unwrap(),
            NoteOutcome::Editor { content: text, format: NoteFormat::Txt, .. } if text == content
        ));
        assert!(store
            .execute(
                New {
                    title: title.clone(),
                    format: NoteFormat::Md
                },
                None
            )
            .is_err());
        let output = root.path().join("导出.txt");
        store
            .execute(
                Export {
                    title,
                    format: NoteFormat::Txt,
                    output: Some(output.clone()),
                },
                None,
            )
            .unwrap();
        assert_eq!(fs::read_to_string(output).unwrap(), content);
    }

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
                    format: NoteFormat::Md,
                },
                None,
            )
            .unwrap();
        let copied = root.path().join("项目 计划.md");
        fs::copy(storage.join("项目 计划.md"), &copied).unwrap();

        let request = serde_json::json!({ "action": "remove", "title": title });
        assert!(matches!(
            store.execute(serde_json::from_value(request).unwrap(), None).unwrap(),
            NoteOutcome::ConfirmRemove { title: pending, .. } if pending == title
        ));
        assert_eq!(
            fs::read_to_string(storage.join("项目 计划.md")).unwrap(),
            "计划正文"
        );

        let remove = || Remove {
            title: title.clone(),
            confirmed: true,
            format: Some(NoteFormat::Md),
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
            format: NoteFormat::Md,
        };
        let original = "# 计划\n\n- **任务** [链接](https://example.com)\n- 完成 `demo`\n";

        assert!(matches!(
            store
                .execute(
                    New {
                        title: title(),
                        format: NoteFormat::Md
                    },
                    None
                )
                .unwrap(),
            NoteOutcome::Editor { create: true, .. }
        ));
        assert!(!storage.exists());
        store.execute(save("草稿", true), None).unwrap();
        assert!(store.execute(save("意外覆盖", true), None).is_err());
        store.execute(save(original, false), None).unwrap();

        let reopened = NoteStore::new(storage);
        assert!(
            matches!(reopened.execute(List, None).unwrap(), NoteOutcome::Message { text } if text == format!("{}  [md]", title()))
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

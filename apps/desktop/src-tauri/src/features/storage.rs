use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read},
    path::{Component, Path, PathBuf},
    sync::{RwLock, RwLockReadGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::platform::config_file::write_json;

use super::{
    aliases::AliasStore,
    appearance::{background::AppearanceStore, welcome::WelcomeStore},
    files::open_targets::OpenTargets,
    notes::NoteStore,
    search_settings::SearchStore,
    tasks::todos::TodoStore,
};

const LOCATION_FILE: &str = "store-location.json";
const ALIASES: &str = "aliases.json";
const APPEARANCE: &str = "appearance.json";
const OPEN_TARGETS: &str = "open-targets.json";
const SEARCH: &str = "search-settings.json";
const TODOS: &str = "todos.json";
const WELCOME: &str = "welcome.json";
const NOTES: &str = "notes";
const BACKGROUNDS: &str = "backgrounds";
const DATA_FILES: [&str; 6] = [ALIASES, APPEARANCE, OPEN_TARGETS, SEARCH, TODOS, WELCOME];

#[derive(Serialize, Deserialize)]
struct Location {
    directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum StoreOperation {
    Get,
    Set { path: PathBuf },
    Reset,
}

#[derive(Serialize)]
pub struct StoreOutcome {
    text: String,
    changed: bool,
}

pub struct Stores {
    directory: PathBuf,
    pub aliases: AliasStore,
    pub appearance: AppearanceStore,
    pub notes: NoteStore,
    pub open_targets: OpenTargets,
    pub search: SearchStore,
    pub todos: TodoStore,
    pub welcome: WelcomeStore,
}

impl Stores {
    fn new(directory: PathBuf) -> Self {
        Self {
            aliases: AliasStore::new(directory.join(ALIASES)),
            appearance: AppearanceStore::new(directory.clone()),
            notes: NoteStore::new(directory.join(NOTES)),
            open_targets: OpenTargets::new(directory.join(OPEN_TARGETS)),
            search: SearchStore::new(directory.join(SEARCH)),
            todos: TodoStore::new(directory.join(TODOS)),
            welcome: WelcomeStore::new(directory.join(WELCOME)),
            directory,
        }
    }
}

pub struct Storage {
    default: PathBuf,
    location_file: PathBuf,
    stores: RwLock<Stores>,
}

impl Storage {
    pub fn new(default: &Path) -> Result<Self, String> {
        fs::create_dir_all(default).map_err(|error| file_error(default, error))?;
        let default = dunce::canonicalize(default).map_err(|error| file_error(default, error))?;
        let location_file = default.join(LOCATION_FILE);
        let directory = match fs::read(&location_file) {
            Ok(bytes) => {
                let location: Location = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("存储位置记录格式有误（{error}）。"))?;
                if !location.directory.is_absolute() {
                    return Err("存储位置记录不是绝对路径。".into());
                }
                location.directory
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => default.clone(),
            Err(error) => return Err(file_error(&location_file, error)),
        };
        Ok(Self {
            default,
            location_file,
            stores: RwLock::new(Stores::new(directory)),
        })
    }

    pub fn stores(&self) -> Result<RwLockReadGuard<'_, Stores>, String> {
        let stores = self.read()?;
        require_directory(&stores.directory)?;
        Ok(stores)
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, Stores>, String> {
        self.stores
            .read()
            .map_err(|_| "存储位置暂时不可用。".into())
    }

    pub fn allow_backgrounds(&self, app: &AppHandle) -> Result<(), String> {
        allow_backgrounds(app, &self.read()?.directory)
    }

    fn describe(&self) -> Result<StoreOutcome, String> {
        let stores = self.read()?;
        let unavailable = if require_directory(&stores.directory).is_err() {
            "（暂不可访问）"
        } else {
            ""
        };
        Ok(StoreOutcome {
            text: format!(
                "当前数据目录  {}{unavailable}\n默认数据目录  {}\n位置记录      {}",
                stores.directory.display(),
                self.default.display(),
                self.location_file.display()
            ),
            changed: false,
        })
    }

    fn migrate(
        &self,
        requested: &Path,
        prepare: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<StoreOutcome, String> {
        let mut stores = self.stores.write().map_err(|_| "存储位置暂时不可用。")?;
        require_directory(&stores.directory)?;
        let source = dunce::canonicalize(&stores.directory)
            .map_err(|error| file_error(&stores.directory, error))?;
        let target = resolve_destination(requested)?;
        if same_path(&source, &target) {
            return Ok(StoreOutcome {
                text: format!("当前数据已存放在「{}」。", target.display()),
                changed: false,
            });
        }
        if is_under(&source, &target) || is_under(&target, &source) {
            return Err("新旧目录不能互相包含，请选择独立的目录。".into());
        }
        let default_target = same_path(&self.default, &target);
        if !default_target && (is_under(&self.default, &target) || is_under(&target, &self.default))
        {
            return Err("请选择默认数据目录以外的独立目录。".into());
        }
        let mut created = CopyFiles::new(&target);
        if target.exists() {
            require_directory(&target)?;
        } else {
            fs::create_dir_all(target.parent().ok_or("无法识别目标目录。")?)
                .map_err(|error| file_error(&target, error))?;
            fs::create_dir(&target).map_err(|error| file_error(&target, error))?;
            created.created_root = true;
        }
        for entry in fs::read_dir(&target).map_err(|error| file_error(&target, error))? {
            let entry = entry.map_err(|error| file_error(&target, error))?;
            if !(default_target && entry.file_name() == LOCATION_FILE) {
                return Err("目标目录必须为空，请选择空目录或输入一个新目录。".into());
            }
        }

        for name in DATA_FILES {
            created.copy(&source, Path::new(name), false)?;
        }
        for name in [NOTES, BACKGROUNDS] {
            created.copy(&source, Path::new(name), true)?;
        }
        prepare(&target)?;
        created.verify(&source)?;
        write_json(
            &self.location_file,
            &Location {
                directory: target.clone(),
            },
        )
        .map_err(|error| file_error(&self.location_file, error))?;

        // The pointer is durable before any of the verified source files are removed.
        created.committed = true;
        *stores = Stores::new(target.clone());
        let remaining = created.clean_source(&source);
        let mut text = format!("数据已迁移到「{}」，现在使用新目录。", target.display());
        if remaining.is_empty() {
            text.push_str("\n旧目录中已迁移的数据已删除。");
        } else {
            text.push_str("\n以下旧目录内容无法安全删除，请检查后手动处理：");
            for path in remaining {
                text.push_str(&format!("\n{}", path.display()));
            }
        }
        Ok(StoreOutcome {
            text,
            changed: true,
        })
    }
}

fn allow_backgrounds(app: &AppHandle, directory: &Path) -> Result<(), String> {
    let path = directory.join(BACKGROUNDS);
    app.asset_protocol_scope()
        .allow_directory(&path, false)
        .map_err(|error| format!("无法加载背景图片目录「{}」（{error}）。", path.display()))
}

#[tauri::command]
pub async fn execute_store(
    app: AppHandle,
    operation: StoreOperation,
) -> Result<StoreOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        match operation {
            StoreOperation::Get => storage.describe(),
            StoreOperation::Set { path } => {
                storage.migrate(&path, |path| allow_backgrounds(&app, path))
            }
            StoreOperation::Reset => {
                storage.migrate(&storage.default, |path| allow_backgrounds(&app, path))
            }
        }
    })
    .await
    .map_err(|_| "无法处理存储位置请求。".to_owned())?
}

fn resolve_destination(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path.components().any(|component| match component {
            Component::ParentDir => true,
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                name.chars().any(char::is_control)
                    || cfg!(windows) && (name.contains(':') || name.ends_with(['.', ' ']))
            }
            #[cfg(windows)]
            Component::Prefix(prefix) => !matches!(prefix.kind(), std::path::Prefix::Disk(_)),
            _ => false,
        })
    {
        return Err("请输入一个有效的绝对目录路径，例如 D:\\Blis 数据。".into());
    }
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error)
                if error.kind() == io::ErrorKind::NotFound && ancestor.parent().is_some() =>
            {
                missing.push(ancestor.file_name().ok_or("无法识别目标目录。")?);
                ancestor = ancestor.parent().ok_or("无法识别目标目录。")?;
            }
            Err(error) => return Err(file_error(ancestor, error)),
        }
    }
    require_directory(ancestor)?;
    let mut target = dunce::canonicalize(ancestor).map_err(|error| file_error(ancestor, error))?;
    target.extend(missing.into_iter().rev());
    Ok(target)
}

fn is_under(path: &Path, parent: &Path) -> bool {
    #[cfg(windows)]
    {
        let mut components = path.components();
        parent.components().all(|part| {
            components.next().is_some_and(|current| {
                current.as_os_str().to_string_lossy().to_lowercase()
                    == part.as_os_str().to_string_lossy().to_lowercase()
            })
        })
    }
    #[cfg(not(windows))]
    path.starts_with(parent)
}

fn same_path(left: &Path, right: &Path) -> bool {
    is_under(left, right) && is_under(right, left)
}

fn is_regular(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & REPARSE_POINT != 0 {
            return false;
        }
    }
    metadata.is_file() || metadata.is_dir()
}

fn require_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| file_error(path, error))?;
    if !metadata.is_dir() || !is_regular(&metadata) {
        return Err(format!("「{}」必须是普通目录。", path.display()));
    }
    Ok(())
}

fn file_error(path: &Path, error: io::Error) -> String {
    format!("无法处理「{}」（{error}）。", path.display())
}

struct CopiedEntry {
    relative: PathBuf,
    directory: bool,
}

struct CopyFiles<'a> {
    target: &'a Path,
    entries: Vec<CopiedEntry>,
    created_root: bool,
    committed: bool,
}

impl<'a> CopyFiles<'a> {
    fn new(target: &'a Path) -> Self {
        Self {
            target,
            entries: Vec::new(),
            created_root: false,
            committed: false,
        }
    }

    fn copy(&mut self, source: &Path, relative: &Path, directory: bool) -> Result<(), String> {
        let from = source.join(relative);
        let to = self.target.join(relative);
        let metadata = match fs::symlink_metadata(&from) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(file_error(&from, error)),
        };
        if !is_regular(&metadata) || directory != metadata.is_dir() {
            return Err(format!("「{}」不是预期的普通文件或目录。", from.display()));
        }
        if directory {
            fs::create_dir(&to).map_err(|error| file_error(&to, error))?;
            self.record(relative, true);
            for entry in fs::read_dir(&from).map_err(|error| file_error(&from, error))? {
                let entry = entry.map_err(|error| file_error(&from, error))?;
                let ty = entry
                    .file_type()
                    .map_err(|error| file_error(&entry.path(), error))?;
                self.copy(source, &relative.join(entry.file_name()), ty.is_dir())?;
            }
        } else {
            let mut input = File::open(&from).map_err(|error| file_error(&from, error))?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&to)
                .map_err(|error| file_error(&to, error))?;
            self.record(relative, false);
            io::copy(&mut input, &mut output)
                .and_then(|_| output.sync_all())
                .map_err(|error| file_error(&to, error))?;
        }
        Ok(())
    }

    fn record(&mut self, relative: &Path, directory: bool) {
        self.entries.push(CopiedEntry {
            relative: relative.to_path_buf(),
            directory,
        });
    }

    fn verify(&self, source: &Path) -> Result<(), String> {
        for entry in &self.entries {
            let old = source.join(&entry.relative);
            let new = self.target.join(&entry.relative);
            if !unchanged_path(&old)
                || !unchanged_path(&new)
                || (!entry.directory
                    && !same_contents(&old, &new).map_err(|error| file_error(&old, error))?)
            {
                return Err(format!("核对「{}」失败，仍使用原目录。", old.display()));
            }
        }
        Ok(())
    }

    fn clean_source(&self, source: &Path) -> Vec<PathBuf> {
        let mut remaining = Vec::new();
        for entry in self.entries.iter().rev() {
            let old = source.join(&entry.relative);
            let new = self.target.join(&entry.relative);
            let removed = unchanged_path(&old)
                && if entry.directory {
                    fs::remove_dir(&old).is_ok()
                } else {
                    unchanged_path(&new)
                        && same_contents(&old, &new).unwrap_or(false)
                        && fs::remove_file(&old).is_ok()
                };
            if !removed {
                remaining.push(old);
            }
        }
        remaining
    }
}

impl Drop for CopyFiles<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for entry in self.entries.iter().rev() {
            let path = self.target.join(&entry.relative);
            if unchanged_path(&path) {
                let _ = if entry.directory {
                    fs::remove_dir(path)
                } else {
                    fs::remove_file(path)
                };
            }
        }
        if self.created_root && unchanged_path(self.target) {
            let _ = fs::remove_dir(self.target);
        }
    }
}

fn unchanged_path(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| is_regular(&metadata))
        && dunce::canonicalize(path).is_ok_and(|current| same_path(&current, path))
}

fn same_contents(left: &Path, right: &Path) -> io::Result<bool> {
    let mut left = File::open(left)?;
    let mut right = File::open(right)?;
    let mut remaining = left.metadata()?.len();
    if remaining != right.metadata()?.len() {
        return Ok(false);
    }
    let mut left_buffer = [0; 16 * 1024];
    let mut right_buffer = [0; 16 * 1024];
    while remaining > 0 {
        let size = remaining.min(left_buffer.len() as u64) as usize;
        left.read_exact(&mut left_buffer[..size])?;
        right.read_exact(&mut right_buffer[..size])?;
        if left_buffer[..size] != right_buffer[..size] {
            return Ok(false);
        }
        remaining -= size as u64;
    }
    Ok(left.read(&mut left_buffer[..1])? == 0 && right.read(&mut right_buffer[..1])? == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_data_cleans_the_source_and_can_move_back_after_restart() {
        let sandbox = tempfile::tempdir().unwrap();
        let default = sandbox.path().join("default");
        let target = sandbox.path().join("new data");
        let storage = Storage::new(&default).unwrap();
        fs::create_dir_all(default.join("notes")).unwrap();
        let note = Path::new("notes").join("计划.md");
        fs::write(default.join(&note), "## 计划").unwrap();
        fs::write(default.join(ALIASES), "{}").unwrap();

        let outcome = storage.migrate(&target, |_| Ok(())).unwrap();
        assert!(outcome.changed);
        assert_eq!(fs::read_to_string(target.join(&note)).unwrap(), "## 计划");
        assert!(!default.join(NOTES).exists());
        assert!(!default.join(ALIASES).exists());
        assert!(default.join(LOCATION_FILE).is_file());

        let reopened = Storage::new(&default).unwrap();
        assert!(same_path(&reopened.stores().unwrap().directory, &target));
        reopened.migrate(&default, |_| Ok(())).unwrap();
        assert!(default.join(&note).is_file());
        assert!(!target.join(NOTES).exists());
        assert!(same_path(
            &Storage::new(&default).unwrap().stores().unwrap().directory,
            &default
        ));
    }

    #[test]
    fn refuses_nonempty_and_nested_targets_without_touching_existing_files() {
        let sandbox = tempfile::tempdir().unwrap();
        let default = sandbox.path().join("default");
        let target = sandbox.path().join("other");
        let storage = Storage::new(&default).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("unrelated.txt"), "existing data").unwrap();
        fs::write(default.join(ALIASES), "blis data").unwrap();

        assert!(storage.migrate(&target, |_| Ok(())).is_err());
        assert!(storage
            .migrate(&default.join("nested"), |_| Ok(()))
            .is_err());
        assert!(!default.join("nested").exists());
        assert!(!default.join(LOCATION_FILE).exists());
        assert_eq!(
            fs::read_to_string(default.join(ALIASES)).unwrap(),
            "blis data"
        );
        assert_eq!(
            fs::read_to_string(target.join("unrelated.txt")).unwrap(),
            "existing data"
        );
    }

    #[test]
    fn verification_failure_rolls_back_copied_files_with_source_intact() {
        let sandbox = tempfile::tempdir().unwrap();
        let default = sandbox.path().join("default");
        let target = sandbox.path().join("target");
        let storage = Storage::new(&default).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(default.join(ALIASES), "original").unwrap();

        let result = storage.migrate(&target, |_| {
            fs::write(default.join(ALIASES), "updated").unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(default.join(ALIASES)).unwrap(),
            "updated"
        );
        assert!(!default.join(LOCATION_FILE).exists());
        assert_eq!(fs::read_dir(target).unwrap().count(), 0);
        assert!(same_path(&storage.stores().unwrap().directory, &default));
    }
}

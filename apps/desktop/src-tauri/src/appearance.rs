use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tempfile::NamedTempFile;

use crate::config_file::write_json;

const VERSION: u32 = 1;
const DEFAULT_THEME: &str = "dark";
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const BUILT_INS: [(&str, &str, &str, bool); 2] = [
    ("dark", "深色", "#101216", false),
    ("light", "亮色", "#F3F5F8", true),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Background {
    Color { color: String },
    Image { extension: String },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    selected: String,
    opacity: u8,
    custom: BTreeMap<String, Background>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: VERSION,
            selected: DEFAULT_THEME.to_owned(),
            opacity: 100,
            custom: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AppearanceOperation {
    Get,
    Select { name: String },
    AddColor { name: String, color: String },
    AddImage { name: String, path: String },
    Remove { name: String },
    Opacity { value: u8 },
    Reset,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Theme {
    id: String,
    label: String,
    built_in: bool,
    color: String,
    image_path: Option<PathBuf>,
    light: bool,
}

#[derive(Serialize)]
pub struct Appearance {
    selected: String,
    opacity: u8,
    themes: Vec<Theme>,
}

pub struct AppearanceStore {
    directory: PathBuf,
    access: Mutex<()>,
}

impl AppearanceStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: AppearanceOperation) -> Result<Appearance, String> {
        let _access = self.access.lock().map_err(|_| "背景配置暂时不可用。")?;
        let mut config = self.read()?;
        let mut imported_path = None;
        let mut removed_path = None;

        match operation {
            AppearanceOperation::Get => return Ok(self.snapshot(config)),
            AppearanceOperation::Select { name } => {
                let name = theme_id(&name)?;
                if !is_builtin(&name) && !config.custom.contains_key(&name) {
                    return Err(format!("未找到背景「{name}」。"));
                }
                if let Some(Background::Image { extension }) = config.custom.get(&name) {
                    if !self.image_path(&name, extension).is_file() {
                        return Err("背景图片不存在，请移除后重新添加。".to_owned());
                    }
                }
                config.selected = name;
            }
            AppearanceOperation::AddColor { name, color } => {
                let name = new_theme_id(&config, &name)?;
                let color = normalize_color(&color)?;
                config.custom.insert(name, Background::Color { color });
            }
            AppearanceOperation::AddImage { name, path } => {
                let name = new_theme_id(&config, &name)?;
                let (extension, data) = read_image(Path::new(&path))?;
                let path = self.image_path(&name, extension);
                save_image(&path, &data)
                    .map_err(|error| format!("无法复制背景图片（{error}）。"))?;
                imported_path = Some(path);
                config.custom.insert(
                    name,
                    Background::Image {
                        extension: extension.to_owned(),
                    },
                );
            }
            AppearanceOperation::Remove { name } => {
                let name = theme_id(&name)?;
                if is_builtin(&name) {
                    return Err("深色和亮色是内置背景，始终保留。".to_owned());
                }
                let removed = config
                    .custom
                    .remove(&name)
                    .ok_or_else(|| format!("未找到背景「{name}」。"))?;
                if let Background::Image { extension } = removed {
                    removed_path = Some(self.image_path(&name, &extension));
                }
                if config.selected == name {
                    config.selected = DEFAULT_THEME.to_owned();
                }
            }
            AppearanceOperation::Opacity { value } => {
                if value > 100 {
                    return Err("不透明度须是 0 到 100 的整数。".to_owned());
                }
                config.opacity = value;
            }
            AppearanceOperation::Reset => {
                config.selected = DEFAULT_THEME.to_owned();
                config.opacity = 100;
            }
        }

        if let Err(error) = write_json(&self.directory.join("appearance.json"), &config) {
            if let Some(path) = imported_path {
                remove_image(&path);
            }
            return Err(format!("无法保存背景配置（{error}）。"));
        }
        if let Some(path) = removed_path {
            remove_image(&path);
        }
        Ok(self.snapshot(config))
    }

    fn read(&self) -> Result<Config, String> {
        let data = match fs::read(self.directory.join("appearance.json")) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(error) => return Err(format!("无法读取背景配置（{error}）。")),
        };
        let config: Config = serde_json::from_slice(&data)
            .map_err(|error| format!("背景配置格式有误（{error}）。"))?;
        if config.version != VERSION
            || config.opacity > 100
            || (!is_builtin(&config.selected) && !config.custom.contains_key(&config.selected))
        {
            return Err("背景配置格式或版本有误。".to_owned());
        }
        for (name, background) in &config.custom {
            if theme_id(name).as_ref() != Ok(name) || is_builtin(name) {
                return Err("背景配置包含无效名称。".to_owned());
            }
            match background {
                Background::Color { color } => {
                    normalize_color(color)?;
                }
                Background::Image { extension }
                    if !matches!(extension.as_str(), "png" | "jpg" | "webp") =>
                {
                    return Err("背景配置包含无效图片格式。".to_owned());
                }
                _ => {}
            }
        }
        Ok(config)
    }

    fn snapshot(&self, config: Config) -> Appearance {
        let mut themes: Vec<_> = BUILT_INS
            .into_iter()
            .map(|(id, label, color, light)| Theme {
                id: id.to_owned(),
                label: label.to_owned(),
                built_in: true,
                color: color.to_owned(),
                image_path: None,
                light,
            })
            .collect();
        for (id, background) in config.custom {
            let (color, image_path, light) = match background {
                Background::Color { color } => {
                    let value = u32::from_str_radix(&color[1..], 16).unwrap_or_default();
                    let luminance =
                        (value >> 16) * 299 + ((value >> 8) & 255) * 587 + (value & 255) * 114;
                    (color, None, luminance >= 150_000)
                }
                Background::Image { extension } => (
                    "#101216".to_owned(),
                    Some(self.image_path(&id, &extension)),
                    false,
                ),
            };
            themes.push(Theme {
                label: id.clone(),
                id,
                built_in: false,
                color,
                image_path,
                light,
            });
        }
        Appearance {
            selected: config.selected,
            opacity: config.opacity,
            themes,
        }
    }

    fn image_path(&self, name: &str, extension: &str) -> PathBuf {
        self.directory
            .join("backgrounds")
            .join(format!("wallpaper-{name}.{extension}"))
    }
}

fn theme_id(value: &str) -> Result<String, &'static str> {
    match value {
        "深色" => return Ok("dark".to_owned()),
        "亮色" => return Ok("light".to_owned()),
        _ => {}
    }
    if value.chars().count() > 64
        || !value.chars().next().is_some_and(char::is_alphanumeric)
        || !value
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("背景名称须以文字或数字开头，可包含短横线和下划线，最多 64 个字符。");
    }
    Ok(value.to_ascii_lowercase())
}

fn is_builtin(name: &str) -> bool {
    BUILT_INS.iter().any(|(id, ..)| *id == name)
}

fn new_theme_id(config: &Config, name: &str) -> Result<String, String> {
    let name = theme_id(name)?;
    if is_builtin(&name) || config.custom.contains_key(&name) {
        return Err(format!("背景「{name}」已存在。"));
    }
    Ok(name)
}

fn normalize_color(color: &str) -> Result<String, &'static str> {
    let hex = color
        .strip_prefix('#')
        .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if hex.is_none() {
        return Err("颜色请用 #RRGGBB 格式，例如 #171B25。");
    }
    Ok(color.to_ascii_uppercase())
}

fn read_image(path: &Path) -> Result<(&'static str, Vec<u8>), String> {
    if !path.is_absolute() || !path.is_file() {
        return Err("请输入存在的图片绝对路径。".to_owned());
    }
    let mut data = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_IMAGE_BYTES + 1).read_to_end(&mut data))
        .map_err(|error| format!("无法读取背景图片（{error}）。"))?;
    if data.len() as u64 > MAX_IMAGE_BYTES {
        return Err("单张背景图片最大为 16 MiB。".to_owned());
    }
    let extension = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if data.starts_with(b"\xff\xd8\xff") {
        "jpg"
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        "webp"
    } else {
        return Err("背景图片须为 PNG、JPEG 或 WebP 格式。".to_owned());
    };
    Ok((extension, data))
}

fn save_image(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing backgrounds directory"))?;
    fs::create_dir_all(parent)?;
    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(data)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

fn remove_image(path: &Path) {
    if let Err(error) = fs::remove_file(path) {
        if error.kind() != io::ErrorKind::NotFound {
            eprintln!("Could not remove background image: {error}");
        }
    }
}

#[tauri::command]
pub async fn execute_appearance(
    app: AppHandle,
    operation: AppearanceOperation,
) -> Result<Appearance, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppearanceStore>().execute(operation))
        .await
        .map_err(|_| "无法处理背景配置。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_ins_are_permanent_and_reset_keeps_custom_themes() {
        let directory = tempfile::tempdir().unwrap();
        let store = AppearanceStore::new(directory.path().to_owned());
        let defaults = store.execute(AppearanceOperation::Get).unwrap();
        assert_eq!(defaults.selected, "dark");
        assert_eq!(defaults.opacity, 100);
        assert_eq!(defaults.themes.len(), 2);
        for name in ["dark", "亮色"] {
            assert!(store
                .execute(AppearanceOperation::Remove {
                    name: name.to_owned()
                })
                .is_err());
        }
        store
            .execute(AppearanceOperation::AddColor {
                name: "海蓝".to_owned(),
                color: "#172942".to_owned(),
            })
            .unwrap();
        store
            .execute(AppearanceOperation::Select {
                name: "海蓝".to_owned(),
            })
            .unwrap();
        store
            .execute(AppearanceOperation::Opacity { value: 0 })
            .unwrap();
        let restarted = AppearanceStore::new(directory.path().to_owned());
        let saved = restarted.execute(AppearanceOperation::Get).unwrap();
        assert_eq!(saved.selected, "海蓝");
        assert_eq!(saved.opacity, 0);
        let reset = restarted.execute(AppearanceOperation::Reset).unwrap();
        assert_eq!(reset.selected, "dark");
        assert_eq!(reset.opacity, 100);
        assert!(reset.themes.iter().any(|theme| theme.id == "海蓝"));
    }

    #[test]
    fn importing_and_removing_a_wallpaper_keeps_the_source_file() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("sample.png");
        let png = [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 200, 232,
            152, 249, 31, 0, 5, 110, 2, 137, 196, 163, 156, 15, 0, 0, 0, 0, 73, 69, 78, 68, 174,
            66, 96, 130,
        ];
        fs::write(&source, png).unwrap();
        let store = AppearanceStore::new(directory.path().join("config"));
        store
            .execute(AppearanceOperation::AddImage {
                name: "夜景".to_owned(),
                path: source.to_string_lossy().into_owned(),
            })
            .unwrap();
        let selected = store
            .execute(AppearanceOperation::Select {
                name: "夜景".to_owned(),
            })
            .unwrap();
        let imported = selected
            .themes
            .into_iter()
            .find(|theme| theme.id == "夜景")
            .unwrap()
            .image_path
            .unwrap();
        assert_eq!(fs::read(&imported).unwrap(), png);
        let removed = store
            .execute(AppearanceOperation::Remove {
                name: "夜景".to_owned(),
            })
            .unwrap();
        assert_eq!(removed.selected, "dark");
        assert!(!imported.exists());
        assert!(source.is_file());
    }
}

use std::{collections::BTreeMap, fs, io, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Url};

use crate::{
    features::files::open_targets::{spawn_app, validate_path, TargetKind},
    platform::{
        config_file::write_json,
        web_url::{open_in_browser, parse_web_url},
    },
};

const VERSION: u32 = 1;
const SYSTEM_BROWSER: &str = "system";
const DEFAULT_SITE: &str = "baidu";
const QUERY_SLOT: &str = "{query}";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Settings {
    version: u32,
    default_browser: String,
    browsers: BTreeMap<String, String>,
    sites: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut browsers = BTreeMap::new();
        #[cfg(windows)]
        if let Some(path) = standard_chrome_path() {
            browsers.insert("chrome".to_owned(), path);
        }
        let sites = [
            ("baidu", "https://www.baidu.com/s?wd={query}"),
            ("bing", "https://www.bing.com/search?q={query}"),
            ("google", "https://www.google.com/search?q={query}"),
            ("github", "https://github.com/search?q={query}"),
            (
                "bilibili",
                "https://search.bilibili.com/all?keyword={query}",
            ),
            (
                "zhihu",
                "https://www.zhihu.com/search?type=content&q={query}",
            ),
        ]
        .into_iter()
        .map(|(id, template)| (id.to_owned(), template.to_owned()))
        .collect();
        Self {
            version: VERSION,
            default_browser: SYSTEM_BROWSER.to_owned(),
            browsers,
            sites,
        }
    }
}

#[cfg(windows)]
fn standard_chrome_path() -> Option<String> {
    let program_files = std::env::var_os("ProgramFiles")?;
    let path = PathBuf::from(program_files).join("Google/Chrome/Application/chrome.exe");
    path.is_file()
        .then_some(path.to_string_lossy().into_owned())
}

#[derive(Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SearchOperation {
    Run {
        site: Option<String>,
        browser: Option<String>,
        words: Vec<String>,
        infer_site: bool,
        allow_url: bool,
    },
    SiteList,
    SiteAdd {
        name: String,
        template: String,
    },
    SiteRemove {
        name: String,
    },
    BrowserList,
    BrowserAdd {
        name: String,
        path: String,
    },
    BrowserRemove {
        name: String,
    },
    BrowserDefault {
        name: String,
    },
}

pub struct SearchStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl SearchStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn execute(&self, operation: SearchOperation) -> Result<String, String> {
        let access = self.access.lock().map_err(|_| "搜索配置暂时不可用。")?;
        let mut settings = self.read()?;
        let message = match operation {
            SearchOperation::Run {
                site,
                browser,
                words,
                infer_site,
                allow_url,
            } => {
                let search_url =
                    resolve_search(&settings, site.as_deref(), &words, infer_site, allow_url)?;
                let browser = match browser {
                    Some(id) => normalize_name(&id)?,
                    None => settings.default_browser.clone(),
                };
                let path = if browser == SYSTEM_BROWSER {
                    None
                } else {
                    Some(registered_browser(&settings, &browser)?.to_owned())
                };
                drop(access);
                if let Some(path) = path {
                    let app = validate_path(TargetKind::App, &path)
                        .map_err(|error| format!("浏览器「{browser}」不可用。{error}"))?;
                    spawn_app(app, &[search_url.as_str()])?;
                    return Ok(format!(
                        "已交给浏览器「{browser}」打开 {}。",
                        destination(&search_url)
                    ));
                }
                open_in_browser(search_url.as_str())?;
                return Ok(format!(
                    "已交给系统默认浏览器打开 {}。",
                    destination(&search_url)
                ));
            }
            SearchOperation::SiteList => {
                return Ok(settings
                    .sites
                    .iter()
                    .map(|(name, template)| format!("{name}  {template}"))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            SearchOperation::BrowserList => {
                let mut rows = vec![format!(
                    "system  Windows 默认浏览器{}",
                    default_mark(&settings, SYSTEM_BROWSER)
                )];
                rows.extend(settings.browsers.iter().map(|(name, path)| {
                    format!("{name}  {path}{}", default_mark(&settings, name))
                }));
                return Ok(rows.join("\n"));
            }
            SearchOperation::SiteAdd { name, template } => {
                let name = normalize_name(&name)?;
                if matches!(name.as_str(), "site" | "browser") {
                    return Err("请使用其他站点名。".to_owned());
                }
                validate_template(&template)?;
                add_entry(&mut settings.sites, &name, template, "站点")?;
                format!("已保存站点「{name}」。")
            }
            SearchOperation::SiteRemove { name } => {
                let name = normalize_name(&name)?;
                if name == DEFAULT_SITE {
                    return Err("默认搜索站点 baidu 始终保留。".to_owned());
                }
                settings
                    .sites
                    .remove(&name)
                    .ok_or_else(|| unknown_site(&name))?;
                format!("已移除站点「{name}」。")
            }
            SearchOperation::BrowserAdd { name, path } => {
                let name = normalize_name(&name)?;
                if name == SYSTEM_BROWSER {
                    return Err("system 代表 Windows 默认浏览器，请使用其他名称。".to_owned());
                }
                validate_path(TargetKind::App, &path)?;
                add_entry(&mut settings.browsers, &name, path, "浏览器")?;
                format!("已保存浏览器「{name}」。")
            }
            SearchOperation::BrowserRemove { name } => {
                let name = normalize_name(&name)?;
                if name == SYSTEM_BROWSER {
                    return Err("system 始终可选。".to_owned());
                }
                settings
                    .browsers
                    .remove(&name)
                    .ok_or_else(|| unknown_browser(&name))?;
                if settings.default_browser == name {
                    settings.default_browser = SYSTEM_BROWSER.to_owned();
                    format!("已移除浏览器「{name}」，已恢复跟随 Windows 默认浏览器。")
                } else {
                    format!("已移除浏览器「{name}」。")
                }
            }
            SearchOperation::BrowserDefault { name } => {
                let name = normalize_name(&name)?;
                if name != SYSTEM_BROWSER {
                    let path = registered_browser(&settings, &name)?;
                    validate_path(TargetKind::App, path)?;
                }
                settings.default_browser = name.clone();
                format!("Blis 的默认搜索浏览器已设为「{name}」。")
            }
        };
        write_json(&self.path, &settings)
            .map_err(|error| format!("无法保存搜索配置（{error}）。"))?;
        Ok(message)
    }

    fn read(&self) -> Result<Settings, String> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(error) => return Err(format!("无法读取搜索配置（{error}）。")),
        };
        let settings: Settings = serde_json::from_slice(&data)
            .map_err(|error| format!("搜索配置格式有误（{error}）。"))?;
        if settings.version != VERSION
            || !settings.sites.contains_key(DEFAULT_SITE)
            || (settings.default_browser != SYSTEM_BROWSER
                && !settings.browsers.contains_key(&settings.default_browser))
        {
            return Err("搜索配置版本或默认设置有误。".to_owned());
        }
        Ok(settings)
    }
}

fn normalize_name(name: &str) -> Result<String, &'static str> {
    if name.chars().count() > 64
        || !name.chars().next().is_some_and(char::is_alphanumeric)
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("名称须以文字或数字开头，可包含短横线、下划线和点，最多 64 个字符。");
    }
    Ok(name.to_ascii_lowercase())
}

fn add_entry(
    entries: &mut BTreeMap<String, String>,
    name: &str,
    value: String,
    kind: &str,
) -> Result<(), String> {
    let std::collections::btree_map::Entry::Vacant(entry) = entries.entry(name.to_owned()) else {
        return Err(format!("{kind}「{name}」已存在。"));
    };
    entry.insert(value);
    Ok(())
}

fn default_mark(settings: &Settings, name: &str) -> &'static str {
    if settings.default_browser == name {
        "  [默认]"
    } else {
        ""
    }
}

fn unknown_site(name: &str) -> String {
    format!("未找到站点「{name}」。输入 search site list 查看。")
}

fn unknown_browser(name: &str) -> String {
    format!("未找到浏览器「{name}」。输入 search browser list 查看。")
}

fn registered_browser<'a>(settings: &'a Settings, name: &str) -> Result<&'a str, String> {
    settings
        .browsers
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| unknown_browser(name))
}

fn destination(url: &Url) -> &str {
    url.host_str().unwrap_or("网页")
}

fn full_web_url(input: &str) -> Result<Url, &'static str> {
    let (scheme, path) = input
        .split_once("://")
        .ok_or("请输入完整的 HTTP 或 HTTPS 网址。")?;
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || !path
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace() && !matches!(c, '/' | '\\'))
    {
        return Err("请输入完整的 HTTP 或 HTTPS 网址。");
    }
    parse_web_url(input)
}

fn validate_template(template: &str) -> Result<Url, &'static str> {
    let url = full_web_url(template)?;
    let before_fragment = template.split('#').next().unwrap_or_default();
    let has_query_value = before_fragment.split_once('?').is_some_and(|(_, query)| {
        query.split('&').any(|pair| {
            pair.split_once('=')
                .is_some_and(|(key, value)| !key.is_empty() && value == QUERY_SLOT)
        })
    });
    if template.matches(QUERY_SLOT).count() != 1
        || !has_query_value
        || url
            .query_pairs()
            .filter(|(_, value)| value == QUERY_SLOT)
            .count()
            != 1
    {
        return Err("模板须在查询参数里包含一个完整的 {query} 值，例如 ?q={query}。");
    }
    Ok(url)
}

fn resolve_search(
    settings: &Settings,
    site: Option<&str>,
    words: &[String],
    infer_site: bool,
    allow_url: bool,
) -> Result<Url, String> {
    let explicit = site.map(normalize_name).transpose()?;
    let inferred = words
        .first()
        .map(|word| word.to_ascii_lowercase())
        .filter(|name| explicit.is_none() && infer_site && settings.sites.contains_key(name));
    let keywords = words
        .get(usize::from(inferred.is_some())..)
        .unwrap_or_default()
        .join(" ");
    let query = keywords.trim();
    if query.is_empty() {
        return Err("请输入关键词或网址。输入 help search 查看用法。".to_owned());
    }
    let site = explicit.or(inferred);
    if site.is_none() && allow_url && words.len() == 1 && has_url_scheme(query) {
        return Ok(full_web_url(query)?);
    }
    let name = site.as_deref().unwrap_or(DEFAULT_SITE);
    let template = settings.sites.get(name).ok_or_else(|| unknown_site(name))?;
    let mut url = validate_template(template)?;
    let pairs: Vec<_> = url
        .query_pairs()
        .map(|(key, value)| {
            (
                key.into_owned(),
                if value == QUERY_SLOT {
                    query.to_owned()
                } else {
                    value.into_owned()
                },
            )
        })
        .collect();
    url.query_pairs_mut().clear().extend_pairs(pairs);
    Ok(parse_web_url(url.as_str())?)
}

fn has_url_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut chars = scheme.bytes();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
}

#[tauri::command]
pub async fn execute_search(app: AppHandle, operation: SearchOperation) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<super::storage::Storage>()
            .stores()?
            .search
            .execute(operation)
    })
    .await
    .map_err(|_| "无法处理搜索请求。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    #[test]
    fn resolves_bilibili_by_name_and_encodes_the_query_value() {
        let settings = Settings::default();
        let url = resolve_search(
            &settings,
            None,
            &words(&["Bilibili", "C++", "&", "中文", "#攻略"]),
            true,
            true,
        )
        .unwrap();
        assert_eq!(url.as_str(), "https://search.bilibili.com/all?keyword=C%2B%2B+%26+%E4%B8%AD%E6%96%87+%23%E6%94%BB%E7%95%A5");
        assert_eq!(url.fragment(), None);
        assert!(resolve_search(&settings, None, &words(&["bilibili"]), true, true).is_err());
        let url = resolve_search(&settings, None, &words(&["bilibili"]), false, true).unwrap();
        assert_eq!(url.as_str(), "https://www.baidu.com/s?wd=bilibili");
    }

    #[test]
    fn explicit_site_keeps_all_keywords_and_fixed_filters() {
        let settings = Settings::default();
        let url = resolve_search(
            &settings,
            Some("ZHihu"),
            &words(&["github", "csgo"]),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://www.zhihu.com/search?type=content&q=github+csgo"
        );
        assert!(resolve_search(&settings, Some("missing"), &words(&["csgo"]), true, true).is_err());
    }

    #[test]
    fn accepts_direct_web_urls_and_literal_search_terms() {
        let settings = Settings::default();
        let direct = "https://example.com/docs?q=a%26b#heading";
        let url = resolve_search(&settings, None, &words(&[direct]), true, true).unwrap();
        assert_eq!(url.as_str(), direct);
        let url = resolve_search(&settings, None, &words(&[direct]), false, false).unwrap();
        assert_eq!(url.query_pairs().next().unwrap().1, direct);
        for invalid in [
            "file:///C:/Windows",
            "javascript:alert(1)",
            "https:///example.com",
            "https://name:password@example.com",
        ] {
            assert!(resolve_search(&settings, None, &words(&[invalid]), true, true).is_err());
        }
        assert!(resolve_search(&settings, None, &["中文".repeat(500)], true, true).is_err());
    }

    #[test]
    fn requires_one_placeholder_as_the_full_query_parameter_value() {
        assert!(
            validate_template("https://example.com/search?type=post&q={query}#results").is_ok()
        );
        for invalid in [
            "https://example.com/search?q=foo{query}",
            "https://example.com/{query}?q=%7Bquery%7D",
            "https://example.com/search?q=all#?q={query}",
            "https://example.com/search?q={query}&other={query}",
            "https://example.com/search?{query}=other",
            "https://example.com/search?q={query}&other=%7Bquery%7D",
        ] {
            assert!(validate_template(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn saves_sites_and_browser_preferences_across_store_instances() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("search-settings.json");
        let store = SearchStore::new(path.clone());
        let template = "https://example.com/search?q={query}";
        store
            .execute(SearchOperation::SiteAdd {
                name: "MySite".to_owned(),
                template: template.to_owned(),
            })
            .unwrap();
        #[cfg(windows)]
        {
            let app_path = std::env::current_exe()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            store
                .execute(SearchOperation::BrowserAdd {
                    name: "test-browser".to_owned(),
                    path: app_path,
                })
                .unwrap();
            store
                .execute(SearchOperation::BrowserDefault {
                    name: "TEST-browser".to_owned(),
                })
                .unwrap();
        }
        let restarted = SearchStore::new(path.clone());
        assert_eq!(
            restarted.read().unwrap().sites.get("mysite").unwrap(),
            template
        );
        assert!(restarted
            .execute(SearchOperation::SiteRemove {
                name: "baidu".to_owned()
            })
            .is_err());
        restarted
            .execute(SearchOperation::SiteRemove {
                name: "mysite".to_owned(),
            })
            .unwrap();
        #[cfg(windows)]
        {
            assert_eq!(restarted.read().unwrap().default_browser, "test-browser");
            restarted
                .execute(SearchOperation::BrowserRemove {
                    name: "test-browser".to_owned(),
                })
                .unwrap();
        }
        let settings = SearchStore::new(path).read().unwrap();
        assert!(!settings.sites.contains_key("mysite"));
        assert_eq!(settings.default_browser, SYSTEM_BROWSER);
    }
}

use std::ffi::OsStr;
use tauri::Url;

use crate::desktop_open::open_with_system;

const MAX_URL_LENGTH: usize = 8192;

pub(crate) fn parse_web_url(input: &str) -> Result<Url, &'static str> {
    if input.len() > MAX_URL_LENGTH {
        return Err("网址过长，请缩短输入。");
    }
    let url = Url::parse(input).map_err(|_| "网址格式有误。")?;
    if !matches!(url.scheme(), "http" | "https") || !url.has_host() {
        return Err("请输入完整的 HTTP 或 HTTPS 网址。");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("网址不能包含账号或密码。");
    }
    if url.as_str().len() > MAX_URL_LENGTH {
        return Err("网址过长，请缩短输入。");
    }
    Ok(url)
}

pub(crate) fn open_in_browser(input: &str) -> Result<(), String> {
    let url = parse_web_url(input)?;
    open_with_system(OsStr::new(url.as_str())).map_err(|_| "无法打开默认浏览器。".to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_web_url;

    #[test]
    fn accepts_web_urls_and_preserves_the_query() {
        let url = parse_web_url("http://127.0.0.1:8000/path?q=a%26b#section").unwrap();
        assert_eq!(url.query(), Some("q=a%26b"));
        assert_eq!(url.fragment(), Some("section"));
    }

    #[test]
    fn rejects_unsupported_schemes_credentials_and_relative_paths() {
        for input in [
            "file:///C:/Windows",
            "javascript:alert(1)",
            "https://name:password@example.com",
            "example.com/page",
        ] {
            assert!(parse_web_url(input).is_err());
        }
    }
}

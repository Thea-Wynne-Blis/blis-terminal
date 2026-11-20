use std::net::IpAddr;

use serde::Deserialize;
use tokio::sync::Semaphore;

static PING_SLOT: Semaphore = Semaphore::const_new(1);

#[derive(Deserialize)]
pub struct PingOperation {
    host: String,
    count: u8,
}

#[tauri::command]
pub async fn execute_ping(operation: PingOperation) -> Result<String, String> {
    let PingOperation { host, count } = operation;
    if !valid_host(&host) || !(1..=10).contains(&count) {
        return Err("请输入一个有效的主机名或 IP，次数为 1–10。".to_owned());
    }
    let permit = PING_SLOT
        .try_acquire()
        .map_err(|_| "已有 Ping 正在运行。")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        system_ping(&host, count)
    })
    .await
    .map_err(|_| "无法处理 Ping 请求。".to_owned())?
}

fn valid_host(host: &str) -> bool {
    let domain = host.strip_suffix('.').unwrap_or(host);
    host.parse::<IpAddr>().is_ok()
        || (domain.len() <= 253
            && domain.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            }))
}

#[cfg(windows)]
fn system_ping(host: &str, count: u8) -> Result<String, String> {
    use std::{env, os::windows::process::CommandExt, path::PathBuf, process::Command};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let executable = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or("找不到 Windows 系统目录。")?
        .join("System32/ping.exe");
    let result = Command::new(executable)
        .args(["-n", &count.to_string(), "-w", "1000", host])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("无法运行 Windows Ping（{error}）。"))?;
    let output = [result.stdout, result.stderr]
        .iter()
        .map(|bytes| decode_output(bytes))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if output.is_empty() {
        return Err(format!("Windows Ping 没有返回结果（{}）。", result.status));
    }
    Ok(output.trim().replace("\r\n", "\n"))
}

#[cfg(windows)]
fn decode_output(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_OEMCP};

    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    let Ok(len) = i32::try_from(bytes.len()) else {
        return String::from_utf8_lossy(bytes).into_owned();
    };
    let mut wide = vec![0u16; bytes.len()];
    // OEM conversion uses at most one UTF-16 unit per input byte.
    let written =
        unsafe { MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), len) };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    String::from_utf16_lossy(&wide[..written as usize])
}

#[cfg(not(windows))]
fn system_ping(_host: &str, _count: u8) -> Result<String, String> {
    Err("此命令需要 Blis 的 Windows 桌面版。".to_owned())
}

use std::{path::PathBuf, time::Duration};

use serde::Deserialize;
use tauri::{
    AppHandle, Manager, PhysicalPosition, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tokio::{sync::Mutex, time::Instant};

const NOTICE_DURATION: Duration = Duration::from_secs(8);

#[derive(Default)]
pub struct TimerNotices(Mutex<u64>);

#[derive(Deserialize)]
pub struct TimerOperation {
    seconds: u32,
    label: String,
}

#[tauri::command]
pub fn start_timer(app: AppHandle, operation: TimerOperation) -> Result<String, String> {
    let label = operation.label.trim();
    if operation.seconds == 0 {
        return Err("倒计时必须大于 0 秒。".to_owned());
    }
    if label.is_empty() || label.chars().count() > 120 || label.chars().any(char::is_control) {
        return Err("请输入一行提醒名称，最多 120 个字符。".to_owned());
    }
    let duration = Duration::from_secs(u64::from(operation.seconds));
    let deadline = Instant::now()
        .checked_add(duration)
        .ok_or("倒计时长度超出范围。")?;
    let output = format!(
        "已开始「{label}」倒计时，{} 后提醒。",
        duration_text(operation.seconds)
    );
    let label = label.to_owned();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep_until(deadline).await;
        if let Err(error) = show_notice(app, label).await {
            eprintln!("Failed to show timer notice: {error}");
        }
    });
    Ok(output)
}

async fn show_notice(app: AppHandle, label: String) -> tauri::Result<()> {
    let state = app.state::<TimerNotices>();
    let mut sequence = state.0.lock().await;
    *sequence = sequence.wrapping_add(1);
    let id = format!("timer-notice-{sequence}");
    let popup_app = app.clone();

    // On Windows, creating a WebView must run outside the event-loop callback.
    let popup = tauri::async_runtime::spawn_blocking(move || create_notice(&popup_app, id, &label))
        .await??;
    tokio::time::sleep(NOTICE_DURATION).await;
    popup.destroy()
}

fn create_notice(app: &AppHandle, id: String, label: &str) -> tauri::Result<WebviewWindow> {
    let mut url = Url::parse("tauri://localhost/index.html").expect("valid application URL");
    url.query_pairs_mut().append_pair("timer", label);
    let path = PathBuf::from(format!("index.html?{}", url.query().unwrap_or_default()));
    let popup = WebviewWindowBuilder::new(app, id, WebviewUrl::App(path))
        .title("Blis Terminal")
        .inner_size(320.0, 120.0)
        .theme(Some(tauri::Theme::Dark))
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;

    if let (Ok(Some(monitor)), Ok(size)) = (popup.primary_monitor(), popup.outer_size()) {
        let work = monitor.work_area();
        let margin = (16.0 * monitor.scale_factor()).round() as u32;
        let offset = |available: u32, window: u32| {
            available.saturating_sub(window.saturating_add(margin)) as i32
        };
        let _ = popup.set_position(PhysicalPosition::new(
            work.position.x + offset(work.size.width, size.width),
            work.position.y + offset(work.size.height, size.height),
        ));
    }
    if let Err(error) = show_without_focus(&popup) {
        let _ = popup.destroy();
        return Err(error);
    }
    Ok(popup)
}

#[cfg(windows)]
fn show_without_focus(popup: &WebviewWindow) -> tauri::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    };

    let hwnd = popup.hwnd()?.0;
    let flags = SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW;
    // The native visibility change keeps the currently active window in focus.
    let shown = unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, flags) };
    if shown == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn show_without_focus(popup: &WebviewWindow) -> tauri::Result<()> {
    popup.show()
}

fn duration_text(mut seconds: u32) -> String {
    let mut text = Vec::new();
    for (unit, scale) in [("天", 86400), ("小时", 3600), ("分钟", 60), ("秒", 1)] {
        let count = seconds / scale;
        if count > 0 {
            text.push(format!("{count} {unit}"));
        }
        seconds %= scale;
    }
    text.join(" ")
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "opens a reminder on the Windows desktop"]
    fn shows_an_unfocused_notice_while_the_terminal_is_hidden() {
        let app = tauri::Builder::default()
            .any_thread()
            .manage(TimerNotices::default())
            .setup(|app| {
                let terminal = WebviewWindowBuilder::new(
                    app,
                    "terminal",
                    WebviewUrl::App("index.html".into()),
                )
                .visible(false)
                .build()?;
                terminal.hide()?;
                let app = app.handle().clone();
                let label = "托盘提醒测试 & 倒计时";
                start_timer(
                    app.clone(),
                    TimerOperation {
                        seconds: 1,
                        label: label.into(),
                    },
                )
                .map_err(std::io::Error::other)?;
                tauri::async_runtime::spawn(async move {
                    let visible = tokio::time::timeout(Duration::from_secs(15), async {
                        loop {
                            if let Some(popup) = app.get_webview_window("timer-notice-1") {
                                if popup.is_visible().unwrap_or(false)
                                    && popup.url().is_ok_and(|url| {
                                        url.query_pairs()
                                            .any(|(key, value)| key == "timer" && value == label)
                                    })
                                {
                                    let focused = popup.is_focused();
                                    eprintln!("Reminder visibility confirmed. Focus: {focused:?}");
                                    break !focused.unwrap_or(true);
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    })
                    .await
                    .unwrap_or(false);
                    let terminal_visible = terminal.is_visible();
                    eprintln!("Terminal visibility: {terminal_visible:?}");
                    let passed = visible && !terminal_visible.unwrap_or(true);
                    app.exit(if passed { 0 } else { 1 });
                });
                Ok(())
            })
            .build(tauri::generate_context!())
            .unwrap();
        assert_eq!(app.run_return(|_, _| {}), 0);
    }
}

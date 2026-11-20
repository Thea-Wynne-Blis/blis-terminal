use std::process::Child;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchStatus {
    WindowVisible,
    ProcessRunning,
    RequestSubmitted,
    SystemHandoff,
}

pub(crate) fn observe_launch(mut child: Child) -> Result<LaunchStatus, String> {
    #[cfg(windows)]
    {
        use std::{
            thread,
            time::{Duration, Instant},
        };
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("无法读取启动状态（{error}）。"))?
            {
                return if status.success() {
                    Ok(LaunchStatus::RequestSubmitted)
                } else {
                    Err(format!("启动进程提前退出（{status}）。"))
                };
            }
            if has_application_window(child.id()) {
                return Ok(LaunchStatus::WindowVisible);
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(150));
        }
    }
    #[cfg(not(windows))]
    child
        .try_wait()
        .map_err(|error| format!("无法读取启动状态（{error}）。"))?;
    Ok(LaunchStatus::ProcessRunning)
}

#[cfg(windows)]
fn has_application_window(pid: u32) -> bool {
    use windows_sys::{
        core::BOOL,
        Win32::{
            Foundation::{HWND, LPARAM},
            UI::WindowsAndMessaging::{
                EnumWindows, GetWindow, GetWindowLongW, GetWindowTextLengthW,
                GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, GW_OWNER, WS_EX_TOOLWINDOW,
            },
        },
    };

    struct Search {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn inspect(window: HWND, data: LPARAM) -> BOOL {
        // EnumWindows calls this synchronously with a pointer to the live Search below.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner_pid = 0;
        unsafe { GetWindowThreadProcessId(window, &mut owner_pid) };
        if owner_pid == search.pid
            && unsafe {
                IsWindowVisible(window) != 0
                    && GetWindow(window, GW_OWNER).is_null()
                    && GetWindowLongW(window, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW == 0
                    && GetWindowTextLengthW(window) > 0
            }
        {
            search.found = true;
            return 0;
        }
        1
    }
    let mut search = Search { pid, found: false };
    unsafe { EnumWindows(Some(inspect), (&mut search as *mut Search) as LPARAM) };
    search.found
}

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalRect, PhysicalSize, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub const TERMINAL_LABEL: &str = "terminal";
const SHORTCUTS: [&str; 2] = ["Alt+Space", "Alt+Shift+Space"];

pub fn register_shortcut(app: &AppHandle) -> Option<&'static str> {
    SHORTCUTS
        .into_iter()
        .find(|shortcut| match app.global_shortcut().register(*shortcut) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("Failed to register {shortcut}: {error}");
                false
            }
        })
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开终端", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = app.default_window_icon().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing bundled terminal icon",
        )
    })?;

    TrayIconBuilder::new()
        .icon(icon.clone())
        .tooltip("Blis Terminal")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => wake(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                wake(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn wake(app: &AppHandle) {
    let app = app.clone();

    // WebView2 creation can block the Windows event loop if called from an event callback.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = show_terminal(&app) {
            eprintln!("Failed to show terminal: {error}");
        }
    });
}

fn show_terminal(app: &AppHandle) -> tauri::Result<()> {
    // Multiple sources can wake the terminal before the first window has finished opening.
    let wake_lock = app.state::<Mutex<()>>();
    let _guard = wake_lock.lock().unwrap_or_else(|error| error.into_inner());

    let window = match app.get_webview_window(TERMINAL_LABEL) {
        Some(window) => window,
        None => {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == TERMINAL_LABEL)
                .ok_or_else(|| std::io::Error::other("missing terminal window configuration"))?;
            let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
            if let Err(error) = fit_to_display(app, &window) {
                eprintln!("Failed to size terminal for the display: {error}");
            }
            window
        }
    };
    if window.is_minimized()? {
        window.unminimize()?;
    }
    window.show()?;
    window.set_focus()
}

fn fit_to_display(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let cursor_monitor = app
        .cursor_position()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y))
        .ok()
        .flatten();
    let monitor = match cursor_monitor {
        Some(monitor) => monitor,
        None => match app.primary_monitor()? {
            Some(monitor) => monitor,
            None => return Ok(()),
        },
    };
    let work_area = monitor.work_area();
    let desired_frame = PhysicalSize::new(
        work_area.size.width / 2,
        (f64::from(work_area.size.height) * 0.63).round() as u32,
    );

    // Move the hidden window first so the frame is measured at the destination DPI.
    if window
        .current_monitor()?
        .is_none_or(|current| current.position() != monitor.position())
    {
        window.set_size(desired_frame)?;
        window.set_position(centered_position(work_area, desired_frame))?;
    }

    let outer = window.outer_size()?;
    let inner = window.inner_size()?;
    let frame_width = outer.width.saturating_sub(inner.width);
    let frame_height = outer.height.saturating_sub(inner.height);
    let available = PhysicalSize::new(
        work_area.size.width.saturating_sub(frame_width).max(1),
        work_area.size.height.saturating_sub(frame_height).max(1),
    );
    let scale = monitor.scale_factor();
    let available_logical = available.to_logical::<f64>(scale);
    let minimum = LogicalSize::new(
        available_logical.width.min(320.0),
        available_logical.height.min(200.0),
    );
    let minimum_pixels = minimum.to_physical::<u32>(scale);

    window.set_min_size(Some(minimum))?;
    window.set_size(PhysicalSize::new(
        desired_frame
            .width
            .saturating_sub(frame_width)
            .clamp(minimum_pixels.width, available.width),
        desired_frame
            .height
            .saturating_sub(frame_height)
            .clamp(minimum_pixels.height, available.height),
    ))?;
    window.set_position(centered_position(work_area, window.outer_size()?))
}

fn centered_position(
    work_area: &PhysicalRect<i32, u32>,
    size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let x_offset = work_area.size.width.saturating_sub(size.width) / 2;
    let y_offset = work_area.size.height.saturating_sub(size.height) / 2;
    PhysicalPosition::new(
        work_area.position.x + x_offset as i32,
        work_area.position.y + y_offset as i32,
    )
}

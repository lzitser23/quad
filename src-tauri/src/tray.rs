//! The system-tray menu and its event handling. A deep module: the tray's interface to the rest
//! of the app is just `build_tray`, `show_main`, and `update_tray_checks`.

use std::sync::OnceLock;

use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::settings::Settings;
use crate::state::shared;
use crate::{hotkeys, ipc};

struct TrayState {
    drag: CheckMenuItem<tauri::Wry>,
    auto: CheckMenuItem<tauri::Wry>,
}
static TRAY: OnceLock<TrayState> = OnceLock::new();
static TRAY_ICON: OnceLock<TrayIcon> = OnceLock::new();

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Reflect the current settings in the tray's check items.
pub fn update_tray_checks() {
    if let Some(t) = TRAY.get() {
        let s = shared().settings.lock().unwrap();
        let _ = t.drag.set_checked(s.drag_snap_enabled);
        let _ = t.auto.set_checked(s.start_with_windows);
    }
}

fn reload_settings(app: &AppHandle) {
    let fresh = Settings::load();
    *shared().settings.lock().unwrap() = fresh;
    hotkeys::register_all(app);
    shared().settings.lock().unwrap().apply_autostart();
    update_tray_checks();
    ipc::emit_state();
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => show_main(app),
        "drag" => {
            {
                let mut s = shared().settings.lock().unwrap();
                s.drag_snap_enabled = !s.drag_snap_enabled;
                s.save();
            }
            update_tray_checks();
            ipc::emit_state();
        }
        "autostart" => {
            {
                let mut s = shared().settings.lock().unwrap();
                s.start_with_windows = !s.start_with_windows;
                s.apply_autostart();
                s.save();
            }
            update_tray_checks();
            ipc::emit_state();
        }
        "reload" => reload_settings(app),
        "opensettings" => ipc::open_settings_file(),
        "openlog" => ipc::open_log(),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let s = shared().settings.lock().unwrap().clone();

    let open = MenuItemBuilder::with_id("open", "Open Quad").build(app)?;
    let drag = CheckMenuItemBuilder::with_id("drag", "Drag-to-snap")
        .checked(s.drag_snap_enabled)
        .build(app)?;
    let auto = CheckMenuItemBuilder::with_id("autostart", "Start with Windows")
        .checked(s.start_with_windows)
        .build(app)?;
    let reload = MenuItemBuilder::with_id("reload", "Reload settings file").build(app)?;
    let openset = MenuItemBuilder::with_id("opensettings", "Open settings file").build(app)?;
    let openlog = MenuItemBuilder::with_id("openlog", "Open log").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Exit Quad").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open)
        .separator()
        .item(&drag)
        .item(&auto)
        .separator()
        .item(&reload)
        .item(&openset)
        .item(&openlog)
        .separator()
        .item(&quit)
        .build()?;

    let _ = TRAY.set(TrayState { drag, auto });

    let icon = app.default_window_icon().cloned().expect("missing default window icon");
    let tray = TrayIconBuilder::with_id("tray")
        .icon(icon)
        .tooltip("Quad — window tiling")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    let _ = TRAY_ICON.set(tray);
    Ok(())
}

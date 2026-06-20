//! The Rust↔JS contract: serialized DTOs, state snapshots/events, and the Tauri command handlers.
//! The TS mirror lives in `web/src/lib/{types.ts,bridge.ts}`.

use std::ffi::c_void;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;

use crate::actions::{self, Action};
use crate::settings;
use crate::state::shared;
use crate::{hotkeys, tray, winmgr};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionDto {
    action: String,
    display: String,
    default_hotkey: String,
    hotkey: String,
    bound: bool,
    registered: bool,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    drag_snap_enabled: bool,
    start_with_windows: bool,
    show_snap_preview: bool,
    snap_edge_threshold_px: i32,
    resize_step_px: i32,
    gap_px: i32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    version: String,
    settings: SettingsDto,
    actions: Vec<ActionDto>,
    registered_count: i32,
    failed_count: i32,
    settings_path: String,
    log_path: String,
}

#[derive(Serialize)]
pub struct ApplyResult {
    ok: bool,
    message: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    drag_snap_enabled: Option<bool>,
    start_with_windows: Option<bool>,
    show_snap_preview: Option<bool>,
    snap_edge_threshold_px: Option<i32>,
    resize_step_px: Option<i32>,
    gap_px: Option<i32>,
}

pub fn build_state() -> AppStateDto {
    let s = shared().settings.lock().unwrap().clone();
    let regs = shared().regs.lock().unwrap();

    let actions: Vec<ActionDto> = actions::ALL
        .iter()
        .map(|info| {
            let spec = s.hotkey_for(info.key);
            let bound = !spec.trim().is_empty();
            let reg = regs.iter().find(|r| r.action == info.action);
            let success = reg.map(|r| r.success).unwrap_or(false);
            let registered = bound && success;
            let error = if bound && !success {
                reg.and_then(|r| r.error.clone())
            } else {
                None
            };
            ActionDto {
                action: info.key.to_string(),
                display: info.display.to_string(),
                default_hotkey: info.default_hotkey.to_string(),
                hotkey: spec,
                bound,
                registered,
                error,
            }
        })
        .collect();

    let registered_count = actions.iter().filter(|a| a.registered).count() as i32;
    let failed_count = actions.iter().filter(|a| a.bound && !a.registered).count() as i32;

    AppStateDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
        settings: SettingsDto {
            drag_snap_enabled: s.drag_snap_enabled,
            start_with_windows: s.start_with_windows,
            show_snap_preview: s.show_snap_preview,
            snap_edge_threshold_px: s.snap_edge_threshold_px,
            resize_step_px: s.resize_step_px,
            gap_px: s.gap_px,
        },
        actions,
        registered_count,
        failed_count,
        settings_path: settings::settings_path().display().to_string(),
        log_path: settings::log_path().display().to_string(),
    }
}

pub fn emit_state() {
    if let Some(app) = shared().app.get() {
        let _ = app.emit("state", build_state());
    }
}

// ---- Commands ---------------------------------------------------------------

#[tauri::command]
pub fn get_state() -> AppStateDto {
    build_state()
}

#[tauri::command]
pub fn update_settings(patch: SettingsPatch) -> AppStateDto {
    {
        let mut s = shared().settings.lock().unwrap();
        if let Some(v) = patch.drag_snap_enabled {
            s.drag_snap_enabled = v;
        }
        if let Some(v) = patch.start_with_windows {
            s.start_with_windows = v;
            s.apply_autostart();
        }
        if let Some(v) = patch.show_snap_preview {
            s.show_snap_preview = v;
        }
        if let Some(v) = patch.snap_edge_threshold_px {
            s.snap_edge_threshold_px = v.clamp(2, 100);
        }
        if let Some(v) = patch.resize_step_px {
            s.resize_step_px = v.clamp(5, 400);
        }
        if let Some(v) = patch.gap_px {
            s.gap_px = v.clamp(0, 100);
        }
        s.save();
    }
    tray::update_tray_checks();
    build_state()
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, action: String, spec: String) -> Result<AppStateDto, String> {
    if Action::from_key(&action).is_some() {
        let spec = spec.trim().to_string();
        if !spec.is_empty() && hotkeys::parse_shortcut(&spec).is_none() {
            return Err(format!("'{spec}' is not a valid hotkey"));
        }
        {
            let mut s = shared().settings.lock().unwrap();
            s.hotkeys.insert(action.clone(), spec.clone());
            s.save();
        }
        hotkeys::register_all(&app);
    }
    Ok(build_state())
}

#[tauri::command]
pub fn apply_action(action: String) -> ApplyResult {
    let a = match Action::from_key(&action) {
        Some(a) => a,
        None => return ApplyResult { ok: false, message: "Unknown action.".into() },
    };
    // Mission Control is global — no target window needed.
    if a == Action::MissionControl {
        winmgr::show_task_view();
        return ApplyResult { ok: true, message: format!("Applied {}", a.display()) };
    }
    let target = shared().last_active.load(Ordering::Relaxed);
    let hwnd = HWND(target as *mut c_void);
    if target == 0 || !winmgr::is_manageable(hwnd) {
        return ApplyResult {
            ok: false,
            message: "No recent window. Click a normal app window, then try again.".into(),
        };
    }
    let settings = shared().settings.lock().unwrap().clone();
    shared().wm.lock().unwrap().execute_on(a, hwnd, &settings);
    winmgr::set_foreground(hwnd);
    ApplyResult { ok: true, message: format!("Applied {}", a.display()) }
}

#[tauri::command]
pub fn open_log() {
    open_path(settings::log_path());
}

#[tauri::command]
pub fn open_settings_file() {
    open_path(settings::settings_path());
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn open_path(p: std::path::PathBuf) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &p.display().to_string()])
        .spawn();
}

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use windows::Win32::Foundation::HWND;

use crate::actions::{self, Action};
use crate::settings::{self, Settings};
use crate::winmgr::{self, Rect, WindowManager};

// ---- Shared state -----------------------------------------------------------

pub struct Reg {
    pub action: Action,
    pub bound: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub struct Shared {
    pub settings: Mutex<Settings>,
    pub wm: Mutex<WindowManager>,
    pub last_active: AtomicIsize,
    pub drag_hwnd: AtomicIsize,
    pub current_zone: Mutex<Option<Rect>>,
    pub overlay: AtomicIsize,
    pub regs: Mutex<Vec<Reg>>,
    pub shortcuts: Mutex<Vec<(Shortcut, Action)>>,
    pub app: OnceLock<AppHandle>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

pub fn shared() -> &'static Shared {
    SHARED.get().expect("shared state not initialized")
}

struct TrayState {
    drag: CheckMenuItem<tauri::Wry>,
    auto: CheckMenuItem<tauri::Wry>,
}
static TRAY: OnceLock<TrayState> = OnceLock::new();
static TRAY_ICON: OnceLock<TrayIcon> = OnceLock::new();

// ---- DTOs (camelCase, shared with the React UI) -----------------------------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ActionDto {
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
struct SettingsDto {
    drag_snap_enabled: bool,
    start_with_windows: bool,
    show_snap_preview: bool,
    snap_edge_threshold_px: i32,
    resize_step_px: i32,
    gap_px: i32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppStateDto {
    version: String,
    settings: SettingsDto,
    actions: Vec<ActionDto>,
    registered_count: i32,
    failed_count: i32,
    settings_path: String,
    log_path: String,
}

#[derive(Serialize)]
struct ApplyResult {
    ok: bool,
    message: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    drag_snap_enabled: Option<bool>,
    start_with_windows: Option<bool>,
    show_snap_preview: Option<bool>,
    snap_edge_threshold_px: Option<i32>,
    resize_step_px: Option<i32>,
    gap_px: Option<i32>,
}

fn build_state() -> AppStateDto {
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

fn emit_state() {
    if let Some(app) = shared().app.get() {
        let _ = app.emit("state", build_state());
    }
}

// ---- Tauri commands ---------------------------------------------------------

#[tauri::command]
fn get_state() -> AppStateDto {
    build_state()
}

#[tauri::command]
fn update_settings(patch: SettingsPatch) -> AppStateDto {
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
    update_tray_checks();
    build_state()
}

#[tauri::command]
fn set_hotkey(app: AppHandle, action: String, spec: String) -> Result<AppStateDto, String> {
    if Action::from_key(&action).is_some() {
        let spec = spec.trim().to_string();
        if !spec.is_empty() && parse_shortcut(&spec).is_none() {
            return Err(format!("'{spec}' is not a valid hotkey"));
        }
        {
            let mut s = shared().settings.lock().unwrap();
            s.hotkeys.insert(action.clone(), spec.clone());
            s.save();
        }
        register_all(&app);
    }
    Ok(build_state())
}

#[tauri::command]
fn apply_action(action: String) -> ApplyResult {
    let a = match Action::from_key(&action) {
        Some(a) => a,
        None => return ApplyResult { ok: false, message: "Unknown action.".into() },
    };
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
fn open_log() {
    open_path(settings::log_path());
}

#[tauri::command]
fn open_settings_file() {
    open_path(settings::settings_path());
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn open_path(p: std::path::PathBuf) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &p.display().to_string()])
        .spawn();
}

// ---- Hotkeys ----------------------------------------------------------------

fn run_foreground(a: Action) {
    let settings = shared().settings.lock().unwrap().clone();
    shared().wm.lock().unwrap().execute(a, &settings);
}

fn register_all(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let mut map: Vec<(Shortcut, Action)> = Vec::new();
    let mut regs: Vec<Reg> = Vec::new();
    let settings = shared().settings.lock().unwrap().clone();

    for info in actions::ALL {
        let spec = settings.hotkey_for(info.key);
        if spec.trim().is_empty() {
            regs.push(Reg { action: info.action, bound: false, success: true, error: None });
            continue;
        }
        match parse_shortcut(&spec) {
            None => regs.push(Reg {
                action: info.action,
                bound: true,
                success: false,
                error: Some("could not parse hotkey".into()),
            }),
            Some(sc) => match gs.register(sc.clone()) {
                Ok(()) => {
                    map.push((sc, info.action));
                    regs.push(Reg { action: info.action, bound: true, success: true, error: None });
                }
                Err(e) => regs.push(Reg {
                    action: info.action,
                    bound: true,
                    success: false,
                    error: Some(describe_err(&e.to_string())),
                }),
            },
        }
    }

    let ok = regs.iter().filter(|r| r.bound && r.success).count();
    let failed = regs.iter().filter(|r| r.bound && !r.success).count();
    settings::log("INFO", &format!("registered {ok} hotkeys, {failed} conflicts"));

    *shared().shortcuts.lock().unwrap() = map;
    *shared().regs.lock().unwrap() = regs;
}

fn describe_err(e: &str) -> String {
    let l = e.to_lowercase();
    if l.contains("already") || l.contains("registered") {
        "already in use by another app or Windows".into()
    } else {
        e.to_string()
    }
}

fn gs_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|_app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let action = {
                let map = shared().shortcuts.lock().unwrap();
                map.iter().find(|(s, _)| s == shortcut).map(|(_, a)| *a)
            };
            if let Some(a) = action {
                run_foreground(a);
            }
        })
        .build()
}

fn parse_shortcut(spec: &str) -> Option<Shortcut> {
    if spec.trim().is_empty() {
        return None;
    }
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for raw in spec.split('+') {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        match t.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "win" | "cmd" | "super" | "meta" => mods |= Modifiers::SUPER,
            _ => {
                if code.is_some() {
                    return None;
                }
                code = Some(map_code(t)?);
            }
        }
    }
    Some(Shortcut::new(Some(mods), code?))
}

fn map_code(t: &str) -> Option<Code> {
    let lower = t.to_ascii_lowercase();
    Some(match lower.as_str() {
        "left" => Code::ArrowLeft,
        "right" => Code::ArrowRight,
        "up" => Code::ArrowUp,
        "down" => Code::ArrowDown,
        "enter" | "return" => Code::Enter,
        "space" => Code::Space,
        "back" | "backspace" => Code::Backspace,
        "delete" | "del" => Code::Delete,
        "insert" | "ins" => Code::Insert,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "tab" => Code::Tab,
        "escape" | "esc" => Code::Escape,
        "oemplus" | "plus" | "=" => Code::Equal,
        "oemminus" | "minus" | "-" => Code::Minus,
        "oemcomma" | "," => Code::Comma,
        "oemperiod" | "." => Code::Period,
        "oem1" | ";" => Code::Semicolon,
        "oem2" | "/" => Code::Slash,
        "oem4" | "[" => Code::BracketLeft,
        "oem6" | "]" => Code::BracketRight,
        _ => {
            if lower.len() == 1 {
                return single_code(lower.chars().next().unwrap());
            }
            if let Some(rest) = lower.strip_prefix('f') {
                if let Ok(n) = rest.parse::<u32>() {
                    return fkey(n);
                }
            }
            return None;
        }
    })
}

fn single_code(c: char) -> Option<Code> {
    Some(match c.to_ascii_uppercase() {
        'A' => Code::KeyA, 'B' => Code::KeyB, 'C' => Code::KeyC, 'D' => Code::KeyD,
        'E' => Code::KeyE, 'F' => Code::KeyF, 'G' => Code::KeyG, 'H' => Code::KeyH,
        'I' => Code::KeyI, 'J' => Code::KeyJ, 'K' => Code::KeyK, 'L' => Code::KeyL,
        'M' => Code::KeyM, 'N' => Code::KeyN, 'O' => Code::KeyO, 'P' => Code::KeyP,
        'Q' => Code::KeyQ, 'R' => Code::KeyR, 'S' => Code::KeyS, 'T' => Code::KeyT,
        'U' => Code::KeyU, 'V' => Code::KeyV, 'W' => Code::KeyW, 'X' => Code::KeyX,
        'Y' => Code::KeyY, 'Z' => Code::KeyZ,
        '0' => Code::Digit0, '1' => Code::Digit1, '2' => Code::Digit2, '3' => Code::Digit3,
        '4' => Code::Digit4, '5' => Code::Digit5, '6' => Code::Digit6, '7' => Code::Digit7,
        '8' => Code::Digit8, '9' => Code::Digit9,
        _ => return None,
    })
}

fn fkey(n: u32) -> Option<Code> {
    Some(match n {
        1 => Code::F1, 2 => Code::F2, 3 => Code::F3, 4 => Code::F4,
        5 => Code::F5, 6 => Code::F6, 7 => Code::F7, 8 => Code::F8,
        9 => Code::F9, 10 => Code::F10, 11 => Code::F11, 12 => Code::F12,
        _ => return None,
    })
}

// ---- Tray -------------------------------------------------------------------

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn update_tray_checks() {
    if let Some(t) = TRAY.get() {
        let s = shared().settings.lock().unwrap();
        let _ = t.drag.set_checked(s.drag_snap_enabled);
        let _ = t.auto.set_checked(s.start_with_windows);
    }
}

fn reload_settings(app: &AppHandle) {
    let fresh = Settings::load();
    *shared().settings.lock().unwrap() = fresh;
    register_all(app);
    shared().settings.lock().unwrap().apply_autostart();
    update_tray_checks();
    emit_state();
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
            emit_state();
        }
        "autostart" => {
            {
                let mut s = shared().settings.lock().unwrap();
                s.start_with_windows = !s.start_with_windows;
                s.apply_autostart();
                s.save();
            }
            update_tray_checks();
            emit_state();
        }
        "reload" => reload_settings(app),
        "opensettings" => open_settings_file(),
        "openlog" => open_log(),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
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

// ---- Entry ------------------------------------------------------------------

pub fn run() {
    let settings = Settings::load();
    let _ = SHARED.set(Shared {
        settings: Mutex::new(settings),
        wm: Mutex::new(WindowManager::new()),
        last_active: AtomicIsize::new(0),
        drag_hwnd: AtomicIsize::new(0),
        current_zone: Mutex::new(None),
        overlay: AtomicIsize::new(0),
        regs: Mutex::new(Vec::new()),
        shortcuts: Mutex::new(Vec::new()),
        app: OnceLock::new(),
    });

    tauri::Builder::default()
        .plugin(gs_plugin())
        .invoke_handler(tauri::generate_handler![
            get_state,
            update_settings,
            set_hotkey,
            apply_action,
            open_log,
            open_settings_file,
            quit_app
        ])
        .setup(|app| {
            let _ = shared().app.set(app.handle().clone());
            shared().settings.lock().unwrap().apply_autostart();
            register_all(app.handle());
            build_tray(app)?;
            crate::native::spawn_worker();
            if std::env::args().any(|a| a == "--open") {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Quad");
}

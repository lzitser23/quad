//! Process-wide shared state. One owner for the singletons that the Tauri commands, the tray,
//! the hotkey handler, and the Win32 worker thread all reach. Mutated behind locks/atomics.

use std::sync::atomic::AtomicIsize;
use std::sync::{Mutex, OnceLock};

use tauri::AppHandle;
use tauri_plugin_global_shortcut::Shortcut;

use crate::actions::Action;
use crate::layout::Rect;
use crate::settings::Settings;
use crate::winmgr::WindowManager;

/// One hotkey registration outcome, surfaced to the UI so conflicts are visible per action.
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

/// Build and install the shared state. Call once, before anything reads `shared()`.
pub fn init() {
    let _ = SHARED.set(Shared {
        settings: Mutex::new(Settings::load()),
        wm: Mutex::new(WindowManager::new()),
        last_active: AtomicIsize::new(0),
        drag_hwnd: AtomicIsize::new(0),
        current_zone: Mutex::new(None),
        overlay: AtomicIsize::new(0),
        regs: Mutex::new(Vec::new()),
        shortcuts: Mutex::new(Vec::new()),
        app: OnceLock::new(),
    });
}

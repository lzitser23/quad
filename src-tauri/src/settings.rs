use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub drag_snap_enabled: bool,
    pub start_with_windows: bool,
    pub show_snap_preview: bool,
    pub snap_edge_threshold_px: i32,
    pub resize_step_px: i32,
    pub gap_px: i32,
    pub hotkeys: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            drag_snap_enabled: true,
            start_with_windows: false,
            show_snap_preview: true,
            snap_edge_threshold_px: 20,
            resize_step_px: 30,
            gap_px: 0,
            hotkeys: HashMap::new(),
        }
    }
}

pub fn dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("Quad")
}
pub fn settings_path() -> PathBuf {
    dir().join("settings.json")
}
pub fn log_path() -> PathBuf {
    dir().join("quad.log")
}

impl Settings {
    pub fn load() -> Settings {
        let _ = std::fs::create_dir_all(dir());
        let mut s: Settings = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        s.fill_defaults();
        s.save();
        s
    }

    pub fn fill_defaults(&mut self) {
        for info in crate::actions::ALL {
            self.hotkeys
                .entry(info.key.to_string())
                .or_insert_with(|| info.default_hotkey.to_string());
        }
    }

    pub fn hotkey_for(&self, key: &str) -> String {
        self.hotkeys.get(key).cloned().unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = std::fs::create_dir_all(dir());
            let _ = std::fs::write(settings_path(), t);
        }
    }

    pub fn apply_autostart(&self) {
        set_autostart(self.start_with_windows);
    }
}

fn set_autostart(enable: bool) {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((run, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let _ = run.set_value("Quad", &format!("\"{}\"", exe.display()));
            }
        } else {
            let _ = run.delete_value("Quad");
        }
    }
}

pub fn log(level: &str, msg: &str) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{secs}] [{level}] {msg}\n");
    let _ = std::fs::create_dir_all(dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

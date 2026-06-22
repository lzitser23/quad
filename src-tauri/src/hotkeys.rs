//! The hotkey-spec grammar (parse `"Ctrl+Alt+Oemplus"` → a `Shortcut`) and global-shortcut
//! registration. The single Rust owner of the spec format; the web side mirrors it in
//! `web/src/lib/hotkeys.ts` — the spec string is the seam between them.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::actions::{self, Action};
use crate::settings;
use crate::state::{shared, Reg};

/// Run an action against the current foreground window (hotkey-driven path).
pub fn run_foreground(a: Action) {
    let settings = shared().settings.lock().unwrap().clone();
    shared().wm.lock().unwrap().execute(a, &settings);
}

/// (Re)register every bound hotkey, recording per-action success/conflict for the UI.
pub fn register_all(app: &AppHandle) {
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

/// The global-shortcut plugin, wired to dispatch a fired shortcut to its action.
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
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

// ---- The grammar ------------------------------------------------------------

/// Parse a spec string (`"Ctrl+Alt+Win+Right"`, `"Ctrl+Alt+Oemplus"`) into a `Shortcut`.
/// Returns `None` for an empty or unparseable spec.
pub fn parse_shortcut(spec: &str) -> Option<Shortcut> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifiers_and_keys() {
        assert_eq!(
            parse_shortcut("Ctrl+Alt+Left"),
            Some(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowLeft))
        );
        assert_eq!(
            parse_shortcut("Ctrl+Alt+Win+Right"),
            Some(Shortcut::new(
                Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER),
                Code::ArrowRight
            ))
        );
        assert_eq!(
            parse_shortcut("Ctrl+Alt+Oemplus"),
            Some(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Equal))
        );
        assert_eq!(
            parse_shortcut("Ctrl+Alt+M"),
            Some(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyM))
        );
    }

    #[test]
    fn case_insensitive_and_aliases() {
        assert_eq!(parse_shortcut("ctrl+alt+back"), parse_shortcut("Control+Alt+Backspace"));
        assert_eq!(parse_shortcut("ctrl+alt+="), parse_shortcut("Ctrl+Alt+OemPlus"));
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert_eq!(parse_shortcut(""), None);
        assert_eq!(parse_shortcut("   "), None);
        assert_eq!(parse_shortcut("Ctrl+Alt+notakey"), None);
        // two non-modifier keys is invalid
        assert_eq!(parse_shortcut("Ctrl+A+B"), None);
    }
}

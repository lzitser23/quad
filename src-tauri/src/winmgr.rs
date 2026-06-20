//! Imperative shell over the pure `layout` module: monitor enumeration, Win32 window I/O, and
//! the per-window cycle/restore state. Resolves HWNDs + monitors, calls `layout`, applies results.

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW,
    GetWindowRect, IsWindow, IsWindowVisible, IsZoomed, SetForegroundWindow, SetWindowPos,
    ShowWindow, GWL_STYLE, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SW_RESTORE, WS_CAPTION,
    WS_MINIMIZE, WS_THICKFRAME,
};

use crate::actions::Action;
use crate::layout;
use crate::settings::Settings;

pub use crate::layout::Rect;

const MONITORINFOF_PRIMARY: u32 = 1;

impl From<RECT> for Rect {
    fn from(r: RECT) -> Self {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }
}

#[derive(Clone, Copy)]
pub struct Monitor {
    pub handle: isize,
    pub bounds: Rect,
    pub work: Rect,
    pub primary: bool,
}

// ---- Monitors ---------------------------------------------------------------

unsafe extern "system" fn enum_proc(hmon: HMONITOR, _hdc: HDC, _rc: *mut RECT, data: LPARAM) -> BOOL {
    let vec = &mut *(data.0 as *mut Vec<Monitor>);
    if let Some(m) = query_monitor(hmon) {
        vec.push(m);
    }
    TRUE
}

unsafe fn query_monitor(hmon: HMONITOR) -> Option<Monitor> {
    let mut mi = MONITORINFOEXW::default();
    mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmon, &mut mi.monitorInfo as *mut MONITORINFO).as_bool() {
        Some(Monitor {
            handle: hmon.0 as isize,
            bounds: mi.monitorInfo.rcMonitor.into(),
            work: mi.monitorInfo.rcWork.into(),
            primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        })
    } else {
        None
    }
}

pub fn all_monitors() -> Vec<Monitor> {
    let mut v: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut v as *mut _ as isize),
        );
    }
    v.sort_by(|a, b| a.bounds.left.cmp(&b.bounds.left).then(a.bounds.top.cmp(&b.bounds.top)));
    v
}

fn primary_monitor() -> Monitor {
    let all = all_monitors();
    all.iter()
        .find(|m| m.primary)
        .or_else(|| all.first())
        .copied()
        .unwrap_or(Monitor {
            handle: 0,
            bounds: Rect::new(0, 0, 1920, 1080),
            work: Rect::new(0, 0, 1920, 1040),
            primary: true,
        })
}

pub fn monitor_from_window(hwnd: HWND) -> Monitor {
    unsafe {
        let h = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        query_monitor(h).unwrap_or_else(primary_monitor)
    }
}

pub fn monitor_from_point(pt: POINT) -> Monitor {
    unsafe {
        let h = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if h.0.is_null() {
            return primary_monitor();
        }
        query_monitor(h).unwrap_or_else(primary_monitor)
    }
}

fn relative(from: &Monitor, step: i32) -> Monitor {
    let all = all_monitors();
    if all.len() <= 1 {
        return *from;
    }
    let idx = all.iter().position(|m| m.handle == from.handle).unwrap_or(0) as i32;
    let n = all.len() as i32;
    let next = (((idx + step) % n) + n) % n;
    all[next as usize]
}

// ---- Window inspection ------------------------------------------------------

pub fn is_manageable(hwnd: HWND) -> bool {
    unsafe {
        if hwnd.0.is_null() {
            return false;
        }
        if !IsWindow(hwnd).as_bool() {
            return false;
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4).is_ok()
            && cloaked != 0
        {
            return false;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        if style & WS_MINIMIZE.0 != 0 {
            return false;
        }
        if style & (WS_CAPTION.0 | WS_THICKFRAME.0) == 0 {
            return false;
        }
        if hwnd == GetShellWindow() {
            return false;
        }
        let mut buf = [0u16; 256];
        let n = GetClassNameW(hwnd, &mut buf);
        let cls = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Windows.UI.Core.CoreWindow"
        ) {
            return false;
        }
        true
    }
}

pub fn visible_rect(hwnd: HWND) -> Rect {
    unsafe {
        let mut r = RECT::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut _ as *mut c_void,
            size_of::<RECT>() as u32,
        )
        .is_ok()
        {
            return r.into();
        }
        let mut g = RECT::default();
        let _ = GetWindowRect(hwnd, &mut g);
        g.into()
    }
}

/// Position a window so its *visible* frame occupies `target`, compensating for the DWM
/// invisible resize border.
pub fn apply_visible_rect(hwnd: HWND, target: Rect) {
    unsafe {
        if IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let mut outer = RECT::default();
        let _ = GetWindowRect(hwnd, &mut outer);
        let mut frame = RECT::default();
        let (il, it, ir, ib) = if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut _ as *mut c_void,
            size_of::<RECT>() as u32,
        )
        .is_ok()
        {
            (
                frame.left - outer.left,
                frame.top - outer.top,
                outer.right - frame.right,
                outer.bottom - frame.bottom,
            )
        } else {
            (0, 0, 0, 0)
        };

        let x = target.left - il;
        let y = target.top - it;
        let w = target.w() + il + ir;
        let h = target.h() + it + ib;
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

pub fn set_foreground(hwnd: HWND) {
    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn cursor_pos() -> POINT {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    p
}

/// Resolve the monitor under the cursor and ask `layout` for the snap target.
pub fn compute_zone(pt: POINT, settings: &Settings) -> Option<Rect> {
    let mon = monitor_from_point(pt);
    layout::zone(pt.x, pt.y, mon.bounds, mon.work, settings.snap_edge_threshold_px)
}

/// Mission Control equivalent — synthesise Win+Tab to open Windows Task View
/// (all windows on the current desktop + the virtual-desktop strip).
pub fn show_task_view() {
    let mk = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [
        // Release the modifiers the triggering hotkey (Ctrl+Alt+M) is still physically holding —
        // otherwise the injected Tab is read as Alt+Tab (a window switcher) instead of Win+Tab.
        mk(VK_LMENU, true),
        mk(VK_RMENU, true),
        mk(VK_LCONTROL, true),
        mk(VK_RCONTROL, true),
        // Clean Win+Tab → Windows Task View (all windows fan out; click one).
        mk(VK_LWIN, false),
        mk(VK_TAB, false),
        mk(VK_TAB, true),
        mk(VK_LWIN, true),
    ];
    unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

// ---- Window manager (cycle + restore state; the imperative shell) -----------

pub struct WindowManager {
    restore: HashMap<isize, Rect>,
    last_applied: HashMap<isize, Rect>,
    cycle: layout::Cycle,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            restore: HashMap::new(),
            last_applied: HashMap::new(),
            cycle: layout::reset(),
        }
    }

    pub fn execute(&mut self, action: Action, settings: &Settings) {
        let hwnd = foreground();
        self.execute_on(action, hwnd, settings);
    }

    pub fn execute_on(&mut self, action: Action, hwnd: HWND, settings: &Settings) {
        // Global action — no window needed.
        if action == Action::MissionControl {
            show_task_view();
            return;
        }
        if !is_manageable(hwnd) {
            return;
        }
        let key = hwnd.0 as isize;
        let mon = monitor_from_window(hwnd);

        if action == Action::Restore {
            if let Some(rrect) = self.restore.get(&key).copied() {
                apply_visible_rect(hwnd, rrect);
                self.last_applied.insert(key, rrect);
            }
            self.cycle = layout::reset();
            return;
        }

        self.capture_restore(hwnd, key);

        if matches!(action, Action::NextDisplay | Action::PreviousDisplay) {
            self.cycle = layout::reset();
            let step = if action == Action::NextDisplay { 1 } else { -1 };
            let to = relative(&mon, step);
            if to.handle != mon.handle {
                let target = layout::map_proportional(visible_rect(hwnd), mon.work, to.work);
                apply_visible_rect(hwnd, target);
                self.last_applied.insert(key, target);
            }
            return;
        }

        let idx = if layout::cycles(action) {
            self.cycle = layout::advance(self.cycle, action, key, 3);
            self.cycle.index()
        } else {
            self.cycle = layout::reset();
            0
        };

        let current = visible_rect(hwnd);
        if let Some(target) =
            layout::target_rect(action, mon.work, current, idx, settings.gap_px, settings.resize_step_px)
        {
            apply_visible_rect(hwnd, target);
            self.last_applied.insert(key, target);
        }
    }

    /// Remember pre-snap geometry the first time, or whenever the user has moved the window since.
    fn capture_restore(&mut self, hwnd: HWND, key: isize) {
        let cur = visible_rect(hwnd);
        let free = match self.last_applied.get(&key) {
            Some(last) => !layout::nearly_eq(cur, *last),
            None => true,
        };
        if !self.restore.contains_key(&key) || free {
            self.restore.insert(key, cur);
        }
    }
}

//! Win32 implementation of the window I/O behind `winmgr`: monitor enumeration, window
//! inspection, and the DWM-aware visible-rect move. Windows are addressed by the opaque
//! `WinId` (an `HWND` cast to `isize`); geometry by `layout::Rect`.

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

use super::{Monitor, Point, Rect, WinId};

const MONITORINFOF_PRIMARY: u32 = 1;

#[inline]
fn hwnd(id: WinId) -> HWND {
    HWND(id as *mut c_void)
}

impl From<RECT> for Rect {
    fn from(r: RECT) -> Self {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }
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

pub fn monitor_from_window(id: WinId) -> Monitor {
    unsafe {
        let h = MonitorFromWindow(hwnd(id), MONITOR_DEFAULTTONEAREST);
        query_monitor(h).unwrap_or_else(super::primary_monitor)
    }
}

pub fn monitor_from_point(pt: Point) -> Monitor {
    unsafe {
        let h = MonitorFromPoint(POINT { x: pt.x, y: pt.y }, MONITOR_DEFAULTTONEAREST);
        if h.0.is_null() {
            return super::primary_monitor();
        }
        query_monitor(h).unwrap_or_else(super::primary_monitor)
    }
}

// ---- Window inspection ------------------------------------------------------

pub fn is_manageable(id: WinId) -> bool {
    unsafe {
        let h = hwnd(id);
        if h.0.is_null() {
            return false;
        }
        if !IsWindow(h).as_bool() {
            return false;
        }
        if !IsWindowVisible(h).as_bool() {
            return false;
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4).is_ok()
            && cloaked != 0
        {
            return false;
        }
        let style = GetWindowLongPtrW(h, GWL_STYLE) as u32;
        if style & WS_MINIMIZE.0 != 0 {
            return false;
        }
        if style & (WS_CAPTION.0 | WS_THICKFRAME.0) == 0 {
            return false;
        }
        if h == GetShellWindow() {
            return false;
        }
        let mut buf = [0u16; 256];
        let n = GetClassNameW(h, &mut buf);
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

pub fn visible_rect(id: WinId) -> Rect {
    unsafe {
        let h = hwnd(id);
        let mut r = RECT::default();
        if DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut _ as *mut c_void,
            size_of::<RECT>() as u32,
        )
        .is_ok()
        {
            return r.into();
        }
        let mut g = RECT::default();
        let _ = GetWindowRect(h, &mut g);
        g.into()
    }
}

/// Position a window so its *visible* frame occupies `target`, compensating for the DWM
/// invisible resize border.
pub fn apply_visible_rect(id: WinId, target: Rect) {
    unsafe {
        let h = hwnd(id);
        if IsZoomed(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let mut outer = RECT::default();
        let _ = GetWindowRect(h, &mut outer);
        let mut frame = RECT::default();
        let (il, it, ir, ib) = if DwmGetWindowAttribute(
            h,
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
        let hgt = target.h() + it + ib;
        let _ = SetWindowPos(
            h,
            HWND::default(),
            x,
            y,
            w,
            hgt,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

pub fn set_foreground(id: WinId) {
    unsafe {
        let _ = SetForegroundWindow(hwnd(id));
    }
}

pub fn foreground() -> WinId {
    unsafe { GetForegroundWindow().0 as isize }
}

pub fn cursor_pos() -> Point {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    Point { x: p.x, y: p.y }
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

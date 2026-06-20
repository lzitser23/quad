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
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW,
    GetWindowRect, IsWindow, IsWindowVisible, IsZoomed, SetForegroundWindow, SetWindowPos,
    ShowWindow, GWL_STYLE, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SW_RESTORE, WS_CAPTION,
    WS_MINIMIZE, WS_THICKFRAME,
};

use crate::actions::Action;
use crate::settings::Settings;

const MONITORINFOF_PRIMARY: u32 = 1;

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Rect { left, top, right, bottom }
    }
    pub fn w(&self) -> i32 {
        self.right - self.left
    }
    pub fn h(&self) -> i32 {
        self.bottom - self.top
    }
    fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect::new(x, y, x + w, y + h)
    }
}

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

fn r(v: f64) -> i32 {
    v.round() as i32
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
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut c_void,
            4,
        )
        .is_ok()
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

// ---- Drag-snap zone detection ----------------------------------------------

/// Maps a cursor position near a screen edge/corner to a snap target (work-area rect).
pub fn compute_zone(pt: POINT, settings: &Settings) -> Option<Rect> {
    let mon = monitor_from_point(pt);
    let b = mon.bounds;
    let wa = mon.work;

    let edge = settings.snap_edge_threshold_px.max(2);
    let corner_w = ((wa.w() as f64 * 0.25) as i32).clamp(80, 400);
    let corner_h = ((wa.h() as f64 * 0.25) as i32).clamp(80, 400);

    let left = pt.x <= b.left + edge;
    let right = pt.x >= b.right - edge - 1;
    let top = pt.y <= b.top + edge;
    let bottom = pt.y >= b.bottom - edge - 1;

    let hw = wa.w() / 2;
    let hh = wa.h() / 2;
    let left_half = Rect::from_xywh(wa.left, wa.top, hw, wa.h());
    let right_half = Rect::from_xywh(wa.right - hw, wa.top, hw, wa.h());
    let bottom_half = Rect::from_xywh(wa.left, wa.bottom - hh, wa.w(), hh);
    let tl = Rect::from_xywh(wa.left, wa.top, hw, hh);
    let tr = Rect::from_xywh(wa.right - hw, wa.top, hw, hh);
    let bl = Rect::from_xywh(wa.left, wa.bottom - hh, hw, hh);
    let br = Rect::from_xywh(wa.right - hw, wa.bottom - hh, hw, hh);

    if left && top {
        return Some(tl);
    }
    if right && top {
        return Some(tr);
    }
    if left && bottom {
        return Some(bl);
    }
    if right && bottom {
        return Some(br);
    }
    if top {
        if pt.x <= wa.left + corner_w {
            return Some(tl);
        }
        if pt.x >= wa.right - corner_w {
            return Some(tr);
        }
        return Some(wa);
    }
    if bottom {
        if pt.x <= wa.left + corner_w {
            return Some(bl);
        }
        if pt.x >= wa.right - corner_w {
            return Some(br);
        }
        return Some(bottom_half);
    }
    if left {
        if pt.y <= wa.top + corner_h {
            return Some(tl);
        }
        if pt.y >= wa.bottom - corner_h {
            return Some(bl);
        }
        return Some(left_half);
    }
    if right {
        if pt.y <= wa.top + corner_h {
            return Some(tr);
        }
        if pt.y >= wa.bottom - corner_h {
            return Some(br);
        }
        return Some(right_half);
    }
    None
}

// ---- Window manager (cycling + restore state) ------------------------------

const HALF_WIDTHS: [f64; 3] = [0.5, 2.0 / 3.0, 1.0 / 3.0];

pub struct WindowManager {
    restore: HashMap<isize, Rect>,
    last_applied: HashMap<isize, Rect>,
    cycle_hwnd: isize,
    cycle_action: Option<Action>,
    cycle_index: usize,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            restore: HashMap::new(),
            last_applied: HashMap::new(),
            cycle_hwnd: 0,
            cycle_action: None,
            cycle_index: 0,
        }
    }

    pub fn execute(&mut self, action: Action, settings: &Settings) {
        let hwnd = foreground();
        self.execute_on(action, hwnd, settings);
    }

    pub fn execute_on(&mut self, action: Action, hwnd: HWND, settings: &Settings) {
        if !is_manageable(hwnd) {
            return;
        }
        let key = hwnd.0 as isize;
        let mon = monitor_from_window(hwnd);
        let wa = mon.work;

        if action == Action::Restore {
            if let Some(rrect) = self.restore.get(&key).copied() {
                apply_visible_rect(hwnd, rrect);
                self.last_applied.insert(key, rrect);
            }
            self.reset_cycle();
            return;
        }

        self.capture_restore(hwnd, key);

        let target: Rect = match action {
            Action::LeftHalf => {
                let f = HALF_WIDTHS[self.advance_cycle(action, key, HALF_WIDTHS.len())];
                let w = r(wa.w() as f64 * f);
                Rect::from_xywh(wa.left, wa.top, w, wa.h())
            }
            Action::RightHalf => {
                let f = HALF_WIDTHS[self.advance_cycle(action, key, HALF_WIDTHS.len())];
                let w = r(wa.w() as f64 * f);
                Rect::from_xywh(wa.right - w, wa.top, w, wa.h())
            }
            Action::TopHalf => {
                self.reset_cycle();
                Rect::from_xywh(wa.left, wa.top, wa.w(), r(wa.h() as f64 * 0.5))
            }
            Action::BottomHalf => {
                self.reset_cycle();
                let h = r(wa.h() as f64 * 0.5);
                Rect::from_xywh(wa.left, wa.bottom - h, wa.w(), h)
            }
            Action::TopLeftQuarter => {
                self.reset_cycle();
                Rect::from_xywh(wa.left, wa.top, r(wa.w() as f64 * 0.5), r(wa.h() as f64 * 0.5))
            }
            Action::TopRightQuarter => {
                self.reset_cycle();
                let w = r(wa.w() as f64 * 0.5);
                Rect::from_xywh(wa.right - w, wa.top, w, r(wa.h() as f64 * 0.5))
            }
            Action::BottomLeftQuarter => {
                self.reset_cycle();
                let h = r(wa.h() as f64 * 0.5);
                Rect::from_xywh(wa.left, wa.bottom - h, r(wa.w() as f64 * 0.5), h)
            }
            Action::BottomRightQuarter => {
                self.reset_cycle();
                let w = r(wa.w() as f64 * 0.5);
                let h = r(wa.h() as f64 * 0.5);
                Rect::from_xywh(wa.right - w, wa.bottom - h, w, h)
            }
            Action::FirstThird => {
                let pos = self.advance_cycle(action, key, 3);
                third_at(wa, pos)
            }
            Action::LastThird => {
                let pos = 2 - self.advance_cycle(action, key, 3);
                third_at(wa, pos)
            }
            Action::CenterThird => {
                self.reset_cycle();
                third_at(wa, 1)
            }
            Action::FirstTwoThirds => {
                self.reset_cycle();
                let (x0, _, x2, _) = thirds(wa);
                Rect::new(x0, wa.top, x2, wa.bottom)
            }
            Action::LastTwoThirds => {
                self.reset_cycle();
                let (_, x1, _, x3) = thirds(wa);
                Rect::new(x1, wa.top, x3, wa.bottom)
            }
            Action::Maximize => {
                self.reset_cycle();
                wa
            }
            Action::AlmostMaximize => {
                self.reset_cycle();
                let w = r(wa.w() as f64 * 0.9);
                let h = r(wa.h() as f64 * 0.9);
                Rect::from_xywh(wa.left + (wa.w() - w) / 2, wa.top + (wa.h() - h) / 2, w, h)
            }
            Action::MaximizeHeight => {
                self.reset_cycle();
                let cur = visible_rect(hwnd);
                let x = cur.left.clamp(wa.left, (wa.right - cur.w()).max(wa.left));
                Rect::from_xywh(x, wa.top, cur.w().min(wa.w()), wa.h())
            }
            Action::Center => {
                self.reset_cycle();
                let cur = visible_rect(hwnd);
                let w = cur.w().min(wa.w());
                let h = cur.h().min(wa.h());
                Rect::from_xywh(wa.left + (wa.w() - w) / 2, wa.top + (wa.h() - h) / 2, w, h)
            }
            Action::MakeLarger => {
                self.reset_cycle();
                grow(visible_rect(hwnd), settings.resize_step_px, wa)
            }
            Action::MakeSmaller => {
                self.reset_cycle();
                shrink(visible_rect(hwnd), settings.resize_step_px, wa)
            }
            Action::NextDisplay => {
                self.reset_cycle();
                self.move_to_display(hwnd, &mon, 1);
                return;
            }
            Action::PreviousDisplay => {
                self.reset_cycle();
                self.move_to_display(hwnd, &mon, -1);
                return;
            }
            Action::Restore => return,
        };

        let target = self.apply_gap(action, target, settings.gap_px);
        apply_visible_rect(hwnd, target);
        self.last_applied.insert(key, target);
    }

    fn move_to_display(&mut self, hwnd: HWND, from: &Monitor, step: i32) {
        let to = relative(from, step);
        if to.handle == from.handle {
            return;
        }
        let cur = visible_rect(hwnd);
        let src = from.work;
        let dst = to.work;
        let rx = (cur.left - src.left) as f64 / src.w().max(1) as f64;
        let ry = (cur.top - src.top) as f64 / src.h().max(1) as f64;
        let rw = (cur.w() as f64 / src.w().max(1) as f64).min(1.0);
        let rh = (cur.h() as f64 / src.h().max(1) as f64).min(1.0);
        let w = r(dst.w() as f64 * rw);
        let h = r(dst.h() as f64 * rh);
        let x = dst.left + r(dst.w() as f64 * rx);
        let y = dst.top + r(dst.h() as f64 * ry);
        let target = clamp_into(Rect::from_xywh(x, y, w, h), dst);
        apply_visible_rect(hwnd, target);
        self.last_applied.insert(hwnd.0 as isize, target);
    }

    fn capture_restore(&mut self, hwnd: HWND, key: isize) {
        let cur = visible_rect(hwnd);
        let free = match self.last_applied.get(&key) {
            Some(last) => !nearly_eq(cur, *last),
            None => true,
        };
        if !self.restore.contains_key(&key) || free {
            self.restore.insert(key, cur);
        }
    }

    fn apply_gap(&self, action: Action, target: Rect, gap: i32) -> Rect {
        if gap <= 0 || !is_tiling(action) {
            return target;
        }
        let g = Rect::new(target.left + gap, target.top + gap, target.right - gap, target.bottom - gap);
        if g.w() > 0 && g.h() > 0 {
            g
        } else {
            target
        }
    }

    fn advance_cycle(&mut self, action: Action, key: isize, len: usize) -> usize {
        if self.cycle_hwnd == key && self.cycle_action == Some(action) {
            self.cycle_index = (self.cycle_index + 1) % len;
        } else {
            self.cycle_index = 0;
        }
        self.cycle_hwnd = key;
        self.cycle_action = Some(action);
        self.cycle_index
    }

    fn reset_cycle(&mut self) {
        self.cycle_hwnd = 0;
        self.cycle_action = None;
        self.cycle_index = 0;
    }
}

fn is_tiling(a: Action) -> bool {
    !matches!(
        a,
        Action::Center
            | Action::MakeLarger
            | Action::MakeSmaller
            | Action::NextDisplay
            | Action::PreviousDisplay
            | Action::Restore
    )
}

fn thirds(wa: Rect) -> (i32, i32, i32, i32) {
    let x0 = wa.left;
    let x1 = wa.left + r(wa.w() as f64 / 3.0);
    let x2 = wa.left + r(wa.w() as f64 * 2.0 / 3.0);
    let x3 = wa.right;
    (x0, x1, x2, x3)
}

fn third_at(wa: Rect, pos: usize) -> Rect {
    let (x0, x1, x2, x3) = thirds(wa);
    match pos {
        0 => Rect::new(x0, wa.top, x1, wa.bottom),
        1 => Rect::new(x1, wa.top, x2, wa.bottom),
        _ => Rect::new(x2, wa.top, x3, wa.bottom),
    }
}

fn grow(cur: Rect, step: i32, wa: Rect) -> Rect {
    let w = (cur.w() + step).min(wa.w());
    let h = (cur.h() + step).min(wa.h());
    let x = cur.left - (w - cur.w()) / 2;
    let y = cur.top - (h - cur.h()) / 2;
    clamp_into(Rect::from_xywh(x, y, w, h), wa)
}

fn shrink(cur: Rect, step: i32, wa: Rect) -> Rect {
    let min_w = ((wa.w() as f64 * 0.25) as i32).max(200);
    let min_h = ((wa.h() as f64 * 0.25) as i32).max(150);
    let w = (cur.w() - step).max(min_w);
    let h = (cur.h() - step).max(min_h);
    let x = cur.left + (cur.w() - w) / 2;
    let y = cur.top + (cur.h() - h) / 2;
    clamp_into(Rect::from_xywh(x, y, w, h), wa)
}

fn clamp_into(rect: Rect, wa: Rect) -> Rect {
    let w = rect.w().min(wa.w());
    let h = rect.h().min(wa.h());
    let x = rect.left.clamp(wa.left, (wa.right - w).max(wa.left));
    let y = rect.top.clamp(wa.top, (wa.bottom - h).max(wa.top));
    Rect::from_xywh(x, y, w, h)
}

fn nearly_eq(a: Rect, b: Rect) -> bool {
    let t = 4;
    (a.left - b.left).abs() <= t
        && (a.top - b.top).abs() <= t
        && (a.w() - b.w()).abs() <= t
        && (a.h() - b.h()).abs() <= t
}

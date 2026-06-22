//! macOS implementation of the window I/O behind `winmgr`, via the Accessibility API (move /
//! inspect the focused window), CoreGraphics (display geometry, cursor), and AppKit (work area,
//! frontmost app).
//!
//! Coordinate space: there is exactly ONE — global, top-left origin, y-down, primary display at
//! (0,0). This is the `CGDisplayBounds` space and the Accessibility `AXPosition` space, so window
//! rects pass between them with NO flip. `NSScreen.frame`/`visibleFrame` (bottom-left, y-up) are
//! used *only* to derive scalar work-area insets (menu bar / Dock), which are flip-invariant
//! differences, and never escape this module as absolutes.
//!
//! A `WinId` here packs `(pid << 32) | cg_window_id`. The AX element is non-`Copy` and unstable, so
//! it is never stored; instead every call re-derives the focused window from the packed pid — which
//! matches how the Windows backend always operates on the foreground window.
//!
//! KNOWN v1 LIMITATION: `foreground()` packs `cg_window_id = 0`, so a `WinId` is effectively
//! per-*app*, not per-window. The `WindowManager`'s restore/last-applied maps therefore key on the
//! app, so for an app with several windows the Restore action can carry one window's saved geometry
//! to another. Single-window-per-app (the common case) is correct. The fix is to mint the focused
//! window's real `CGWindowID` into `cg_window_id` (the pack/unpack plumbing is already in place).

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_app_kit::{NSScreen, NSWorkspace};
use objc2_application_services::{
    AXError, AXIsProcessTrusted, AXUIElement, AXValue, AXValueType,
};
use objc2_core_foundation::{CFBoolean, CFRetained, CFString, CFType, Type};
use objc2_core_graphics::{
    CGDisplayBounds, CGError, CGEvent, CGGetActiveDisplayList, CGMainDisplayID,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{MainThreadMarker, NSNumber, NSString};

use super::{Monitor, Point, Rect, WinId};

// ---- WinId packing ----------------------------------------------------------

#[inline]
fn pack(pid: i32, cg_win: u32) -> WinId {
    (((pid as i64) << 32) | (cg_win as i64 & 0xFFFF_FFFF)) as isize
}

#[inline]
fn unpack(id: WinId) -> (i32, u32) {
    let v = id as i64;
    ((v >> 32) as i32, v as u32)
}

// ---- Accessibility helpers --------------------------------------------------

const AX_FOCUSED_WINDOW: &str = "AXFocusedWindow";
const AX_POSITION: &str = "AXPosition";
const AX_SIZE: &str = "AXSize";
const AX_MAIN: &str = "AXMain";
const AX_RAISE_ACTION: &str = "AXRaise";

#[inline]
fn cfstr(s: &'static str) -> CFRetained<CFString> {
    CFString::from_static_str(s)
}

#[inline]
fn as_cftype<T: AsRef<CFType>>(v: &T) -> &CFType {
    v.as_ref()
}

/// AX application element for a pid. Bounds the per-message wait so a hung target app can't
/// freeze the hotkey thread.
fn ax_app(pid: i32) -> CFRetained<AXUIElement> {
    let el = unsafe { AXUIElement::new_application(pid as libc::pid_t) };
    unsafe { el.set_messaging_timeout(0.25) };
    el
}

/// The focused window element of the app owning `pid`, or `None` if AX is denied / no window.
fn ax_focused_window(pid: i32) -> Option<CFRetained<AXUIElement>> {
    if pid <= 0 {
        return None;
    }
    let app = ax_app(pid);
    copy_attr_as::<AXUIElement>(&app, AX_FOCUSED_WINDOW)
}

/// Copy an object-valued AX attribute and reinterpret it as `T` (a CF type). Owns the +1 ref.
fn copy_attr_as<T: Type>(el: &AXUIElement, attr: &'static str) -> Option<CFRetained<T>> {
    let key = cfstr(attr);
    let mut out: *const CFType = std::ptr::null();
    let err = unsafe { el.copy_attribute_value(&key, NonNull::from(&mut out)) };
    if err == AXError::Success && !out.is_null() {
        let cf = unsafe { CFRetained::from_raw(NonNull::new(out as *mut CFType)?) };
        Some(unsafe { CFRetained::cast_unchecked::<T>(cf) })
    } else {
        None
    }
}

fn read_ax_point(win: &AXUIElement, attr: &'static str) -> Option<CGPoint> {
    let val = copy_attr_as::<AXValue>(win, attr)?;
    let mut p = CGPoint { x: 0.0, y: 0.0 };
    let ok = unsafe {
        val.value(
            AXValueType::CGPoint,
            NonNull::new(&mut p as *mut _ as *mut c_void)?,
        )
    };
    ok.then_some(p)
}

fn read_ax_size(win: &AXUIElement, attr: &'static str) -> Option<CGSize> {
    let val = copy_attr_as::<AXValue>(win, attr)?;
    let mut s = CGSize { width: 0.0, height: 0.0 };
    let ok = unsafe {
        val.value(
            AXValueType::CGSize,
            NonNull::new(&mut s as *mut _ as *mut c_void)?,
        )
    };
    ok.then_some(s)
}

fn set_ax_value(win: &AXUIElement, attr: &'static str, ty: AXValueType, ptr: *mut c_void) {
    let Some(v) = (unsafe { AXValue::new(ty, NonNull::new(ptr).unwrap()) }) else {
        return;
    };
    let key = cfstr(attr);
    let _ = unsafe { win.set_attribute_value(&key, as_cftype(&v)) };
}

fn set_ax_bool(win: &AXUIElement, attr: &'static str, b: bool) {
    let key = cfstr(attr);
    let v = CFBoolean::new(b);
    let _ = unsafe { win.set_attribute_value(&key, as_cftype(&v)) };
}

// ---- Monitors ---------------------------------------------------------------

#[inline]
fn cgrect_to_rect(r: CGRect) -> Rect {
    let l = r.origin.x.round() as i32;
    let t = r.origin.y.round() as i32;
    Rect::new(
        l,
        t,
        l + r.size.width.round() as i32,
        t + r.size.height.round() as i32,
    )
}

/// Per-display work-area insets `(left, top, right, bottom)` in points, derived from NSScreen.
/// These are scalar differences between `frame` and `visibleFrame`, so they are identical in
/// either Y orientation and can be applied to the top-left `CGDisplayBounds` with no flip.
fn screen_insets() -> std::collections::HashMap<u32, (f64, f64, f64, f64)> {
    let mut map = std::collections::HashMap::new();
    let Some(mtm) = MainThreadMarker::new() else {
        return map; // not on the main thread — callers fall back to full bounds
    };
    let key = NSString::from_str("NSScreenNumber");
    for screen in NSScreen::screens(mtm).iter() {
        let desc = screen.deviceDescription();
        let Some(obj) = desc.objectForKey(&key) else {
            continue;
        };
        let Ok(num) = obj.downcast::<NSNumber>() else {
            continue;
        };
        let display_id: u32 = num.unsignedIntValue();
        let f: CGRect = screen.frame();
        let vf: CGRect = screen.visibleFrame();
        let top = (f.origin.y + f.size.height) - (vf.origin.y + vf.size.height);
        let bottom = vf.origin.y - f.origin.y;
        let left = vf.origin.x - f.origin.x;
        let right = (f.origin.x + f.size.width) - (vf.origin.x + vf.size.width);
        map.insert(display_id, (left, top, right, bottom));
    }
    map
}

pub fn all_monitors() -> Vec<Monitor> {
    let insets = screen_insets();
    let main = CGMainDisplayID();

    let mut ids = [0u32; 16];
    let mut count: u32 = 0;
    let err = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if err != CGError::Success {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(count as usize);
    for &id in &ids[..count as usize] {
        let bounds = cgrect_to_rect(CGDisplayBounds(id));
        let (l, t, r, b) = insets.get(&id).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));
        let work = Rect::new(
            bounds.left + l.round() as i32,
            bounds.top + t.round() as i32,
            bounds.right - r.round() as i32,
            bounds.bottom - b.round() as i32,
        );
        out.push(Monitor {
            handle: id as isize,
            bounds,
            work,
            primary: id == main,
        });
    }
    out.sort_by(|a, b| a.bounds.left.cmp(&b.bounds.left).then(a.bounds.top.cmp(&b.bounds.top)));
    out
}

fn nearest_by_center(all: &[Monitor], pt: Point) -> Option<Monitor> {
    all.iter()
        .min_by_key(|m| {
            let cx = (m.bounds.left + m.bounds.right) / 2;
            let cy = (m.bounds.top + m.bounds.bottom) / 2;
            let dx = (cx - pt.x) as i64;
            let dy = (cy - pt.y) as i64;
            dx * dx + dy * dy
        })
        .copied()
}

pub fn monitor_from_point(pt: Point) -> Monitor {
    let all = all_monitors();
    all.iter()
        .find(|m| {
            pt.x >= m.bounds.left
                && pt.x < m.bounds.right
                && pt.y >= m.bounds.top
                && pt.y < m.bounds.bottom
        })
        .copied()
        .or_else(|| nearest_by_center(&all, pt))
        .unwrap_or_else(super::primary_monitor)
}

pub fn monitor_from_window(id: WinId) -> Monitor {
    let r = visible_rect(id);
    monitor_from_point(Point {
        x: (r.left + r.right) / 2,
        y: (r.top + r.bottom) / 2,
    })
}

// ---- Window inspection / movement -------------------------------------------

/// A window is manageable if the frontmost app exposes a focused window we can address. (macOS
/// AX reports the active window as `AXFocusedWindow`; minimized/background windows are not it.)
pub fn is_manageable(id: WinId) -> bool {
    let (pid, _) = unpack(id);
    ax_focused_window(pid).is_some()
}

pub fn visible_rect(id: WinId) -> Rect {
    let (pid, _) = unpack(id);
    let Some(win) = ax_focused_window(pid) else {
        return Rect::new(0, 0, 0, 0);
    };
    let pos = read_ax_point(&win, AX_POSITION).unwrap_or(CGPoint { x: 0.0, y: 0.0 });
    let sz = read_ax_size(&win, AX_SIZE).unwrap_or(CGSize { width: 0.0, height: 0.0 });
    // AX position/size are already global top-left, y-down — our space. No flip.
    let l = pos.x.round() as i32;
    let t = pos.y.round() as i32;
    Rect::new(l, t, l + sz.width.round() as i32, t + sz.height.round() as i32)
}

/// Set the focused window's frame to `target` (global top-left). macOS AX frames are the visible
/// frame already (no DWM-style inset compensation). Order size → position → size so apps that clamp
/// size to the old display settle on the target.
pub fn apply_visible_rect(id: WinId, target: Rect) {
    let (pid, _) = unpack(id);
    let Some(win) = ax_focused_window(pid) else {
        return;
    };
    let mut sz = CGSize {
        width: target.w() as f64,
        height: target.h() as f64,
    };
    let mut pos = CGPoint {
        x: target.left as f64,
        y: target.top as f64,
    };
    set_ax_value(&win, AX_SIZE, AXValueType::CGSize, &mut sz as *mut _ as *mut c_void);
    set_ax_value(&win, AX_POSITION, AXValueType::CGPoint, &mut pos as *mut _ as *mut c_void);
    set_ax_value(&win, AX_SIZE, AXValueType::CGSize, &mut sz as *mut _ as *mut c_void);
}

pub fn set_foreground(id: WinId) {
    // AXRaise + AXMain order the specific window — the reliable path. (App-level activation via
    // NSRunningApplication.activateWithOptions is deprecated/no-op on modern macOS, so it's omitted.)
    let (pid, _) = unpack(id);
    if let Some(win) = ax_focused_window(pid) {
        set_ax_bool(&win, AX_MAIN, true); // may fail on non-standard windows — ignored
        let key = cfstr(AX_RAISE_ACTION);
        let _ = unsafe { win.perform_action(&key) };
    }
}

pub fn foreground() -> WinId {
    let ws = NSWorkspace::sharedWorkspace();
    let Some(app) = ws.frontmostApplication() else {
        return 0;
    };
    let pid = app.processIdentifier();
    if pid <= 0 {
        return 0;
    }
    // cg_win is identity-only for our usage; the element is re-derived from the pid each call.
    pack(pid, 0)
}

/// The frontmost window, but only if it belongs to another process. Used by the macOS worker to
/// keep `last_active` pointed at the user's real target window — so that when Quad itself becomes
/// frontmost (the user clicking an in-app action), Quad's own window is never picked as the target.
/// This mirrors how the Windows backend's foreground hook skips Quad's own (borderless) window.
pub fn foreground_other_app() -> WinId {
    let id = foreground();
    if id == 0 {
        return 0;
    }
    let (pid, _) = unpack(id);
    if pid == std::process::id() as i32 {
        0
    } else {
        id
    }
}

pub fn cursor_pos() -> Point {
    // CGEvent::location() is the top-left Quartz global point — our space, no flip. (Only used by
    // the Windows-only drag-snap worker; provided here so the shared interface compiles.)
    let Some(ev) = CGEvent::new(None) else {
        return Point::default();
    };
    let p = CGEvent::location(Some(&ev));
    Point {
        x: p.x.round() as i32,
        y: p.y.round() as i32,
    }
}

/// Mission Control. There is no public AX/Cocoa API; launching the Mission Control app toggles it
/// reliably (mirrors the Windows backend, which synthesises Win+Tab for Task View).
pub fn show_task_view() {
    let _ = std::process::Command::new("open")
        .args(["-a", "Mission Control"])
        .spawn();
}

// ---- Accessibility trust -----------------------------------------------------

/// Whether this process is trusted for the Accessibility API (required to move other apps'
/// windows). Wired into startup in `app.rs`.
pub fn ax_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// On first run without trust, point the user at the Accessibility settings pane. (Granting
/// requires a relaunch before AX calls start succeeding on the running process.)
pub fn ensure_accessibility() {
    if ax_trusted() {
        return;
    }
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

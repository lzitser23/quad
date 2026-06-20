use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::Ordering;

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer, RegisterClassExW,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, TranslateMessage,
    EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, HWND_TOPMOST,
    LWA_ALPHA, MSG, OBJID_WINDOW, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::app::{shared, Shared};
use crate::winmgr;

const TIMER_ID: usize = 0xC0DE;

unsafe extern "system" fn overlay_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

pub fn spawn_worker() {
    std::thread::spawn(|| unsafe {
        worker_main();
    });
}

unsafe fn worker_main() {
    let hmod = GetModuleHandleW(None).unwrap_or_default();
    let hinst: HINSTANCE = hmod.into();

    // Overlay window: a translucent teal rectangle for the drag-snap preview.
    let class_name = w!("WinRectOverlay");
    let brush = CreateSolidBrush(COLORREF(0x00A6_B814)); // teal #14b8a6 as 0x00BBGGRR
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(overlay_wndproc),
        hInstance: hinst,
        lpszClassName: class_name,
        hbrBackground: brush,
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let overlay = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
        class_name,
        w!(""),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        hinst,
        None,
    )
    .unwrap_or_default();
    let _ = SetLayeredWindowAttributes(overlay, COLORREF(0), 150, LWA_ALPHA);
    shared().overlay.store(overlay.0 as isize, Ordering::Relaxed);

    // Foreground tracking (for click-to-apply) + window move/size (for drag-snap).
    let _ = SetWinEventHook(
        EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_FOREGROUND,
        HMODULE::default(),
        Some(winevent_proc),
        0,
        0,
        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
    );
    let _ = SetWinEventHook(
        EVENT_SYSTEM_MOVESIZESTART,
        EVENT_SYSTEM_MOVESIZEEND,
        HMODULE::default(),
        Some(winevent_proc),
        0,
        0,
        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
    );

    let mut msg = MSG::default();
    loop {
        let r = GetMessageW(&mut msg, None, 0, 0);
        if r.0 <= 0 {
            break;
        }
        if msg.message == WM_TIMER {
            update_preview(shared());
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn winevent_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    idobject: i32,
    _idchild: i32,
    _thread: u32,
    _time: u32,
) {
    if idobject != OBJID_WINDOW.0 || hwnd.0.is_null() {
        return;
    }
    let s = shared();
    match event {
        EVENT_SYSTEM_FOREGROUND => {
            if winmgr::is_manageable(hwnd) {
                s.last_active.store(hwnd.0 as isize, Ordering::Relaxed);
            }
        }
        EVENT_SYSTEM_MOVESIZESTART => {
            if winmgr::is_manageable(hwnd) {
                s.drag_hwnd.store(hwnd.0 as isize, Ordering::Relaxed);
                *s.current_zone.lock().unwrap() = None;
                SetTimer(None, TIMER_ID, 16, None);
            }
        }
        EVENT_SYSTEM_MOVESIZEEND => {
            let dh = s.drag_hwnd.swap(0, Ordering::Relaxed);
            let _ = KillTimer(None, TIMER_ID);
            hide_overlay(s);
            let zone = s.current_zone.lock().unwrap().take();
            let enabled = s.settings.lock().unwrap().drag_snap_enabled;
            if enabled && dh != 0 {
                if let Some(z) = zone {
                    winmgr::apply_visible_rect(HWND(dh as *mut c_void), z);
                }
            }
        }
        _ => {}
    }
}

unsafe fn update_preview(s: &Shared) {
    if s.drag_hwnd.load(Ordering::Relaxed) == 0 {
        return;
    }
    let pt = winmgr::cursor_pos();
    let settings = s.settings.lock().unwrap().clone();
    let zone = winmgr::compute_zone(pt, &settings);

    {
        let mut cz = s.current_zone.lock().unwrap();
        if *cz == zone {
            return;
        }
        *cz = zone;
    }

    match (zone, settings.show_snap_preview) {
        (Some(z), true) => show_overlay(s, z),
        _ => hide_overlay(s),
    }
}

unsafe fn show_overlay(s: &Shared, z: winmgr::Rect) {
    let overlay = HWND(s.overlay.load(Ordering::Relaxed) as *mut c_void);
    if overlay.0.is_null() {
        return;
    }
    let _ = SetWindowPos(
        overlay,
        HWND_TOPMOST,
        z.left,
        z.top,
        z.w(),
        z.h(),
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
}

unsafe fn hide_overlay(s: &Shared) {
    let overlay = HWND(s.overlay.load(Ordering::Relaxed) as *mut c_void);
    if !overlay.0.is_null() {
        let _ = ShowWindow(overlay, SW_HIDE);
    }
}

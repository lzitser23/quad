//! Imperative shell over the pure `layout` module. The public surface is platform-neutral:
//! windows are addressed by an opaque `WinId`, geometry by `layout::Rect`, screen points by
//! `Point` (global, top-left origin). The OS-specific window I/O lives in the `windows` /
//! `macos` submodules behind `cfg` and is reached here through the `sys` alias.

use std::collections::HashMap;

use crate::actions::Action;
use crate::layout;
use crate::settings::Settings;

pub use crate::layout::Rect;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as sys;

// Platform window I/O reached by other modules (ipc, native).
pub use sys::{apply_visible_rect, cursor_pos, is_manageable, set_foreground, show_task_view};

// macOS extras: the Accessibility-permission nudge (called from `app.rs` on startup) and the
// foreground tracker the macOS worker polls to keep `last_active` on the user's real window.
#[cfg(target_os = "macos")]
pub use macos::{ensure_accessibility, foreground_other_app};

/// Opaque window handle. On Windows this is an `HWND` cast to `isize`; on macOS it encodes the
/// CoreGraphics window id. Flows through the shared state as a plain integer (`AtomicIsize`).
pub type WinId = isize;

/// A screen point in global, top-left-origin coordinates.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A display: full `bounds` and `work` area (minus taskbar / menu bar / Dock), both in global
/// top-left coordinates. `handle` is an opaque per-platform id used only for identity/ordering.
#[derive(Clone, Copy)]
pub struct Monitor {
    pub handle: isize,
    pub bounds: Rect,
    pub work: Rect,
    pub primary: bool,
}

fn primary_monitor() -> Monitor {
    let all = sys::all_monitors();
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

fn relative(from: &Monitor, step: i32) -> Monitor {
    let all = sys::all_monitors();
    if all.len() <= 1 {
        return *from;
    }
    let idx = all.iter().position(|m| m.handle == from.handle).unwrap_or(0) as i32;
    let n = all.len() as i32;
    let next = (((idx + step) % n) + n) % n;
    all[next as usize]
}

/// Resolve the monitor under `pt` and ask `layout` for the snap target.
pub fn compute_zone(pt: Point, settings: &Settings) -> Option<Rect> {
    let mon = sys::monitor_from_point(pt);
    layout::zone(pt.x, pt.y, mon.bounds, mon.work, settings.snap_edge_threshold_px)
}

// ---- Window manager (cycle + restore state; the imperative shell) -----------

pub struct WindowManager {
    restore: HashMap<WinId, Rect>,
    last_applied: HashMap<WinId, Rect>,
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
        let id = sys::foreground();
        self.execute_on(action, id, settings);
    }

    pub fn execute_on(&mut self, action: Action, id: WinId, settings: &Settings) {
        // Global action — no window needed.
        if action == Action::MissionControl {
            sys::show_task_view();
            return;
        }
        if !sys::is_manageable(id) {
            return;
        }
        let key = id;
        let mon = sys::monitor_from_window(id);

        if action == Action::Restore {
            if let Some(rrect) = self.restore.get(&key).copied() {
                sys::apply_visible_rect(id, rrect);
                self.last_applied.insert(key, rrect);
            }
            self.cycle = layout::reset();
            return;
        }

        self.capture_restore(id, key);

        if matches!(action, Action::NextDisplay | Action::PreviousDisplay) {
            self.cycle = layout::reset();
            let step = if action == Action::NextDisplay { 1 } else { -1 };
            let to = relative(&mon, step);
            if to.handle != mon.handle {
                let target = layout::map_proportional(sys::visible_rect(id), mon.work, to.work);
                sys::apply_visible_rect(id, target);
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

        let current = sys::visible_rect(id);
        if let Some(target) =
            layout::target_rect(action, mon.work, current, idx, settings.gap_px, settings.resize_step_px)
        {
            sys::apply_visible_rect(id, target);
            self.last_applied.insert(key, target);
        }
    }

    /// Remember pre-snap geometry the first time, or whenever the user has moved the window since.
    fn capture_restore(&mut self, id: WinId, key: WinId) {
        let cur = sys::visible_rect(id);
        let free = match self.last_applied.get(&key) {
            Some(last) => !layout::nearly_eq(cur, *last),
            None => true,
        };
        if !self.restore.contains_key(&key) || free {
            self.restore.insert(key, cur);
        }
    }
}

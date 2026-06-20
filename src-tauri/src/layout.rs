//! Pure tiling geometry, size-cycling, and drag-snap zone math.
//!
//! No Win32, no I/O, no mutable singletons — every function is a value transform, so the
//! interface is the test surface. The imperative shell (`winmgr`) resolves monitors and HWNDs,
//! calls in here, and applies the result.

use crate::actions::Action;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
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
    pub fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect::new(x, y, x + w, y + h)
    }
    pub fn w(&self) -> i32 {
        self.right - self.left
    }
    pub fn h(&self) -> i32 {
        self.bottom - self.top
    }
}

const HALF_WIDTHS: [f64; 3] = [0.5, 2.0 / 3.0, 1.0 / 3.0];

fn r(v: f64) -> i32 {
    v.round() as i32
}

// ---- Size cycling (pure state transition) -----------------------------------

/// Where a window sits in its size cycle: which window, which action, which step.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Cycle {
    key: isize,
    action: Option<Action>,
    index: usize,
}

impl Cycle {
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Actions whose repeated invocation cycles through sizes/positions.
pub fn cycles(action: Action) -> bool {
    matches!(
        action,
        Action::LeftHalf | Action::RightHalf | Action::FirstThird | Action::LastThird
    )
}

/// Advance the cycle when the same action repeats on the same window; otherwise restart at 0.
pub fn advance(prev: Cycle, action: Action, key: isize, len: usize) -> Cycle {
    let index = if prev.key == key && prev.action == Some(action) {
        (prev.index + 1) % len.max(1)
    } else {
        0
    };
    Cycle { key, action: Some(action), index }
}

pub fn reset() -> Cycle {
    Cycle::default()
}

// ---- Target rectangle -------------------------------------------------------

/// Target rect for an action within one work area `wa`, given the window's `current` visible rect
/// and cycle step `idx`. Returns `None` for actions the shell owns: `Restore` (stateful),
/// `Next/PreviousDisplay` (cross-monitor), and `MissionControl` (no rect at all).
pub fn target_rect(
    action: Action,
    wa: Rect,
    current: Rect,
    idx: usize,
    gap: i32,
    resize_step: i32,
) -> Option<Rect> {
    let base = match action {
        Action::LeftHalf => {
            let w = r(wa.w() as f64 * HALF_WIDTHS[idx % 3]);
            Rect::from_xywh(wa.left, wa.top, w, wa.h())
        }
        Action::RightHalf => {
            let w = r(wa.w() as f64 * HALF_WIDTHS[idx % 3]);
            Rect::from_xywh(wa.right - w, wa.top, w, wa.h())
        }
        Action::TopHalf => Rect::from_xywh(wa.left, wa.top, wa.w(), r(wa.h() as f64 * 0.5)),
        Action::BottomHalf => {
            let h = r(wa.h() as f64 * 0.5);
            Rect::from_xywh(wa.left, wa.bottom - h, wa.w(), h)
        }
        Action::TopLeftQuarter => {
            Rect::from_xywh(wa.left, wa.top, r(wa.w() as f64 * 0.5), r(wa.h() as f64 * 0.5))
        }
        Action::TopRightQuarter => {
            let w = r(wa.w() as f64 * 0.5);
            Rect::from_xywh(wa.right - w, wa.top, w, r(wa.h() as f64 * 0.5))
        }
        Action::BottomLeftQuarter => {
            let h = r(wa.h() as f64 * 0.5);
            Rect::from_xywh(wa.left, wa.bottom - h, r(wa.w() as f64 * 0.5), h)
        }
        Action::BottomRightQuarter => {
            let w = r(wa.w() as f64 * 0.5);
            let h = r(wa.h() as f64 * 0.5);
            Rect::from_xywh(wa.right - w, wa.bottom - h, w, h)
        }
        Action::FirstThird => third_at(wa, idx % 3),
        Action::LastThird => third_at(wa, 2 - (idx % 3)),
        Action::CenterThird => third_at(wa, 1),
        Action::FirstTwoThirds => {
            let (x0, _, x2, _) = thirds(wa);
            Rect::new(x0, wa.top, x2, wa.bottom)
        }
        Action::LastTwoThirds => {
            let (_, x1, _, x3) = thirds(wa);
            Rect::new(x1, wa.top, x3, wa.bottom)
        }
        Action::Maximize => wa,
        Action::AlmostMaximize => {
            let w = r(wa.w() as f64 * 0.9);
            let h = r(wa.h() as f64 * 0.9);
            Rect::from_xywh(wa.left + (wa.w() - w) / 2, wa.top + (wa.h() - h) / 2, w, h)
        }
        Action::MaximizeHeight => {
            let x = current.left.clamp(wa.left, (wa.right - current.w()).max(wa.left));
            Rect::from_xywh(x, wa.top, current.w().min(wa.w()), wa.h())
        }
        Action::Center => {
            let w = current.w().min(wa.w());
            let h = current.h().min(wa.h());
            Rect::from_xywh(wa.left + (wa.w() - w) / 2, wa.top + (wa.h() - h) / 2, w, h)
        }
        Action::MakeLarger => grow(current, resize_step, wa),
        Action::MakeSmaller => shrink(current, resize_step, wa),
        Action::Restore
        | Action::NextDisplay
        | Action::PreviousDisplay
        | Action::MissionControl => return None,
    };
    Some(apply_gap(action, base, gap))
}

/// Map a window's rect proportionally from one work area to another (for cross-monitor moves).
pub fn map_proportional(cur: Rect, src: Rect, dst: Rect) -> Rect {
    let rx = (cur.left - src.left) as f64 / src.w().max(1) as f64;
    let ry = (cur.top - src.top) as f64 / src.h().max(1) as f64;
    let rw = (cur.w() as f64 / src.w().max(1) as f64).min(1.0);
    let rh = (cur.h() as f64 / src.h().max(1) as f64).min(1.0);
    let w = r(dst.w() as f64 * rw);
    let h = r(dst.h() as f64 * rh);
    let x = dst.left + r(dst.w() as f64 * rx);
    let y = dst.top + r(dst.h() as f64 * ry);
    clamp_into(Rect::from_xywh(x, y, w, h), dst)
}

/// Two rects are "the same" within a few px (used to detect manual moves for Restore).
pub fn nearly_eq(a: Rect, b: Rect) -> bool {
    let t = 4;
    (a.left - b.left).abs() <= t
        && (a.top - b.top).abs() <= t
        && (a.w() - b.w()).abs() <= t
        && (a.h() - b.h()).abs() <= t
}

// ---- Drag-snap zone ---------------------------------------------------------

/// Map a cursor position (physical px) near a screen edge/corner to a snap target in the work area.
/// `bounds` is the full monitor; `wa` the work area; the snap fills `wa`.
pub fn zone(px: i32, py: i32, bounds: Rect, wa: Rect, edge_px: i32) -> Option<Rect> {
    let edge = edge_px.max(2);
    let corner_w = ((wa.w() as f64 * 0.25) as i32).clamp(80, 400);
    let corner_h = ((wa.h() as f64 * 0.25) as i32).clamp(80, 400);

    let left = px <= bounds.left + edge;
    let right = px >= bounds.right - edge - 1;
    let top = py <= bounds.top + edge;
    let bottom = py >= bounds.bottom - edge - 1;

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
        if px <= wa.left + corner_w {
            return Some(tl);
        }
        if px >= wa.right - corner_w {
            return Some(tr);
        }
        return Some(wa);
    }
    if bottom {
        if px <= wa.left + corner_w {
            return Some(bl);
        }
        if px >= wa.right - corner_w {
            return Some(br);
        }
        return Some(bottom_half);
    }
    if left {
        if py <= wa.top + corner_h {
            return Some(tl);
        }
        if py >= wa.bottom - corner_h {
            return Some(bl);
        }
        return Some(left_half);
    }
    if right {
        if py <= wa.top + corner_h {
            return Some(tr);
        }
        if py >= wa.bottom - corner_h {
            return Some(br);
        }
        return Some(right_half);
    }
    None
}

// ---- Internal helpers -------------------------------------------------------

fn is_tiling(a: Action) -> bool {
    !matches!(
        a,
        Action::Center
            | Action::MakeLarger
            | Action::MakeSmaller
            | Action::NextDisplay
            | Action::PreviousDisplay
            | Action::Restore
            | Action::MissionControl
    )
}

fn apply_gap(action: Action, target: Rect, gap: i32) -> Rect {
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

// ---- Tests: the interface is the test surface -------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;

    // A 1920×1040 work area (1080p minus a 40px taskbar) at the origin.
    fn wa() -> Rect {
        Rect::new(0, 0, 1920, 1040)
    }
    fn tr(a: Action, idx: usize) -> Rect {
        target_rect(a, wa(), Rect::new(100, 100, 900, 700), idx, 0, 30).unwrap()
    }

    #[test]
    fn left_half_cycles_half_two_thirds_one_third() {
        assert_eq!(tr(Action::LeftHalf, 0), Rect::new(0, 0, 960, 1040));
        assert_eq!(tr(Action::LeftHalf, 1), Rect::new(0, 0, 1280, 1040));
        assert_eq!(tr(Action::LeftHalf, 2), Rect::new(0, 0, 640, 1040));
        assert_eq!(tr(Action::LeftHalf, 3), Rect::new(0, 0, 960, 1040)); // wraps
    }

    #[test]
    fn right_half_anchors_right() {
        assert_eq!(tr(Action::RightHalf, 0), Rect::new(960, 0, 1920, 1040));
        assert_eq!(tr(Action::RightHalf, 1), Rect::new(640, 0, 1920, 1040));
    }

    #[test]
    fn quarters() {
        assert_eq!(tr(Action::TopLeftQuarter, 0), Rect::new(0, 0, 960, 520));
        assert_eq!(tr(Action::BottomRightQuarter, 0), Rect::new(960, 520, 1920, 1040));
    }

    #[test]
    fn thirds_cycle_directionally() {
        // First Third walks first → center → last; Last Third walks last → center → first.
        assert_eq!(tr(Action::FirstThird, 0), Rect::new(0, 0, 640, 1040));
        assert_eq!(tr(Action::FirstThird, 1), Rect::new(640, 0, 1280, 1040));
        assert_eq!(tr(Action::LastThird, 0), Rect::new(1280, 0, 1920, 1040));
        assert_eq!(tr(Action::LastThird, 1), Rect::new(640, 0, 1280, 1040));
    }

    #[test]
    fn maximize_fills_work_area() {
        assert_eq!(tr(Action::Maximize, 0), wa());
    }

    #[test]
    fn center_keeps_size() {
        let cur = Rect::new(100, 100, 900, 600); // 800×500
        let got = target_rect(Action::Center, wa(), cur, 0, 0, 30).unwrap();
        assert_eq!(got.w(), 800);
        assert_eq!(got.h(), 500);
        assert_eq!(got, Rect::from_xywh((1920 - 800) / 2, (1040 - 500) / 2, 800, 500));
    }

    #[test]
    fn gap_insets_tiling_actions_only() {
        let half = target_rect(Action::LeftHalf, wa(), Rect::default(), 0, 8, 30).unwrap();
        assert_eq!(half, Rect::new(8, 8, 952, 1032));
        // Center is not a tiling action — gap must not apply.
        let cur = Rect::new(0, 0, 800, 500);
        let c0 = target_rect(Action::Center, wa(), cur, 0, 0, 30).unwrap();
        let c8 = target_rect(Action::Center, wa(), cur, 0, 8, 30).unwrap();
        assert_eq!(c0, c8);
    }

    #[test]
    fn shell_owned_actions_return_none() {
        for a in [Action::Restore, Action::NextDisplay, Action::PreviousDisplay, Action::MissionControl] {
            assert_eq!(target_rect(a, wa(), Rect::default(), 0, 0, 30), None);
        }
    }

    #[test]
    fn cycle_advances_on_repeat_and_resets_on_change() {
        let c0 = advance(reset(), Action::LeftHalf, 1, 3);
        assert_eq!(c0.index(), 0);
        let c1 = advance(c0, Action::LeftHalf, 1, 3);
        assert_eq!(c1.index(), 1);
        let c2 = advance(c1, Action::LeftHalf, 1, 3);
        assert_eq!(c2.index(), 2);
        assert_eq!(advance(c2, Action::LeftHalf, 1, 3).index(), 0); // wraps
        // Different window resets.
        assert_eq!(advance(c1, Action::LeftHalf, 2, 3).index(), 0);
        // Different action resets.
        assert_eq!(advance(c1, Action::RightHalf, 1, 3).index(), 0);
    }

    #[test]
    fn zone_maps_edges_and_corners() {
        let b = Rect::new(0, 0, 1920, 1080);
        let work = wa();
        // top-left corner → top-left quarter
        assert_eq!(zone(1, 1, b, work, 20), Some(Rect::new(0, 0, 960, 520)));
        // top center → maximize
        assert_eq!(zone(960, 1, b, work, 20), Some(work));
        // left edge middle → left half
        assert_eq!(zone(1, 540, b, work, 20), Some(Rect::new(0, 0, 960, 1040)));
        // away from edges → nothing
        assert_eq!(zone(960, 540, b, work, 20), None);
    }

    #[test]
    fn map_proportional_recenters_across_monitors() {
        let src = Rect::new(0, 0, 1000, 1000);
        let dst = Rect::new(2000, 0, 4000, 1000); // 2000×1000, offset right
        let cur = Rect::new(0, 0, 500, 500); // top-left quarter of src
        let got = map_proportional(cur, src, dst);
        assert_eq!(got.left, 2000);
        assert_eq!(got.top, 0);
        assert_eq!(got.w(), 1000); // half of dst width
        assert_eq!(got.h(), 500);
    }
}

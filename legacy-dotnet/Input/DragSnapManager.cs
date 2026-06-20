using System.Drawing;
using WinRect.Config;
using WinRect.Core;
using WinRect.Interop;
using WinRect.UI;
using static WinRect.Interop.NativeMethods;

namespace WinRect.Input;

/// <summary>
/// Drag-a-window-to-a-screen-edge snapping (Rectangle / Aero style). Uses the OS move/size
/// events (not a global mouse hook) to know when a drag starts and ends; a lightweight timer
/// tracks the cursor in between to drive the live preview overlay. Snaps on release.
/// </summary>
public sealed class DragSnapManager : IDisposable
{
    private readonly WindowManager _wm;
    private readonly AppSettings _settings;
    private readonly WinEventHook _hook = new();
    private readonly SnapOverlay _overlay = new();
    private readonly System.Windows.Forms.Timer _timer;

    private IntPtr _dragHwnd;
    private Rectangle? _zone;
    private bool _enabled;

    public DragSnapManager(WindowManager wm, AppSettings settings)
    {
        _wm = wm;
        _settings = settings;
        _timer = new System.Windows.Forms.Timer { Interval = 16 };
        _timer.Tick += (_, _) => UpdatePreview();
        _hook.Event += OnWinEvent;
    }

    public void SetEnabled(bool enabled)
    {
        if (enabled == _enabled) return;
        _enabled = enabled;
        if (enabled)
        {
            _hook.Start(EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND);
        }
        else
        {
            _hook.Stop();
            EndDrag(apply: false);
        }
    }

    private void OnWinEvent(uint eventType, IntPtr hwnd)
    {
        if (eventType == EVENT_SYSTEM_MOVESIZESTART) BeginDrag(hwnd);
        else if (eventType == EVENT_SYSTEM_MOVESIZEEND) EndDrag(apply: true);
    }

    private void BeginDrag(IntPtr hwnd)
    {
        if (!WindowManager.IsManageable(hwnd)) { _dragHwnd = IntPtr.Zero; return; }
        _dragHwnd = hwnd;
        _zone = null;
        _timer.Start();
    }

    private void UpdatePreview()
    {
        if (_dragHwnd == IntPtr.Zero) return;
        if (!GetCursorPos(out var pt)) return;

        var zone = ComputeZone(pt);
        if (zone == _zone) return;
        _zone = zone;

        if (zone is { } z && _settings.ShowSnapPreview) _overlay.ShowAt(z);
        else _overlay.HideOverlay();
    }

    private void EndDrag(bool apply)
    {
        _timer.Stop();
        _overlay.HideOverlay();
        if (apply && _dragHwnd != IntPtr.Zero && _zone is { } z)
            _wm.ApplyVisibleRect(_dragHwnd, z);
        _dragHwnd = IntPtr.Zero;
        _zone = null;
    }

    /// <summary>Maps a cursor position near a screen edge/corner to the snap target (work-area rect).</summary>
    private Rectangle? ComputeZone(POINT pt)
    {
        var mon = Monitors.FromPoint(pt);
        var b = mon.Bounds;       // trigger when the cursor reaches the physical screen edge
        var wa = mon.WorkArea;    // but snap within the usable work area

        int edge = Math.Max(2, _settings.SnapEdgeThresholdPx);
        int cornerW = Math.Clamp((int)(wa.Width * 0.25), 80, 400);
        int cornerH = Math.Clamp((int)(wa.Height * 0.25), 80, 400);

        bool left = pt.X <= b.Left + edge;
        bool right = pt.X >= b.Right - edge - 1;
        bool top = pt.Y <= b.Top + edge;
        bool bottom = pt.Y >= b.Bottom - edge - 1;

        int hw = wa.Width / 2, hh = wa.Height / 2;
        Rectangle LeftHalf = new(wa.Left, wa.Top, hw, wa.Height);
        Rectangle RightHalf = new(wa.Right - hw, wa.Top, hw, wa.Height);
        Rectangle BottomHalf = new(wa.Left, wa.Bottom - hh, wa.Width, hh);
        Rectangle TL = new(wa.Left, wa.Top, hw, hh);
        Rectangle TR = new(wa.Right - hw, wa.Top, hw, hh);
        Rectangle BL = new(wa.Left, wa.Bottom - hh, hw, hh);
        Rectangle BR = new(wa.Right - hw, wa.Bottom - hh, hw, hh);

        // Exact corners first.
        if (left && top) return TL;
        if (right && top) return TR;
        if (left && bottom) return BL;
        if (right && bottom) return BR;

        if (top)
        {
            if (pt.X <= wa.Left + cornerW) return TL;
            if (pt.X >= wa.Right - cornerW) return TR;
            return wa; // top center → maximize (fill work area)
        }
        if (bottom)
        {
            if (pt.X <= wa.Left + cornerW) return BL;
            if (pt.X >= wa.Right - cornerW) return BR;
            return BottomHalf;
        }
        if (left)
        {
            if (pt.Y <= wa.Top + cornerH) return TL;
            if (pt.Y >= wa.Bottom - cornerH) return BL;
            return LeftHalf;
        }
        if (right)
        {
            if (pt.Y <= wa.Top + cornerH) return TR;
            if (pt.Y >= wa.Bottom - cornerH) return BR;
            return RightHalf;
        }

        return null;
    }

    public void Dispose()
    {
        _timer.Dispose();
        _hook.Dispose();
        _overlay.Dispose();
    }
}

using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;
using WinRect.Config;
using WinRect.Interop;
using static WinRect.Interop.NativeMethods;

namespace WinRect.Core;

/// <summary>
/// Executes window-management actions against the foreground window: computes the target
/// rectangle (with Rectangle-style size cycling), remembers pre-snap geometry for Restore,
/// and applies it flush to the screen using DWM invisible-border compensation.
/// </summary>
public sealed class WindowManager
{
    private readonly AppSettings _settings;

    // Per-window memory.
    private readonly Dictionary<IntPtr, Rectangle> _restore = new();      // pre-snap geometry
    private readonly Dictionary<IntPtr, Rectangle> _lastApplied = new();  // last rect WE set

    // Cycle state: repeated presses of the same cycling action on the same window advance.
    private IntPtr _cycleHwnd;
    private WindowAction _cycleAction;
    private int _cycleIndex;

    private static readonly double[] HalfWidthCycle = { 0.5, 2.0 / 3.0, 1.0 / 3.0 };

    public WindowManager(AppSettings settings) => _settings = settings;

    /// <summary>Run an action against the current foreground window.</summary>
    public void Execute(WindowAction action) => ExecuteOn(action, GetForegroundWindow());

    /// <summary>Run an action against a specific window (used by click-to-apply from the UI).</summary>
    public void ExecuteOn(WindowAction action, IntPtr hwnd)
    {
        if (!IsManageable(hwnd))
        {
            Log.Info($"{action}: target window is not manageable");
            return;
        }

        PrunePerhaps();

        var mon = Monitors.FromWindow(hwnd);
        var wa = mon.WorkArea;

        if (action == WindowAction.Restore)
        {
            DoRestore(hwnd);
            ResetCycle();
            return;
        }

        // Remember where the window was before this snap, so Restore can bring it back.
        CaptureRestoreRect(hwnd);

        Rectangle target;
        switch (action)
        {
            case WindowAction.LeftHalf:
            {
                double f = HalfWidthCycle[AdvanceCycle(action, hwnd, HalfWidthCycle.Length)];
                int w = R(wa.Width * f);
                target = new Rectangle(wa.Left, wa.Top, w, wa.Height);
                break;
            }
            case WindowAction.RightHalf:
            {
                double f = HalfWidthCycle[AdvanceCycle(action, hwnd, HalfWidthCycle.Length)];
                int w = R(wa.Width * f);
                target = new Rectangle(wa.Right - w, wa.Top, w, wa.Height);
                break;
            }
            case WindowAction.TopHalf:
                ResetCycle();
                target = new Rectangle(wa.Left, wa.Top, wa.Width, R(wa.Height * 0.5));
                break;
            case WindowAction.BottomHalf:
            {
                ResetCycle();
                int h = R(wa.Height * 0.5);
                target = new Rectangle(wa.Left, wa.Bottom - h, wa.Width, h);
                break;
            }
            case WindowAction.TopLeftQuarter:
                ResetCycle();
                target = new Rectangle(wa.Left, wa.Top, R(wa.Width * 0.5), R(wa.Height * 0.5));
                break;
            case WindowAction.TopRightQuarter:
            {
                ResetCycle();
                int w = R(wa.Width * 0.5);
                target = new Rectangle(wa.Right - w, wa.Top, w, R(wa.Height * 0.5));
                break;
            }
            case WindowAction.BottomLeftQuarter:
            {
                ResetCycle();
                int h = R(wa.Height * 0.5);
                target = new Rectangle(wa.Left, wa.Bottom - h, R(wa.Width * 0.5), h);
                break;
            }
            case WindowAction.BottomRightQuarter:
            {
                ResetCycle();
                int w = R(wa.Width * 0.5);
                int h = R(wa.Height * 0.5);
                target = new Rectangle(wa.Right - w, wa.Bottom - h, w, h);
                break;
            }
            case WindowAction.FirstThird:
            {
                int pos = AdvanceCycle(action, hwnd, 3);           // 0→1→2 (first→center→last)
                target = ThirdAt(wa, pos);
                break;
            }
            case WindowAction.LastThird:
            {
                int pos = 2 - AdvanceCycle(action, hwnd, 3);       // 2→1→0 (last→center→first)
                target = ThirdAt(wa, pos);
                break;
            }
            case WindowAction.CenterThird:
                ResetCycle();
                target = ThirdAt(wa, 1);
                break;
            case WindowAction.FirstTwoThirds:
            {
                ResetCycle();
                var (x0, _, x2, _) = Thirds(wa);
                target = new Rectangle(x0, wa.Top, x2 - x0, wa.Height);
                break;
            }
            case WindowAction.LastTwoThirds:
            {
                ResetCycle();
                var (_, x1, _, x3) = Thirds(wa);
                target = new Rectangle(x1, wa.Top, x3 - x1, wa.Height);
                break;
            }
            case WindowAction.Maximize:
                ResetCycle();
                target = wa;   // fill work area as a normal window (Rectangle behavior)
                break;
            case WindowAction.AlmostMaximize:
            {
                ResetCycle();
                int w = R(wa.Width * 0.9), h = R(wa.Height * 0.9);
                target = new Rectangle(wa.Left + (wa.Width - w) / 2, wa.Top + (wa.Height - h) / 2, w, h);
                break;
            }
            case WindowAction.MaximizeHeight:
            {
                ResetCycle();
                var cur = GetVisibleRect(hwnd);
                int x = Math.Clamp(cur.Left, wa.Left, Math.Max(wa.Left, wa.Right - cur.Width));
                target = new Rectangle(x, wa.Top, Math.Min(cur.Width, wa.Width), wa.Height);
                break;
            }
            case WindowAction.Center:
            {
                ResetCycle();
                var cur = GetVisibleRect(hwnd);
                int w = Math.Min(cur.Width, wa.Width), h = Math.Min(cur.Height, wa.Height);
                target = new Rectangle(wa.Left + (wa.Width - w) / 2, wa.Top + (wa.Height - h) / 2, w, h);
                break;
            }
            case WindowAction.MakeLarger:
            {
                ResetCycle();
                int step = _settings.ResizeStepPx;
                var cur = GetVisibleRect(hwnd);
                target = Grow(cur, step, wa);
                break;
            }
            case WindowAction.MakeSmaller:
            {
                ResetCycle();
                int step = _settings.ResizeStepPx;
                var cur = GetVisibleRect(hwnd);
                target = Shrink(cur, step, wa);
                break;
            }
            case WindowAction.NextDisplay:
            case WindowAction.PreviousDisplay:
                ResetCycle();
                MoveToDisplay(hwnd, mon, action == WindowAction.NextDisplay ? +1 : -1);
                return;
            default:
                return;
        }

        target = ApplyGap(action, target);
        ApplyVisibleRect(hwnd, target);
    }

    // ---- Multi-monitor moves -------------------------------------------------

    private void MoveToDisplay(IntPtr hwnd, MonitorInfo from, int step)
    {
        var to = Monitors.Relative(from, step);
        if (to.Handle == from.Handle) return; // single display

        var cur = GetVisibleRect(hwnd);
        var src = from.WorkArea;
        var dst = to.WorkArea;

        // Preserve the window's relative position & size proportionally across displays
        // (handles different resolutions / DPI gracefully).
        double rx = (cur.Left - src.Left) / (double)src.Width;
        double ry = (cur.Top - src.Top) / (double)src.Height;
        double rw = Math.Min(1.0, cur.Width / (double)src.Width);
        double rh = Math.Min(1.0, cur.Height / (double)src.Height);

        int w = R(dst.Width * rw), h = R(dst.Height * rh);
        int x = dst.Left + R(dst.Width * rx);
        int y = dst.Top + R(dst.Height * ry);

        var target = ClampInto(new Rectangle(x, y, w, h), dst);
        ApplyVisibleRect(hwnd, target);
    }

    // ---- Core apply ----------------------------------------------------------

    /// <summary>Positions the window so its *visible* frame exactly occupies <paramref name="target"/>.</summary>
    public void ApplyVisibleRect(IntPtr hwnd, Rectangle target)
    {
        if (IsZoomed(hwnd))
            ShowWindow(hwnd, SW_RESTORE);

        // Compute invisible-border insets: GetWindowRect includes the DWM resize border,
        // the extended frame bounds give the true visible frame.
        int insetL = 0, insetT = 0, insetR = 0, insetB = 0;
        if (GetWindowRect(hwnd, out RECT outer) &&
            DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, out RECT frame, Marshal.SizeOf<RECT>()) == 0)
        {
            insetL = frame.Left - outer.Left;
            insetT = frame.Top - outer.Top;
            insetR = outer.Right - frame.Right;
            insetB = outer.Bottom - frame.Bottom;
        }

        int x = target.Left - insetL;
        int y = target.Top - insetT;
        int w = target.Width + insetL + insetR;
        int h = target.Height + insetT + insetB;

        bool ok = SetWindowPos(hwnd, HWND_TOP, x, y, w, h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER);

        if (ok) _lastApplied[hwnd] = target;
        else Log.Warn($"SetWindowPos failed (win32 {Marshal.GetLastWin32Error()}) for {hwnd}");
    }

    // ---- Restore memory ------------------------------------------------------

    private void CaptureRestoreRect(IntPtr hwnd)
    {
        var cur = GetVisibleRect(hwnd);
        bool windowIsFree = !_lastApplied.TryGetValue(hwnd, out var last) || !NearlyEqual(cur, last);
        if (!_restore.ContainsKey(hwnd) || windowIsFree)
            _restore[hwnd] = cur;
    }

    private void DoRestore(IntPtr hwnd)
    {
        if (_restore.TryGetValue(hwnd, out var r))
            ApplyVisibleRect(hwnd, r);
        else
            Log.Info("Restore: no remembered geometry for this window");
    }

    // ---- Geometry helpers ----------------------------------------------------

    private static (int x0, int x1, int x2, int x3) Thirds(Rectangle wa)
    {
        int x0 = wa.Left;
        int x1 = wa.Left + (int)Math.Round(wa.Width / 3.0);
        int x2 = wa.Left + (int)Math.Round(wa.Width * 2.0 / 3.0);
        int x3 = wa.Right;
        return (x0, x1, x2, x3);
    }

    private static Rectangle ThirdAt(Rectangle wa, int pos)
    {
        var (x0, x1, x2, x3) = Thirds(wa);
        return pos switch
        {
            0 => new Rectangle(x0, wa.Top, x1 - x0, wa.Height),
            1 => new Rectangle(x1, wa.Top, x2 - x1, wa.Height),
            _ => new Rectangle(x2, wa.Top, x3 - x2, wa.Height),
        };
    }

    private static Rectangle Grow(Rectangle cur, int step, Rectangle wa)
    {
        int w = Math.Min(cur.Width + step, wa.Width);
        int h = Math.Min(cur.Height + step, wa.Height);
        int x = cur.X - (w - cur.Width) / 2;
        int y = cur.Y - (h - cur.Height) / 2;
        return ClampInto(new Rectangle(x, y, w, h), wa);
    }

    private static Rectangle Shrink(Rectangle cur, int step, Rectangle wa)
    {
        int minW = Math.Max(R(wa.Width * 0.25), 200);
        int minH = Math.Max(R(wa.Height * 0.25), 150);
        int w = Math.Max(cur.Width - step, minW);
        int h = Math.Max(cur.Height - step, minH);
        int x = cur.X + (cur.Width - w) / 2;
        int y = cur.Y + (cur.Height - h) / 2;
        return ClampInto(new Rectangle(x, y, w, h), wa);
    }

    /// <summary>Keeps a rectangle within a monitor's work area (shifting, then clamping size).</summary>
    private static Rectangle ClampInto(Rectangle r, Rectangle wa)
    {
        int w = Math.Min(r.Width, wa.Width);
        int h = Math.Min(r.Height, wa.Height);
        int x = Math.Clamp(r.X, wa.Left, wa.Right - w);
        int y = Math.Clamp(r.Y, wa.Top, wa.Bottom - h);
        return new Rectangle(x, y, w, h);
    }

    private Rectangle ApplyGap(WindowAction action, Rectangle target)
    {
        int gap = _settings.GapPx;
        if (gap <= 0 || !IsTiling(action)) return target;
        var r = Rectangle.Inflate(target, -gap, -gap);
        return r.Width > 0 && r.Height > 0 ? r : target;
    }

    private static bool IsTiling(WindowAction a) => a switch
    {
        WindowAction.Center or WindowAction.MakeLarger or WindowAction.MakeSmaller
            or WindowAction.NextDisplay or WindowAction.PreviousDisplay or WindowAction.Restore => false,
        _ => true,
    };

    // ---- Cycle state ---------------------------------------------------------

    private int AdvanceCycle(WindowAction action, IntPtr hwnd, int length)
    {
        if (hwnd == _cycleHwnd && action == _cycleAction)
            _cycleIndex = (_cycleIndex + 1) % length;
        else
            _cycleIndex = 0;
        _cycleHwnd = hwnd;
        _cycleAction = action;
        return _cycleIndex;
    }

    private void ResetCycle()
    {
        _cycleHwnd = IntPtr.Zero;
        _cycleIndex = 0;
    }

    // ---- Window inspection ---------------------------------------------------

    public static Rectangle GetVisibleRect(IntPtr hwnd)
    {
        if (DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, out RECT dwm, Marshal.SizeOf<RECT>()) == 0)
            return Monitors.ToRectangle(dwm);
        GetWindowRect(hwnd, out RECT r);
        return Monitors.ToRectangle(r);
    }

    public static bool IsManageable(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !IsWindow(hwnd) || !IsWindowVisible(hwnd)) return false;
        if (IsCloaked(hwnd)) return false;

        long style = GetWindowStyle(hwnd, GWL_STYLE);
        if ((style & WS_MINIMIZE) != 0) return false;
        // Must be a normal, sizeable/captioned top-level window (skip pure tool/popup chrome).
        if ((style & (WS_CAPTION | WS_THICKFRAME)) == 0) return false;

        if (hwnd == GetShellWindow()) return false;
        var cls = ClassName(hwnd);
        if (cls is "Progman" or "WorkerW" or "Shell_TrayWnd" or "Windows.UI.Core.CoreWindow") return false;

        return true;
    }

    private static bool IsCloaked(IntPtr hwnd)
    {
        if (DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, out int v, sizeof(int)) == 0)
            return v != 0;
        return false;
    }

    private static string ClassName(IntPtr hwnd)
    {
        var sb = new StringBuilder(256);
        int n = GetClassName(hwnd, sb, sb.Capacity);
        return n > 0 ? sb.ToString() : "";
    }

    // ---- misc ----------------------------------------------------------------

    private static int R(double v) => (int)Math.Round(v);

    private static bool NearlyEqual(Rectangle a, Rectangle b, int tol = 4) =>
        Math.Abs(a.X - b.X) <= tol && Math.Abs(a.Y - b.Y) <= tol &&
        Math.Abs(a.Width - b.Width) <= tol && Math.Abs(a.Height - b.Height) <= tol;

    private void PrunePerhaps()
    {
        if (_restore.Count + _lastApplied.Count < 128) return;
        foreach (var h in _restore.Keys.Where(k => !IsWindow(k)).ToList()) _restore.Remove(h);
        foreach (var h in _lastApplied.Keys.Where(k => !IsWindow(k)).ToList()) _lastApplied.Remove(h);
    }
}

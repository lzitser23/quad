using System.Drawing;
using System.Runtime.InteropServices;
using WinRect.Interop;
using static WinRect.Interop.NativeMethods;

namespace WinRect.Core;

/// <summary>A display, with its full bounds and its work area (taskbar excluded).</summary>
public sealed record MonitorInfo(IntPtr Handle, Rectangle Bounds, Rectangle WorkArea, bool IsPrimary, string DeviceName);

/// <summary>Monitor enumeration and left-to-right ordering for multi-display moves.</summary>
public static class Monitors
{
    public static Rectangle ToRectangle(RECT r) => Rectangle.FromLTRB(r.Left, r.Top, r.Right, r.Bottom);

    /// <summary>All monitors, ordered left-to-right then top-to-bottom (stable order for next/prev).</summary>
    public static List<MonitorInfo> All()
    {
        var list = new List<MonitorInfo>();
        EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (IntPtr hMon, IntPtr hdc, ref RECT rc, IntPtr data) =>
        {
            var mi = Query(hMon);
            if (mi is not null) list.Add(mi);
            return true; // continue enumeration
        }, IntPtr.Zero);

        return list
            .OrderBy(m => m.Bounds.Left)
            .ThenBy(m => m.Bounds.Top)
            .ToList();
    }

    public static MonitorInfo? Query(IntPtr hMonitor)
    {
        var info = new MONITORINFOEX { cbSize = Marshal.SizeOf<MONITORINFOEX>() };
        if (!GetMonitorInfo(hMonitor, ref info)) return null;
        bool primary = (info.dwFlags & MONITORINFOEX.MONITORINFOF_PRIMARY) != 0;
        return new MonitorInfo(hMonitor, ToRectangle(info.rcMonitor), ToRectangle(info.rcWork), primary, info.szDevice);
    }

    public static MonitorInfo FromWindow(IntPtr hWnd)
    {
        var h = MonitorFromWindow(hWnd, MONITOR_DEFAULTTONEAREST);
        return Query(h) ?? Primary();
    }

    public static MonitorInfo FromPoint(POINT pt)
    {
        var h = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        return Query(h) ?? Primary();
    }

    public static MonitorInfo Primary()
    {
        var all = All();
        return all.FirstOrDefault(m => m.IsPrimary) ?? all.First();
    }

    /// <summary>The monitor <paramref name="step"/> places away in the ordered ring (wraps around).</summary>
    public static MonitorInfo Relative(MonitorInfo from, int step)
    {
        var all = All();
        if (all.Count <= 1) return from;
        int idx = all.FindIndex(m => m.Handle == from.Handle);
        if (idx < 0) idx = 0;
        int next = ((idx + step) % all.Count + all.Count) % all.Count;
        return all[next];
    }
}

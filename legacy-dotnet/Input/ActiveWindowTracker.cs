using WinRect.Core;
using WinRect.Interop;

namespace WinRect.Input;

/// <summary>
/// Remembers the most recently focused *manageable* window that is not one of WinRect's own
/// windows. The WinEvent hook uses WINEVENT_SKIPOWNPROCESS, so opening WinRect's UI does not
/// overwrite this — letting "click-to-apply" act on the window the user was actually using.
/// </summary>
public sealed class ActiveWindowTracker : IDisposable
{
    private readonly WinEventHook _hook = new();

    public IntPtr LastActive { get; private set; }

    public ActiveWindowTracker()
    {
        var fg = NativeMethods.GetForegroundWindow();
        if (WindowManager.IsManageable(fg)) LastActive = fg;

        _hook.Event += OnForeground;
        _hook.Start(NativeMethods.EVENT_SYSTEM_FOREGROUND, NativeMethods.EVENT_SYSTEM_FOREGROUND);
    }

    private void OnForeground(uint eventType, IntPtr hwnd)
    {
        if (WindowManager.IsManageable(hwnd)) LastActive = hwnd;
    }

    public void Dispose() => _hook.Dispose();
}

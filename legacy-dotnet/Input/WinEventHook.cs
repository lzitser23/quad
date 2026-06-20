using WinRect.Interop;

namespace WinRect.Input;

/// <summary>
/// Thin wrapper over SetWinEventHook. Out-of-context hooks are delivered on the thread that
/// installed them (our UI thread, which pumps messages), so handlers may touch UI state safely.
/// </summary>
public sealed class WinEventHook : IDisposable
{
    private NativeMethods.WinEventProc? _proc; // keep the delegate alive for the hook's lifetime
    private IntPtr _hook;

    /// <summary>(eventType, hwnd) for top-level window events only.</summary>
    public event Action<uint, IntPtr>? Event;

    public void Start(uint eventMin, uint eventMax)
    {
        if (_hook != IntPtr.Zero) return;
        _proc = OnEvent;
        _hook = NativeMethods.SetWinEventHook(eventMin, eventMax, IntPtr.Zero, _proc, 0, 0,
            NativeMethods.WINEVENT_OUTOFCONTEXT | NativeMethods.WINEVENT_SKIPOWNPROCESS);
        if (_hook == IntPtr.Zero) Log.Warn($"SetWinEventHook failed for {eventMin:X}-{eventMax:X}");
    }

    public void Stop()
    {
        if (_hook != IntPtr.Zero)
        {
            NativeMethods.UnhookWinEvent(_hook);
            _hook = IntPtr.Zero;
        }
        _proc = null;
    }

    private void OnEvent(IntPtr hHook, uint eventType, IntPtr hwnd, int idObject, int idChild,
        uint dwEventThread, uint dwmsEventTime)
    {
        if (idObject != NativeMethods.OBJID_WINDOW || hwnd == IntPtr.Zero) return;
        Event?.Invoke(eventType, hwnd);
    }

    public void Dispose() => Stop();
}
